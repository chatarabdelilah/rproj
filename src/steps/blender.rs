use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde_json::Value;

use crate::steps::probe;

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
/// Idempotent: the expected module name is read directly out of the zip's
/// own top-level entry (via Python's stdlib `zipfile`, no extraction needed)
/// rather than inferred by diffing Blender's addons folder before/after -
/// that diffing approach only ever detects a change the first time the
/// addon doesn't already exist on disk, so it silently stops finding
/// anything the moment the addon has been installed once (including from
/// before this idempotency check existed), which is exactly what caused
/// `rproj setup` to look like it was reinstalling on every run.
pub fn install_addon(zip_path: &Path) -> Result<()> {
    let zip_path_str = zip_path.to_string_lossy().replace('\\', "/");
    let script = format!(
        r#"
import bpy, os, zipfile

zip_path = r"{zip_path_str}"
names = zipfile.ZipFile(zip_path).namelist()
candidates = sorted({{
    n.split('/')[0] for n in names
    if n.strip('/') and not n.startswith('__MACOSX') and '.' not in n.split('/')[0]
}})
module = candidates[0] if candidates else None

addons_dir = bpy.utils.user_resource('SCRIPTS', path="addons")
already_installed = bool(module) and (
    os.path.isdir(os.path.join(addons_dir, module)) or os.path.isfile(os.path.join(addons_dir, module + ".py"))
)

if already_installed:
    print("RPROJ_ALREADY_INSTALLED:" + module)
else:
    bpy.ops.preferences.addon_install(filepath=zip_path)
    if module:
        try:
            bpy.ops.preferences.addon_enable(module=module)
            print("RPROJ_ENABLED:" + module)
        except Exception as e:
            print("RPROJ_ENABLE_FAILED:" + module + ":" + str(e))
    else:
        print("RPROJ_UNKNOWN_MODULE")
    bpy.ops.wm.save_userpref()
"#
    );
    let stdout = run_headless_script(&script)?;

    if let Some(module) = stdout.lines().find_map(|l| l.strip_prefix("RPROJ_ALREADY_INSTALLED:")) {
        println!("check: Roblox Blender plugin already installed ({module})");
    }
    Ok(())
}

pub fn print_account_link_instructions() {
    println!(
        "\nBlender is set up, but two one-time steps still need to happen inside Blender\n\
         itself (they need a UI/browser, so they can't be scripted):\n\
         1. Open Blender\n\
         2. Edit > Preferences > Add-ons > find \"Roblox\" and expand it\n\
         3. Open the N-panel in a 3D viewport (press N) > \"Roblox\" tab > click\n\
         \"Install Dependencies\", then restart Blender when it finishes\n\
         4. After restarting, follow the sign-in prompt to connect your Roblox\n\
         account via Open Cloud\n\
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

/// Runs `blender --background --python <script>` with stdout/stderr
/// captured explicitly (rather than only inherited) and always printed,
/// since Blender is a Windows GUI-subsystem executable and its console
/// output doesn't reliably flow through to an inherited parent console -
/// so a bare `Command::status()` can report a failure with nothing useful
/// printed above it. This guarantees whatever Blender wrote is visible.
/// Returns the captured stdout so callers can parse `RPROJ_*:` markers.
fn run_headless_script(script: &str) -> Result<String> {
    let script_path = std::env::temp_dir().join(format!("rproj-blender-{}.py", std::process::id()));
    fs::write(&script_path, script)?;

    let blender_exe = locate_blender_exe()?;
    println!(
        "\n> {} --background --python {}",
        blender_exe.display(),
        script_path.display()
    );
    let output = std::process::Command::new(&blender_exe)
        .args(["--background", "--python", &script_path.to_string_lossy()])
        .output()
        .with_context(|| format!("failed to spawn `{}`", blender_exe.display()));
    let _ = fs::remove_file(&script_path);
    let output = output?;

    if !output.stdout.is_empty() {
        print!("{}", String::from_utf8_lossy(&output.stdout));
    }
    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }

    if !output.status.success() {
        bail!(
            "blender exited with {} (see output above - if it just says \
             \"Python script failed, check the message in the system console\", \
             the addon zip may be corrupt or Blender is older than the 3.2+ this \
             plugin requires)",
            output.status
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Finds blender.exe even when it's not on PATH. winget installs Blender
/// but doesn't reliably put it on PATH for an already-running shell (or at
/// all, depending on the installer) - the same PATH gotcha this project's
/// own dev setup hit with cargo/rustc. Falls back to scanning the standard
/// winget/installer location under Program Files.
fn locate_blender_exe() -> Result<PathBuf> {
    if probe("blender", &["--version"]) {
        return Ok(PathBuf::from("blender"));
    }

    let program_files = std::env::var_os("ProgramFiles")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Program Files"));
    let base = program_files.join("Blender Foundation");

    let mut candidates: Vec<PathBuf> = fs::read_dir(&base)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .map(|e| e.path().join("blender.exe"))
        .filter(|p| p.is_file())
        .collect();
    candidates.sort();

    candidates.pop().with_context(|| {
        format!(
            "blender isn't on PATH and no install was found under {} - is Blender installed?",
            base.display()
        )
    })
}
