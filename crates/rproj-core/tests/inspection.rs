use std::fs;
use std::path::Path;

use rproj_core::{FileState, PresenceState, ProjectKind, Realm, ResolutionState, inspect};
use tempfile::TempDir;

const MANIFEST: &str = "[package]\nname = 'owner/game'\nversion = '0.1.0'\nrealm = 'shared'\n";

fn write(root: &Path, name: &str, contents: &str) {
    let path = root.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

fn project() -> TempDir {
    let directory = TempDir::new().unwrap();
    write(
        directory.path(),
        "default.project.json",
        r#"{"name":"Example","tree":{"$className":"DataModel"}}"#,
    );
    directory
}

#[test]
fn plain_rojo_project_is_inspected_without_adoption_or_writes() {
    let directory = project();
    let path = directory.path().join("default.project.json");
    let before = fs::read(&path).unwrap();
    let modified = fs::metadata(&path).unwrap().modified().unwrap();
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    let original_permissions = permissions.clone();
    permissions.set_readonly(true);
    fs::set_permissions(&path, permissions).unwrap();

    let snapshot = inspect(directory.path()).unwrap();

    fs::set_permissions(&path, original_permissions).unwrap();
    assert_eq!(snapshot.root, directory.path().canonicalize().unwrap());
    assert_eq!(snapshot.schema_version, 1);
    assert_eq!(snapshot.kind, ProjectKind::Rojo);
    assert_eq!(snapshot.name.as_deref(), Some("Example"));
    assert!(snapshot.actions.watch.available);
    assert!(!snapshot.actions.check.available);
    assert!(!snapshot.actions.test.available);
    assert_eq!(snapshot.files.rproj.state, FileState::Missing);
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), modified);
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[test]
fn all_realms_and_manual_unknown_packages_are_authoritative() {
    let directory = project();
    write(directory.path(), "rproj.toml", "packages = ['reflex']\n");
    write(
        directory.path(),
        "wally.toml",
        &format!(
            "{MANIFEST}\n[dependencies]\nCustomAlias = 'unknown-scope/unlisted-package@^2.0.0'\n[server-dependencies]\nStorage = 'owner/storage@=3.0.0'\n[dev-dependencies]\nJest = 'roblox/jest@3.20.1'\n"
        ),
    );
    let snapshot = inspect(directory.path()).unwrap();
    assert_eq!(snapshot.kind, ProjectKind::Rproj);
    assert_eq!(snapshot.saved_packages, ["reflex"]);
    assert_eq!(snapshot.packages.len(), 3);
    let custom = &snapshot.packages[0];
    assert_eq!(custom.alias, "CustomAlias");
    assert_eq!(
        custom.package.as_deref(),
        Some("unknown-scope/unlisted-package")
    );
    assert_eq!(custom.version_requirement.as_deref(), Some("^2.0.0"));
    assert_eq!(custom.realm, Realm::Shared);
    assert_eq!(custom.resolution, ResolutionState::Unlocked);
    assert_eq!(snapshot.packages[1].realm, Realm::Server);
    assert_eq!(snapshot.packages[2].realm, Realm::Dev);
    assert!(
        snapshot
            .packages
            .iter()
            .all(|package| package.presence.state == PresenceState::Missing)
    );
}

