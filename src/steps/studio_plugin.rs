use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::steps::execution::{MessageKind, Reporter};
use crate::steps::github_get_text;
use crate::ui;

/// `%LOCALAPPDATA%\Roblox\Plugins` - the same folder `rojo plugin install` targets.
pub fn studio_plugins_dir() -> Result<PathBuf> {
    let local_app_data = std::env::var_os("LOCALAPPDATA").context("LOCALAPPDATA is not set")?;
    Ok(PathBuf::from(local_app_data).join("Roblox").join("Plugins"))
}

/// Refreshes a protocol-coupled plugin while preserving the previous file
/// if the download or final replacement fails.
pub fn refresh_from_latest_release(
    plugin: &str,
    github_repo: &str,
    asset_suffix: &str,
) -> Result<()> {
    install_release_asset(plugin, github_repo, asset_suffix, true, None)
}

pub(crate) fn install_release_asset(
    plugin: &str,
    github_repo: &str,
    asset_suffix: &str,
    replace: bool,
    reporter: Option<&Reporter>,
) -> Result<()> {
    let api_url = format!("https://api.github.com/repos/{github_repo}/releases/latest");
    let body = github_get_text(&api_url)?;
    let release: Value =
        serde_json::from_str(&body).context("failed to parse GitHub release JSON")?;

    let assets = release["assets"]
        .as_array()
        .context("release response had no assets array")?;
    let asset = assets
        .iter()
        .find(|a| {
            a["name"]
                .as_str()
                .is_some_and(|n| n.ends_with(asset_suffix))
        })
        .with_context(|| {
            format!("no *{asset_suffix} asset found in latest release of {github_repo}")
        })?;

    let name = asset["name"].as_str().context("asset has no name")?;
    let download_url = asset["browser_download_url"]
        .as_str()
        .context("asset has no browser_download_url")?;

    let plugins_dir = studio_plugins_dir()?;
    fs::create_dir_all(&plugins_dir)?;
    let dest = plugins_dir.join(name);
    if dest.exists() && !replace {
        report(
            reporter,
            MessageKind::Already,
            &format!("{name} already in Studio plugins"),
        );
        return Ok(());
    }

    crate::interrupt::check()?;
    let bytes = ureq::get(download_url)
        .header("User-Agent", "rproj")
        .call()
        .with_context(|| format!("failed to download {download_url}"))?
        .body_mut()
        .read_to_vec()
        .context("failed to read release asset body")?;
    crate::interrupt::check()?;

    if bytes.is_empty() {
        bail!("downloaded asset {name} was empty");
    }
    let pending = plugins_dir.join(format!(".{name}.rproj-new"));
    fs::write(&pending, bytes)?;
    if dest.exists() {
        let backup = plugins_dir.join(format!(".{name}.rproj-old"));
        let _ = fs::remove_file(&backup);
        fs::rename(&dest, &backup).with_context(|| {
            format!(
                "failed to prepare replacement for {plugin} at {}",
                dest.display()
            )
        })?;
        if let Err(replace_error) = fs::rename(&pending, &dest) {
            if let Err(restore_error) = fs::rename(&backup, &dest) {
                bail!(
                    "failed to replace {plugin} ({replace_error}); restoring the previous plugin also failed ({restore_error}). Recover it from {}",
                    backup.display()
                );
            }
            return Err(replace_error).with_context(|| format!("failed to replace {plugin}"));
        }
        let _ = fs::remove_file(backup);
        report(reporter, MessageKind::Installed, &format!("updated {name}"));
    } else {
        fs::rename(&pending, &dest)
            .with_context(|| format!("failed to install {}", dest.display()))?;
        report(
            reporter,
            MessageKind::Installed,
            &format!("installed {name}"),
        );
    }
    Ok(())
}

fn report(reporter: Option<&Reporter>, kind: MessageKind, text: &str) {
    if let Some(reporter) = reporter {
        reporter.message(kind, text);
    } else {
        ui::ok(text);
    }
}
