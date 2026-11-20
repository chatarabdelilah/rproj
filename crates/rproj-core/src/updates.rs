//! Read-only, project-scoped update inventory and stable release comparisons.
use crate::{Diagnostic, PresenceState, ResolutionState};
use anyhow::Result;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    path::{Path, PathBuf},
};

#[cfg(test)]
mod tests;

pub const VERSION: u32 = 1;
pub const OFFICIAL_REGISTRY: &str = "https://github.com/UpliftGames/wally-index";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Local {
    Package {
        alias: String,
        realm: String,
        package: Option<String>,
        declaration: String,
        registry: Option<String>,
        requirement: Option<String>,
        locked: Option<String>,
        resolution: ResolutionState,
        presence: PresenceState,
    },
    Tool {
        alias: String,
        declaration: String,
        executable: Option<PathBuf>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Source {
    Wally(String),
    GitHub(String),
}

fn component(text: &str) -> bool {
    !text.is_empty()
        && text != "."
        && text != ".."
        && text
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.'))
}

fn pair(text: &str) -> bool {
    text.split_once('/')
        .is_some_and(|(a, b)| component(a) && component(b))
}

impl Local {
    pub fn alias(&self) -> &str {
        match self {
            Self::Tool { alias, .. } | Self::Package { alias, .. } => alias,
        }
    }
    pub fn is_package(&self) -> bool {
        matches!(self, Self::Package { .. })
    }
    pub fn source(&self) -> Result<Source, String> {
        match self {
            Self::Package {
                registry,
                package,
                requirement,
                ..
            } => {
                let url = registry.as_deref().unwrap_or("").trim_end_matches('/');
                if !url
                    .strip_suffix(".git")
                    .unwrap_or(url)
                    .eq_ignore_ascii_case(OFFICIAL_REGISTRY)
                {
                    return Err("Missing or unsupported Wally registry.".into());
                }
                let name = package
                    .as_deref()
                    .filter(|name| pair(name))
                    .ok_or("Invalid Wally package identity.")?;
                VersionReq::parse(requirement.as_deref().unwrap_or(""))
                    .map_err(|_| "Invalid Wally requirement.")?;
                Ok(Source::Wally(name.into()))
            }
            Self::Tool { declaration, .. } => {
                let (repository, pin) = declaration
                    .split_once('@')
                    .ok_or("Expected a GitHub owner/repository@version pin.")?;
                if !pair(repository) {
                    return Err("Unsupported tool repository.".into());
                }
                Version::parse(pin.strip_prefix('v').unwrap_or(pin))
                    .ok()
                    .filter(|v| v.pre.is_empty())
                    .ok_or("The project pin is not an exact stable version.")?;
                Ok(Source::GitHub(repository.to_ascii_lowercase()))
            }
        }
    }
    pub fn compare(&self, versions: &[Version]) -> Comparison {
        match self {
            Self::Package {
                locked,
                requirement,
                resolution,
                ..
            } => {
                let req = requirement
                    .as_deref()
                    .and_then(|s| VersionReq::parse(s).ok());
                let mut result = compare(locked.as_deref(), req.as_ref(), versions);
                if *resolution != ResolutionState::Resolved {
                    result.status = Status::Unknown;
                    result.within_requirement_status =
                        result.within_requirement.as_ref().map(|_| Status::Unknown);
                    result.reason = Some(format!("Lockfile resolution is {resolution:?}."));
                }
                result
            }
            Self::Tool { declaration, .. } => {
                let pin = declaration
                    .split_once('@')
                    .map(|(_, v)| v.strip_prefix('v').unwrap_or(v));
                compare(pin, None, versions)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Current,
    NewerAvailable,
    LocalAhead,
    Unknown,
}

impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Self::Current => "Current",
            Self::NewerAvailable => "Newer release available",
            Self::LocalAhead => "Local version ahead",
            Self::Unknown => "Unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comparison {
    pub status: Status,
    pub latest_stable: Option<String>,
    pub within_requirement: Option<String>,
    pub within_requirement_status: Option<Status>,
    pub reason: Option<String>,
}
impl Comparison {
    pub fn unknown(reason: impl Into<String>) -> Self {
        Self {
            status: Status::Unknown,
            latest_stable: None,
            within_requirement: None,
            within_requirement_status: None,
            reason: Some(reason.into()),
        }
    }
}
fn compare(local: Option<&str>, req: Option<&VersionReq>, versions: &[Version]) -> Comparison {
    let stable = versions.iter().filter(|v| v.pre.is_empty());
    let Some(latest) = stable.clone().max_by(|a, b| a.cmp_precedence(b)) else {
        return Comparison::unknown("No stable semantic release found.");
    };
    let compatible = req.and_then(|r| {
        stable
            .filter(|v| r.matches(v))
            .max_by(|a, b| a.cmp_precedence(b))
    });
    let local = local.and_then(|s| Version::parse(s).ok());
    let against = |candidate: &Version| match local.as_ref().map(|v| v.cmp_precedence(candidate)) {
        Some(Ordering::Less) => Status::NewerAvailable,
        Some(Ordering::Equal) => Status::Current,
        Some(Ordering::Greater) => Status::LocalAhead,
        None => Status::Unknown,
    };
    let status = against(latest);
    Comparison {
        status,
        latest_stable: Some(latest.to_string()),
        within_requirement: compatible.map(ToString::to_string),
        within_requirement_status: compatible.map(against),
        reason: (status == Status::Unknown).then(|| "No comparable local version.".into()),
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Inventory {
    pub project: PathBuf,
    pub name: Option<String>,
    pub entries: Vec<Local>,
    pub diagnostics: Vec<Diagnostic>,
}
pub fn inventory(path: &Path) -> Result<Inventory> {
    let (snapshot, registry) = crate::inspection::inspect_with_registry(path)?;
    let mut entries: Vec<_> = snapshot
        .packages
        .into_iter()
        .map(|p| Local::Package {
            alias: p.alias,
            realm: p.realm.section().into(),
            package: p.package,
            declaration: p.declaration,
            registry: registry.clone(),
            requirement: p.version_requirement,
            locked: p.resolved.map(|r| r.version),
            resolution: p.resolution,
            presence: p.presence.state,
        })
        .collect();
    entries.extend(snapshot.tools.into_iter().map(|t| Local::Tool {
        alias: t.alias,
        declaration: t.source,
        executable: t.executable,
    }));
    Ok(Inventory {
        project: snapshot.root,
        name: snapshot.name,
        entries,
        diagnostics: snapshot.diagnostics,
    })
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entry {
    pub local: Local,
    pub comparison: Comparison,
    /// False when the source was unavailable or its response incomplete.
    pub observed: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Report {
    pub schema_version: u32,
    pub project: PathBuf,
    pub name: Option<String>,
    pub checked_at_unix_seconds: u64,
    pub entries: Vec<Entry>,
    pub diagnostics: Vec<Diagnostic>,
}
