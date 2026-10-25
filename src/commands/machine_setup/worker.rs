use std::process::Command;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
    mpsc::{self, Receiver},
};
use std::thread::JoinHandle;

use crate::catalog::tool_catalog::{self, ToolKind};
use crate::steps::execution::{MessageKind, Reporter};
use crate::steps::{blender, bootstrap, studio_plugin, toolchain, vscode};

use super::model::Selection;
#[cfg(test)]
use super::model::SelectionConfig;

pub use rproj_core::machine_setup::{Item, Outcome, Status, plan};

pub enum Event {
    Started(usize),
    Message(MessageKind, String),
    Finished(usize, Status),
    Done(Outcome),
    Completed(rproj_core::machine_setup::SetupResult),
}

pub struct Worker {
    pub receiver: Option<Receiver<Event>>,
    pub stop: Arc<AtomicBool>,
    pub dropped: Arc<AtomicUsize>,
    handle: Option<JoinHandle<()>>,
}

impl Worker {
    pub fn start(context: rproj_core::machine_setup::SetupContext, selection: Selection) -> Self {
        Self::launch(move |stop, emit| {
            let review = super::checked::prepare(&context, &selection)?;
            super::checked::execute(
                &review,
                &|| stop.load(Ordering::SeqCst) || crate::interrupt::requested(),
                emit,
                &mut || Ok(()),
            )
        })
    }

    #[cfg(test)]
    pub(super) fn spawn(
        selection: Selection,
        execute: impl Fn(&Item, &Selection, &Reporter) -> anyhow::Result<()> + Send + 'static,
    ) -> Self {
        Self::launch(move |stop, emit| {
            Ok(run_items(
                &selection,
                &|| stop.load(Ordering::SeqCst),
                emit,
                execute,
            ))
        })
    }

    fn launch(
        run: impl FnOnce(
            &AtomicBool,
            Arc<dyn Fn(Event) + Send + Sync>,
        ) -> anyhow::Result<rproj_core::machine_setup::SetupResult>
        + Send
        + 'static,
    ) -> Self {
        let (sender, receiver) = mpsc::sync_channel(128);
        let stop = Arc::new(AtomicBool::new(false));
        let dropped = Arc::new(AtomicUsize::new(0));
        let cancellation = stop.clone();
        let lost = dropped.clone();
        let handle = std::thread::spawn(move || {
            let events = sender.clone();
            let emit: Arc<dyn Fn(Event) + Send + Sync> = Arc::new(move |event| {
                if matches!(event, Event::Message(MessageKind::Output, _)) {
                    if events.try_send(event).is_err() {
                        lost.fetch_add(1, Ordering::Relaxed);
                    }
                } else {
                    let _ = events.send(event);
                }
            });
            let result =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(&cancellation, emit)));
            match result {
                Ok(Ok(result)) => {
                    let _ = sender.send(Event::Completed(result));
                }
                other => {
                    let message = match other {
                        Ok(Err(error)) => format!("Setup failed: {error:#}"),
                        _ => "Setup worker failed unexpectedly; completed installations may remain in place.".into(),
                    };
                    let _ = sender.send(Event::Message(MessageKind::Warning, message));
                    let _ = sender.send(Event::Done(Outcome::Failed));
                }
            }
        });
        Self {
            receiver: Some(receiver),
            stop,
            dropped,
            handle: Some(handle),
        }
    }

    pub fn request_stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.request_stop();
        // Disconnect first so a worker cannot block on a full event queue during unwinding.
        self.receiver.take();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

