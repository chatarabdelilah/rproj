use super::*;
use rproj_core::project_discovery::{Discovery, discover};
use std::path::Path;

#[cfg(test)]
mod tests;
mod view;

pub(super) struct Scan {
    root: Option<PathBuf>,
    found: Discovery,
}
type Inspection = Result<(ProjectSnapshot, String), String>;

/// Each lane owns at most one filesystem task. Changes queue only the latest
/// request; leaving the browser invalidates results without touching workspace operations.
#[derive(Default)]
struct Lane {
    revision: u64,
    active: Option<u64>,
}
impl Lane {
    fn invalidate(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }
    fn start(&mut self) -> Option<u64> {
        if self.active.is_some() {
            return None;
        }
        self.active = Some(self.revision);
        self.active
    }
    fn finish(&mut self, id: u64) -> Option<bool> {
        if self.active != Some(id) {
            return None;
        }
        self.active = None;
        Some(self.revision == id)
    }
}

pub(super) struct Browser {
    pub(super) visible: bool,
    launch: Result<PathBuf, String>,
    scan: Lane,
    preview: Lane,
    opening: Lane,
    listing: Option<Scan>,
    indices: Vec<usize>,
    selected: Option<PathBuf>,
    detail: Option<Result<ProjectSnapshot, String>>,
    notice: Option<String>,
    query: Entity<InputState>,
    scroll: UniformListScrollHandle,
    focus: FocusHandle,
    rows_focus: FocusHandle,
    pub(super) empty_focus: FocusHandle,
    return_to_empty: bool,
    _input_subscription: Subscription,
}
impl Browser {
    pub(super) fn new(window: &mut Window, cx: &mut Context<Workbench>) -> Self {
        let query =
            cx.new(|cx| InputState::new(window, cx).placeholder("Search project names or paths"));
        let subscription = cx.subscribe(&query, |this, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) && this.browser.visible {
                this.filter_projects(cx);
            }
        });
        Self {
            visible: false,
            launch: std::env::current_dir()
                .map_err(|e| format!("Launch directory unavailable: {e}")),
            scan: Lane::default(),
            preview: Lane::default(),
            opening: Lane::default(),
            listing: None,
            indices: Vec::new(),
            selected: None,
            detail: None,
            notice: None,
            query,
            scroll: UniformListScrollHandle::new(),
            focus: cx.focus_handle(),
            rows_focus: cx.focus_handle().tab_stop(true),
            empty_focus: cx.focus_handle().tab_stop(true),
            return_to_empty: false,
            _input_subscription: subscription,
        }
    }

    pub(super) fn leave(&mut self) {
        if self.visible {
            self.visible = false;
            self.scan.invalidate();
            self.preview.invalidate();
            self.opening.invalidate();
        }
    }
    fn index(&mut self, query: &str) {
        self.indices.clear();
        let Some(scan) = &self.listing else { return };
        let query = query.trim().to_lowercase();
        self.indices.extend(
            scan.found
                .entries
                .iter()
                .enumerate()
                .filter(|(_, p)| {
                    query.is_empty()
                        || p.name.to_lowercase().contains(&query)
                        || p.path.to_string_lossy().to_lowercase().contains(&query)
                })
                .map(|(i, _)| i),
        );
    }
    fn at(&self, visible_index: usize) -> Option<&rproj_core::project_discovery::ProjectEntry> {
        self.listing
            .as_ref()?
            .found
            .entries
            .get(*self.indices.get(visible_index)?)
    }
    fn contains(&self, path: &Path) -> bool {
        self.indices.iter().any(|&i| {
            self.listing
                .as_ref()
                .is_some_and(|s| s.found.entries[i].path == path)
        })
    }
}

/// A preview and an explicit Open both inspect the selected canonical directory.
/// Open always calls this again; it never promotes a cached preview.
fn inspect_target(path: &Path) -> Result<ProjectSnapshot, String> {
    let read = || -> anyhow::Result<_> {
        anyhow::ensure!(
            path.canonicalize()? == path,
            "Project folder was redirected"
        );
        let snapshot = rproj_core::inspect(path)?;
        anyhow::ensure!(
            snapshot.root == path && path.canonicalize()? == path,
            "Project folder changed during inspection"
        );
        anyhow::ensure!(
            snapshot.files.rproj.state != rproj_core::FileState::Missing
                || snapshot.files.rojo.state != rproj_core::FileState::Missing,
            "Project is no longer recognized"
        );
        Ok(snapshot)
    };
    read().map_err(|e| {
        format!(
            "Cannot open {}: {e:#}. Refresh the projects list.",
            path.display()
        )
    })
}

