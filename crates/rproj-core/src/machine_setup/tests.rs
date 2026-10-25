use super::*;
use std::collections::BTreeSet;

fn selection() -> Selection {
    Selection {
        root: "projects".into(),
        apps: BTreeSet::new(),
        tools: BTreeSet::new(),
        plugins: BTreeSet::new(),
        extensions: BTreeSet::new(),
    }
}

fn context(saved: Selection) -> SetupContext {
    SetupContext {
        config_path: "config.toml".into(),
        original: None,
        saved,
        identities: vec![],
        detected: Default::default(),
    }
}

#[test]
fn review_roundtrip_retains_read_only_root_unknown_keys_and_order() {
    let mut saved = selection();
    saved.tools.insert("future-tool".into());
    let mut edited = saved.clone();
    edited.tools.insert("rojo".into());
    let review = SetupReview::new(context(saved), edited).unwrap();
    assert_eq!(
        review.items,
        vec![Item::Rokit, Item::ProjectsFolder, Item::Tool("rojo".into())]
    );
    assert_eq!(
        serde_json::from_str::<SetupReview>(&serde_json::to_string(&review).unwrap()).unwrap(),
        review
    );
}

#[test]
fn category_edits_keep_inactive_choices_and_do_not_adopt_unknown_inputs() {
    let mut saved = selection();
    saved.plugins.extend([
        "future".into(),
        "blender-plugin".into(),
        "rojo-plugin".into(),
    ]);
    saved.replace_category(Category::Plugins, &BTreeSet::from(["not-catalog".into()]));
    assert_eq!(
        saved.plugins,
        BTreeSet::from(["future".into(), "blender-plugin".into()])
    );
    assert!(!saved.active(Category::Blender));
    assert_eq!(plan(&saved), vec![Item::Rokit, Item::ProjectsFolder]);
    saved.apps.insert("blender".into());
    assert!(plan(&saved).contains(&Item::Plugin("blender-plugin".into())));
}

#[test]
fn review_cannot_remove_unknown_choices_or_change_the_folder() {
    let mut saved = selection();
    saved.extensions.insert("future".into());
    let mut edited = saved.clone();
    edited.extensions.clear();
    assert!(SetupReview::new(context(saved.clone()), edited).is_err());
    let mut edited = saved.clone();
    edited.root = "elsewhere".into();
    assert!(SetupReview::new(context(saved), edited).is_err());
}
