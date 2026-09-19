# rproj 0.16.0 - T4: Saved Setup Manager

Published as an alpha release. The verified crates.io archive identifies clean commit `d54a9a874d8f23f5a57ebd573b6f1cd806506000`; the annotated tag and GitHub prerelease use that exact commit.

## Changes

- Home adds Saved Setups after New Project. Browse and inspect existing compositions, filter names, refresh, and manage them without Machine Setup.
- Edit opens composition review using the existing controls. Save stays open; leaving protects unsaved changes. No project or application is created or modified.
- Rename and Duplicate preserve bytes. Delete names the setup and defaults to No. Missing storage is an empty state; malformed entries remain inspectable and manageable.
- Source-change checks protect against detected external edits. Unknown TOML fields and untouched provenance/exclusions survive guided saves; unsupported compositions are read-only where safe revision cannot be determined.
- Saved setups move out of Catalog. New Project's picker and valid direct `--like` / `--save-setup` behavior remain.

## Limits And Recovery

Changed composition saves may reformat TOML and remove comments. Names must be safe single names; links and destination collisions are refused. Case-only rename requires an intermediate distinct name. Rename can leave both copies if original removal fails; inspect and refresh before retrying. These operations are not multi-file transactions and do not defeat every external-writer race.

There is no standalone composition creation or raw editor. New setups are saved during New Project. T5 remains Ratatui Machine Setup; package additions, background Watch, and the broader audit are separate. Model import is permanently dropped.

## Verification

351 ordinary tests passed; 19 prerequisite-dependent tests remain ignored. The applicable saved-setup replay test passed for Wally and Git submodules. Formatting, clippy, locked packaging, local CodeRabbit review, and Windows stable/Rust 1.89/package CI passed. [PR #24](https://github.com/chatarabdelilah/rproj/pull/24) records the reviewed commit and main verification. Owner publication and archive/tag/release alignment are verified.