#[test]
fn lock_resolution_and_file_presence_are_independent() {
    let directory = project();
    write(
        directory.path(),
        "wally.toml",
        &format!(
            "{MANIFEST}\n[dependencies]\nGood = 'owner/good@^1.0.0'\nStale = 'owner/stale@=2.0.0'\nAdded = 'owner/added@1.0.0'\n"
        ),
    );
    write(
        directory.path(),
        "wally.lock",
        concat!(
            "[[package]]\nname = 'owner/game'\nversion = '0.1.0'\ndependencies = [['Good', 'owner/good@1.2.0'], ['Stale', 'owner/stale@1.0.0'], ['Removed', 'owner/removed@1.0.0']]\n",
            "[[package]]\nname = 'owner/good'\nversion = '1.2.0'\n",
            "[[package]]\nname = 'owner/stale'\nversion = '1.0.0'\n",
            "[[package]]\nname = 'owner/removed'\nversion = '1.0.0'\n",
        ),
    );
    write(directory.path(), "Packages/Stale.luau", "return nil\n");
    fs::create_dir_all(directory.path().join("Packages/_Index")).unwrap();
    let snapshot = inspect(directory.path()).unwrap();
    let good = snapshot
        .packages
        .iter()
        .find(|package| package.alias == "Good")
        .unwrap();
    let stale = snapshot
        .packages
        .iter()
        .find(|package| package.alias == "Stale")
        .unwrap();
    let added = snapshot
        .packages
        .iter()
        .find(|package| package.alias == "Added")
        .unwrap();
    assert_eq!(good.resolution, ResolutionState::Resolved);
    assert_eq!(good.resolved.as_ref().unwrap().version, "1.2.0");
    assert_eq!(good.presence.state, PresenceState::Missing);
    assert_eq!(stale.resolution, ResolutionState::Stale);
    assert_eq!(stale.presence.state, PresenceState::Present);
    assert_eq!(added.resolution, ResolutionState::Stale);
    assert!(added.resolved.is_none());
    assert_eq!(snapshot.packages.len(), 3);
    assert!(
        snapshot
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "removed_dependency")
    );
}

#[test]
fn malformed_files_are_diagnostics_not_an_empty_project() {
    let directory = project();
    write(directory.path(), "rproj.toml", "packages = 4\n");
    write(directory.path(), "default.project.json", "{");
    write(directory.path(), "wally.toml", "[dependencies\n");
    write(
        directory.path(),
        "wally.lock",
        "[package]\nname = 'not-an-array'\n",
    );
    write(directory.path(), "rokit.toml", "tools = 12\n");
    let snapshot = inspect(directory.path()).unwrap();
    for file in [
        &snapshot.files.rproj,
        &snapshot.files.rojo,
        &snapshot.files.wally,
        &snapshot.files.wally_lock,
        &snapshot.files.rokit,
    ] {
        assert_eq!(file.state, FileState::Invalid, "{}", file.path.display());
        assert!(
            snapshot
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.path == file.path)
        );
    }
    assert_eq!(snapshot.kind, ProjectKind::Unrecognized);
    assert!(!snapshot.actions.watch.available);
    assert!(!snapshot.actions.test.available);
}

#[test]
fn empty_manifest_dependencies_are_distinct_from_absent_or_invalid() {
    let directory = project();
    write(directory.path(), "wally.toml", MANIFEST);
    let snapshot = inspect(directory.path()).unwrap();
    assert_eq!(snapshot.files.wally.state, FileState::Ready);
    assert!(snapshot.packages.is_empty());
    assert!(
        !snapshot
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.path == snapshot.files.wally.path)
    );
    fs::remove_file(directory.path().join("wally.toml")).unwrap();
    let snapshot = inspect(directory.path()).unwrap();
    assert_eq!(snapshot.files.wally.state, FileState::Missing);
    write(directory.path(), "wally.toml", "");
    let snapshot = inspect(directory.path()).unwrap();
    assert_eq!(snapshot.files.wally.state, FileState::Invalid);
}

#[test]
fn all_realms_check_their_actual_alias_files_and_index() {
    let directory = project();
    write(
        directory.path(),
        "wally.toml",
        &format!(
            "{MANIFEST}\n[dependencies]\nShared = 'owner/shared@1'\n[server-dependencies]\nServer = 'owner/server@1'\n[dev-dependencies]\nDev = 'owner/dev@1'\n"
        ),
    );
    write(directory.path(), "Packages/Shared.lua", "");
    write(directory.path(), "ServerPackages/Server.luau", "");
    write(directory.path(), "DevPackages/Dev.luau", "");
    for directory_name in ["Packages", "ServerPackages", "DevPackages"] {
        fs::create_dir_all(directory.path().join(directory_name).join("_Index")).unwrap();
    }
    let snapshot = inspect(directory.path()).unwrap();
    assert!(
        snapshot
            .packages
            .iter()
            .all(|package| package.presence.state == PresenceState::Present)
    );
    fs::remove_dir(directory.path().join("DevPackages/_Index")).unwrap();
    let snapshot = inspect(directory.path()).unwrap();
    assert_eq!(snapshot.packages[2].presence.state, PresenceState::Missing);
    assert!(snapshot.packages[2].presence.path.is_some());
}

