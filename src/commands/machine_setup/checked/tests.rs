use super::*;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use worker::Status;

fn fixture() -> (tempfile::TempDir, SetupReview) {
    let root = tempfile::tempdir().unwrap();
    let context = capture_at(
        &root.path().join("config space/config.toml"),
        &root.path().join("Projects 日本語"),
    )
    .unwrap();
    let review = prepare(&context, &context.saved).unwrap();
    (root, review)
}

fn quiet() -> Arc<dyn Fn(Event) + Send + Sync> {
    Arc::new(|_| {})
}

#[test]
fn preparation_is_read_only_and_preserves_missing_paths() {
    let (root, review) = fixture();
    assert!(review.context.original.is_none());
    assert!(!review.context.config_path.exists());
    assert!(!review.selection.root.exists());
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    assert_eq!(review.items, vec![Item::Rokit, Item::ProjectsFolder]);
}

#[test]
fn tampered_order_root_and_unknown_choices_are_refused_before_effects() {
    let (_root, original) = fixture();
    for variant in 0..3 {
        let mut review = original.clone();
        match variant {
            0 => review.items.reverse(),
            1 => review.selection.root.push("other"),
            _ => {
                review.selection.apps.insert("unknown-app".into());
            }
        }
        assert!(
            execute_with(
                &review,
                &|| false,
                quiet(),
                &mut || panic!("mutation"),
                |_, _, _| panic!("installer"),
                |_| Ok(())
            )
            .is_err()
        );
    }
}

#[test]
fn changed_config_missing_target_and_directory_identity_require_reload() {
    for variant in 0..3 {
        let (root, mut review) = fixture();
        match variant {
            0 => {
                fs::create_dir(review.context.config_path.parent().unwrap()).unwrap();
                fs::write(&review.context.config_path, "last_checked='external'").unwrap();
            }
            1 => {
                fs::create_dir(&review.selection.root).unwrap();
            }
            _ => {
                let parent = root.path().join("existing");
                fs::create_dir(&parent).unwrap();
                let context =
                    capture_at(&parent.join("config.toml"), &review.selection.root).unwrap();
                review = prepare(&context, &context.saved).unwrap();
                fs::rename(&parent, root.path().join("old")).unwrap();
                fs::create_dir(&parent).unwrap();
            }
        }
        assert!(
            execute_with(
                &review,
                &|| false,
                quiet(),
                &mut || panic!("mutation"),
                |_, _, _| panic!("installer"),
                |_| Ok(())
            )
            .is_err()
        );
    }
}

#[test]
fn prerequisite_failure_never_claims_machine_changes() {
    let (_root, review) = fixture();
    let error = execute_with(
        &review,
        &|| false,
        quiet(),
        &mut || panic!("mutation"),
        |_, _, _| panic!("installer"),
        |_| anyhow::bail!("missing prerequisite"),
    )
    .unwrap_err();
    assert!(error.to_string().contains("missing prerequisite"));
}

#[test]
fn successful_run_records_before_installer_and_saves_exact_baseline() {
    let (_root, review) = fixture();
    let begun = AtomicBool::new(false);
    let result = execute_with(
        &review,
        &|| false,
        quiet(),
        &mut || {
            begun.store(true, Ordering::SeqCst);
            Ok(())
        },
        |item, selection, _| {
            assert!(begun.load(Ordering::SeqCst));
            if *item == Item::ProjectsFolder {
                fs::create_dir(&selection.root)?;
            }
            Ok(())
        },
        |_| Ok(()),
    )
    .unwrap();
    assert_eq!(result.outcome, Outcome::Completed);
    let Persistence::Saved { contents } = result.persistence else {
        panic!("not saved: {:?}", result.persistence)
    };
    assert_eq!(
        fs::read_to_string(&review.context.config_path).unwrap(),
        contents
    );
    assert!(
        toml::from_str::<GlobalConfig>(&contents)
            .unwrap()
            .machine_configured()
    );
}

