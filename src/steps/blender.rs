use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::steps::execution::{MessageKind, Reporter};
use crate::steps::{github_get_text, probe};
use crate::ui;

/// Roblox's stud scale: 1 stud = 0.28 meters. Setting Blender's scene unit
/// scale to this means 1 Blender unit lines up with 1 Roblox stud.
const ROBLOX_STUD_SCALE: f64 = 0.28;

/// Downloads the latest `.zip` release asset of the official Roblox Blender
/// plugin to a temp file and returns its path. `github_repo` comes from the
/// catalog's "blender-plugin" entry rather than being duplicated as a
/// literal here.
pub fn download_latest_plugin_zip(github_repo: &str) -> Result<PathBuf> {
    let api_url = format!("https://api.github.com/repos/{github_repo}/releases/latest");
    let body = github_get_text(&api_url)?;
    let release: Value =
        serde_json::from_str(&body).context("failed to parse GitHub release JSON")?;

    let assets = release["assets"]
        .as_array()
        .context("release response had no assets array")?;
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

pub(crate) fn install_addon_with(zip_path: &Path, reporter: Option<&Reporter>) -> Result<()> {
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
    let stdout = run_headless_script_with(&script, reporter)?;

    if let Some(module) = stdout
        .lines()
        .find_map(|l| l.strip_prefix("RPROJ_ALREADY_INSTALLED:"))
    {
        let text = format!("Blender add-on already installed ({module})");
        if let Some(reporter) = reporter {
            reporter.message(MessageKind::Already, &text);
        } else {
            ui::ok(&text);
        }
    }
    Ok(())
}

pub(crate) fn account_link_instructions(reporter: &Reporter) {
    reporter.message(
        MessageKind::Manual,
        "Blender add-on needs two one-time manual steps (they need a UI, so can't be scripted)",
    );
    reporter.message(
        MessageKind::Detail,
        "1. Blender > Edit > Preferences > Add-ons > find \"Roblox\", expand it\n\
         2. In a 3D viewport press N > \"Roblox\" tab > Install Dependencies, then restart\n\
         3. Follow the sign-in prompt to connect your account via Open Cloud\n\
         Docs: https://create.roblox.com/docs/art/modeling/roblox-blender-plugin",
    );
}

/// Scaffolds `blender/scene.blend` in the project with Roblox's stud scale
/// already configured, so every new Blender scene starts correctly sized.
pub fn scaffold_starter_scene(project_dir: &Path) -> Result<()> {
    scaffold_starter_scene_with(project_dir, locate_blender_exe, |exe, script| {
        run_headless_script_at(exe, script, None)
    })
}

pub(crate) fn preflight_starter_scene(project_dir: &Path) -> Result<()> {
    starter_scene_executable(project_dir, locate_blender_exe).map(|_| ())
}

fn starter_scene_executable(
    project_dir: &Path,
    locate: impl FnOnce() -> Result<PathBuf>,
) -> Result<Option<PathBuf>> {
    crate::interrupt::check()?;
    let blender_dir = project_dir.join("blender");
    match fs::symlink_metadata(&blender_dir) {
        Ok(metadata) => {
            #[cfg(windows)]
            let linked = {
                use std::os::windows::fs::MetadataExt;
                metadata.file_attributes() & 0x400 != 0
            };
            #[cfg(not(windows))]
            let linked = metadata.file_type().is_symlink();
            if linked {
                bail!(
                    "{} is a linked directory; refusing to create a scene outside the project",
                    blender_dir.display()
                );
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(error)
                .with_context(|| format!("failed to inspect {}", blender_dir.display()));
        }
    }
    let dest = project_dir.join("blender/scene.blend");
    match fs::symlink_metadata(&dest) {
        Ok(metadata) if metadata.is_file() => return Ok(None),
        Ok(_) => bail!(
            "{} already exists and is not a regular file; leave it intact and choose another scene location",
            dest.display()
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(error).with_context(|| format!("failed to inspect {}", dest.display()));
        }
    }
    locate().map(Some).context(
        "Assets 3D needs Blender to create blender/scene.blend. Install Blender (https://www.blender.org/download/), make it available on PATH, or deselect the Blender scene. No scene was created; retry creation or run `rproj setup blender` inside an existing project",
    )
}

fn scaffold_starter_scene_with(
    project_dir: &Path,
    locate: impl FnOnce() -> Result<PathBuf>,
    run: impl FnOnce(&Path, &str) -> Result<String>,
) -> Result<()> {
    let Some(blender_exe) = starter_scene_executable(project_dir, locate)? else {
        ui::ok("blender/scene.blend already exists");
        return Ok(());
    };
    let blender_dir = project_dir.join("blender");
    let dest = blender_dir.join("scene.blend");
    fs::create_dir_all(&blender_dir)?;
    let staged = tempfile::Builder::new()
        .prefix(".rproj-scene-")
        .suffix(".blend")
        .tempfile_in(&blender_dir)?
        .into_temp_path();

    let dest_str = serde_json::to_string(&staged.to_string_lossy())?;
    let script = format!(
        r#"
import bpy
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.context.scene.unit_settings.system = 'METRIC'
bpy.context.scene.unit_settings.scale_length = {ROBLOX_STUD_SCALE}
bpy.context.preferences.filepaths.save_version = 0
bpy.ops.wm.save_as_mainfile(filepath={dest_str})
"#
    );
    run(&blender_exe, &script)?;
    crate::interrupt::check()?;
    if fs::metadata(&staged)?.len() == 0 {
        bail!(
            "Blender completed without writing the starter scene; blender/scene.blend was not created"
        );
    }
    // Another process may create a scene while Blender runs. Commit only if
    // the destination is still free, preserving that process's work.
    if let Err(error) = staged.persist_noclobber(&dest) {
        if error.error.kind() == std::io::ErrorKind::AlreadyExists && dest.is_file() {
            ui::ok("blender/scene.blend appeared while Blender ran; preserved the existing scene");
            return Ok(());
        }
        return Err(error.error).context("failed to save blender/scene.blend without overwriting");
    }
    ui::ok("scaffolded blender/scene.blend");
    Ok(())
}

/// Runs `blender --background --python <script>` with stdout/stderr
/// captured explicitly (rather than only inherited) and always printed,
/// since Blender is a Windows GUI-subsystem executable and its console
/// output doesn't reliably flow through to an inherited parent console -
/// so a bare `Command::status()` can report a failure with nothing useful
/// printed above it. This guarantees whatever Blender wrote is visible.
/// Returns the captured stdout so callers can parse `RPROJ_*:` markers.
fn run_headless_script_with(script: &str, reporter: Option<&Reporter>) -> Result<String> {
    let blender_exe = locate_blender_exe()?;
    run_headless_script_at(&blender_exe, script, reporter)
}

fn run_headless_script_at(
    blender_exe: &Path,
    script: &str,
    reporter: Option<&Reporter>,
) -> Result<String> {
    crate::interrupt::check()?;
    let script_dir = tempfile::tempdir().context("failed to create Blender script directory")?;
    let script_path = script_dir.path().join("script.py");
    fs::write(&script_path, script)?;

    if let Some(reporter) = reporter {
        let output = reporter.capture(std::process::Command::new(blender_exe).args([
            "--background",
            "--python",
            &script_path.to_string_lossy(),
        ]))?;
        if !output.success {
            bail!(
                "Blender add-on installation failed; see output. Check the ZIP and Blender version (3.2+ required)."
            );
        }
        return Ok(output.stdout);
    }
    crate::interrupt::check()?;
    ui::command(
        &blender_exe.to_string_lossy(),
        &["--background", "--python", &script_path.to_string_lossy()],
    );
    let output = std::process::Command::new(blender_exe)
        .args(["--background", "--python", &script_path.to_string_lossy()])
        .output()
        .with_context(|| format!("failed to spawn `{}`", blender_exe.display()));
    let output = output?;
    crate::diagnostics::tool_exit("blender", output.status);
    crate::interrupt::check()?;

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    // Blender is a Windows GUI-subsystem executable, so this captured
    // output is the only place its diagnostics appear at all. Shown once,
    // on failure or when verbose.
    if !output.status.success() || ui::is_verbose() {
        ui::passthrough(&stdout, &stderr);
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
    Ok(stdout)
}

/// Finds blender.exe even when it's not on PATH. winget installs Blender
/// but doesn't reliably put it on PATH for an already-running shell (or at
/// all, depending on the installer) - the same PATH gotcha this project's
/// own dev setup hit with cargo/rustc. Falls back to scanning the standard
/// winget/installer location under Program Files.
fn locate_blender_exe() -> Result<PathBuf> {
    let on_path = probe("blender", &["--version"]);
    crate::interrupt::check()?;
    if on_path {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires an installed Blender; writes only an isolated temporary project"]
    fn installed_blender_creates_and_preserves_a_starter_scene() {
        let root = tempfile::tempdir().unwrap();
        scaffold_starter_scene(root.path()).unwrap();
        let path = root.path().join("blender/scene.blend");
        let bytes = fs::read(&path).unwrap();
        assert!(!bytes.is_empty());
        scaffold_starter_scene(root.path()).unwrap();
        assert_eq!(fs::read(path).unwrap(), bytes);
        assert_eq!(
            fs::read_dir(root.path().join("blender")).unwrap().count(),
            1
        );
    }

    fn write_scene_script_output(script: &str) {
        assert!(script.contains("scale_length = 0.28"));
        let filename = script
            .lines()
            .find_map(|line| {
                line.strip_prefix("bpy.ops.wm.save_as_mainfile(filepath=")
                    .and_then(|value| value.strip_suffix(')'))
            })
            .unwrap();
        let path: String = serde_json::from_str(filename).unwrap();
        fs::write(path, b"BLENDER fixture scene").unwrap();
    }

    #[test]
    fn missing_blender_fails_before_creating_project_or_scene_directories() {
        let root = tempfile::tempdir().unwrap();
        let project = root.path().join("uncreated");
        let error = scaffold_starter_scene_with(
            &project,
            || bail!("fixture missing executable"),
            |_, _| panic!("missing Blender must not execute a script"),
        )
        .unwrap_err();
        let message = format!("{error:#}");
        assert!(message.contains("Assets 3D needs Blender"), "{message}");
        assert!(message.contains("rproj setup blender"), "{message}");
        assert!(!project.exists());
    }

    #[test]
    fn detected_blender_creates_a_scaled_scene_without_machine_preferences() {
        let root = tempfile::tempdir().unwrap();
        let project = root.path().join("quoted ' project é");
        scaffold_starter_scene_with(
            &project,
            || Ok(PathBuf::from("discovered-blender")),
            |exe, script| {
                assert_eq!(exe, Path::new("discovered-blender"));
                write_scene_script_output(script);
                Ok(String::new())
            },
        )
        .unwrap();
        assert_eq!(
            fs::read(project.join("blender/scene.blend")).unwrap(),
            b"BLENDER fixture scene"
        );
        assert_eq!(fs::read_dir(project.join("blender")).unwrap().count(), 1);
    }

    #[test]
    fn existing_scene_needs_no_blender_and_is_never_overwritten() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("blender")).unwrap();
        let scene = root.path().join("blender/scene.blend");
        fs::write(&scene, b"owner scene").unwrap();
        scaffold_starter_scene_with(
            root.path(),
            || panic!("existing scene must not need Blender"),
            |_, _| panic!("existing scene must not execute Blender"),
        )
        .unwrap();
        assert_eq!(fs::read(scene).unwrap(), b"owner scene");
    }

    #[test]
    fn linked_blender_folder_is_refused_without_writing_outside_the_project() {
        let project = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let linked = project.path().join("blender");
        #[cfg(windows)]
        {
            let status = std::process::Command::new("cmd")
                .args(["/C", "mklink", "/J"])
                .arg(&linked)
                .arg(outside.path())
                .output()
                .unwrap();
            assert!(
                status.status.success(),
                "{}",
                String::from_utf8_lossy(&status.stderr)
            );
        }
        #[cfg(unix)]
        std::os::unix::fs::symlink(outside.path(), &linked).unwrap();
        let error = scaffold_starter_scene_with(
            project.path(),
            || panic!("linked directory must fail before locating Blender"),
            |_, _| panic!("linked directory must not run Blender"),
        )
        .unwrap_err();
        assert!(error.to_string().contains("linked directory"));
        assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
        #[cfg(windows)]
        fs::remove_dir(&linked).unwrap();
        #[cfg(unix)]
        fs::remove_file(&linked).unwrap();
    }

    #[test]
    fn failed_or_empty_blender_output_leaves_no_partial_scene() {
        for fails in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let error = scaffold_starter_scene_with(
                root.path(),
                || Ok(PathBuf::from("blender")),
                |_, script| {
                    if fails {
                        write_scene_script_output(script);
                        bail!("fixture Blender failed");
                    }
                    Ok(String::new())
                },
            )
            .unwrap_err();
            assert!(!error.to_string().is_empty());
            assert_eq!(
                fs::read_dir(root.path().join("blender")).unwrap().count(),
                0
            );
        }
    }

    #[test]
    fn scene_created_during_blender_execution_is_preserved() {
        let root = tempfile::tempdir().unwrap();
        let scene = root.path().join("blender/scene.blend");
        scaffold_starter_scene_with(
            root.path(),
            || Ok(PathBuf::from("blender")),
            |_, script| {
                write_scene_script_output(script);
                fs::write(&scene, b"concurrent owner scene")?;
                Ok(String::new())
            },
        )
        .unwrap();
        assert_eq!(fs::read(scene).unwrap(), b"concurrent owner scene");
        assert_eq!(
            fs::read_dir(root.path().join("blender")).unwrap().count(),
            1
        );
    }
}
