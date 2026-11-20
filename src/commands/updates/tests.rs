use super::*;

#[test]
fn provider_pages_keep_repository_identity_and_require_a_complete_listing() {
    let source = Source::GitHub("custom/rojo".into());
    let base = serde_json::json!({"draft":false,"prerelease":false,"tag_name":"v1.2.0"});
    let full = serde_json::to_string(&vec![base; PAGE_SIZE]).unwrap();
    let mut page = 0;
    let actual=releases(&source,|url|{
        page+=1;
        assert_eq!(url,format!("https://api.github.com/repos/custom/rojo/releases?per_page=100&page={page}"));
        Ok(if page==1 {full.clone()} else {r#"[{"draft":true,"prerelease":false,"tag_name":"v9.0.0"},{"draft":false,"prerelease":true,"tag_name":"v8.0.0"},{"draft":false,"prerelease":false,"tag_name":"v1.3.0"}]"#.into()})
    }).unwrap();
    assert_eq!(actual.iter().max().unwrap().to_string(), "1.3.0");
    assert!(
        releases(&source, |_| Ok(full.clone()))
            .unwrap_err()
            .to_string()
            .contains("page limit")
    );
    assert!(releases(&source, |_| Ok("{}".into())).is_err());
}

#[test]
fn aliases_share_metadata_and_failures_remain_local_to_their_source() {
    let root = tempfile::tempdir().unwrap();
    let manifest = format!(
        "[package]\nname='sample/game'\nversion='0.1.0'\nregistry='{}'\n[dependencies]\nFirst='sample/signals@1'\n[dev-dependencies]\nOther='sample/signals@=1.2.0'\n",
        updates::OFFICIAL_REGISTRY
    );
    std::fs::write(root.path().join("wally.toml"), &manifest).unwrap();
    std::fs::write(
        root.path().join("rokit.toml"),
        "[tools]\nrojo='sample/rojo@7.4.0'\n",
    )
    .unwrap();
    let mut calls = 0;
    let result = report(updates::inventory(root.path()).unwrap(), |source| {
        calls += 1;
        if let Source::GitHub(_) = source {
            bail!("Rate limited")
        }
        releases(source, |url| {
            assert!(url.ends_with("/sample/signals"));
            Ok(r#"{"package":{"name":"sample/signals","version":"1.2.0"}}
{"package":{"name":"sample/signals","version":"1.3.0"}}"#
                .into())
        })
    });
    assert_eq!(calls, 2);
    assert_eq!(result.entries.len(), 3);
    assert_eq!(
        result.entries[0].comparison.within_requirement.as_deref(),
        Some("1.3.0")
    );
    assert_eq!(
        result.entries[1].comparison.within_requirement.as_deref(),
        Some("1.2.0")
    );
    assert_eq!(
        result.entries[0].comparison.status,
        updates::Status::Unknown
    );
    assert_eq!(
        result.entries[2].comparison.reason.as_deref(),
        Some("Rate limited")
    );
    assert_eq!(
        std::fs::read_to_string(root.path().join("wally.toml")).unwrap(),
        manifest
    );
}