#[test]
fn stop_finishes_active_item_skips_remaining_and_does_not_save() {
    let (_root, review) = fixture();
    let stop = AtomicBool::new(false);
    let ran = AtomicUsize::new(0);
    let result = execute_with(
        &review,
        &|| stop.load(Ordering::SeqCst),
        quiet(),
        &mut || Ok(()),
        |_, _, _| {
            ran.fetch_add(1, Ordering::SeqCst);
            stop.store(true, Ordering::SeqCst);
            Ok(())
        },
        |_| Ok(()),
    )
    .unwrap();
    assert_eq!(ran.load(Ordering::SeqCst), 1);
    assert_eq!(result.outcome, Outcome::Cancelled);
    assert_eq!(result.statuses, vec![Status::Installed, Status::Skipped]);
    assert_eq!(result.persistence, Persistence::NotSaved);
    assert!(!review.context.config_path.exists());
}

#[test]
fn stop_before_mutation_has_no_effects() {
    let (_root, review) = fixture();
    let result = execute_with(
        &review,
        &|| true,
        quiet(),
        &mut || panic!("mutation"),
        |_, _, _| panic!("installer"),
        |_| Ok(()),
    )
    .unwrap();
    assert_eq!(result.outcome, Outcome::Cancelled);
    assert!(!result.changes_may_have_begun);
    assert_eq!(result.statuses, vec![Status::Skipped; 2]);
}

#[test]
fn fatal_and_warning_results_have_different_persistence() {
    for fatal in [false, true] {
        let (_root, mut review) = fixture();
        review.selection.tools.insert("rojo".into());
        review = prepare(&review.context, &review.selection).unwrap();
        let result = execute_with(
            &review,
            &|| false,
            quiet(),
            &mut || Ok(()),
            |item, _, _| {
                if fatal || matches!(item, Item::Tool(_)) {
                    anyhow::bail!("fake installer failed");
                }
                Ok(())
            },
            |_| Ok(()),
        )
        .unwrap();
        assert_eq!(
            result.outcome,
            if fatal {
                Outcome::Failed
            } else {
                Outcome::Warnings
            }
        );
        assert_eq!(
            review.context.config_path.exists(),
            !fatal,
            "{:?}",
            result.persistence
        );
        assert_eq!(
            matches!(result.persistence, Persistence::Saved { .. }),
            !fatal
        );
    }
}

#[test]
fn configuration_changed_during_installation_is_not_overwritten() {
    let (_root, review) = fixture();
    let result = execute_with(
        &review,
        &|| false,
        quiet(),
        &mut || Ok(()),
        |item, _, _| {
            if *item == Item::ProjectsFolder {
                fs::create_dir(review.context.config_path.parent().unwrap())?;
                fs::write(&review.context.config_path, "last_checked='external'")?;
            }
            Ok(())
        },
        |_| Ok(()),
    )
    .unwrap();
    assert_eq!(result.outcome, Outcome::Completed);
    assert!(matches!(result.persistence, Persistence::Failed { .. }));
    assert!(result.changes_may_have_begun);
    assert_eq!(
        fs::read_to_string(&review.context.config_path).unwrap(),
        "last_checked='external'"
    );
}

#[test]
fn execution_lock_excludes_a_second_setup_and_releases_on_drop() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("setup.lock");
    let first = SetupLock::acquire(&path).unwrap();
    assert!(SetupLock::acquire(&path).is_err());
    drop(first);
    assert!(SetupLock::acquire(&path).is_ok());
}

#[test]
fn warning_cannot_be_erased_by_later_already_message() {
    let (_root, review) = fixture();
    let result = execute_with(
        &review,
        &|| false,
        quiet(),
        &mut || Ok(()),
        |_, _, reporter| {
            reporter.message(crate::steps::execution::MessageKind::Warning, "failed");
            reporter.message(
                crate::steps::execution::MessageKind::Already,
                "another component present",
            );
            Ok(())
        },
        |_| Ok(()),
    )
    .unwrap();
    assert_eq!(result.outcome, Outcome::Failed);
    assert_eq!(result.statuses, vec![Status::Failed, Status::Skipped]);
    assert_eq!(result.persistence, Persistence::NotSaved);
}

