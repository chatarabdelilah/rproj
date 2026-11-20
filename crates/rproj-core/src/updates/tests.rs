use super::*;

#[test]
fn stable_requirements_and_precedence() {
    let versions = ["0.2.1", "0.2.5", "0.3.0", "1.0.0-beta.1"].map(|v| Version::parse(v).unwrap());
    for (local, req, allowed, status) in [
        ("0.2.1", "^0.2.1", "0.2.5", Status::NewerAvailable),
        ("0.2.1", "=0.2.1", "0.2.1", Status::NewerAvailable),
        ("0.3.0+build", "*", "0.3.0", Status::Current),
        ("0.4.0", "*", "0.3.0", Status::LocalAhead),
    ] {
        let result = compare(
            Some(local),
            Some(&VersionReq::parse(req).unwrap()),
            &versions,
        );
        assert_eq!(result.status, status);
        assert_eq!(result.within_requirement.as_deref(), Some(allowed));
        if req == "=0.2.1" {
            assert_eq!(result.within_requirement_status, Some(Status::Current));
        }
    }
    assert_eq!(compare(None, None, &versions).status, Status::Unknown);
    assert_eq!(
        compare(Some("0.3.0"), None, &versions[3..]).status,
        Status::Unknown
    );
}

#[test]
fn skyline_shows_both_compatible_and_latest_stable() {
    let versions: Vec<_> = ["2.0.0", "1.2.0", "3.0.0-alpha.1", "1.3.0"]
        .map(|version| Version::parse(version).unwrap())
        .into();
    let result = compare(
        Some("1.2.0"),
        Some(&VersionReq::parse("^1.2.0").unwrap()),
        &versions,
    );
    assert_eq!(result.status, Status::NewerAvailable);
    assert_eq!(result.within_requirement.as_deref(), Some("1.3.0"));
    assert_eq!(result.latest_stable.as_deref(), Some("2.0.0"));
}