#[test]
fn lockfiles_from_another_project_cannot_resolve_current_dependencies() {
    let directory = project();
    write(
        directory.path(),
        "wally.toml",
        &format!("{MANIFEST}\n[dependencies]\nPackage = 'owner/package@1'\n"),
    );
    write(
        directory.path(),
        "wally.lock",
        concat!(
            "[[package]]\nname = 'owner/other'\nversion = '0.1.0'\ndependencies = [['Package', 'owner/package@1.0.0']]\n",
            "[[package]]\nname = 'owner/package'\nversion = '1.0.0'\n",
        ),
    );
    let snapshot = inspect(directory.path()).unwrap();
    assert_eq!(snapshot.packages[0].resolution, ResolutionState::Stale);
    assert!(snapshot.packages[0].resolved.is_none());
    assert!(
        snapshot
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "stale_lock")
    );
}

#[test]
fn broken_lock_and_dependency_values_preserve_visible_declarations() {
    let directory = project();
    write(
        directory.path(),
        "wally.toml",
        &format!(
            "{MANIFEST}\n[dependencies]\nGood = 'owner/good@1'\nBroken = 17\n'../outside' = 'owner/outside@1'\n"
        ),
    );
    write(
        directory.path(),
        "wally.lock",
        "[[package]]\nname = 'owner/game'\nversion = '0.1.0'\ndependencies = [['Good', 3]]\n",
    );
    let snapshot = inspect(directory.path()).unwrap();
    assert_eq!(snapshot.packages.len(), 3);
    assert_eq!(snapshot.files.wally.state, FileState::Invalid);
    assert_eq!(snapshot.files.wally_lock.state, FileState::Invalid);
    let unsafe_alias = snapshot
        .packages
        .iter()
        .find(|package| package.alias == "../outside")
        .unwrap();
    assert_eq!(unsafe_alias.presence.state, PresenceState::Unknown);
    assert!(unsafe_alias.presence.path.is_none());
    assert!(
        snapshot
            .packages
            .iter()
            .all(|package| package.resolution == ResolutionState::Unknown)
    );
}

#[test]
fn known_configuration_enables_actions_without_executing_them() {
    let directory = project();
    write(
        directory.path(),
        "rproj.toml",
        "package_workflow = 'wally'\n[capabilities]\ntest = 'testez'\n",
    );
    write(
        directory.path(),
        ".lute/check.luau",
        "error('Inspection must never run me')\n",
    );
    write(
        directory.path(),
        "rokit.toml",
        "[tools]\nrojo = 'rojo-rbx/rojo@7.5.1'\n",
    );
    let snapshot = inspect(directory.path()).unwrap();
    assert!(snapshot.actions.check.available);
    assert!(snapshot.actions.test.available);
    assert!(snapshot.actions.watch.available);
    assert_eq!(snapshot.tools[0].alias, "rojo");
    assert_eq!(snapshot.tools[0].source, "rojo-rbx/rojo@7.5.1");
    assert_eq!(
        snapshot.tools[0].version_requirement.as_deref(),
        Some("7.5.1")
    );
    let json = serde_json::to_value(snapshot).unwrap();
    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["files"]["rokit"]["state"], "ready");
}

#[test]
fn incompatible_or_incomplete_jest_configuration_disables_execution() {
    let directory = project();
    for workflow in ["none", "wally"] {
        write(
            directory.path(),
            "rproj.toml",
            &format!("package_workflow = '{workflow}'\n[capabilities]\ntest = 'jest-roblox'\n"),
        );
        let snapshot = inspect(directory.path()).unwrap();
        assert!(!snapshot.actions.test.available);
        assert!(!snapshot.actions.watch.available);
        assert!(snapshot.actions.test.reason.unwrap().contains("Wally") || workflow == "wally");
    }
}

#[test]
fn missing_roots_and_non_directories_fail_but_empty_directories_are_inspectable() {
    let directory = TempDir::new().unwrap();
    let snapshot = inspect(directory.path()).unwrap();
    assert_eq!(snapshot.kind, ProjectKind::Unrecognized);
    assert_eq!(snapshot.files.rojo.state, FileState::Missing);
    assert!(!snapshot.actions.watch.available);
    assert!(inspect(&directory.path().join("missing")).is_err());
    write(directory.path(), "file", "content");
    assert!(inspect(&directory.path().join("file")).is_err());
}

