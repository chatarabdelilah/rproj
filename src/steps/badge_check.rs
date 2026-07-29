//! Checks the catalog's hand-curated maintenance badges against what
//! upstream actually says.
//!
//! # Why this is a CI job and not a runtime feature
//!
//! Live badge checking at runtime is the obvious answer and it does not
//! survive contact with the numbers:
//!
//! * **Rate limit.** 50 catalog entries against GitHub's 60 requests/hour
//!   per *IP* for unauthenticated calls - a budget already shared with
//!   every other GitHub-touching tool on the machine, rokit included. One
//!   `rproj new` would spend it; a second run in the same hour would show
//!   no badges at all, or fail.
//! * **Latency.** 50 sequential HTTPS round trips ahead of the first
//!   prompt. Parallelising them means a thread pool or an async runtime,
//!   which this program deliberately does not have.
//! * **Offline.** A tool whose whole job is bootstrapping a machine has to
//!   work on a plane, behind a proxy, and on a locked-down network.
//!
//! And the deeper problem: **"last commit" is a poor proxy for
//!  "maintained".** A feature-complete library with no commits in
//! eighteen months (`t`, `promise`) is fine. A repository with daily
//! commits may be pre-alpha churn. The question a badge answers - *is this
//! a good choice for a new project* - is a judgement, and no timestamp
//! makes it.
//!
//! So the badge stays curated, and this checks the parts that *are*
//! objective:
//!
//! | signal | verdict |
//! | --- | --- |
//! | repository archived | **contradiction** - fatal |
//! | no push in 18 months, badge says Active | reported, not fatal |
//!
//! Same fatal/review split the book's own purge gate uses: fail on the
//! unambiguous, report the judgement calls. Run authenticated in CI, where
//! the limit is 5,000/hour and nobody is waiting.

use std::collections::BTreeMap;

use crate::catalog::Maintenance;
use crate::catalog::tool_catalog::{PLUGINS, ROKIT_TOOLS, SYSTEM_APPS, ToolKind, VSCODE_EXTENSIONS};
use crate::catalog::wally_packages::PACKAGES;

/// One catalogued thing that has a GitHub repository behind it.
#[derive(Debug, PartialEq, Eq)]
pub struct Tracked {
    pub key: &'static str,
    pub badge: Maintenance,
    /// `owner/repo`.
    pub repo: String,
}

/// Every catalog entry with a resolvable GitHub repository, deduplicated
/// by `owner/repo` so a monorepo backing four packages is one request.
///
/// Deduplication matters: `charm`, `charmSync`, `reactCharm` and
/// `videCharm` all live in one repository, so checking them separately
/// would spend four requests to learn one fact.
pub fn tracked_entries() -> Vec<Tracked> {
    let mut by_repo: BTreeMap<String, Tracked> = BTreeMap::new();

    for spec in PACKAGES {
        if let Some(repo) = github_slug(spec.git_repo) {
            by_repo.entry(repo.clone()).or_insert(Tracked {
                key: spec.key,
                badge: spec.maintenance,
                repo,
            });
        }
    }
    for entry in SYSTEM_APPS
        .iter()
        .chain(ROKIT_TOOLS)
        .chain(PLUGINS)
        .chain(VSCODE_EXTENSIONS)
    {
        // Only the kinds that name a repository. A winget id or a
        // marketplace extension id is not a GitHub slug, and guessing one
        // from a tool name would check the wrong project.
        let slug = match entry.kind {
            ToolKind::StudioPlugin { github_repo, .. } | ToolKind::BlenderAddon { github_repo } => {
                github_slug(github_repo)
            }
            _ => None,
        };
        if let Some(repo) = slug {
            by_repo.entry(repo.clone()).or_insert(Tracked {
                key: entry.key,
                badge: entry.maintenance,
                repo,
            });
        }
    }
    by_repo.into_values().collect()
}

