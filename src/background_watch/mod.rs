#[cfg(not(windows))]
use std::path::Path;

#[cfg(not(windows))]
use anyhow::Result;

#[cfg(windows)]
mod windows;
#[cfg(all(windows, test))]
pub(crate) use windows::with_unresponsive_fixture;
#[cfg(windows)]
pub(crate) use windows::{
    internal_entry, mutation_guard, show_logs, show_status, start_in, stop, summary,
};

#[cfg(not(windows))]
pub(crate) struct MutationGuard;

#[cfg(not(windows))]
pub(crate) fn mutation_guard(_: Option<&Path>, _: bool) -> Result<MutationGuard> {
    Ok(MutationGuard)
}
#[cfg(not(windows))]
pub(crate) fn internal_entry() -> Option<Result<()>> {
    None
}
#[cfg(not(windows))]
pub(crate) fn summary() -> String {
    "Background Watch is available on Windows.".into()
}
#[cfg(not(windows))]
pub(crate) fn start_in(_: &Path) -> Result<()> {
    unsupported()
}
#[cfg(not(windows))]
pub(crate) fn stop() -> Result<()> {
    unsupported()
}
#[cfg(not(windows))]
pub(crate) fn show_logs() -> Result<()> {
    unsupported()
}
#[cfg(not(windows))]
pub(crate) fn show_status() -> Result<()> {
    unsupported()
}
#[cfg(not(windows))]
fn unsupported() -> Result<()> {
    anyhow::bail!(
        "Background Watch requires Windows; `rproj watch` remains available in the foreground."
    )
}
