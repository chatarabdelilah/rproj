# Native Project Template implementation

Approved October 30, 2026, against clean main `2f19bb0`. Published version remains
0.22.0. The owner approved Explorer / Advanced JSON → Review → Save and selected
machine/global Rojo for native validation, independent of the browsed project.

## Delivery sequence

1. Shared editor/metadata/policy and captured Save/Reset planning with checked
   persistence; retain terminal entry points and existing generation behavior.
2. Private matching-worker context, preparation, Save and Reset; bounded reports,
   owned descendants, explicit uncertainty and wait-for-completion closing.
3. Native Soft controls, retained machine draft, complete diffs, protected source
   mounts, JSON repair and independent cross-draft close protection.

Each PR receives appropriate tests, Clippy, package checks, local CodeRabbit and
direct diff review, followed by reviewed-head/main CI and merged-branch cleanup.

## Settled boundaries

Editing requires no setup or installed Rojo. Changed native saves require global
Rojo validation of all eight generation variants; Reset removes the override
without Rojo. Missing/changed sources require explicit reload. No-op/reverted
edits preserve bytes. Unfamiliar fields survive; incompatible properties require
explicit repair. Source and parent identities are checked before mutation.

Validation, Save and Reset expose no cancellation. Ordinary navigation remains
responsive; template edits freeze during the operation. Close waits for confirmed
completion, then protects other drafts. Unconfirmed teardown blocks closing and
further mutations. Watch remains independent. Earlier wireframes stay unchanged.

## Evidence

PR 1 merged. Regression fixtures preceded extraction. Test and review outcomes
follow below. No native preview is authorized.
Native memory stability remains unresolved; real-window and macOS/Linux desktop
acceptance remain unverified. No installation, version bump or publication.

### PR 1 progress

The built-in template and all eight generated variants were captured before
extraction. The first snapshot command used an incorrect exact filter (zero
matching tests); the corrected named test ran one test and passed. Generation
still matches that fixture after extraction. Tree/history, bundled reflection
metadata, structural policy, project settings and typed attributes now live in
an optional core feature used by the CLI. Cross-platform core CI enables it.

Save/Reset capture original bytes and file/ancestor identities, including missing
parents. Preparation creates no destination; no-op saves preserve bytes and do
not create an override. Changed saves stage with existing permissions, exclusively
claim missing directories, recheck before replacement, and confirm saved bytes.
Reset needs no validator. Mutation reporting precedes staging. Replaced identities
require reload even when restored bytes are equal. Integer attributes retain exact
signed 64-bit values; unfamiliar attribute shapes require explicit repair.

Initial extraction checks caught a module import, missing diff filename argument,
and test-only policy import; each was corrected. Three persistence fixtures used
invalid placeholder documents and now use valid templates. The first full CLI
run passed 525 tests with two failures: replacement errors lost their context,
and a terminal recovery test assumed recreating equal bytes restored identity.
Error context and explicit reopen recovery were corrected; all eleven focused
terminal PTY tests then passed (one real-Rojo test remains ignored). Strict Clippy
also identified one nested condition, corrected without suppressions. A local
documentation write used the wrong encoding; the affected file was restored from
Git and updated as UTF-8. Those initial checks did not establish final validation;
the later runs are recorded below. Reviewed-head/main CI remains pending.

The first full corrected run passed 767 CLI/core tests, with 24 prerequisite-based
tests ignored. Core without the optional editor feature, strict Clippy and fresh
core/CLI package verification also passed. A further regression test covers
preserving an unfamiliar Attributes container during an attempted addition.

CodeRabbit's first review returned two findings. Terminal stale-source errors now
explain that this frontend has no Reload action and requires preserving desired
draft changes separately before exiting/reopening. The fixture line-ending
concern required no code change: the existing `.gitattributes` sets
`* text=auto eol=lf`, and `git check-attr` confirms this fixture uses LF.
Subsequent review and validation results follow below.

