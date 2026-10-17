# Upgrade proposal acceptance

October 18, 2026. This records design-prototype validation, not native execution
or Windows acceptance. The owner selected native Upgrade as the next parity gap,
wireframe-first delivery and unified diffs. The [design](desktop-upgrade-design.md)
records the proposed behavior; visual approval remains pending.

## Confirmed automated evidence

The local deterministic suite passes **30 checks**, with **28 numeric palette
comparisons** across Soft Light/Dark text, disabled labels, selection, diff rows,
errors and focus colors. Primary text checks use a minimum 4.5:1 ratio; focus
checks use 3:1. These compare declared color pairs, not rendered screenshots.

- Fragment/JavaScript syntax, a useful static initial review, initial runtime
  content without optional helpers/icons, text-safe diff rendering and unique
  element/accessibility references pass. Saved state cannot replay Apply, and
  incompatible project keys fall back to the initial review.
- Diff checks cover creation/removal, old/new line numbers, three-line context,
  separated hunks, tabs/control characters, CRLF/missing final newline and the
  existing bounded large-region fallback. Sample policies cover Wally/None,
  custom settings, excluded CI, Jest mounts/configuration, skipped-only plans
  and blocking preparation errors. They are source-derived samples, not Rust
  planner/executor tests.
- Captured review retention, default-Keep replacement/refresh, stale inputs and
  confirmations, acknowledged same-project Stop, unchanged other-project Watch,
  late events and cancellation racing confirmed completion pass. Successful or
  partially applied samples leave only remaining changes on explicit refresh.
- Captured Activity output, bounded eviction, filtering, empty/waiting/no-match
  states, manual collapse and retained height pass. Cancellation stays pending;
  closing waits for teardown, unconfirmed teardown refuses close, and other dirty
  drafts still require explicit discard after cancellation.
- DOM focus/default-Keep behavior, Escape/picker cancellation, long paths,
  appearance/wrap/width state and presence throughout scenario changes pass.
  Source checks confirm stacked/adjacent responsive rules and semantic native
  controls. They do not establish browser Tab/Enter/Space activation or geometry.

The validation command is `node target/upgrade-design/validate.cjs`. Its local
artifacts include `baseline.json`, `controller.js`, `validation.log` and
`validation.json`. The baseline captures source hashes and both earlier working
wireframes; validation confirms those files remain unchanged. Prototype and
validation artifacts are thread-local response content, not shipped product files.

## Unverified and deferred

No supported inline inspection route is exposed by the available tools, so actual
rendering remains unverified. No retired browser/native automation route was
retried. Pixel geometry, real keyboard activation, native modal/assistive behavior,
host Lucide rendering, System appearance changes and scaling are not passes.

Native preview launches remain paused. The existing memory-stability problem is
unresolved; no OS troubleshooting or VM setup belongs to this milestone. Real
project mutation, external-tool behavior, process-tree teardown and other-platform
desktop acceptance are not established by this local simulation. No Rust, public
API, protocol, persisted format, preference schema, version or publication changed.

## One-minute design review

1. Select `.luaurc` and `selene.toml`; check file reasons and unified diffs
   (20 seconds).
2. Open Apply all proposed changes, read the project/counts and use Keep reviewing
   or Escape (20 seconds).
3. Choose Changes and a skipped file, then select `.luaurc`; check that it shows
   an explanation and stays separate from proposed writes (20 seconds).

Native implementation planning follows the owner's design decision. Delivery
evidence, including exact head/main CI and merged-branch cleanup, belongs to the
[design documentation PR #123](https://github.com/chatarabdelilah/rproj/pull/123).
