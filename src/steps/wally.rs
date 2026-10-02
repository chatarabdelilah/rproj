use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::catalog::wally_packages::{self, Realm};
use crate::steps::{rojo, run_in};
use crate::ui;

pub fn ensure_wally_init(project_dir: &Path) -> Result<()> {
    if project_dir.join("wally.toml").exists() {
        ui::ok("wally.toml already exists");
        return Ok(());
    }
    run_in("wally", &["init"], Some(project_dir))
}

/// Writes wally.toml from the catalog for the given selected package keys.
/// Idempotent: if the file already lists exactly this set of dependency
/// keys, it's left untouched.
pub fn write_wally_toml(project_dir: &Path, package_name: &str, selected: &[String]) -> Result<()> {
    let path = project_dir.join("wally.toml");

    if path.exists() {
        let content = fs::read_to_string(&path)?;
        let has_all = selected.iter().all(|key| {
            let manifest_key = wally_packages::find(key).map(manifest_key).unwrap_or(key);
            content
                .lines()
                .any(|l| l.trim_start().starts_with(&format!("{manifest_key} =")))
        });
        if has_all {
            ui::ok("wally.toml already configured");
            return Ok(());
        }
    }

    let body = render_wally_toml(package_name, selected)?;
    replace_file(&path, body.as_bytes())?;
    ui::ok("wrote wally.toml");
    Ok(())
}

pub fn render_wally_toml(package_name: &str, selected: &[String]) -> Result<String> {
    let mut specs = Vec::new();
    for key in selected {
        specs.push(
            wally_packages::find(key)
                .with_context(|| format!("unknown package key `{key}` in selection"))?,
        );
    }

    let mut body = format!(
        "[package]\nname = \"{package_name}\"\nversion = \"0.1.0\"\nregistry = \"https://github.com/UpliftGames/wally-index\"\nrealm = \"shared\"\n\n[dependencies]\n"
    );
    for spec in specs.iter().filter(|s| s.realm == Realm::Shared) {
        body.push_str(&format!("{} = \"{}\"\n", spec.key, spec.source));
    }

    // A server-realm package under `[dependencies]` doesn't merely land in
    // the wrong folder - wally refuses to resolve it and the install fails
    // outright, taking the whole scaffold with it. The section is only
    // emitted when something needs it, so the common project keeps a
    // one-section manifest.
    let server: Vec<_> = specs.iter().filter(|s| s.realm == Realm::Server).collect();
    if !server.is_empty() {
        body.push_str("\n[server-dependencies]\n");
        for spec in server {
            body.push_str(&format!("{} = \"{}\"\n", spec.key, spec.source));
        }
    }

    let dev: Vec<_> = specs.iter().filter(|s| s.realm == Realm::Dev).collect();
    if !dev.is_empty() {
        body.push_str("\n[dev-dependencies]\n");
        for spec in dev {
            body.push_str(&format!("{} = \"{}\"\n", manifest_key(spec), spec.source));
        }
    }
    Ok(body)
}

/// Adds catalog-managed dependencies to an existing Wally manifest while
/// preserving unrelated packages, comments, and package metadata.
pub fn merge_selected(project_dir: &Path, selected: &[String]) -> Result<()> {
    let path = project_dir.join("wally.toml");
    let existing =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let updated = merge_selected_contents(&existing, selected)?;
    if updated == existing {
        ui::ok("wally.toml already contains selected packages");
    } else {
        replace_file(&path, updated.as_bytes())?;
        ui::ok("updated wally.toml");
    }
    Ok(())
}