#[test]
fn stop_at_the_persistence_boundary_keeps_completed_installations_unsaved() {
    let (_root, review) = fixture();
    let parent = review.context.config_path.parent().unwrap();
    let result = execute_with(
        &review,
        &|| parent.exists(),
        quiet(),
        &mut || Ok(()),
        |_, _, _| Ok(()),
        |_| Ok(()),
    )
    .unwrap();
    assert_eq!(result.outcome, Outcome::Cancelled);
    assert_eq!(result.statuses, vec![Status::Installed; 2]);
    assert_eq!(result.persistence, Persistence::NotSaved);
    assert!(!review.context.config_path.exists());
    assert_eq!(fs::read_dir(parent).unwrap().count(), 0);
}

#[test]
fn existing_unknown_and_inactive_choices_survive_a_completed_save() {
    let root = tempfile::tempdir().unwrap();
    let config = GlobalConfig {
        roblox_projects_root: Some(root.path().join("projects")),
        selected_studio_plugins: vec!["future-plugin".into(), "blender-plugin".into()],
        selected_vscode_extensions: vec!["theme-catppuccin".into()],
        last_checked: Some("old".into()),
        ..Default::default()
    };
    let path = root.path().join("config.toml");
    fs::write(&path, toml::to_string(&config).unwrap()).unwrap();
    let context = capture_at(&path, &root.path().join("unused")).unwrap();
    let review = prepare(&context, &context.saved).unwrap();
    assert_eq!(review.items, vec![Item::Rokit, Item::ProjectsFolder]);
    let result = execute_with(
        &review,
        &|| false,
        quiet(),
        &mut || Ok(()),
        |_, _, _| Ok(()),
        |_| Ok(()),
    )
    .unwrap();
    assert!(matches!(result.persistence, Persistence::Saved { .. }));
    let saved: GlobalConfig = toml::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(Selection::load(&saved).unwrap(), context.saved);
}

#[test]
fn missing_projects_folder_appearing_during_bootstrap_is_not_adopted() {
    let (_root, review) = fixture();
    let result = execute_with(
        &review,
        &|| false,
        quiet(),
        &mut || Ok(()),
        |item, selection, _| {
            assert_eq!(*item, Item::Rokit);
            fs::create_dir(&selection.root)?;
            Ok(())
        },
        |_| Ok(()),
    )
    .unwrap();
    assert_eq!(result.outcome, Outcome::Failed);
    assert_eq!(result.persistence, Persistence::NotSaved);
}

#[cfg(windows)]
#[test]
fn linked_machine_paths_are_refused() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("target");
    let link = root.path().join("link");
    fs::create_dir(&target).unwrap();
    let status = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(&link)
        .arg(&target)
        .output()
        .unwrap();
    assert!(status.status.success());
    assert!(capture_at(&link.join("config.toml"), &root.path().join("projects")).is_err());
    assert!(capture_at(&root.path().join("config.toml"), &link).is_err());
    fs::remove_dir(&link).unwrap();
    assert!(target.is_dir());
}

#[cfg(windows)]
#[test]
fn any_observed_watch_blocks_execution_but_not_preparation() {
    let (root, review) = fixture();
    let project = root.path().join("other-project");
    fs::create_dir(&project).unwrap();
    crate::background_watch::with_unresponsive_fixture(&project, || {
        assert!(prepare(&review.context, &review.selection).is_ok());
        let error = guarded(&root.path().join("setup.lock"), || -> Result<()> {
            panic!("installer must not start")
        })
        .unwrap_err();
        assert!(error.to_string().contains("Watch"));
    });
}

#[test]
fn explicit_reload_allows_a_second_checked_run_without_rebasing_a_stale_review() {
    let (_root, review) = fixture();
    let first = execute_with(
        &review,
        &|| false,
        quiet(),
        &mut || Ok(()),
        |item, selection, _| {
            if *item == Item::ProjectsFolder {
                fs::create_dir(&selection.root)?;
            }
            Ok(())
        },
        |_| Ok(()),
    )
    .unwrap();
    assert!(matches!(first.persistence, Persistence::Saved { .. }));
    assert!(prepare(&review.context, &review.selection).is_err());
    let fresh = capture_at(&review.context.config_path, &review.selection.root).unwrap();
    let reviewed = prepare(&fresh, &fresh.saved).unwrap();
    let second = execute_with(
        &reviewed,
        &|| false,
        quiet(),
        &mut || Ok(()),
        |_, _, _| Ok(()),
        |_| Ok(()),
    )
    .unwrap();
    assert!(matches!(second.persistence, Persistence::Saved { .. }));
}
