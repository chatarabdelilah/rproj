use std::{
    collections::BTreeMap,
    io::{self, Write},
    path::Path,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, bail, ensure};
use rproj_core::updates::{self, Comparison, Entry, Inventory, Local, Report, Source};
use semver::Version;
use serde::Deserialize;

const PAGE_SIZE: usize = 100;
const MAX_PAGES: usize = 5;

#[cfg(test)]
mod tests;

#[derive(Deserialize)]
struct WallyRecord {
    package: WallyPackage,
}
#[derive(Deserialize)]
struct WallyPackage {
    name: String,
    version: String,
}
#[derive(Deserialize)]
struct Release {
    draft: bool,
    prerelease: bool,
    tag_name: String,
}

fn releases(source: &Source, mut get: impl FnMut(&str) -> Result<String>) -> Result<Vec<Version>> {
    let mut versions = Vec::new();
    match source {
        Source::Wally(name) => {
            let body = get(&format!(
                "https://raw.githubusercontent.com/UpliftGames/wally-index/main/{name}"
            ))?;
            for line in body.lines().filter(|line| !line.trim().is_empty()) {
                let record: WallyRecord =
                    serde_json::from_str(line).context("Malformed Wally index record")?;
                ensure!(
                    &record.package.name == name,
                    "Unexpected Wally package identity"
                );
                versions.push(
                    Version::parse(&record.package.version)
                        .context("Invalid Wally release version")?,
                );
            }
            Ok(versions)
        }
        Source::GitHub(repo) => {
            for page in 1..=MAX_PAGES {
                let url = format!(
                    "https://api.github.com/repos/{repo}/releases?per_page={PAGE_SIZE}&page={page}"
                );
                let records: Vec<Release> =
                    serde_json::from_str(&get(&url)?).context("Malformed GitHub release list")?;
                let complete = records.len() < PAGE_SIZE;
                for release in records.into_iter().filter(|r| !r.draft && !r.prerelease) {
                    if let Ok(v) = Version::parse(
                        release
                            .tag_name
                            .strip_prefix('v')
                            .unwrap_or(&release.tag_name),
                    ) {
                        versions.push(v);
                    }
                }
                if complete {
                    return Ok(versions);
                }
            }
            bail!("GitHub release history exceeds the page limit; latest version unknown")
        }
    }
}

fn report(inventory: Inventory, mut lookup: impl FnMut(&Source) -> Result<Vec<Version>>) -> Report {
    let mut cache = BTreeMap::new();
    let entries = inventory
        .entries
        .into_iter()
        .map(|local| {
            let (comparison, observed) = match local.source() {
                Err(reason) => (Comparison::unknown(reason), false),
                Ok(source) => {
                    let observation = cache
                        .entry(source.clone())
                        .or_insert_with(|| lookup(&source).map_err(|e| format!("{e:#}")));
                    match observation {
                        Ok(versions) => (local.compare(versions), true),
                        Err(error) => (Comparison::unknown(error.as_str()), false),
                    }
                }
            };
            Entry {
                local,
                comparison,
                observed,
            }
        })
        .collect();
    Report {
        schema_version: updates::VERSION,
        project: inventory.project,
        name: inventory.name,
        entries,
        checked_at_unix_seconds: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        diagnostics: inventory.diagnostics,
    }
}

fn collect(path: &Path) -> Result<Report> {
    collect_inventory(updates::inventory(path)?)
}

pub(crate) fn collect_selected(path: &Path, local: &Local) -> Result<Entry> {
    let mut inventory = updates::inventory(path)?;
    inventory.entries.retain(|e| e == local);
    ensure!(
        inventory.entries.len() == 1,
        "Selected declaration changed or is ambiguous; check for updates again"
    );
    collect_inventory(inventory)?
        .entries
        .pop()
        .context("Missing selected release")
}