The second review found two pre-existing model defects exposed by extraction:
missing/stale paths could panic several mutators, and moving an inferred root
service did not materialize its class. Mutators now refuse missing selections
without changing history, and moves retain the class. Dedicated regression tests
cover both. The preceding full run passed 768 CLI/core tests; package verification
and portable version/inspection smoke passed, retaining the three pre-existing
legacy notice gaps. Validation is rerun after these final model corrections.

The third review identified inconsistent root-service uniqueness: Duplicate and
Move could bypass the existing Add/Change Class checks. Both now use bundled
service metadata to refuse a second root service of the same class. The regression
also verifies that ordinary folders remain duplicable. This correction is included
in the final validation and review rather than treating earlier runs as final.

Final CLI/core validation passed 770 tests (24 prerequisite-dependent tests remain
ignored), strict Clippy and formatting. Core without the optional feature also
passed. Core/CLI package verification and portable version/inspection smoke passed
after the final model corrections. The three legacy notice gaps remain unchanged.
The fourth CodeRabbit review found no runtime issue and one documentation wording
issue about pending evidence; the wording and final results are corrected here
and inspected directly. All 122 existing headless desktop tests and desktop
Clippy also passed. PR #140 reviewed-head CI passed; the merged-main failure is recorded below. No native
preview or real-window acceptance is claimed.


### PR 1 delivery and Machine Setup Stop correction

PR #140 merged at `17d50f1`; reviewed head `3e00d2e` passed all seven
executed CI jobs in run `41043744364`, attempt 1. The merged tree is identical.
Merged-main run `41046504749`, attempt 1, exposed a real scheduling race in
`pre_requested_stop_skips_every_item_without_saving`: the first installer could
start before an already-requested Stop reached the worker. The stable Windows
job failed with `MayHaveBegun` instead of `NotStarted`. This is not counted as a
pass or dismissed as a flaky test; worker PR 2 is paused for a corrective PR.

The first correction wrote a pending Stop before launch. All 14 focused worker
tests, the full CLI/core suite, strict Clippy and joint package verification
passed. CodeRabbit identified a remaining interval between checking Stop and
spawning the process. The earlier callback regression initially needed a
qualified event type to compile.

The revised private setup protocol requires a one-byte startup decision on
the owned Apply worker's stdin. The parent samples Stop after spawning the
process and delivering Started. The worker waits for this decision before
entering the installer backend. A pending Stop skips every item. An invalid
decision or disconnected pipe produces a failed report before installation.
Later Stop requests still let the active item finish. Context and preparation
use their existing paths. The matching-worker check rejects the old protocol.

Tests cover Stop in the post-spawn Started callback, disconnected and malformed
startup input, active-item completion and late Stop after save. The revised
suite passed 772 CLI/core tests with 24 prerequisite-dependent tests ignored,
plus all 122 headless desktop tests. Formatting and strict CLI/core Clippy
passed. The second local CodeRabbit review reported no findings.

An initial package recheck reused a Cargo registry overlay from the first
correction. The archived CLI compiled against the older core, which produced
warnings about unbound pattern names. The startup markers now use qualified
constant paths so an incompatible core fails to compile. Package verification
uses a fresh overlay for the corrected archives.


### PR 2 — matching private worker

PR #141 corrected the startup race on main at `41f2d69`. Its reviewed head
`08ff9d5` passed seven executed jobs in run `41203098152` on the first attempt.
The merged tree matches the reviewed one. All seven executed main jobs passed
in run `41203644146`, attempt 1. Both merged branches for #140 and #141
were removed after verifying their trees.

The matching CLI serves bounded context, preparation and confirmed Save/Reset
requests. The destination always comes from the machine template configuration.
The worker verifies the captured source and all parent identities even when the
override or its directory was absent. An unchanged Save preserves the original
bytes. Reset can remove a malformed override without Rojo. Changed saves validate
the same six production and two Jest variants as the terminal editor, with both
sourcemap and build for each variant in a disposable directory.

