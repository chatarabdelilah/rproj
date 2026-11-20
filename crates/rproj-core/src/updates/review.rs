//! Private contracts used by the matching desktop worker.
use super::{Entry, Local, Status};
use anyhow::{Context, Result, ensure};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const PROTOCOL: u32 = 1;
#[cfg(test)]
mod tests;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selection {
    pub project: PathBuf,
    pub local: Local,
    pub version: String,
}

impl Selection {
    /// The exact declaration to install. A compatible Wally range keeps its spelling.
    pub fn declaration(&self) -> Result<String> {
        self.local.source().map_err(anyhow::Error::msg)?;
        let target = Version::parse(&self.version)?;
        ensure!(target.pre.is_empty(), "Choose a stable release");
        match &self.local {
            Local::Tool { declaration, .. } => {
                let (repo, pin) = declaration.split_once('@').context("Invalid tool pin")?;
                let old = Version::parse(pin.trim_start_matches('v'))?;
                ensure!(
                    target.cmp_precedence(&old).is_gt(),
                    "Choose a newer tool release"
                );
                Ok(format!("{repo}@{target}"))
            }
            Local::Package {
                package,
                requirement,
                declaration,
                locked,
                resolution,
                ..
            } => {
                ensure!(
                    *resolution == crate::ResolutionState::Resolved,
                    "Unknown locked version"
                );
                let old = Version::parse(locked.as_deref().context("Missing locked version")?)?;
                ensure!(
                    target.cmp_precedence(&old).is_gt(),
                    "Choose a newer package release"
                );
                let req = requirement.as_deref().context("Missing declaration")?;
                if VersionReq::parse(req)?.matches(&target) {
                    return Ok(declaration.clone());
                }
                let prefix = if req.starts_with('=') {
                    "="
                } else if req.starts_with('^') {
                    "^"
                } else {
                    ""
                };
                Version::parse(&req[prefix.len()..])
                    .context("This requirement needs a manual declaration edit")?;
                Ok(format!(
                    "{}@{prefix}{target}",
                    package.as_deref().context("Unknown package")?
                ))
            }
        }
    }
}

pub fn choices(project: &std::path::Path, entry: &Entry) -> Vec<Selection> {
    let mut result = Vec::new();
    if !entry.observed || entry.comparison.status != Status::NewerAvailable {
        return result;
    }
    for version in [
        &entry.comparison.within_requirement,
        &entry.comparison.latest_stable,
    ]
    .into_iter()
    .flatten()
    {
        let s = Selection {
            project: project.to_owned(),
            local: entry.local.clone(),
            version: version.clone(),
        };
        if s.declaration().is_ok() && !result.contains(&s) {
            result.push(s);
        }
    }
    result
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    pub selection: Selection,
    pub inputs: String,
    pub outputs: String,
    pub changes: Vec<String>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    Prepare(Selection),
    Apply(Plan),
}
impl Action {
    pub fn selection(&self) -> &Selection {
        match self {
            Self::Prepare(s) => s,
            Self::Apply(p) => &p.selection,
        }
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    pub protocol: u32,
    pub id: String,
    pub action: Action,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Report {
    pub protocol: u32,
    pub id: String,
    pub selection: Selection,
    pub state: State,
}
#[derive(Debug, Serialize, Deserialize)]
pub enum State {
    Preparing,
    Prepared(Box<Plan>),
    Writing,
    Applied,
    Failed {
        possible_writes: bool,
        reason: String,
    },
    Uncertain(String),
}

impl Report {
    pub fn verify(&self, request: &Request, success: bool) -> Result<()> {
        ensure!(
            self.protocol == PROTOCOL
                && self.id == request.id
                && self.selection == *request.action.selection(),
            "Update report has a different owner"
        );
        let valid = match (&request.action, &self.state) {
            (Action::Prepare(_), State::Prepared(plan)) => {
                success && plan.selection == self.selection
            }
            (Action::Apply(_), State::Applied) => success,
            (
                _,
                State::Failed {
                    possible_writes,
                    reason,
                },
            ) => {
                !success
                    && !reason.is_empty()
                    && (!possible_writes || matches!(request.action, Action::Apply(_)))
            }
            _ => false,
        };
        ensure!(valid, "Update report and worker exit disagree");
        Ok(())
    }
}