#[test]
fn files_replaced_by_directories_are_unreadable_not_missing() {
    let directory = project();
    fs::create_dir(directory.path().join("rproj.toml")).unwrap();
    let snapshot = inspect(directory.path()).unwrap();
    assert_eq!(snapshot.files.rproj.state, FileState::Unreadable);
    assert!(!snapshot.actions.watch.available);
}

#[test]
fn repeated_aliases_across_realms_resolve_without_losing_packages() {
    let directory = project();
    write(
        directory.path(),
        "wally.toml",
        &format!(
            "{MANIFEST}\n[dependencies]\nSame = 'owner/client@1'\n[server-dependencies]\nSame = 'owner/server@2'\n"
        ),
    );
    write(
        directory.path(),
        "wally.lock",
        concat!(
            "[[package]]\nname = 'owner/game'\nversion = '0.1.0'\ndependencies = [['Same', 'owner/client@1.2.0'], ['Same', 'owner/server@2.1.0']]\n",
            "[[package]]\nname = 'owner/client'\nversion = '1.2.0'\n",
            "[[package]]\nname = 'owner/server'\nversion = '2.1.0'\n",
        ),
    );
    let snapshot = inspect(directory.path()).unwrap();
    assert_eq!(snapshot.files.wally_lock.state, FileState::Ready);
    assert_eq!(snapshot.packages.len(), 2);
    assert_eq!(
        snapshot.packages[0].resolved.as_ref().unwrap().version,
        "1.2.0"
    );
    assert_eq!(
        snapshot.packages[1].resolved.as_ref().unwrap().version,
        "2.1.0"
    );
    assert!(
        snapshot
            .packages
            .iter()
            .all(|dependency| dependency.resolution == ResolutionState::Resolved)
    );
}

#[test]
fn ambiguous_cross_realm_resolutions_are_unknown_instead_of_guessed() {
    let directory = project();
    write(
        directory.path(),
        "wally.toml",
        &format!(
            "{MANIFEST}\n[dependencies]\nSame = 'owner/package@1'\n[server-dependencies]\nSame = 'owner/package@^1.1'\n"
        ),
    );
    write(
        directory.path(),
        "wally.lock",
        concat!(
            "[[package]]\nname = 'owner/game'\nversion = '0.1.0'\ndependencies = [['Same', 'owner/package@1.2.0'], ['Same', 'owner/package@1.3.0']]\n",
            "[[package]]\nname = 'owner/package'\nversion = '1.2.0'\n",
            "[[package]]\nname = 'owner/package'\nversion = '1.3.0'\n",
        ),
    );
    let snapshot = inspect(directory.path()).unwrap();
    assert!(
        snapshot
            .packages
            .iter()
            .all(
                |dependency| dependency.resolution == ResolutionState::Unknown
                    && dependency.resolved.is_none()
            )
    );
    assert!(
        snapshot
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "ambiguous_resolution")
    );
}

#[test]
fn malformed_transitive_lock_entries_are_visible_even_without_a_manifest() {
    let directory = project();
    write(
        directory.path(),
        "wally.lock",
        "[[package]]\nname = 'owner/game'\nversion = '0.1.0'\ndependencies = []\n[[package]]\nname = 'owner/broken'\nversion = 3\ndependencies = []\n",
    );
    let snapshot = inspect(directory.path()).unwrap();
    assert_eq!(snapshot.files.wally_lock.state, FileState::Invalid);
}

#[test]
fn legacy_testez_selection_and_open_cloud_jest_keep_existing_behavior() {
    let directory = project();
    write(directory.path(), "rproj.toml", "packages = ['testez']\n");
    assert!(inspect(directory.path()).unwrap().actions.test.available);
    write(
        directory.path(),
        "rproj.toml",
        "[capabilities]\ntest = 'test'\n",
    );
    assert!(inspect(directory.path()).unwrap().actions.test.available);
    write(
        directory.path(),
        "rproj.toml",
        "[capabilities]\ntest = 'jest-roblox-open-cloud'\n",
    );
    assert!(!inspect(directory.path()).unwrap().actions.test.available);
    write(directory.path(), "wally.toml", MANIFEST);
    assert!(inspect(directory.path()).unwrap().actions.test.available);
}

