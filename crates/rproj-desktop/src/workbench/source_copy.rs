use super::*;
use std::sync::atomic::AtomicBool;

mod controller;
mod display;
#[cfg(test)]
mod tests;
mod view;

pub(super) use display::Prepared;

struct Request {
    id: u64,
    project: PathBuf,
    abandoned: Arc<AtomicBool>,
}

pub(super) struct Session {
    pub(super) visible: bool,
    owner: Option<PathBuf>,
    revision: u64,
    snapshot: Option<Arc<Prepared>>,
    active: Option<Request>,
    queued: Option<Request>,
    status: String,
    failed_refresh: bool,
    selected: usize,
    dialog_open: bool,
    focus: FocusHandle,
    files_focus: FocusHandle,
    skips_focus: FocusHandle,
    preview_focus: FocusHandle,
    files_scroll: UniformListScrollHandle,
    skips_scroll: UniformListScrollHandle,
    pub(super) rows: ListState,
    page_scroll: ScrollHandle,
    notice: Option<(Option<u64>, String)>,
}

impl Session {
    pub(super) fn new(cx: &mut App) -> Self {
        Self {
            visible: false,
            owner: None,
            revision: 0,
            snapshot: None,
            active: None,
            queued: None,
            status: String::new(),
            failed_refresh: false,
            selected: 0,
            dialog_open: false,
            focus: cx.focus_handle(),
            files_focus: cx.focus_handle().tab_stop(true),
            skips_focus: cx.focus_handle().tab_stop(true),
            preview_focus: cx.focus_handle().tab_stop(true),
            files_scroll: UniformListScrollHandle::new(),
            skips_scroll: UniformListScrollHandle::new(),
            rows: ListState::new(0, ListAlignment::Top, px(40.)),
            page_scroll: ScrollHandle::new(),
            notice: None,
        }
    }

    fn request(&mut self, project: PathBuf) {
        self.revision += 1;
        if self.owner.as_ref() != Some(&project) {
            self.snapshot = None;
            self.selected = 0;
            self.rows.reset(0);
        }
        self.owner = Some(project.clone());
        self.status = "Collecting a snapshot of src…".into();
        self.failed_refresh = false;
        if let Some(active) = &self.active {
            active.abandoned.store(true, Ordering::Relaxed);
        }
        self.queued = Some(Request {
            id: self.revision,
            project,
            abandoned: Arc::new(AtomicBool::new(false)),
        });
    }

    fn take_request(&mut self) -> Option<Request> {
        if self.active.is_some() {
            return None;
        }
        let request = self.queued.take()?;
        self.active = Some(Request {
            id: request.id,
            project: request.project.clone(),
            abandoned: request.abandoned.clone(),
        });
        Some(request)
    }

    fn accept(&mut self, id: u64, result: Result<Prepared, String>) -> Option<bool> {
        if self.active.as_ref().map(|r| r.id) != Some(id) {
            return None;
        }
        self.active = None;
        if id != self.revision {
            return None;
        }
        let failed = result.is_err();
        match result {
            Ok(prepared) => {
                self.owner = Some(prepared.raw.project.clone());
                self.status = prepared.summary();
                self.selected = 0;
                self.rows.reset(prepared.rows.len());
                self.rows.scroll_to(ListOffset::default());
                self.files_scroll.scroll_to_item(0, ScrollStrategy::Top);
                self.snapshot = Some(Arc::new(prepared));
                self.failed_refresh = false;
            }
            Err(error) => {
                self.failed_refresh = self.snapshot.is_some();
                self.status = if self.failed_refresh {
                    format!("Refresh failed. Previous snapshot retained: {error}")
                } else {
                    format!("Collection failed: {error}")
                };
            }
        }
        Some(failed)
    }

    fn busy(&self) -> bool {
        self.active.is_some() || self.queued.is_some()
    }
    fn copyable(&self) -> bool {
        !self.busy()
            && self
                .snapshot
                .as_ref()
                .is_some_and(|p| !p.raw.files.is_empty())
    }
    pub(super) fn row_count(&self) -> usize {
        self.snapshot.as_ref().map_or(0, |p| p.rows.len())
    }
    fn name(&self) -> String {
        self.owner
            .as_ref()
            .map(|p| project_name(p))
            .unwrap_or_default()
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if let Some(request) = &self.active {
            request.abandoned.store(true, Ordering::Relaxed);
        }
    }
}

fn project_name(path: &std::path::Path) -> String {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
pub(super) use tests::{audit_seed, audit_state};
