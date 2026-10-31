use super::{Draft, LoadedTemplate, Page, Session, TemplateSource, builtin_project_template};
pub(super) fn loaded(original: Option<&str>) -> LoadedTemplate {
    let path = std::env::temp_dir().join("template-ui-fixture/default.project.json");
    let parents = path
        .ancestors()
        .skip(1)
        .map(|p| (p.to_owned(), Some("dir".into())))
        .collect();
    LoadedTemplate::new(TemplateSource {
        path,
        parents,
        identity: original.map(|_| "file".into()),
        original: original.map(str::to_owned),
    })
}
#[test]
fn invalid_json_preserves_the_explorer_model_and_raw_repair() {
    let mut d = Draft::new(loaded(None));
    let original = d.model.as_ref().unwrap().value().clone();
    d.raw = "{".into();
    assert!(d.apply_json().is_err() && d.dirty());
    assert_eq!(d.model.as_ref().unwrap().value(), &original);
    let mut replacement = original.clone();
    replacement["servePort"] = 40999.into();
    d.raw = serde_json::to_string(&replacement).unwrap();
    d.apply_json().unwrap();
    assert!(d.dirty());
    assert!(d.model.as_mut().unwrap().undo());
    assert!(!d.dirty());
}
#[test]
fn malformed_override_opens_for_repair_and_preserves_bytes_until_success() {
    let mut s = Session::default();
    s.load(loaded(Some("bad json")));
    assert!(s.page == Page::Json && !s.draft.as_ref().unwrap().dirty());
    let d = s.draft.as_mut().unwrap();
    d.raw = serde_json::to_string(&builtin_project_template()).unwrap();
    d.apply_json().unwrap();
    assert!(d.dirty());
    assert_eq!(d.loaded.source.original.as_deref(), Some("bad json"));
}
