use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use serde_json::Value;

use crate::steps::github_get_text;
use crate::ui;

/// `%LOCALAPPDATA%\Roblox\Plugins` - the same folder `rojo plugin install` targets.
pub fn studio_plugins_dir() -> Result<PathBuf> {
    let local_app_data =
        std::env::var_os("LOCALAPPDATA").context("LOCALAPPDATA is not set")?;
    Ok(PathBuf::from(local_app_data).join("Roblox").join("Plugins"))
}

/// Downloads the latest release asset from `owner/repo` whose filename ends
/// with `asset_suffix` (e.g. ".rbxmx") and copies it into the Studio plugins
/// folder. Skips the download if a file with that name is already there.
pub fn install_from_latest_release(github_repo: &str, asset_suffix: &str) -> Result<()> {
    let api_url = format!("https://api.github.com/repos/{github_repo}/releases/latest");
    let body = github_get_text(&api_url)?;
    let release: Value =
        serde_json::from_str(&body).context("failed to parse GitHub release JSON")?;

    let assets = release["assets"]
        .as_array()
        .context("release response had no assets array")?;
    let asset = assets
        .iter()
        .find(|a| a["name"].as_str().is_some_and(|n| n.ends_with(asset_suffix)))
        .with_context(|| format!("no *{asset_suffix} asset found in latest release of {github_repo}"))?;

    let name = asset["name"].as_str().context("asset has no name")?;
    let download_url = asset["browser_download_url"]
        .as_str()
        .context("asset has no browser_download_url")?;

    let plugins_dir = studio_plugins_dir()?;
    fs::create_dir_all(&plugins_dir)?;
    let dest = plugins_dir.join(name);
    if dest.exists() {
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
    fs::write(&dest, bytes)?;
    ui::ok(&format!("installed {name}"));
    Ok(())
}