fn merge_selected_contents(existing: &str, selected: &[String]) -> Result<String> {
    let manifest: toml::Value = toml::from_str(existing).context("failed to parse wally.toml")?;
    let newline = if existing.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let had_final_newline = existing.ends_with('\n');
    let mut lines: Vec<String> = existing.lines().map(str::to_owned).collect();
    for key in selected {
        let spec = wally_packages::find(key)
            .with_context(|| format!("unknown selected Wally package `{key}`"))?;
        let alias = manifest_key(spec);
        let section = match spec.realm {
            Realm::Shared => "dependencies",
            Realm::Server => "server-dependencies",
            Realm::Dev => "dev-dependencies",
        };
        let existing_sections = dependency_sections(&manifest, alias);
        match existing_sections.as_slice() {
            [] => insert_dependency(&mut lines, section, alias, spec.source),
            [existing] if *existing == section => {}
            _ => bail!(
                "Wally alias `{alias}` exists in [{}], but selected package `{key}` requires [{section}]; move the existing pin to the required section and retry",
                existing_sections.join(", ")
            ),
        }
    }
    let mut updated = lines.join(newline);
    if had_final_newline {
        updated.push_str(newline);
    }
    let _: toml::Value = toml::from_str(&updated).context("updated wally.toml is invalid")?;
    Ok(updated)
}

fn dependency_sections<'a>(manifest: &'a toml::Value, alias: &str) -> Vec<&'a str> {
    ["dependencies", "server-dependencies", "dev-dependencies"]
        .into_iter()
        .filter(|section| {
            manifest
                .get(section)
                .and_then(toml::Value::as_table)
                .is_some_and(|dependencies| dependencies.contains_key(alias))
        })
        .collect()
}

fn insert_dependency(lines: &mut Vec<String>, section: &str, alias: &str, source: &str) {
    let header = format!("[{section}]");
    if let Some(start) = lines
        .iter()
        .position(|line| section_header(line) == Some(section))
    {
        let end = lines[start + 1..]
            .iter()
            .position(|line| section_header(line).is_some())
            .map(|offset| start + 1 + offset)
            .unwrap_or(lines.len());
        lines.insert(end, format!("{alias} = \"{source}\""));
    } else {
        if lines.last().is_some_and(|line| !line.is_empty()) {
            lines.push(String::new());
        }
        lines.push(header);
        lines.push(format!("{alias} = \"{source}\""));
    }
}

fn section_header(line: &str) -> Option<&str> {
    line.split('#')
        .next()
        .map(str::trim)
        .and_then(|line| line.strip_prefix('['))
        .and_then(|line| line.strip_suffix(']'))
        .map(str::trim)
}

fn replace_file(path: &Path, contents: &[u8]) -> Result<()> {
    let parent = path.parent().context("Wally manifest path has no parent")?;
    let mut staged = tempfile::NamedTempFile::new_in(parent)
        .with_context(|| format!("failed to stage {}", path.display()))?;
    staged
        .write_all(contents)
        .with_context(|| format!("failed to stage {}", path.display()))?;
    staged
        .as_file()
        .sync_all()
        .with_context(|| format!("failed to sync staged {}", path.display()))?;
    staged
        .persist(path)
        .map(|_| ())
        .map_err(|error| error.error)
        .with_context(|| format!("failed to replace {}", path.display()))
}

fn manifest_key(spec: &wally_packages::PackageSpec) -> &'static str {
    spec.alias()
}

pub fn wally_install(project_dir: &Path) -> Result<()> {
    run_in("wally", &["install"], Some(project_dir))
}

/// `Packages/` with a capital P - the name wally hardcodes for the shared
/// realm. Every other spelling only works by accident on Windows.
pub const PACKAGES_DIR: &str = "Packages";

/// Where wally puts `[server-dependencies]`. A separate folder, not a
/// subfolder of `Packages/`, so everything that touches vendored package
/// code has to know about both.
pub const SERVER_PACKAGES_DIR: &str = "ServerPackages";
pub const DEV_PACKAGES_DIR: &str = "DevPackages";

/// Retypes the link files in whichever package folders exist.
///
/// Both folders are passed in one invocation rather than two runs: the tool
/// accepts multiple paths, and a project with no server-realm package has no
/// `ServerPackages/` at all - naming a missing directory is an error, so the
/// argument list is built from what's on disk.
pub fn wally_package_types(project_dir: &Path) -> Result<()> {
    let mut args = vec!["-s", "sourcemap.json"];
    for dir in [PACKAGES_DIR, SERVER_PACKAGES_DIR, DEV_PACKAGES_DIR] {
        if project_dir.join(dir).is_dir() {
            args.push(dir);
        }
    }
    run_in("wally-package-types", &args, Some(project_dir))
}

