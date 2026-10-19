use super::*;
use rproj_core::upgrade::{UpgradeReview, diff::Row, protocol::*};

mod controller;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod ui_tests;
mod view;

pub(super) struct PreparedReview {
    pub(super) review: UpgradeReview,
    pub(super) documents: Vec<Arc<Vec<Row>>>,
}
pub(super) enum Preparation {
    Prepared(Box<PreparedReview>),
    Failed(String),
    Cancelled,
}
impl Preparation {
    fn from_worker(result: UpgradePreparation) -> Self {
        match result {
            UpgradePreparation::Prepared(review) => {
                if let Err(error) = review.validate() {
                    return Self::Failed(format!("Invalid captured Upgrade review: {error:#}"));
                }
                let Some(documents) = (0..review.files.len())
                    .map(|i| review.diff(i).map(Arc::new))
                    .collect::<Option<Vec<_>>>()
                else {
                    return Self::Failed(
                        "Could not build every proposed file diff; explicitly Refresh review"
                            .into(),
                    );
                };
                Self::Prepared(Box::new(PreparedReview {
                    review: *review,
                    documents,
                }))
            }
            UpgradePreparation::Failed(error) => Self::Failed(error),
            UpgradePreparation::Cancelled => Self::Cancelled,
        }
    }
}

#[derive(Default)]
pub(super) struct UpgradeSession {
    pub(super) visible: bool,
    pub(super) project: Option<PathBuf>,
    pub(super) project_name: Option<String>,
    pub(super) prepared: Option<PreparedReview>,
    pub(super) revision: u64,
    pub(super) selected: usize,
    pub(super) loading: bool,
    pub(super) error: Option<String>,
    pub(super) support: Option<Result<(), String>>,
    pub(super) result: Option<Result<UpgradeOutcome, String>>,
    pub(super) unwrapped: bool,
    pub(super) dialog_open: bool,
    pub(super) close_pending: bool,
    pub(super) teardown_uncertain: bool,
}
impl UpgradeSession {
    pub(super) fn begin(&mut self, project: PathBuf) {
        self.revision = self.revision.wrapping_add(1);
        self.project = Some(project);
        self.project_name = None;
        self.visible = true;
        self.loading = true;
        self.prepared = None;
        self.result = None;
        self.error = None;
        self.selected = 0;
    }
    pub(super) fn ready(&self) -> Option<&UpgradeReview> {
        if self.loading || self.result.is_some() || self.error.is_some() || self.teardown_uncertain
        {
            return None;
        }
        self.prepared
            .as_ref()
            .map(|p| &p.review)
            .filter(|p| Some(&p.project) == self.project.as_ref())
    }
    pub(super) fn watch_conflict(&self, watch: Option<&WatchStatus>) -> bool {
        watch.is_some_and(|watch| {
            self.project.is_some()
                && watch.project == self.project
                && !matches!(
                    watch.state,
                    rproj_core::operations::WatchState::Stopped
                        | rproj_core::operations::WatchState::Failed
                )
        })
    }
    pub(super) fn row_count(&self) -> usize {
        self.prepared
            .as_ref()
            .and_then(|p| p.documents.get(self.selected))
            .map_or(0, |r| r.len())
    }
    pub(super) fn name(&self) -> String {
        if let Some(name) = &self.project_name {
            return name.clone();
        }
        self.project
            .as_ref()
            .and_then(|p| p.file_name())
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    }
}
