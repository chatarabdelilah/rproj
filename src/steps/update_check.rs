//! Tells the user when a newer rproj has been published.
//!
//! Three rules, because a version check is the kind of feature that is
//! easy to make annoying:
//!
//! 1. **It never blocks and never fails the run.** Every error path is an
//!    early return. A version check that breaks `rproj new` because a
//!    network was down would be worse than no check at all.
//! 2. **It asks at most once a day.** The answer is cached with a
//!    timestamp, so the common case is reading one small file.
//! 3. **It is silent when there is nothing to say.** No "you are up to
//!    date" line - that is noise on every single run.

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::GlobalConfig;
use crate::ui;

const CURRENT: &str = env!("CARGO_PKG_VERSION");
const CHECK_EVERY_SECS: u64 = 60 * 60 * 24;

/// Prints a one-line nudge if a newer version is on crates.io.
///
/// Returns nothing and reports nothing: a caller cannot act on a failed
/// version check, so there is no `Result` to handle (the `probe` argument
/// from the process boundary, applied to a network call).
pub fn nudge_if_outdated() {
    if std::env::var_os("RPROJ_NO_UPDATE_CHECK").is_some() {
        return;
    }
    let Some(latest) = latest_version() else { return };
    if is_newer(&latest, CURRENT) {
        ui::detail(&format!(
            "rproj {latest} is available (you have {CURRENT}) - cargo install rproj --force"
        ));
    }
}

/// The cached answer if it is fresh, otherwise a fresh lookup.
fn latest_version() -> Option<String> {
    let path = cache_path()?;
    if let Some(cached) = read_fresh_cache(&path) {
        return Some(cached);
    }
    let latest = fetch_latest()?;
    // Written before use, so a failure to parse the response cannot cause
    // a request on every single run afterwards.
    let _ = write_cache(&path, &latest);
    Some(latest)
}

fn cache_path() -> Option<PathBuf> {
    Some(GlobalConfig::dirs().ok()?.cache_dir().join("latest-version"))
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// `<unix seconds>\n<version>`, or `None` if absent, malformed or stale.
fn read_fresh_cache(path: &PathBuf) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    let (stamp, version) = text.split_once('\n')?;
    let stamp: u64 = stamp.trim().parse().ok()?;
    // Saturating, so a clock that moved backwards reads as "stale" rather
    // than underflowing into a value that is never stale again.
    if now_secs().saturating_sub(stamp) > CHECK_EVERY_SECS {
        return None;
    }
    let version = version.trim();
    (!version.is_empty()).then(|| version.to_string())
}

fn write_cache(path: &PathBuf, version: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, format!("{}\n{version}\n", now_secs()))
}

/// The newest non-yanked version from the crates.io API.
///
/// A short timeout, because this runs before the user's actual command and
/// a stalled connection must not delay it.
fn fetch_latest() -> Option<String> {
    let body = ureq::get("https://crates.io/api/v1/crates/rproj/versions")
        // crates.io requires a User-Agent that identifies the caller and
        // rejects requests without one.
        .header("User-Agent", concat!("rproj/", env!("CARGO_PKG_VERSION")))
        .config()
        .timeout_global(Some(std::time::Duration::from_secs(3)))
        .build()
        .call()
        .ok()?
        .body_mut()
        .read_to_string()
        .ok()?;
    newest_from_json(&body)
}

/// Parsed here rather than inline so the response shape is testable
/// without the network (Chapter-61 reasoning: the parse is the part that
/// can be wrong).
fn newest_from_json(body: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    value
        .get("versions")?
        .as_array()?
        .iter()
        .filter(|v| v.get("yanked").and_then(|y| y.as_bool()) != Some(true))
        .filter_map(|v| v.get("num")?.as_str())
        .max_by(|a, b| compare(a, b))
        .map(str::to_string)
}

/// Numeric, component-wise comparison of two dotted versions.
///
/// String comparison is wrong here in a way that would matter within one
/// release: `"0.10.0" < "0.2.0"` lexically, so the nudge would stop firing
/// exactly when the minor version reached double digits. A pre-release
/// suffix (`1.0.0-rc.1`) sorts by its numeric prefix and is treated as
/// equal to the release, which is enough for a "there is something newer"
/// hint and avoids a full semver dependency.
fn compare(a: &str, b: &str) -> std::cmp::Ordering {
    fn parts(v: &str) -> Vec<u64> {
        v.split(['.', '-', '+'])
            .map_while(|p| p.parse::<u64>().ok())
            .collect()
    }
    parts(a).cmp(&parts(b))
}

fn is_newer(candidate: &str, current: &str) -> bool {
    compare(candidate, current) == std::cmp::Ordering::Greater
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bug string comparison would introduce, and the reason this
    /// function exists rather than `a > b`.
    #[test]
    fn versions_compare_numerically_not_lexically() {
        assert!(is_newer("0.10.0", "0.2.0"), "10 > 2, despite \"1\" < \"2\"");
        assert!(is_newer("0.2.1", "0.2.0"));
        assert!(is_newer("1.0.0", "0.99.99"));
        assert!(!is_newer("0.2.0", "0.2.0"));
        assert!(!is_newer("0.1.9", "0.2.0"));
    }

    #[test]
    fn a_prerelease_suffix_does_not_break_the_comparison() {
        // Sorts on the numeric prefix; equal to the release, not greater.
        assert!(!is_newer("0.2.0-rc.1", "0.2.0"));
        assert!(is_newer("0.3.0-rc.1", "0.2.0"));
    }

    #[test]
    fn the_newest_non_yanked_version_wins() {
        let body = r#"{"versions":[
            {"num":"0.1.0","yanked":false},
            {"num":"0.10.0","yanked":false},
            {"num":"0.2.0","yanked":false}
        ]}"#;
        assert_eq!(newest_from_json(body).as_deref(), Some("0.10.0"));
    }

    /// A yanked release is not something to nudge anyone towards.
    #[test]
    fn a_yanked_version_is_ignored() {
        let body = r#"{"versions":[
            {"num":"0.2.0","yanked":false},
            {"num":"0.3.0","yanked":true}
        ]}"#;
        assert_eq!(newest_from_json(body).as_deref(), Some("0.2.0"));
    }

    /// Every malformed shape yields `None` rather than panicking, because
    /// this runs before the user's actual command.
    #[test]
    fn a_malformed_response_is_none() {
        for body in ["", "not json", "{}", r#"{"versions":[]}"#, r#"{"versions":{}}"#] {
            assert_eq!(newest_from_json(body), None, "{body}");
        }
    }
}
