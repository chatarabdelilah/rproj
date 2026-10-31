//! Private Machine Setup worker. Stop is cooperative at item boundaries.
use crate::commands::machine_setup::{checked, worker::Event};
use anyhow::{Context, Result, ensure};
use rproj_core::machine_setup::{
    Outcome, Persistence, Selection, SetupContext, SetupResult, SetupReview, Status,
    protocol::{self, *},
};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

trait Backend: Sync {
    fn context(&self) -> Result<SetupContext>;
    fn prepare(&self, context: &SetupContext, selection: &Selection) -> Result<SetupReview>;
    fn apply(
        &self,
        review: &SetupReview,
        stop: &(dyn Fn() -> bool + Sync),
        emit: Arc<dyn Fn(Event) + Send + Sync>,
        mutation: &mut dyn FnMut() -> Result<()>,
    ) -> Result<SetupResult>;
}
struct Machine;
impl Backend for Machine {
    fn context(&self) -> Result<SetupContext> {
        checked::capture()
    }
    fn prepare(&self, context: &SetupContext, selection: &Selection) -> Result<SetupReview> {
        checked::prepare_ready(context, selection)
    }
    fn apply(
        &self,
        review: &SetupReview,
        stop: &(dyn Fn() -> bool + Sync),
        emit: Arc<dyn Fn(Event) + Send + Sync>,
        mutation: &mut dyn FnMut() -> Result<()>,
    ) -> Result<SetupResult> {
        checked::execute(review, stop, emit, mutation)
    }
}

pub(super) fn job(directory: &Path) -> Result<()> {
    job_with(directory, &Machine, std::io::stdin().lock())
}

struct Progress {
    report: Mutex<SetupReport>,
    path: PathBuf,
    stop: AtomicBool,
}
impl Progress {
    fn update(&self, change: impl FnOnce(&mut SetupReport)) -> Result<()> {
        let mut report = self
            .report
            .lock()
            .map_err(|_| anyhow::anyhow!("Setup progress lock failed"))?;
        if report.status.is_some() {
            return Ok(());
        }
        change(&mut report);
        report.sequence = report
            .sequence
            .checked_add(1)
            .context("Setup progress sequence exhausted")?;
        write_atomic(&self.path, &*report)
    }
    fn event(&self, event: Event) {
        let result = match event {
            Event::Started(index) => self.update(|report| {
                report.active = Some(index);
                if let Some(status) = report.statuses.get_mut(index) {
                    *status = Status::Running;
                }
            }),
            Event::Finished(index, status) => self.update(|report| {
                report.active = None;
                if let Some(item) = report.statuses.get_mut(index) {
                    *item = status;
                }
            }),
            Event::Message(_, message) => {
                // Owner disconnection must not abort the active installer.
                let _ = writeln!(std::io::stdout(), "{message}");
                Ok(())
            }
            Event::Done(_) | Event::Completed(_) => Ok(()),
        };
        if let Err(error) = result {
            // Finish the current item even when the progress channel fails.
            self.stop.store(true, Ordering::Release);
            let _ = writeln!(
                std::io::stderr(),
                "Setup progress could not be recorded; stopping after this item: {error:#}"
            );
        }
    }
    fn await_start(&self, startup: &mut impl Read) -> Result<()> {
        let mut signal = [0u8];
        startup
            .read_exact(&mut signal)
            .context("Machine Setup owner disconnected before authorizing the first item")?;
        match signal[0] {
            protocol::START_SETUP => Ok(()),
            protocol::STOP_BEFORE_SETUP => {
                self.stop.store(true, Ordering::Release);
                self.update(|report| report.stop_acknowledged = true)
            }
            _ => anyhow::bail!("Invalid Machine Setup startup decision"),
        }
    }

    fn observe_stop(&self, directory: &Path, request: &SetupRequest) {
        if self.stop.load(Ordering::Acquire) {
            return;
        }
        if let Ok(control) = read_json::<StopAfterItem>(&directory.join("stop.json"))
            && control.check(request).is_ok()
        {
            self.stop.store(true, Ordering::Release);
            if let Err(error) = self.update(|report| report.stop_acknowledged = true) {
                let _ = writeln!(
                    std::io::stderr(),
                    "Could not acknowledge Stop; the active item will still finish: {error:#}"
                );
            }
        }
    }
}

fn job_with(directory: &Path, backend: &impl Backend, mut startup: impl Read) -> Result<()> {
    let request: SetupRequest = read_json(&directory.join("request.json"))?;
    request.validate()?;
    let progress = Arc::new(Progress {
        report: Mutex::new(SetupReport::initial(&request)),
        path: directory.join("report.json"),
        stop: AtomicBool::new(false),
    });
    progress.update(|_| {})?;
    let result = match &request.action {
        SetupAction::Context => backend
            .context()
            .and_then(|loaded| progress.update(|r| r.loaded = Some(loaded))),
        SetupAction::Prepare { context, selection } => backend
            .prepare(context, selection)
            .and_then(|prepared| progress.update(|r| r.prepared = Some(prepared))),
        SetupAction::Apply(review) => progress.await_start(&mut startup).and_then(|_| {
            progress.observe_stop(directory, &request);
            let finished = AtomicBool::new(false);
            std::thread::scope(|scope| {
                let monitor = scope.spawn(|| {
                    while !finished.load(Ordering::Acquire) {
                        progress.observe_stop(directory, &request);
                        std::thread::sleep(Duration::from_millis(25));
                    }
                });
                let events = progress.clone();
                let result = backend.apply(
                    review,
                    &|| progress.stop.load(Ordering::Acquire) || crate::interrupt::requested(),
                    Arc::new(move |event| events.event(event)),
                    &mut || progress.update(|r| r.effects = WriteState::MayHaveBegun),
                );
                finished.store(true, Ordering::Release);
                monitor
                    .join()
                    .map_err(|_| anyhow::anyhow!("Setup Stop monitor failed"))?;
                result.and_then(|result| {
                    progress.update(|r| {
                        r.active = None;
                        r.statuses.clone_from(&result.statuses);
                        r.result = Some(result);
                    })
                })
            })
        }),
    };
    let mut report = progress
        .report
        .lock()
        .map_err(|_| anyhow::anyhow!("Setup progress lock failed"))?;
    report.active = None;
    report.sequence += 1;
    match result {
        Ok(()) => {
            report.status = Some(match report.result.as_ref() {
                Some(result) => match (&result.outcome, &result.persistence) {
                    (Outcome::Completed | Outcome::Warnings, Persistence::Saved { .. }) => {
                        SetupStatus::Succeeded
                    }
                    (Outcome::Cancelled, Persistence::NotSaved) => SetupStatus::Stopped,
                    _ => SetupStatus::Failed,
                },
                None => SetupStatus::Succeeded,
            });
            if matches!(request.action, SetupAction::Apply(_))
                && report.status == Some(SetupStatus::Succeeded)
            {
                report.effects = WriteState::Completed;
            }
        }
        Err(error) => {
            report.status = Some(if report.effects == WriteState::NotStarted {
                SetupStatus::Failed
            } else {
                SetupStatus::Uncertain
            });
            report.loaded = None;
            report.prepared = None;
            report.result = None;
            report.message = Some(format!("{error:#}"));
        }
    }
    report.check(&request)?;
    write_atomic(&progress.path, &*report)?;
    ensure!(
        report.status == Some(SetupStatus::Succeeded),
        "{}",
        report
            .message
            .as_deref()
            .unwrap_or("Machine Setup did not complete; inspect its item and persistence results")
    );
    Ok(())
}

#[cfg(all(test, windows))]
mod tests;