/// Installs this project's Wally dependencies and restores the types on the
/// generated link files. Always use this rather than `wally_install` alone.
///
/// `wally install` rewrites every link file in `packages/` from scratch, and
/// what it writes is a bare `return require(script.Parent._Index[...])` with
/// no `export type` lines at all. So an install doesn't just *fail to add*
/// types - it actively strips the ones `wally-package-types` put there on
/// the previous run, and every package silently degrades to `any`.
///
/// That made `rproj watch` destructive: it installed and went straight to
/// watching, so the documented first thing to run after `rproj new` undid
/// the scaffold's own retyping, and the only way back was typing
/// `wally-package-types -s sourcemap.json packages/` by hand.
///
/// The order is fixed by how wally-package-types works: it resolves each
/// link file's require through the sourcemap to find the real module, so
/// the packages have to be on disk before the sourcemap is generated, and
/// the sourcemap has to exist before the retyping runs.
pub fn sync(project_dir: &Path) -> Result<()> {
    sync_for_project(project_dir, "default.project.json")
}

pub fn sync_for_project(project_dir: &Path, project_file: &str) -> Result<()> {
    wally_install(project_dir)?;
    refresh(project_dir, project_file)
}

/// Refreshes generated Wally metadata for tests without rewriting an already
/// complete package tree. `wally install` replaces `_Index` directories and
/// link files; doing that while `rproj watch` has Rojo observing those paths
/// can crash the watcher during the replacement window.
/// Retyping can also reject its own previously rewritten links, so reuse their
/// types and regenerate only the sourcemap when all dependencies are present.
pub fn sync_for_test(project_dir: &Path, project_file: &str, selected: &[String]) -> Result<()> {
    if packages_ready(project_dir, selected) {
        ui::ok("Wally packages already installed");
        refresh_sourcemap(project_dir, project_file)
    } else {
        sync_for_project(project_dir, project_file)
    }
}

fn refresh(project_dir: &Path, project_file: &str) -> Result<()> {
    refresh_sourcemap(project_dir, project_file)?;
    // Safe with no packages: an empty directory is a successful no-op.
    wally_package_types(project_dir)
}

fn refresh_sourcemap(project_dir: &Path, project_file: &str) -> Result<()> {
    ensure_sync_mounts(project_dir, project_file)?;
    rojo::generate_sourcemap_from(project_dir, project_file)
}

fn packages_ready(project_dir: &Path, selected: &[String]) -> bool {
    packages_ready_from_manifest(project_dir, selected).unwrap_or(false)
}

fn packages_ready_from_manifest(project_dir: &Path, selected: &[String]) -> Option<bool> {
    let manifest: toml::Value = fs::read_to_string(project_dir.join("wally.toml"))
        .ok()
        .and_then(|source| toml::from_str(&source).ok())?;
    let lock: toml::Value = fs::read_to_string(project_dir.join("wally.lock"))
        .ok()
        .and_then(|source| toml::from_str(&source).ok())?;
    let dependencies = direct_dependencies(&manifest)?;
    let locked = locked_direct_dependencies(&manifest, &lock)?;
    if dependencies.len() != locked.len() {
        return Some(false);
    }

    for key in selected {
        let spec = wally_packages::find(key)?;
        let expected_section = realm_section(spec.realm);
        if !dependencies
            .iter()
            .any(|(section, alias, _)| *section == expected_section && *alias == manifest_key(spec))
        {
            return Some(false);
        }
    }

    Some(dependencies.iter().all(|(section, alias, source)| {
        locked
            .get(*alias)
            .is_some_and(|resolved| normalize_source(resolved) == normalize_source(source))
            && package_alias_exists(project_dir, section, alias)
    }))
}

