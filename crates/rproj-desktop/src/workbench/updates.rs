use super::*;
use rproj_core::updates::{Comparison, Entry, Local, Report};
use std::collections::BTreeMap;

mod apply;
#[cfg(test)]
mod tests;
mod view;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Group {
    #[default]
    All,
    Packages,
    Tools,
}
impl Group {
    fn includes(self, local: &Local) -> bool {
        self == Self::All || (self == Self::Packages) == local.is_package()
    }
}

fn key(local: &Local) -> String {
    match local {
        Local::Package { alias, realm, .. } => format!("{realm}:{alias}"),
        Local::Tool { alias, .. } => format!("tool:{alias}"),
    }
}
struct Observation {
    entry: Entry,
    time: u64,
}
#[derive(Default)]
pub(super) struct Project {
    fresh_check: bool,
    entries: Vec<Local>,
    loaded: bool,
    group: Group,
    selected: Option<String>,
    pending: Option<(u64, Vec<Local>)>,
    observations: BTreeMap<String, Observation>,
    failures: BTreeMap<String, (Local, String)>,
    summary: String,
    last_success: Option<u64>,
}
impl Project {
    pub(super) fn inventory(&mut self, entries: Vec<Local>) {
        self.loaded = true;
        self.entries = entries;
        self.filter(self.group);
    }
    fn filter(&mut self, group: Group) {
        self.group = group;
        let mut visible = self.entries.iter().filter(|e| group.includes(e));
        if !visible
            .clone()
            .any(|e| self.selected.as_ref() == Some(&key(e)))
        {
            self.selected = visible.next().map(key);
        }
    }
    fn complete(&mut self, id: u64, result: Result<Report, String>) -> Option<(String, bool)> {
        if self.pending.as_ref()?.0 != id {
            return None;
        }
        let (_, captured) = self.pending.take()?;
        self.fresh_check = false;
        let outcome = match result {
            Err(error) => (format!("Check failed · {error}"), true),
            Ok(report) if !captured.iter().eq(report.entries.iter().map(|e| &e.local)) => (
                "Project inputs changed before collection. Check again.".into(),
                true,
            ),
            Ok(report) => {
                self.fresh_check = true;
                let total = report.entries.len();
                let mut observed = 0;
                self.failures.clear();
                for entry in report.entries {
                    if entry.observed {
                        observed += 1;
                        self.observations.insert(
                            key(&entry.local),
                            Observation {
                                entry,
                                time: report.checked_at_unix_seconds,
                            },
                        );
                    } else {
                        let reason = entry
                            .comparison
                            .reason
                            .unwrap_or_else(|| "Release source unavailable".into());
                        self.failures
                            .insert(key(&entry.local), (entry.local, reason));
                    }
                }
                let problem = report
                    .diagnostics
                    .iter()
                    .find(|d| d.severity == rproj_core::DiagnosticSeverity::Error);
                if observed > 0 || (total == 0 && problem.is_none()) {
                    self.last_success = Some(report.checked_at_unix_seconds);
                }
                if let Some(issue) = problem {
                    (
                        format!("Checked {observed} of {total} entries · {}", issue.message),
                        true,
                    )
                } else if observed < total {
                    (
                        format!("{} of {total} entries unavailable", total - observed),
                        true,
                    )
                } else {
                    (format!("Checked {total} entries"), false)
                }
            }
        };
        self.summary = outcome.0.clone();
        Some(outcome)
    }
    fn current(&self, local: &Local) -> (Option<&Observation>, Option<&str>, bool) {
        let key = key(local);
        let observation = self.observations.get(&key);
        let failure = self.failures.get(&key);
        let stale = observation.is_some_and(|o| o.entry.local != *local)
            || failure.is_some_and(|(e, _)| e != local);
        (
            observation.filter(|o| o.entry.local == *local),
            failure
                .filter(|(e, _)| e == local)
                .map(|(_, msg)| msg.as_str()),
            stale,
        )
    }
}
#[derive(Default)]
pub(super) struct Session {
    pub(super) apply: apply::ReviewSession,
    pub(super) projects: BTreeMap<PathBuf, Project>,
    sequence: u64,
    pub(super) probe: u64,
    pub(super) support: Option<Result<(), String>>,
}
impl Workbench {
    pub(super) fn probe_updates(&self) {
        let Some(client) = self.client.clone() else {
            return;
        };
        let id = self.updates.probe;
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let _ = sender.send(Event::UpdatesSupport(
                id,
                client.updates_support().map_err(|e| format!("{e:#}")),
            ));
            let _ = sender.send(Event::ReviewedUpdateSupport(
                id,
                client
                    .reviewed_update_support()
                    .map_err(|e| format!("{e:#}")),
            ));
        });
    }
    fn start_updates(&mut self, cx: &mut Context<Self>) {
        let (Some(path), Some(client), Some(Ok(()))) = (
            self.project.clone(),
            self.client.clone(),
            &self.updates.support,
        ) else {
            return;
        };
        let Some(state) = self.updates.projects.get_mut(&path) else {
            return;
        };
        if !state.loaded || state.pending.is_some() || self.snapshot.is_none() {
            return;
        }
        if self
            .updates
            .apply
            .active
            .as_ref()
            .is_some_and(|(_, s)| s.project == path)
        {
            return;
        }
        if self
            .updates
            .apply
            .plan
            .as_ref()
            .is_some_and(|p| p.selection.project == path)
        {
            self.updates.apply.plan = None;
        }
        self.updates.sequence += 1;
        let id = self.updates.sequence;
        state.pending = Some((id, state.entries.clone()));
        self.updates_notice(&path, "Checking packages and tools", true);
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let report = client.check_updates(&path).map_err(|e| format!("{e:#}"));
            let _ = sender.send(Event::UpdatesComplete(id, path, Box::new(report)));
        });
        cx.notify();
    }
    pub(super) fn finish_updates(
        &mut self,
        id: u64,
        path: PathBuf,
        result: Result<Report, String>,
    ) {
        if let Some((summary, failed)) = self
            .updates
            .projects
            .get_mut(&path)
            .and_then(|p| p.complete(id, result))
        {
            self.updates_notice(&path, &summary, failed);
        }
    }
    fn updates_notice(&mut self, path: &std::path::Path, text: &str, expand: bool) {
        let origin = format!("{} · Updates", path.display());
        let summary = format!(
            "{} · Updates · {text}",
            path.file_name().unwrap_or_default().to_string_lossy()
        );
        self.read_activity(origin, summary, text, expand);
    }
}