/// `owner/repo` from a GitHub URL or an already-bare slug, or `None` if it
/// is neither.
///
/// Returns `None` rather than guessing for anything else, so a non-GitHub
/// host is skipped instead of producing a request for a repository that
/// does not exist.
pub fn github_slug(reference: &str) -> Option<String> {
    let rest = reference
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .strip_prefix("https://github.com/")
        .or_else(|| reference.strip_prefix("http://github.com/"))
        .or_else(|| reference.strip_prefix("git@github.com:"))
        .unwrap_or(reference);

    // A bare `owner/repo` and nothing more. Two segments, both non-empty,
    // neither looking like a URL scheme or a version pin.
    let mut parts = rest.split('/');
    let owner = parts.next()?;
    let repo = parts.next()?;
    if parts.next().is_some() || owner.is_empty() || repo.is_empty() || owner.contains(':') {
        return None;
    }
    Some(format!("{owner}/{}", repo.trim_end_matches(".git")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_github_url_reduces_to_owner_and_repo() {
        assert_eq!(
            github_slug("https://github.com/jsdotlua/react-lua").as_deref(),
            Some("jsdotlua/react-lua")
        );
        assert_eq!(
            github_slug("https://github.com/littensy/charm.git").as_deref(),
            Some("littensy/charm")
        );
        assert_eq!(
            github_slug("Roblox/roblox-blender-plugin").as_deref(),
            Some("Roblox/roblox-blender-plugin")
        );
    }

    /// Anything that is not a GitHub repository is skipped rather than
    /// guessed at - a wrong slug checks somebody else's project and
    /// reports its status as ours.
    #[test]
    fn a_non_github_reference_is_skipped() {
        assert_eq!(github_slug("https://wally.run/package/foo"), None);
        assert_eq!(github_slug("Git.Git"), None);
        assert_eq!(github_slug(""), None);
        assert_eq!(github_slug("owner"), None);
    }

    /// The monorepo case: four catalogued packages, one repository, one
    /// request. Checking them separately would spend four to learn one
    /// fact, against a 60/hour budget.
    #[test]
    fn entries_sharing_a_repository_are_deduplicated() {
        let tracked = tracked_entries();
        let mut repos: Vec<&str> = tracked.iter().map(|t| t.repo.as_str()).collect();
        let before = repos.len();
        repos.sort_unstable();
        repos.dedup();
        assert_eq!(before, repos.len(), "duplicate repositories: {repos:?}");
        assert!(before > 10, "expected most of the catalog to be tracked, got {before}");
    }

    /// **The freshness gate.** Queries every tracked repository and fails
    /// when a curated badge contradicts an objective fact.
    ///
    /// `#[ignore]`d because it needs the network and spends one request per
    /// repository. Run it in CI on a schedule, with `GITHUB_TOKEN` set:
    ///
    /// ```text
    /// cargo test --locked -- --ignored badges_do_not_contradict_upstream
    /// ```
    #[test]
    #[ignore = "queries the GitHub API once per catalogued repo; run in CI with GITHUB_TOKEN"]
    fn badges_do_not_contradict_upstream() {
        let tracked = tracked_entries();
        let token = std::env::var("GITHUB_TOKEN").ok();
        if token.is_none() {
            eprintln!(
                "note: no GITHUB_TOKEN - {} requests against a 60/hour \
                 unauthenticated limit may not complete",
                tracked.len()
            );
        }

        let mut archived = Vec::new();
        let mut stale = Vec::new();
        let mut unreachable = Vec::new();

        for entry in &tracked {
            let url = format!("https://api.github.com/repos/{}", entry.repo);
            let mut request = ureq::get(&url)
                .header("User-Agent", concat!("rproj/", env!("CARGO_PKG_VERSION")))
                .header("Accept", "application/vnd.github+json");
            if let Some(token) = &token {
                request = request.header("Authorization", &format!("Bearer {token}"));
            }

            let body = match request.call() {
                Ok(mut response) => response.body_mut().read_to_string().unwrap_or_default(),
                Err(err) => {
                    unreachable.push(format!("{} ({}): {err}", entry.key, entry.repo));
                    continue;
                }
            };
            let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) else {
                unreachable.push(format!("{} ({}): unparseable response", entry.key, entry.repo));
                continue;
            };

            if json.get("archived").and_then(|v| v.as_bool()) == Some(true)
                && entry.badge != Maintenance::Legacy
            {
                archived.push(format!(
                    "{} ({}) is archived upstream but badged {:?}",
                    entry.key, entry.repo, entry.badge
                ));
            }

            // Reported, never fatal: a feature-complete library legitimately
            // stops receiving commits, and calling that "unmaintained" is
            // exactly the judgement a curated badge exists to make.
            if entry.badge == Maintenance::Active
                && let Some(pushed) = json.get("pushed_at").and_then(|v| v.as_str())
                && let Some(year) = pushed.get(..4).and_then(|y| y.parse::<i32>().ok())
                && year < 2025
            {
                stale.push(format!(
                    "{} ({}) badged Active, last push {pushed}",
                    entry.key, entry.repo
                ));
            }
        }

        for line in &stale {
            eprintln!("  review  {line}");
        }
        for line in &unreachable {
            eprintln!("  skipped {line}");
        }

        assert!(
            archived.is_empty(),
            "curated badges contradict upstream:\n  {}",
            archived.join("\n  ")
        );
    }
}