fn collect_inventory(inventory: Inventory) -> Result<Report> {
    let deadline = Instant::now() + Duration::from_secs(18);
    let mut requests = 0;
    let mut limited_hosts = std::collections::BTreeSet::new();
    Ok(report(inventory, |source| {
        let host = match source {
            Source::GitHub(_) => "GitHub",
            Source::Wally(_) => "Wally index",
        };
        ensure!(!limited_hosts.contains(host), "{host} rate limit reached");
        releases(source, |url| {
            let remaining = deadline.saturating_duration_since(Instant::now());
            ensure!(
                !remaining.is_zero() && requests < 100,
                "Discovery budget exhausted; check incomplete"
            );
            requests += 1;
            let request = ureq::get(url)
                .header("User-Agent", concat!("rproj/", env!("CARGO_PKG_VERSION")))
                .config()
                .http_status_as_error(false)
                .max_redirects(0)
                .timeout_global(Some(remaining.min(Duration::from_secs(5))))
                .build();
            let mut response = request.call().context("Release metadata unavailable")?;
            if response.status().as_u16() == 429
                || (response.status().as_u16() == 403
                    && (response
                        .headers()
                        .get("x-ratelimit-remaining")
                        .is_some_and(|v| v == "0")
                        || response.headers().contains_key("retry-after")))
            {
                limited_hosts.insert(host);
                bail!("{host} rate limit reached");
            }
            ensure!(
                response.status().is_success(),
                "Release service returned {}",
                response.status()
            );
            response
                .body_mut()
                .with_config()
                .limit(2 * 1024 * 1024)
                .read_to_string()
                .context("Incomplete or oversized release metadata")
        })
    }))
}

pub fn run(path: Option<&Path>, json: bool) -> Result<()> {
    let report = collect(path.unwrap_or(Path::new(".")))?;
    let mut out = io::stdout().lock();
    if json {
        serde_json::to_writer_pretty(&mut out, &report)?;
        writeln!(out)?;
    } else {
        writeln!(
            out,
            "Project: {} · checked at {} (Unix seconds)",
            report.project.display(),
            report.checked_at_unix_seconds
        )?;
        for (package, label) in [(true, "Packages"), (false, "Tools")] {
            writeln!(out, "{label}:")?;
            for row in report
                .entries
                .iter()
                .filter(|e| e.local.is_package() == package)
            {
                match &row.local {
                    Local::Package {
                        alias,
                        realm,
                        declaration,
                        locked,
                        presence,
                        registry,
                        ..
                    } => writeln!(
                        out,
                        "  {realm}.{alias}: {declaration} · locked {} · files {presence:?} · registry {}",
                        locked.as_deref().unwrap_or("unknown"),
                        registry.as_deref().unwrap_or("unknown")
                    )?,
                    Local::Tool {
                        alias,
                        declaration,
                        executable,
                    } => writeln!(
                        out,
                        "  {alias}: {declaration} · PATH {}",
                        executable
                            .as_ref()
                            .map_or_else(|| "not detected".into(), |p| p.display().to_string())
                    )?,
                }
                let c = &row.comparison;
                writeln!(
                    out,
                    "    {} · latest stable {}{}",
                    c.status.label(),
                    c.latest_stable.as_deref().unwrap_or("unknown"),
                    if package {
                        format!(
                            " · within requirement {}",
                            c.within_requirement.as_deref().unwrap_or("none known")
                        )
                    } else {
                        String::new()
                    }
                )?;
                if let Some(reason) = &c.reason {
                    writeln!(out, "    {reason}")?;
                }
            }
        }
        for diagnostic in &report.diagnostics {
            writeln!(out, "{}: {}", diagnostic.path.display(), diagnostic.message)?;
        }
    }
    Ok(())
}

pub fn worker() -> Result<()> {
    let report = serde_json::to_string(&collect(Path::new("."))?)?;
    ensure!(
        report.len() <= 2 * 1024 * 1024,
        "Update report exceeds the worker message limit; use rproj updates --json"
    );
    println!("{report}");
    Ok(())
}
