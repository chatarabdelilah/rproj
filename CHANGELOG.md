# Changelog

## 0.23.1

Windows x64 Machine Setup installs a verified prebuilt Rokit and explains how to recover when the Visual C++ runtime is missing. [Release notes](docs/releases/release-notes-0.23.1.md).

## 0.23.0

Added on-demand project package and tool update checks and improved Git and Rokit setup on Windows. [Release notes](docs/releases/release-notes-0.23.0.md).

## 0.22.0

Published [0.22.0](docs/releases/release-notes-0.22.0.md) adds Ratatui Upgrade file-diff
review with default-No Apply-all confirmation and refuses unsafe Selene TOML
merges before writes. It remains public alpha.

## 0.21.2

Published [0.21.2](docs/releases/release-notes-0.21.2.md) makes recreated Selene
configuration complete after one Upgrade, eliminating a spacing-only second
review. A second run leaves every project file unchanged.

## 0.21.1

Published [0.21.1](docs/releases/release-notes-0.21.1.md) makes upgrade review cover every
file write, including `.gitignore`, `.luaurc`, and TestEZ's `tests/.luaurc`.
Housekeeping-only repairs require confirmation; cancellation writes nothing,
and skipped files are reported clearly.

## 0.21.0

Published [0.21.0](docs/releases/release-notes-0.21.0.md) removes Git-submodule dependencies. New projects and saved compositions support Wally or None. Old `git-submodules` records are unsupported; there is no compatibility layer, automatic conversion or migration tooling. Existing repositories, templates and dependency folders are not deleted or converted.

## 0.20.4

This release introduced the [official Wally tool requirement](docs/releases/release-notes-0.20.4.md): older projects must explicitly update their tool pin before upgrading managed Wally CI; upgrade explains the requirement and preserves files. Missing-Git guidance, background Watch improvements, missing-Cargo and missing-WinGet recovery, and UI Labs 2.4.2 remain. rproj remains alpha software.

## Earlier releases

- [0.20.3](docs/releases/release-notes-0.20.3.md)
- [0.20.2](docs/releases/release-notes-0.20.2.md)
- [0.20.1](docs/releases/release-notes-0.20.1.md)
- [0.20.0](docs/releases/release-notes-0.20.0.md)
- [0.19.1](docs/releases/release-notes-0.19.1.md)
- [0.19.0](docs/releases/release-notes-0.19.0.md)
- [0.18.1](docs/releases/release-notes-0.18.1.md)
- [0.18.0](docs/releases/release-notes-0.18.0.md)
- [0.17.0](docs/releases/release-notes-0.17.0.md)
- [0.16.0](docs/releases/release-notes-0.16.0.md)
- [0.15.0](docs/releases/release-notes-0.15.0.md)
- [0.14.0](docs/releases/release-notes-0.14.0.md)
- [0.13.1](docs/releases/release-notes-0.13.1.md)
- [0.13.0](docs/releases/release-notes-0.13.0.md)
