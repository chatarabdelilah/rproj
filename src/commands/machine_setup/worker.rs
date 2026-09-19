use std::process::Command;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
    mpsc::{self, Receiver, SyncSender},
};
use std::thread::JoinHandle;

use crate::catalog::tool_catalog::{self, ToolKind};
use crate::steps::execution::{MessageKind, Reporter};
use crate::steps::{blender, bootstrap, studio_plugin, toolchain, vscode};

use super::model::{Category, Selection};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Item {
    Rokit,
    App(String),
    ProjectsFolder,
    Tool(String),
    Plugin(String),
    Extension(String),
}

impl Item {
    pub fn label(&self) -> &str {
        match self {
            Self::Rokit => "Rokit bootstrap",
            Self::ProjectsFolder => "Projects folder",
            Self::App(key) | Self::Tool(key) | Self::Plugin(key) | Self::Extension(key) => key,
        }
    }
    fn fatal(&self) -> bool {
        matches!(self, Self::Rokit | Self::ProjectsFolder)
    }
}

pub fn plan(selection: &Selection) -> Vec<Item> {
    let mut items = vec![Item::Rokit];
    for entry in tool_catalog::SYSTEM_APPS
        .iter()
        .filter(|entry| selection.apps.contains(entry.key))
    {
        items.push(Item::App(entry.key.into()));
    }
    items.push(Item::ProjectsFolder);
    for entry in tool_catalog::ROKIT_TOOLS
        .iter()
        .filter(|entry| selection.tools.contains(entry.key))
    {
        items.push(Item::Tool(entry.key.into()));
    }
    for category in [Category::Plugins, Category::Blender] {
        if selection.active(category) {
            for entry in tool_catalog::PLUGINS
                .iter()
                .filter(|entry| category.contains(entry) && selection.plugins.contains(entry.key))
            {
                items.push(Item::Plugin(entry.key.into()));
            }
        }
    }
    if selection.active(Category::Extensions) {
        for entry in tool_catalog::VSCODE_EXTENSIONS
            .iter()
            .filter(|entry| selection.extensions.contains(entry.key))
        {
            items.push(Item::Extension(entry.key.into()));
        }
    }
    items
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Pending,
    Running,
    Already,
    Installed,
    Manual,
    Failed,
    Skipped,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Completed,
    Warnings,
    Cancelled,
    Failed,
}

pub enum Event {
    Started(usize),
    Message(MessageKind, String),
    Finished(usize, Status),
    Done(Outcome),
}

pub struct Worker {
    pub receiver: Option<Receiver<Event>>,
    pub stop: Arc<AtomicBool>,
    pub dropped: Arc<AtomicUsize>,
    handle: Option<JoinHandle<()>>,
}

impl Worker {
    pub fn start(selection: Selection) -> Self {
        Self::spawn(selection, execute)
    }

    pub(super) fn spawn(
        selection: Selection,
        execute: impl Fn(&Item, &Selection, &Reporter) -> anyhow::Result<()> + Send + 'static,
    ) -> Self {
        let (sender, receiver) = mpsc::sync_channel(128);
        let stop = Arc::new(AtomicBool::new(false));
        let dropped = Arc::new(AtomicUsize::new(0));
        let cancellation = stop.clone();
        let lost = dropped.clone();
        let handle = std::thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                run(&selection, &cancellation, &sender, &lost, execute)
            }));
            if result.is_err() {
                let _ = sender.send(Event::Message(
                    MessageKind::Warning,
                    "Setup worker failed unexpectedly; completed installations remain in place."
                        .into(),
                ));
                let _ = sender.send(Event::Done(Outcome::Failed));
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

fn run(
    selection: &Selection,
    stop: &AtomicBool,
    sender: &SyncSender<Event>,
    dropped: &Arc<AtomicUsize>,
    execute: impl Fn(&Item, &Selection, &Reporter) -> anyhow::Result<()>,
) {
    let mut warnings = false;
    for (index, item) in plan(selection).iter().enumerate() {
        if stop.load(Ordering::SeqCst) {
            let _ = sender.send(Event::Done(Outcome::Cancelled));
            return;
        }
        if sender.send(Event::Started(index)).is_err() {
            return;
        }
        let status = Arc::new(std::sync::Mutex::new(Status::Installed));
        let observed = status.clone();
        let events = sender.clone();
        let dropped = dropped.clone();
        let reporter = Reporter(Arc::new(move |kind, text| {
            let next = match kind {
                MessageKind::Already => Some(Status::Already),
                MessageKind::Manual => Some(Status::Manual),
                MessageKind::Warning => Some(Status::Failed),
                _ => None,
            };
            if let Some(next) = next {
                *observed.lock().unwrap_or_else(|error| error.into_inner()) = next;
            }
            let event = Event::Message(kind, text.into());
            if kind == MessageKind::Output {
                if events.try_send(event).is_err() {
                    dropped.fetch_add(1, Ordering::Relaxed);
                }
            } else {
                let _ = events.send(event);
            }
        }));
        let result = execute(item, selection, &reporter);
        if let Err(error) = &result {
            let _ = sender.send(Event::Message(
                MessageKind::Warning,
                format!("{}: {error:#}", item.label()),
            ));
            *status.lock().unwrap_or_else(|error| error.into_inner()) = Status::Failed;
        }
        let status = *status.lock().unwrap_or_else(|error| error.into_inner());
        warnings |= matches!(status, Status::Failed | Status::Manual | Status::Skipped);
        let _ = sender.send(Event::Finished(index, status));
        if result.is_err() && item.fatal() {
            let _ = sender.send(Event::Done(Outcome::Failed));
            return;
        }
    }
    let outcome = if stop.load(Ordering::SeqCst) {
        Outcome::Cancelled
    } else if warnings {
        Outcome::Warnings
    } else {
        Outcome::Completed
    };
    let _ = sender.send(Event::Done(outcome));
}

fn execute(item: &Item, selection: &Selection, reporter: &Reporter) -> anyhow::Result<()> {
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
        selection.extensions.insert("luau-lsp".into());
        selection.plugins.insert("blender-plugin".into());
        assert_eq!(plan(&selection), vec![Item::Rokit, Item::ProjectsFolder]);
    }

    #[test]
    fn fixture_worker_finishes_without_machine_operations() {
        let worker = Worker::spawn(selection(), |_, _, _| Ok(()));
        loop {
            if let Event::Done(outcome) = worker.receiver.as_ref().unwrap().recv().unwrap() {
                assert_eq!(outcome, Outcome::Completed);
                break;
            }
        }
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
                Event::Done(outcome) => {
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
                Event::Done(outcome) => {
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
            if let Event::Done(outcome) = worker.receiver.as_ref().unwrap().recv().unwrap() {
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
            if let Event::Done(outcome) = worker.receiver.as_ref().unwrap().recv().unwrap() {
                assert_eq!(outcome, Outcome::Cancelled);
                break;
            }
        }
    }
}
