use super::*;
use crate::updates::OFFICIAL_REGISTRY;
use crate::{PresenceState, ResolutionState};

fn package(req: &str) -> Local {
    Local::Package {
        alias: "Signals".into(),
        realm: "dependencies".into(),
        package: Some("sample/signals".into()),
        declaration: format!("sample/signals@{req}"),
        registry: Some(OFFICIAL_REGISTRY.into()),
        requirement: Some(req.into()),
        locked: Some("1.2.0".into()),
        resolution: ResolutionState::Resolved,
        presence: PresenceState::Present,
    }
}

#[test]
fn offers_only_reproducible_newer_observed_targets() {
    for (req, expected) in [
        (
            "^1.2.0",
            vec!["sample/signals@^1.2.0", "sample/signals@^2.0.0"],
        ),
        ("=1.2.0", vec!["sample/signals@=2.0.0"]),
        (">=1.2.0, <2.0.0", vec!["sample/signals@>=1.2.0, <2.0.0"]),
    ] {
        let local = package(req);
        let comparison =
            local.compare(&["1.2.0", "1.3.0", "2.0.0"].map(|v| Version::parse(v).unwrap()));
        let mut entry = Entry {
            local,
            comparison,
            observed: true,
        };
        let project = std::path::Path::new("Skyline");
        assert_eq!(
            choices(project, &entry)
                .iter()
                .map(|s| s.declaration().unwrap())
                .collect::<Vec<_>>(),
            expected
        );
        entry.observed = false;
        assert!(choices(project, &entry).is_empty());
    }
}

#[test]
fn completion_requires_matching_owner_phase_and_exit() {
    let selection = Selection {
        project: "Skyline".into(),
        local: package("^1.2.0"),
        version: "1.3.0".into(),
    };
    let request = Request {
        protocol: PROTOCOL,
        id: "one".into(),
        action: Action::Prepare(selection.clone()),
    };
    let mut report = Report {
        protocol: PROTOCOL,
        id: "one".into(),
        selection,
        state: State::Writing,
    };
    assert!(report.verify(&request, true).is_err());
    report.state = State::Failed {
        possible_writes: true,
        reason: "failed".into(),
    };
    assert!(report.verify(&request, false).is_err());
    report.state = State::Failed {
        possible_writes: false,
        reason: "failed".into(),
    };
    assert!(report.verify(&request, false).is_ok());
    report.id = "another".into();
    assert!(report.verify(&request, false).is_err());
}