Native validation resolves the installed executable named by the global Rokit
manifest, or a standalone Rojo from absolute PATH directories when no pin exists.
Known project-aware launcher directories are excluded. Preparation captures the
global manifest, executable identity and content digest. Execution resolves them
again and repeats validation before changing the override. The terminal still
uses its existing tool lookup. No installation or Watch action is involved.

The CLI writes a bounded atomic report before possible mutation and returns
the exact saved baseline on success. The client waits for the complete Windows
job and both output streams. A lost or contradictory terminal report yields an
uncertain outcome. Unconfirmed teardown preserves operation storage. The fake
worker covers missing, malformed and foreign reports, a real fixture write
followed by report loss, finite descendants, JSON repair/reset, missing or
rejecting Rojo, saved-byte fidelity, stale inputs and 2 MiB refusal.

Before replaying the worker on corrected main, 782 CLI/core tests passed, 24
prerequisite-dependent tests were ignored, and strict Clippy and joint packaging
passed. The first local worker review found two issues. Started now follows a
successful spawn, and the worker compares the reviewed and authoritative paths
after applying the same normalization used during capture. Both have regression
coverage. Final rebased validation and delivery are tracked separately.

The second review identified an output-size check that belonged in checked
persistence as well as at the IPC boundary. The formatted replacement is now
measured before invoking Rojo and again before reporting possible mutation.
A small-limit regression verifies that neither validation nor staging starts
and the missing template directory stays absent.

The final worker revision passed 786 CLI/core tests, with 24 prerequisite-based
tests ignored. Strict CLI/core Clippy, formatting and fresh joint Cargo package
verification passed. The third local CodeRabbit review returned zero findings.
Windows portable smoke covers version and inspection only. The three known
legacy notice gaps and paused real-window acceptance remain separate.


### PR 3 — native controls

The private worker merged in PR #142 at `897cd4d`. Reviewed head `af6ee2c`
(run `41230495071`) and merged main (run `41231123470`) passed all seven
executed CI jobs on their first attempts. The reviewed and merged trees match.

Project Template opens below project navigation. Explorer and Advanced JSON
share a session draft and the core model's undo/redo history. The inspector
uses bundled Roblox class/property metadata and the existing parsers for
properties, attributes and project settings. Applying invalid JSON preserves
the last structured draft. Malformed saved text can be repaired or reset.
Unapplied field input survives browsing and participates in discard protection.
Managed mounts are read-only and incompatible properties stay visible for repair.

Review captures the machine template path, shows semantic changes and a complete
wrapped diff, and requires explicit confirmation. Reset has a separate removal
review and shows the resulting built-in tree. Activity retains the outcome and
source across navigation. Worker operations have no Cancel action. The close
dialog defaults to Keep open, waits for a confirmed result, and then checks other
dirty drafts. Unconfirmed teardown blocks closing and further mutations.

During headless validation the first builds caught an ambiguous test attribute,
a missing element ID on an accessibility alert, and incomplete wiring for the
review summary and worker capability response. Two focus assertions required
registering test support before tracking the focus handle. A field test used
a silent setter and was corrected to use GPUI's change-emitting replacement.
A full run caught hidden Light/Dark controls in the minimum window after adding
the new sidebar entry. Scrolling is now confined to navigation. The first local
native CodeRabbit review also identified a missing Activity completion line and
a duplicate unlabeled root entry in Move. Both were corrected.

Final local validation passed all 132 headless desktop tests and 786 CLI/core
tests. Another 24 tests were ignored for missing prerequisites. Formatting,
strict Clippy for all three crates, and verification of fresh CLI/core package
archives passed. The Windows portable package passed its version and inspection
checks. It retained notices for 590 dependencies, with the same three legacy
notice gaps. The second local CodeRabbit review had one documentation finding:
record these final results. The evidence correction was inspected directly.

Windows real-window and memory-stability acceptance remain paused. macOS and
Linux desktop acceptance is unverified. Published version remains 0.22.0.