fn direct_dependencies(manifest: &toml::Value) -> Option<Vec<(&str, &str, &str)>> {
    let mut dependencies = Vec::new();
    for section in ["dependencies", "server-dependencies", "dev-dependencies"] {
        let Some(table) = manifest.get(section) else {
            continue;
        };
        for (alias, source) in table.as_table()? {
            dependencies.push((section, alias.as_str(), source.as_str()?));
        }
    }
    Some(dependencies)
}

fn locked_direct_dependencies(
    manifest: &toml::Value,
    lock: &toml::Value,
) -> Option<BTreeMap<String, String>> {
    let package = manifest.get("package")?.as_table()?;
    let name = package.get("name")?.as_str()?;
    let version = package.get("version")?.as_str()?;
    let root = lock.get("package")?.as_array()?.iter().find_map(|entry| {
        let entry = entry.as_table()?;
        (entry.get("name")?.as_str()? == name && entry.get("version")?.as_str()? == version)
            .then_some(entry)
    })?;
    let mut dependencies = BTreeMap::new();
    for dependency in root.get("dependencies")?.as_array()? {
        let dependency = dependency.as_array()?;
        if dependency.len() != 2 {
            return None;
        }
        dependencies.insert(
            dependency[0].as_str()?.to_string(),
            dependency[1].as_str()?.to_string(),
        );
    }
    Some(dependencies)
}

fn normalize_source(source: &str) -> String {
    let Some((package, version)) = source.rsplit_once('@') else {
        return source.to_string();
    };
    format!("{package}@{}", version.strip_prefix('=').unwrap_or(version))
}

fn realm_section(realm: Realm) -> &'static str {
    match realm {
        Realm::Shared => "dependencies",
        Realm::Server => "server-dependencies",
        Realm::Dev => "dev-dependencies",
    }
}

fn package_alias_exists(project_dir: &Path, section: &str, alias: &str) -> bool {
    let directory = match section {
        "dependencies" => PACKAGES_DIR,
        "server-dependencies" => SERVER_PACKAGES_DIR,
        "dev-dependencies" => DEV_PACKAGES_DIR,
        _ => return false,
    };
    let root = project_dir.join(directory);
    root.join("_Index").is_dir()
        && ["lua", "luau"]
            .iter()
            .any(|extension| root.join(format!("{alias}.{extension}")).is_file())
}