impl Workbench {
    pub(super) fn show_projects(
        &mut self,
        empty: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.browser.visible {
            self.browser.return_to_empty = empty;
        }
        self.browser.visible = true;
        let owner = cx.entity().downgrade();
        window.on_next_frame(move |w, cx| {
            let _ = owner.update(cx, |this, cx| {
                if this.browser.visible {
                    this.browser.query.read(cx).focus_handle(cx).focus(w, cx);
                }
            });
        });
        self.refresh_projects(cx);
    }
    fn back_from_projects(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.browser.leave();
        // The project switcher is the first tab stop in the shell. The empty
        // entry has its own stable handle because its view was unmounted.
        let empty = self.browser.return_to_empty;
        let focus = if empty {
            self.browser.empty_focus.clone()
        } else {
            self.focus.clone()
        };
        w.on_next_frame(move |w, cx| {
            focus.focus(w, cx);
            if !empty {
                w.focus_next(cx);
            }
        });
        cx.notify();
    }
    pub(super) fn refresh_projects(&mut self, cx: &mut Context<Self>) {
        self.browser.opening.invalidate();
        self.browser.scan.invalidate();
        self.spawn_projects_scan();
        cx.notify();
    }
    fn spawn_projects_scan(&mut self) {
        let Some(id) = self.browser.scan.start() else {
            return;
        };
        let launch = self.browser.launch.clone();
        let client = self.client.clone();
        let reason = self.client_error.clone();
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let root = match client {
                Some(client) => client
                    .projects_root(launch.as_deref().ok())
                    .map_err(|e| format!("{e:#}")),
                None => Err(reason.unwrap_or_else(|| {
                    "Matching CLI unavailable; configured folder could not be read.".into()
                })),
            };
            let mut found = discover(&root, launch.as_deref().ok());
            if let Err(warning) = launch {
                found.warnings.push(warning);
            }
            let _ = sender.send(Event::BrowserScan(
                id,
                Scan {
                    root: root.ok(),
                    found,
                },
            ));
        });
    }
    pub(super) fn accept_projects_scan(&mut self, id: u64, scan: Scan, cx: &mut Context<Self>) {
        let Some(current) = self.browser.scan.finish(id) else {
            return;
        };
        if !self.browser.visible {
            return;
        }
        if !current {
            self.spawn_projects_scan();
            return;
        }
        let first = self.browser.listing.is_none();
        self.browser.listing = Some(scan);
        let query = self.browser.query.read(cx).value();
        self.browser.index(&query);
        let previous = self.browser.selected.clone();
        let selected = previous
            .clone()
            .filter(|p| self.browser.contains(p))
            .or_else(|| {
                if first {
                    self.browser.at(0).map(|p| p.path.clone())
                } else {
                    None
                }
            });
        self.browser.notice = if previous.is_some() && selected.is_none() {
            Some("The selected project is no longer listed. Choose another preview.".into())
        } else {
            None
        };
        if self.browser.selected == selected {
            self.browser.preview.invalidate();
            self.spawn_project_preview();
            cx.notify();
        } else {
            self.select_project_preview(selected, cx);
        }
    }
    fn filter_projects(&mut self, cx: &mut Context<Self>) {
        let query = self.browser.query.read(cx).value();
        self.browser.index(&query);
        let selected = self
            .browser
            .selected
            .clone()
            .filter(|p| self.browser.contains(p))
            .or_else(|| self.browser.at(0).map(|p| p.path.clone()));
        self.browser.notice = None;
        self.select_project_preview(selected, cx);
    }
    fn select_project_preview(&mut self, path: Option<PathBuf>, cx: &mut Context<Self>) {
        self.browser.preview.invalidate();
        self.browser.opening.invalidate();
        self.browser.selected = path;
        self.browser.detail = None;
        self.spawn_project_preview();
        cx.notify();
    }
    fn spawn_project_preview(&mut self) {
        let Some(path) = self.browser.selected.clone() else {
            return;
        };
        let Some(id) = self.browser.preview.start() else {
            return;
        };
        let tx = self.sender.clone();
        std::thread::spawn(move || {
            let result = inspect_target(&path);
            let _ = tx.send(Event::BrowserPreview(id, path, Box::new(result)));
        });
    }
    pub(super) fn accept_project_preview(
        &mut self,
        id: u64,
        path: PathBuf,
        result: Result<ProjectSnapshot, String>,
        cx: &mut Context<Self>,
    ) {
        let Some(current) = self.browser.preview.finish(id) else {
            return;
        };
        if !self.browser.visible {
            return;
        }
        if current && self.browser.selected.as_ref() == Some(&path) {
            self.browser.detail = Some(result);
        } else {
            self.spawn_project_preview();
        }
        cx.notify();
    }
    fn open_previewed_project(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.browser.selected.clone() else {
            return;
        };
        if !self.browser.visible
            || !matches!(&self.browser.detail,Some(Ok(snapshot)) if snapshot.root==path)
        {
            return;
        }
        let Some(id) = self.browser.opening.start() else {
            return;
        };
        let tx = self.sender.clone();
        std::thread::spawn(move || {
            let result = inspect_target(&path).and_then(|snapshot| {
                let fingerprint = serde_json::to_string(&snapshot).map_err(|e| e.to_string())?;
                Ok((snapshot, fingerprint))
            });
            let _ = tx.send(Event::BrowserOpen(id, path, Box::new(result)));
        });
        cx.notify();
    }
    pub(super) fn accept_browser_open(
        &mut self,
        id: u64,
        path: PathBuf,
        result: Inspection,
        cx: &mut Context<Self>,
    ) {
        if self.browser.opening.finish(id) != Some(true)
            || !self.browser.visible
            || self.browser.selected.as_ref() != Some(&path)
        {
            return;
        }
        match result {
            Ok((snapshot, fingerprint)) if snapshot.root == path => {
                self.enter_project(path.clone());
                self.section = Section::Overview;
                self.inspection_fingerprint = fingerprint;
                self.snapshot = Some(snapshot);
                self.remember(path);
            }
            Err(e) => {
                self.browser.detail = Some(Err(e));
            }
            _ => {
                self.browser.detail = Some(Err(
                    "Project folder changed. Refresh the projects list.".into(),
                ));
            }
        }
        cx.notify();
    }
}
