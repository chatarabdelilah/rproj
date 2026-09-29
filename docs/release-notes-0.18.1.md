# rproj 0.18.1 — Project composition and reliable local testing

Alpha release candidate. Not published yet.

## Changes

- New Project opens at Review so the complete composition and Create action are
  visible immediately. Start point still offers Guided, Expert, and saved setups.
- Projects > Edit Packages & Capabilities adds choices to an existing recorded
  project without removing its current choices. It preserves unrelated Wally
  dependencies and comments, rejects concurrent `rproj.toml` edits, restores the
  original record after a failed apply, and names files that may need review.
- `rproj test` avoids replacing a complete Wally tree while Watch observes it.
  It verifies the manifest, lockfile, dependency realms, direct pins, and installed
  aliases first; stale or incomplete state still runs normal Wally recovery.
- Existing compatible `ReplicatedStorage.devPackages` mounts are preserved,
  including Folder metadata and attributes. Incompatible collisions still fail
  without changing the project.
- Fresh Machine Setup and New Project optional choices start unchecked. Saved
  selections remain selected. Confirmed global Rokit adds use `--force`, including
  pinned sources, so trust prompts do not break setup.
- Scribe Studio is available as an unchecked manual Studio-plugin choice.
- TestEZ always includes `testez-companion.toml`. Catalog details support mouse
  wheel scrolling, and generated StarterPlayer/StarterPlayerScripts nodes include
  explicit class names.

## Existing projects

Run `rproj upgrade` after installing this version. Existing project compositions
can add packages and Testing from Projects > Edit Packages & Capabilities. Stop an
active Watch before saving package additions through that editor.

If a Wally alias already uses a custom pin, rproj preserves it when it is in the
required dependency realm. A realm mismatch is reported for manual correction.
Test skips installation only when `wally.toml`, `wally.lock`, and the installed
aliases agree.

## Verification and limits

The candidate passed 404 ordinary tests with 19 prerequisite-dependent tests
ignored, formatting, clippy, real installed-Rojo validation of every built-in
template variant, and locked package verification. CodeRabbit identified ten
applicable recovery, preservation, and freshness findings in PR #35; all were
corrected and the final complete-diff reviews reported zero findings.

Reviewed-head and merged-main Windows stable, Rust 1.89, and package CI passed for
PRs #33–#35. Live Open Cloud, fresh Linux generated-project execution, fresh
Windows provisioning, and a Scribe Studio playtest remain unverified. This release
adds no dependencies.
