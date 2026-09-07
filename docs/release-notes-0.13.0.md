# rproj 0.13.0

Alpha release: diagnostic logging and built-in project creation.

## Changes

- New Project in the workspace hub now uses Ratatui for guided/expert package selection, dependency strategy, capabilities, compatible implementations, and review.
- Saved setups open directly in Review. Revise decisions, omit optional files, change the project name, or save a new named setup before confirming creation.
- Required runner wiring stays protected. Switching Jest away from Wally requires choosing TestEZ or disabling Testing; unrelated capabilities remain selected.
- Creation occurs only after explicit confirmation and terminal restoration. Existing destinations and named setups are never overwritten by the hub, including concurrent creation races.
- Local, bounded diagnostic logs record semantic actions and command outcomes without recording raw keystrokes, template contents, or opaque runner arguments. Nothing is uploaded automatically. Review logs for private names and paths before sharing.

## Compatibility

Direct `rproj new` prompts and flags remain available, including their existing saved-setup replacement behavior. Project schemas, generated formats, template storage, and external tool boundaries remain unchanged. Hub creation requires recorded Machine Setup and does not provision machine applications from a draft. No new dependencies or dependency-version upgrades are included.

After publication, install or update with `cargo install rproj --version 0.13.0 --locked`. Existing projects do not need regeneration to use the new hub workflow.

## Verification And Limits

The candidate includes 301 ordinary passing tests and 19 explicitly ignored tests (320 discovered). Live verification covers 14 serial project/toolchain scenarios, including Jest starter-spec success and deliberate failure, saved setup replay/refusal, hub cancellation, confirmation, and concurrent destination protection. Three separate real-Rojo template/Inspector checks cover the existing validation boundary. Release PR and main CI verify Windows stable, Rust 1.89, and packaged-crate builds.

This remains alpha software. Fresh-Windows provisioning and live Open Cloud execution have not been verified. Logs are local and redacted conservatively, not guaranteed anonymous. External command failures after confirmation may leave a partial project for inspection; no destructive rollback is attempted. Model import, Tauri, and embedded linting/formatting are not included.
