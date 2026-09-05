use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::steps::github_get_text;
use crate::ui;

/// `%LOCALAPPDATA%\Roblox\Plugins` - the same folder `rojo plugin install` targets.
pub fn studio_plugins_dir() -> Result<PathBuf> {
    let local_app_data = std::env::var_os("LOCALAPPDATA").context("LOCALAPPDATA is not set")?;
    Ok(PathBuf::from(local_app_data).join("Roblox").join("Plugins"))
}

/// Downloads the latest release asset from `owner/repo` whose filename ends
/// with `asset_suffix` (e.g. ".rbxmx") and copies it into the Studio plugins
/// folder. Skips the download if a file with that name is already there.
pub fn install_from_latest_release(
    plugin: &str,
    github_repo: &str,
    asset_suffix: &str,
) -> Result<()> {
    install_release_asset(plugin, github_repo, asset_suffix, false)
}

/// Refreshes a protocol-coupled plugin while preserving the previous file
/// if the download or final replacement fails.
pub fn refresh_from_latest_release(
    plugin: &str,
    github_repo: &str,
    asset_suffix: &str,
) -> Result<()> {
    install_release_asset(plugin, github_repo, asset_suffix, true)
}

fn install_release_asset(
    plugin: &str,
    github_repo: &str,
    asset_suffix: &str,
    replace: bool,
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
        ui::ok(&format!("{name} already in Studio plugins"));
        return Ok(());
    }

    let bytes = ureq::get(download_url)
        .header("User-Agent", "rproj")
        .call()
        .with_context(|| format!("failed to download {download_url}"))?
        .body_mut()
        .read_to_vec()
        .context("failed to read release asset body")?;

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
        ui::ok(&format!("updated {name}"));
    } else {
        fs::rename(&pending, &dest)
            .with_context(|| format!("failed to install {}", dest.display()))?;
        ui::ok(&format!("installed {name}"));
    }
    Ok(())
}