pub(super) fn run_items(
    selection: &Selection,
    stop: &(dyn Fn() -> bool + Sync),
    emit: Arc<dyn Fn(Event) + Send + Sync>,
    execute: impl Fn(&Item, &Selection, &Reporter) -> anyhow::Result<()>,
) -> rproj_core::machine_setup::SetupResult {
    use rproj_core::machine_setup::{Persistence, SetupResult};
    let items = plan(selection);
    let mut result = SetupResult {
        outcome: Outcome::Completed,
        statuses: vec![Status::Skipped; items.len()],
        changes_may_have_begun: false,
        persistence: Persistence::NotSaved,
    };
    for (index, item) in items.iter().enumerate() {
        if stop() {
            result.outcome = Outcome::Cancelled;
            return result;
        }
        emit(Event::Started(index));
        let status = Arc::new(std::sync::Mutex::new(Status::Installed));
        let observed = status.clone();
        let events = emit.clone();
        let reporter = Reporter(Arc::new(move |kind, text| {
            let next = match kind {
                MessageKind::Already => Some(Status::Already),
                MessageKind::Manual => Some(Status::Manual),
                MessageKind::Warning => Some(Status::Failed),
                _ => None,
            };
            if let Some(next) = next {
                let mut current = observed.lock().unwrap_or_else(|error| error.into_inner());
                // A later informational message must not erase a warning/manual step.
                if *current != Status::Failed
                    && (*current != Status::Manual || next == Status::Failed)
                {
                    *current = next;
                }
            }
            events(Event::Message(kind, text.into()));
        }));
        result.changes_may_have_begun = true;
        let executed = execute(item, selection, &reporter);
        if let Err(error) = &executed {
            emit(Event::Message(
                MessageKind::Warning,
                format!("{}: {error:#}", item.label()),
            ));
            *status.lock().unwrap_or_else(|error| error.into_inner()) = Status::Failed;
        }
        let status = *status.lock().unwrap_or_else(|error| error.into_inner());
        result.statuses[index] = status;
        if matches!(status, Status::Failed | Status::Manual | Status::Skipped) {
            result.outcome = Outcome::Warnings;
        }
        emit(Event::Finished(index, status));
        if status == Status::Failed && item.fatal() {
            result.outcome = Outcome::Failed;
            return result;
        }
    }
    if stop() {
        result.outcome = Outcome::Cancelled;
    }
    result
}