#[test]
fn syntax_diagnostics_do_not_echo_unrelated_secret_values() {
    let directory = project();
    write(
        directory.path(),
        "wally.toml",
        "token = 'private-fixture-value\n",
    );
    write(
        directory.path(),
        "rproj.toml",
        "package_workflow = 'another-private-fixture-value'\n",
    );
    let snapshot = inspect(directory.path()).unwrap();
    assert_eq!(snapshot.files.wally.state, FileState::Invalid);
    assert_eq!(snapshot.files.rproj.state, FileState::Invalid);
    let json = serde_json::to_string(&snapshot).unwrap();
    assert!(!json.contains("private-fixture-value"));
    assert!(json.contains("line 1"));
}

#[test]
fn oversized_configuration_is_reported_without_loading_it() {
    let directory = project();
    let path = directory.path().join("wally.toml");
    fs::File::create(path)
        .unwrap()
        .set_len(16 * 1024 * 1024 + 1)
        .unwrap();
    let snapshot = inspect(directory.path()).unwrap();
    assert_eq!(snapshot.files.wally.state, FileState::Unreadable);
    assert!(
        snapshot
            .diagnostics
            .iter()
            .any(|d| d.message.contains("16 MiB limit"))
    );
}

#[test]
fn missing_direct_and_transitive_lock_targets_are_not_reported_resolved() {
    for extra_entries in [
        "",
        "[[package]]\nname = 'owner/parent'\nversion = '2.0.0'\n",
        "[[package]]\nname = 'owner/parent'\nversion = '1.0.0'\ndependencies = [['Child', 'owner/child@1.0.0']]\n",
    ] {
        let directory = project();
        write(
            directory.path(),
            "wally.toml",
            &format!("{MANIFEST}\n[dependencies]\nParent = 'owner/parent@1.0.0'\n"),
        );
        let lock = format!(
            "[[package]]\nname = 'owner/game'\nversion = '0.1.0'\ndependencies = [['Parent', 'owner/parent@1.0.0']]\n{extra_entries}"
        );
        write(directory.path(), "wally.lock", &lock);
        let snapshot = inspect(directory.path()).unwrap();
        assert_eq!(snapshot.files.wally_lock.state, FileState::Invalid);
        assert_eq!(snapshot.packages[0].resolution, ResolutionState::Unknown);
        assert!(snapshot.packages[0].resolved.is_none());
        assert!(
            snapshot
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "missing_locked_package")
        );
        assert_eq!(
            fs::read_to_string(directory.path().join("wally.lock")).unwrap(),
            lock
        );
    }
}

#[test]
fn empty_or_incomplete_tool_pins_stay_visible_with_invalid_diagnostics() {
    for source in [
        "",
        " ",
        "owner/repo",
        "owner/repo@",
        "owner/repo@ ",
        "@1.0.0",
        "owner/repo@1@2",
    ] {
        let directory = project();
        let manifest = format!("[tools]\nrojo = {source:?}\n");
        write(directory.path(), "rokit.toml", &manifest);
        let snapshot = inspect(directory.path()).unwrap();
        assert_eq!(snapshot.files.rokit.state, FileState::Invalid, "{source}");
        assert_eq!(snapshot.tools.len(), 1);
        assert_eq!(snapshot.tools[0].source, source);
        assert!(snapshot.tools[0].version_requirement.is_none());
        assert!(
            snapshot
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "invalid_tool")
        );
        assert!(!snapshot.actions.watch.available);
        assert_eq!(
            fs::read_to_string(directory.path().join("rokit.toml")).unwrap(),
            manifest
        );
    }
}

#[test]
fn unknown_tool_sources_and_rokit_provider_prefixes_remain_visible() {
    for (source, version) in [
        ("unknown-org/unlisted@1.0.0", "1.0.0"),
        ("github:Owner/Tool@6.9", "6.9"),
        (" Owner / Tool @ 1.2.3 ", "1.2.3"),
    ] {
        let directory = project();
        write(
            directory.path(),
            "rokit.toml",
            &format!("[tools]\nunlisted = {source:?}\n"),
        );
        let snapshot = inspect(directory.path()).unwrap();
        assert_eq!(snapshot.files.rokit.state, FileState::Ready);
        assert_eq!(snapshot.tools[0].source, source);
        assert_eq!(
            snapshot.tools[0].version_requirement.as_deref(),
            Some(version)
        );
    }
}