fn ensure_sync_mounts(project_dir: &Path, project_file: &str) -> Result<()> {
    // With zero dependencies `wally install` doesn't just skip creating
    // Packages/ - it *removes* an existing empty one (verified). The
    // project file maps that path, and rojo refuses to generate a sourcemap
    // at all when a mapped $path is missing, so recreating it here is what
    // keeps a no-dependency project working.
    fs::create_dir_all(project_dir.join(PACKAGES_DIR))?;
    let source = project_dir.join(project_file);
    let mounts_dev = if source.is_file() {
        let document: serde_json::Value = serde_json::from_str(&fs::read_to_string(source)?)?;
        document
            .pointer("/tree/ReplicatedStorage/devPackages/$path")
            .and_then(serde_json::Value::as_str)
            == Some(DEV_PACKAGES_DIR)
    } else {
        false
    };
    if project_file == crate::steps::jest::PROJECT_FILE || mounts_dev {
        fs::create_dir_all(project_dir.join(DEV_PACKAGES_DIR))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jest_packages_use_exact_dev_dependencies_and_aliases() {
        let selected = vec!["jest".into(), "jest-globals".into()];
        let manifest = render_wally_toml("rproj/demo", &selected).unwrap();
        assert!(manifest.contains("[dev-dependencies]"), "{manifest}");
        assert!(
            manifest.contains("Jest = \"roblox/jest@=3.20.1\""),
            "{manifest}"
        );
        assert!(
            manifest.contains("JestGlobals = \"roblox/jest-globals@=3.20.1\""),
            "{manifest}"
        );
    }

    #[test]
    fn ordinary_dependency_output_is_unchanged() {
        let manifest = render_wally_toml("rproj/demo", &["promise".into()]).unwrap();
        assert!(manifest.contains("[dependencies]\npromise ="), "{manifest}");
        assert!(!manifest.contains("dev-dependencies"), "{manifest}");
    }

    #[test]
    fn additive_manifest_merge_preserves_unknown_entries_and_comments() {
        let existing = "# keep me\n[package]\nname = \"owner/game\"\nversion = \"1.0.0\"\nregistry = \"custom\"\nrealm = \"shared\"\n\n[dependencies]\ncustom = \"owner/custom@1.0.0\" # retained\n";
        let merged = merge_selected_contents(existing, &["testez".into()]).unwrap();
        assert!(merged.contains("# keep me"), "{merged}");
        assert!(
            merged.contains("custom = \"owner/custom@1.0.0\" # retained"),
            "{merged}"
        );
        assert!(
            merged.contains("testez = \"roblox/testez@0.4.1\""),
            "{merged}"
        );
        assert!(merged.contains("registry = \"custom\""), "{merged}");
    }

    #[test]
    fn additive_manifest_merge_uses_sections_with_trailing_comments() {
        let existing = "[package]\nname = \"owner/game\"\nversion = \"1.0.0\"\nrealm = \"shared\"\n\n[dependencies] # project packages\ncustom = \"owner/custom@1.0.0\"\n";
        let merged = merge_selected_contents(existing, &["testez".into()]).unwrap();
        assert_eq!(merged.matches("[dependencies]").count(), 1, "{merged}");
        assert!(merged.contains("testez = \"roblox/testez@0.4.1\""));
    }

    #[test]
    fn additive_manifest_merge_preserves_custom_catalog_pins_and_final_newline_state() {
        let existing = "[package]\nname = \"owner/game\"\nversion = \"1.0.0\"\nrealm = \"shared\"\n\n[dependencies]\ntestez = \"owner/fork@1.0.0\"";
        let merged = merge_selected_contents(existing, &["testez".into()]).unwrap();
        assert_eq!(merged, existing);
        assert!(!merged.ends_with('\n'));
        assert!(!merged.contains("roblox/testez"));
    }

    #[test]
    fn additive_manifest_merge_preserves_crlf_without_doubling_carriage_returns() {
        let existing = "[package]\r\nname = \"owner/game\"\r\nversion = \"1.0.0\"\r\nrealm = \"shared\"\r\n\r\n[dependencies]\r\n";
        let merged = merge_selected_contents(existing, &["testez".into()]).unwrap();
        assert!(!merged.contains("\r\r\n"), "{merged:?}");
        assert!(!merged.replace("\r\n", "").contains('\n'), "{merged:?}");
        assert!(merged.ends_with("\r\n"), "{merged:?}");
    }

    #[test]
    fn additive_manifest_merge_rejects_unknown_keys_and_wrong_realms() {
        let base = "[package]\nname = \"owner/game\"\nversion = \"1.0.0\"\nrealm = \"shared\"\n";
        let unknown = merge_selected_contents(base, &["missing".into()])
            .unwrap_err()
            .to_string();
        assert!(unknown.contains("unknown selected Wally package `missing`"));

        let wrong_realm = format!("{base}\n[dependencies]\nprofilestore = \"owner/fork@1.0.0\"\n");
        let error = merge_selected_contents(&wrong_realm, &["profilestore".into()])
            .unwrap_err()
            .to_string();
        assert!(error.contains("requires [server-dependencies]"), "{error}");
        assert!(error.contains("[dependencies]"), "{error}");
    }

    #[test]
    fn failed_atomic_manifest_replacement_preserves_the_existing_target() {
        let root = tempfile::tempdir().unwrap();
        let blocked = root.path().join("wally.toml");
        fs::create_dir(&blocked).unwrap();
        fs::write(blocked.join("owned"), "preserve").unwrap();
        assert!(replace_file(&blocked, b"replacement").is_err());
        assert_eq!(
            fs::read_to_string(blocked.join("owned")).unwrap(),
            "preserve"
        );
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    }

    #[test]
    fn jest_sync_recognizes_the_development_package_mount() {
        let project = tempfile::tempdir().unwrap();
        ensure_sync_mounts(project.path(), crate::steps::jest::PROJECT_FILE).unwrap();
        assert!(project.path().join(PACKAGES_DIR).is_dir());
        assert!(project.path().join(DEV_PACKAGES_DIR).is_dir());
    }

    #[test]
    fn default_sync_recognizes_development_packages_without_jest_config() {
        let project = tempfile::tempdir().unwrap();
        fs::write(
            project.path().join("default.project.json"),
            r#"{"tree":{"ReplicatedStorage":{"devPackages":{"$path":"DevPackages"}}}}"#,
        )
        .unwrap();
        ensure_sync_mounts(project.path(), "default.project.json").unwrap();
        assert!(project.path().join(DEV_PACKAGES_DIR).is_dir());
    }

    #[test]
    fn test_sync_recognizes_complete_shared_and_development_aliases() {
        let project = tempfile::tempdir().unwrap();
        fs::write(
            project.path().join("wally.toml"),
            "[package]\nname = \"rproj/demo\"\nversion = \"0.1.0\"\nrealm = \"shared\"\n\n[dependencies]\ntestez = \"roblox/testez@0.4.1\"\n\n[dev-dependencies]\nJest = \"roblox/jest@=3.20.1\"\n",
        )
        .unwrap();
        fs::write(
            project.path().join("wally.lock"),
            "[[package]]\nname = \"rproj/demo\"\nversion = \"0.1.0\"\ndependencies = [[\"testez\", \"roblox/testez@0.4.1\"], [\"Jest\", \"roblox/jest@3.20.1\"]]\n",
        )
        .unwrap();
        for (directory, alias) in [(PACKAGES_DIR, "testez"), (DEV_PACKAGES_DIR, "Jest")] {
            fs::create_dir_all(project.path().join(directory).join("_Index")).unwrap();
            fs::write(
                project.path().join(directory).join(format!("{alias}.luau")),
                "",
            )
            .unwrap();
        }
        assert!(packages_ready(
            project.path(),
            &["testez".into(), "jest".into()]
        ));
        fs::remove_file(project.path().join(DEV_PACKAGES_DIR).join("Jest.luau")).unwrap();
        assert!(!packages_ready(project.path(), &["jest".into()]));
    }

    #[test]
    fn test_sync_reinstalls_when_manifest_lock_or_custom_aliases_are_stale() {
        let project = tempfile::tempdir().unwrap();
        fs::write(
            project.path().join("wally.toml"),
            "[package]\nname = \"rproj/demo\"\nversion = \"0.1.0\"\nrealm = \"shared\"\n\n[dependencies]\ntestez = \"roblox/testez@0.4.2\"\ncustom = \"owner/custom@1.0.0\"\n",
        )
        .unwrap();
        fs::write(
            project.path().join("wally.lock"),
            "[[package]]\nname = \"rproj/demo\"\nversion = \"0.1.0\"\ndependencies = [[\"testez\", \"roblox/testez@0.4.1\"], [\"custom\", \"owner/custom@1.0.0\"]]\n",
        )
        .unwrap();
        fs::create_dir_all(project.path().join(PACKAGES_DIR).join("_Index")).unwrap();
        for alias in ["testez", "custom"] {
            fs::write(
                project
                    .path()
                    .join(PACKAGES_DIR)
                    .join(format!("{alias}.lua")),
                "",
            )
            .unwrap();
        }
        assert!(!packages_ready(project.path(), &["testez".into()]));

        fs::write(
            project.path().join("wally.lock"),
            "[[package]]\nname = \"rproj/demo\"\nversion = \"0.1.0\"\ndependencies = [[\"testez\", \"roblox/testez@0.4.2\"], [\"custom\", \"owner/custom@1.0.0\"]]\n",
        )
        .unwrap();
        fs::remove_file(project.path().join(PACKAGES_DIR).join("custom.lua")).unwrap();
        assert!(!packages_ready(project.path(), &["testez".into()]));
    }
}
