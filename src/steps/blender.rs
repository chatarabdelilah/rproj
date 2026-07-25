use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde_json::Value;

use crate::steps::run;

const PLUGIN_REPO: &str = "Roblox/roblox-blender-plugin";
/// Roblox's stud scale: 1 stud = 0.28 meters. Setting Blender's scene unit
/// scale to this means 1 Blender unit lines up with 1 Roblox stud.
const ROBLOX_STUD_SCALE: f64 = 0.28;

/// Downloads the latest `.zip` release asset of the official Roblox Blender
/// plugin to a temp file and returns its path.
pub fn download_latest_plugin_zip() -> Result<PathBuf> {
    let api_url = format!("https://api.github.com/repos/{PLUGIN_REPO}/releases/latest");
    let body = ureq::get(&api_url)
        .header("User-Agent", "rproj")
        .call()
        .context("failed to query latest release of the Roblox Blender plugin")?
        .body_mut()
        .read_to_string()
        .context("failed to read GitHub release response")?;
    let release: Value = serde_json::from_str(&body).context("failed to parse GitHub release JSON")?;

    let assets = release["assets"].as_array().context("release response had no assets array")?;
    let asset = assets
        .iter()
        .find(|a| a["name"].as_str().is_some_and(|n| n.ends_with(".zip")))
        .context("no .zip asset found in latest Roblox Blender plugin release")?;
    let name = asset["name"].as_str().context("asset has no name")?;
    let download_url = asset["browser_download_url"]
        .as_str()
        .context("asset has no browser_download_url")?;

    let bytes = ureq::get(download_url)
        .header("User-Agent", "rproj")
        .call()
        .with_context(|| format!("failed to download {download_url}"))?
        .body_mut()
        .read_to_vec()
        .context("failed to read plugin zip body")?;

    let dest = std::env::temp_dir().join(name);
    fs::write(&dest, bytes)?;
    Ok(dest)
}

/// Installs the addon zip into Blender and enables it, run headlessly.
/// The module name to enable is discovered at runtime (by diffing Blender's
/// addons folder before/after install) rather than hardcoded, since it's
/// not something we can reliably know in advance from outside Blender.
pub fn install_addon(zip_path: &Path) -> Result<()> {
    let zip_path_str = zip_path.to_string_lossy().replace('\\', "/");
    let script = format!(
        r#"
import bpy, os
addons_dir = bpy.utils.user_resource('SCRIPTS', path="addons")
before = set(os.listdir(addons_dir)) if os.path.isdir(addons_dir) else set()
bpy.ops.preferences.addon_install(filepath=r"{zip_path_str}")
after = set(os.listdir(addons_dir)) if os.path.isdir(addons_dir) else set()
new_items = [n for n in (after - before) if not n.startswith('__')]
for name in new_items:
    module = name[:-3] if name.endswith('.py') else name
    try:
        bpy.ops.preferences.addon_enable(module=module)
        print("RPROJ_ENABLED:" + module)
    except Exception as e:
        print("RPROJ_ENABLE_FAILED:" + module + ":" + str(e))
bpy.ops.wm.save_userpref()
"#
    );
    run_headless_script(&script)
}

pub fn print_account_link_instructions() {
    println!(
        "\nBlender is set up, but linking your Roblox account is a manual, one-time step\n\
         (it needs a browser sign-in, so it can't be scripted):\n\
         1. Open Blender\n\
         2. Edit > Preferences > Add-ons > find \"Roblox\"\n\
         3. Follow the sign-in prompt to connect your Roblox account via Open Cloud\n\
         See: https://create.roblox.com/docs/art/modeling/roblox-blender-plugin"
    );
}

/// Scaffolds `blender/scene.blend` in the project with Roblox's stud scale
/// already configured, so every new Blender scene starts correctly sized.
pub fn scaffold_starter_scene(project_dir: &Path) -> Result<()> {
    let blender_dir = project_dir.join("blender");
    let dest = blender_dir.join("scene.blend");
    if dest.exists() {
        println!("check: blender/scene.blend already exists");
        return Ok(());
    }
    fs::create_dir_all(&blender_dir)?;

    let dest_str = dest.to_string_lossy().replace('\\', "/");
    let script = format!(
        r#"
import bpy
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.context.scene.unit_settings.system = 'METRIC'
bpy.context.scene.unit_settings.scale_length = {ROBLOX_STUD_SCALE}
bpy.ops.wm.save_as_mainfile(filepath=r"{dest_str}")
"#
    );
    run_headless_script(&script)?;
    println!("scaffolded blender/scene.blend");
    Ok(())
}

fn run_headless_script(script: &str) -> Result<()> {
    let script_path = std::env::temp_dir().join(format!("rproj-blender-{}.py", std::process::id()));
    fs::write(&script_path, script)?;
    let result = run(
        "blender",
        &[
            "--background",
            "--python",
            script_path.to_string_lossy().as_ref(),
        ],
    );
    let _ = fs::remove_file(&script_path);
    if result.is_err() {
        bail!("headless Blender script failed - see output above");
    }
    result
}
