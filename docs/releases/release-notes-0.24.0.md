# rproj 0.24.0 — alpha candidate

The Windows desktop workbench replaces the interactive terminal hub.
Run `rproj app` to open it. Running `rproj` by itself prints command usage.

- Direct commands remain available. `rproj setup` takes selections through
  terminal prompts, and `rproj upgrade` prints file diffs before a default-No
  confirmation. `rproj upgrade --yes` remains available for automation.
- `rproj info` prints the catalog. A named lookup still prints one entry.
- `rproj configure project` opens Project Template in the matching desktop.
  Use the Windows portable ZIP to get the desktop and its CLI worker together.

The CLI, core library, and desktop use matching version 0.24.0. The desktop ZIP
is distributed through CI artifacts. Cargo installation supplies the CLI.

Windows desktop acceptance covered first-run setup and project creation,
Configure Tools, saved setup reuse, template recovery and save, and Composition
review, draft retention, and Apply. CLI terminal tests cover setup cancellation
and Upgrade confirmation and conflicts. The release run did not repeat live
machine-wide installation or exercise the new CLI-to-desktop template launch
in a real window.

The workspace RustSec audit reported zero vulnerabilities and nine maintenance
warnings. [Ratatui retirement](https://github.com/chatarabdelilah/rproj/pull/235)