pub(super) fn execute(
    item: &Item,
    selection: &Selection,
    reporter: &Reporter,
) -> anyhow::Result<()> {
    match item {
        Item::Rokit => bootstrap::ensure_rokit_with(Some(reporter)),
        Item::ProjectsFolder => {
            if selection.root.is_dir() {
                reporter.message(MessageKind::Already, "Projects folder already exists");
            } else {
                std::fs::create_dir_all(&selection.root)?;
            }
            Ok(())
        }
        Item::App(key) => {
            let entry = tool_catalog::find(key).expect("planned catalog entry");
            if bootstrap::is_installed(entry) {
                reporter.message(MessageKind::Already, "Application already installed");
                return Ok(());
            }
            if let ToolKind::SystemApp { winget_id, .. } = entry.kind {
                let result = bootstrap::install_winget(winget_id, Some(reporter));
                if result
                    .as_ref()
                    .is_err_and(|error| error.to_string().contains("installer hash"))
                {
                    reporter.message(MessageKind::Detail, bootstrap::WINGET_HASH_HELP);
                }
                return result;
            }
            unreachable!()
        }
        Item::Tool(key) => {
            let ToolKind::RokitTool { rokit_source } = tool_catalog::find(key).unwrap().kind else {
                unreachable!()
            };
            toolchain::add_global_tool_with(rokit_source, reporter)
        }
        Item::Extension(key) => {
            let ToolKind::VsCodeExtension { extension_id } = tool_catalog::find(key).unwrap().kind
            else {
                unreachable!()
            };
            vscode::ensure_extension_with(extension_id, reporter)
        }
        Item::Plugin(key) => match tool_catalog::find(key).unwrap().kind {
            ToolKind::StudioPlugin {
                github_repo,
                asset_suffix,
            } => studio_plugin::install_release_asset(
                key,
                github_repo,
                asset_suffix,
                key == "jest-roblox-plugin",
                Some(reporter),
            ),
            ToolKind::StudioPluginManual { install_url, .. } => {
                reporter.message(
                    MessageKind::Manual,
                    &format!("Install manually from {install_url}"),
                );
                Ok(())
            }
            ToolKind::StudioPluginViaCli { .. } => {
                if tool_catalog::find("studio").is_some_and(bootstrap::is_installed) {
                    reporter.run(Command::new("rojo").args(["plugin", "install"]))
                } else {
                    anyhow::bail!("Rojo plugin needs Roblox Studio; install Studio and retry")
                }
            }
            ToolKind::BlenderAddon { github_repo } => {
                let zip = blender::download_latest_plugin_zip(github_repo)?;
                blender::install_addon_with(&zip, Some(reporter))?;
                blender::account_link_instructions(reporter);
                Ok(())
            }
            _ => unreachable!(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn selection() -> Selection {
        Selection::load(&crate::config::GlobalConfig {
            roblox_projects_root: Some("fixture".into()),
            last_checked: Some("1".into()),
            ..Default::default()
        })
        .unwrap()
    }

    #[test]
    fn plan_ignores_unknown_and_inactive_entries() {
        let mut selection = selection();
        selection.tools.insert("future".into());
        selection.tools.insert("studio".into());
        selection.extensions.insert("luau-lsp".into());
        selection.plugins.insert("blender-plugin".into());
        assert_eq!(plan(&selection), vec![Item::Rokit, Item::ProjectsFolder]);
    }

    #[test]
    fn scribe_is_an_explicit_manual_plugin_step() {
        let mut selected = selection();
        let item = Item::Plugin("scribe-studio".into());
        assert!(!plan(&selected).contains(&item));
        selected.plugins.insert("scribe-studio".into());
        assert!(plan(&selected).contains(&item));
        let messages = Arc::new(std::sync::Mutex::new(Vec::new()));
        let captured = messages.clone();
        let reporter = Reporter(Arc::new(move |kind, text| {
            captured.lock().unwrap().push((kind, text.to_owned()));
        }));
        execute(&item, &selected, &reporter).unwrap();
        let messages = messages.lock().unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].0, MessageKind::Manual);
        assert!(messages[0].1.contains("113609038046646/Scribe-Studio"));
    }

    #[test]
    fn fixture_worker_finishes_without_machine_operations() {
        let worker = Worker::spawn(selection(), |_, _, _| Ok(()));
        loop {
            if let Event::Completed(result) = worker.receiver.as_ref().unwrap().recv().unwrap() {
                let outcome = result.outcome;
                assert_eq!(outcome, Outcome::Completed);
                break;
            }
        }
    }

    #[test]
    fn panicked_worker_reports_failure_without_starting_another_item() {
        let worker = Worker::spawn(selection(), |_, _, _| panic!("fixture worker panic"));
        let mut started = 0;
        loop {
            match worker.receiver.as_ref().unwrap().recv().unwrap() {
                Event::Started(_) => started += 1,
                Event::Done(outcome) => {
                    assert_eq!(outcome, Outcome::Failed);
                    break;
                }
                _ => {}
            }
        }
        assert_eq!(started, 1);
    }

    #[test]
    fn recoverable_failure_continues_but_fatal_failure_does_not() {
        let mut selected = selection();
        selected.apps.insert("studio".into());
        let worker = Worker::spawn(selected, |item, _, _| {
            if matches!(item, Item::App(_)) {
                anyhow::bail!("fixture installer failure");
            }
            Ok(())
        });
        let mut finished = Vec::new();
        loop {
            match worker.receiver.as_ref().unwrap().recv().unwrap() {
                Event::Finished(index, status) => finished.push((index, status)),
                Event::Completed(result) => {
                    let outcome = result.outcome;
                    assert_eq!(outcome, Outcome::Warnings);
                    break;
                }
                _ => {}
            }
        }
        assert_eq!(
            finished,
            vec![
                (0, Status::Installed),
                (1, Status::Failed),
                (2, Status::Installed)
            ]
        );
        let worker = Worker::spawn(selection(), |_, _, _| anyhow::bail!("bootstrap failed"));
        let mut starts = 0;
        loop {
            match worker.receiver.as_ref().unwrap().recv().unwrap() {
                Event::Started(_) => starts += 1,
                Event::Completed(result) => {
                    let outcome = result.outcome;
                    assert_eq!(outcome, Outcome::Failed);
                    break;
                }
                _ => {}
            }
        }
        assert_eq!(starts, 1);
    }

    #[test]
    fn saturated_output_does_not_drop_outcomes_or_deadlock() {
        let worker = Worker::spawn(selection(), |_, _, reporter| {
            for _ in 0..1000 {
                reporter.message(MessageKind::Output, "fixture output");
            }
            Ok(())
        });
        loop {
            if let Event::Completed(result) = worker.receiver.as_ref().unwrap().recv().unwrap() {
                let outcome = result.outcome;
                assert_eq!(outcome, Outcome::Completed);
                break;
            }
        }
    }

    #[test]
    fn cancellation_waits_for_active_item_and_skips_next() {
        let (entered, active) = mpsc::channel();
        let (release, wait) = mpsc::channel();
        let worker = Worker::spawn(selection(), move |item, _, _| {
            assert_eq!(*item, Item::Rokit);
            entered.send(()).unwrap();
            wait.recv().unwrap();
            Ok(())
        });
        active.recv().unwrap();
        worker.request_stop();
        release.send(()).unwrap();
        loop {
            if let Event::Completed(result) = worker.receiver.as_ref().unwrap().recv().unwrap() {
                let outcome = result.outcome;
                assert_eq!(outcome, Outcome::Cancelled);
                break;
            }
        }
    }
}
