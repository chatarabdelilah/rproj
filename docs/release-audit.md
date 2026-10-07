# Release-Hardening Audit

## October 7: complete upgrade review implementation

Work is on `codex/complete-upgrade-review` from clean main `69a02e6`, after
published 0.21.0. The housekeeping-only PTY regression failed on the original
runtime: it reported already up to date, wrote `.gitignore` and `.luaurc`, and
never offered confirmation. Upgrade now plans these files and TestEZ's scoped
`tests/.luaurc` before confirmation, using shared pure content planners also
called by creation. All targets and inputs participate in conflict checking and
staged replacements; post-apply merge writes are removed.

`cargo test --locked --offline --test upgrade` passes **28 tests**. The seven
new regressions cover housekeeping-only review and rejection/Esc/Ctrl+C,
interactive and `--yes` saves with custom values and byte-preserving reruns,
edited/deleted/new targets during confirmation, unreadable targets, unparseable
documents, and a later housekeeping staging failure preserving earlier targets.
The latter fixture provides parent directories in advance: existing staging
semantics may leave newly created empty directories on preparation failure.

`cargo test --locked --offline` passes **485 ordinary tests, 23 ignored**,
excluding nested child-test summaries. Formatting and strict all-target Clippy
pass. Local CodeRabbit iteration review reports zero findings across all nine
files. Packaging, final branch review, and reviewed-head/main CI remain pending.
This is an unreleased runtime correction; no version bump, publication,
or tag is part of this PR. Fresh-machine and authenticated Open Cloud acceptance
remain separate gaps; no host applications or user projects were changed.

## October 7: 0.21.0 publication verified

The owner published 0.21.0 at `2026-10-07T12:15:35.090245Z`. Crates.io confirms
the version is not yanked and gives checksum
`40c69158d8c771dc62d3a9eda6d171f78f1a1d199bcf27e478e43b6dbc50c8d8`.
The downloaded official archive matches that checksum. Its 105 files are all
byte-identical to a clean locked package rebuilt from release commit
`07dcf17fb8a841c2f842a3753f58ad10507f99e1`. Both manifests and the root lockfile
identify 0.21.0; `.cargo_vcs_info.json` is clean and identifies that commit.
All 95 source/test files also match the checkout. The retired submodule generator
and excluded docs, CI, CodeGraph and build directories are absent.

Release preparation [PR #94](https://github.com/chatarabdelilah/rproj/pull/94)
passed reviewed-head CI
[37617082282](https://github.com/chatarabdelilah/rproj/actions/runs/37617082282)
at `b8d473bfd0fc66e60fe1c8a2628cc603f9b4ac22` and merged-main CI
[37617634582](https://github.com/chatarabdelilah/rproj/actions/runs/37617634582)
at `07dcf17fb8a841c2f842a3753f58ad10507f99e1`. Complete trees match.
All four required jobs passed; each Windows toolchain passed **479 ordinary
tests, 23 ignored**. Local CodeRabbit iteration and final review had zero findings
across all seven release files. The release-preparation branch was removed only
after main CI and merged-state verification.

The annotated `v0.21.0` tag peels to the official archive commit. The matching
[GitHub alpha prerelease](https://github.com/chatarabdelilah/rproj/releases/tag/v0.21.0)
is not a draft and uses the dedicated release notes. Earlier shipped tags remain
unchanged. Publication alignment changes documentation only; runtime/test source,
dependency resolution and CI remain identical to the reviewed release. The prior
local gates remain applicable; publication-docs reviewed-head and main CI must
also pass before its branch is removed.

Fresh-Windows installation/retry, authenticated Open Cloud, full fresh-Linux
project execution and manual Studio checks remain unverified. VM provisioning
is still deferred; no host applications, global tool pins, historical patched
Wally caches or existing projects were changed.

## October 7: 0.21.0 alpha release preparation

The owner requested continuation into release preparation after full Git-submodule
retirement. Clean main is `63519bf432de2528e150aba2d183d695fa248807`; published
0.20.4 remains the registry's latest version with official checksum
`a626cfb6726474a925e8c88b0f0f43268227cc96978a544c02ffee43ab2d67ad`.
Registry and remote-tag checks show 0.21.0 is available. The minor alpha version
reflects the intentional unsupported workflow removal. Preparation is on
`codex/release-0-21-0`; only the root package version changes in Cargo.toml and
Cargo.lock. Runtime/test source, dependency resolution and CI remain unchanged
from the reviewed retirement.

Retirement [PR #93](https://github.com/chatarabdelilah/rproj/pull/93) passed
final-head CI [37611026446](https://github.com/chatarabdelilah/rproj/actions/runs/37611026446)
at `1d53ff3d3568372ba200d333c48e464c99ddb0b0` and main CI
[37611420065](https://github.com/chatarabdelilah/rproj/actions/runs/37611420065)
at `63519bf432de2528e150aba2d183d695fa248807`. Complete trees match. Each Windows
toolchain passed 479 ordinary tests with 23 ignored on each run; all four required
jobs passed, including official-Wally Linux and packaging. The retired branch's
exact-lease remote deletion and local removal were verified after main CI.

The isolated candidate binary reports `rproj 0.21.0`. Formatting, the complete
locked offline ordinary suite (**479 passed, 23 ignored**, with nested child
results excluded from totals) and strict all-target Clippy passed using
`target/retire-submodules-target`. Runtime/test source and CI compare unchanged
against `63519bf`; the only lockfile change is the root package version.

Candidate installed-tool checks ran serially and passed in disposable fixtures:
all eight Rojo variants, Wally creation/gate, deliberate gate failures, None Home
creation, saved Wally replay and unsupported saved-record refusal. The separate
official 1.7.0 Windows generic-default/const compatibility check also passed using
the existing isolated release binary. Its `RPROJ_WPT_TEST_BIN` override was scoped
to the command; the host's historical patched cache remains untouched. No
machine-wide applications were provisioned. Ignored checks are not counted as
ordinary passes; fresh-machine and authenticated Open Cloud acceptance remain
unverified.

Preliminary locked packaging with `--allow-dirty` verified compilation and the
105-file inventory while changes remained available for iteration review. All 95
archived source/test files match the checkout; the retired generator and excluded
documentation/CI/index/build directories are absent. Final `cargo package --locked`
passed from clean candidate commit `d44dc1e67bf46fef1cf4cef236c28cfa8f386151`:
105 files, verified compilation, version 0.21.0 in both manifests and the root
lockfile, and clean `.cargo_vcs_info.json` matching that commit. The subsequent
evidence updates affect excluded documentation only. CodeRabbit iteration initially
hit its 28-minute service
cooldown. After the full wait, the single retry at 13:43 Brussels time completed
with **zero findings across all seven release files**. Final branch review at
`35bb05cebd083c28d842dedfae141d1ca335aee7` also completed with **zero findings
across all seven files**, after waiting until the next included slot cleared at
13:48 Brussels time. The final evidence correction affects excluded documentation
only. Reviewed-head CI, guarded merge, main CI and branch cleanup are tracked in
[release-preparation PR #94](https://github.com/chatarabdelilah/rproj/pull/94).
Owner publication, published archive identity, tag and matching GitHub
alpha release are pending; no shipped identity is changed.

## October 7: unreleased Git-submodule retirement

The owner approved full alpha-stage removal, without legacy support, automatic
conversion or migration tooling. New projects, direct prompts, saved compositions
and Catalog guidance now offer Wally or None. Persisted `git-submodules` values
are rejected through existing configuration-error handling before creation or
composition. Ordinary Git initialization and missing-Git recovery remain.

Generation and restoration, source-layout metadata, manual dependency closure,
the modules artifact/mount, vendoring guards, special exclusions and generated CI
submodule checkout are removed. Upstream URLs, canonical module names, Wally
aliases, realms and testing remain. Existing repositories, templates, `.gitmodules`
files and dependency folders are not deleted or converted. Historical sections
below describe earlier releases and are retained as evidence.

Local verification passed on `codex/retire-git-submodules`, based on clean main
`33b184a`: formatting, the complete locked offline ordinary suite (**479 passed,
23 ignored**, excluding the nested child result from totals), and strict locked
all-target Clippy. No setup-cancellation checks were excluded: the owner profile
reported Watch stopped. The isolated build directory is
`target/retire-submodules-target`. The initial sandbox run denied PTY/process
access; the complete Windows-access run passed after correcting two obsolete
test selections. Removed tests reduce the former 490/25 ordinary inventory.

Installed-tool checks ran serially and passed in disposable fixtures: all eight
Rojo template variants, Wally creation and quality gate, deliberate gate failures,
None creation through Home, Wally saved-setup replay and unsupported saved-record
refusal before reconfiguration/creation. Existing custom-template fields,
including an owner-controlled `modules` node, remain preserved. No machine-wide
application was provisioned and no owner project/template/dependency folder was
converted or removed. Rokit/Wally caches remain shared by these existing checks.

CodeRabbit iteration reported two minor architecture-doc findings; both were
corrected. Final branch review found one remaining retired-workflow mention in
Catalog documentation; that documentation-only correction was inspected directly,
as required by the working agreement. No runtime findings remain. Locked packaging
passed from clean commit `1639e77`: 105 files and verified compilation. The
subsequent documentation corrections affect excluded files only.

Initial head CI [37609989027](https://github.com/chatarabdelilah/rproj/actions/runs/37609989027)
at `2659c10` passed Rust 1.89 (479/23), official-Wally Linux and packaging; Windows
stable exposed a Saved Setup resize assertion comparing a partially cleared
Actions pane after returning to composition. The existing terminal regression now
derives its expected pane from TestBackend and waits for every physical row at
initial scrolling, restored size, End and Back. It retains the same file, filter,
selection, scroll and terminal assertions. The focused check, full 479/23 ordinary
suite and strict Clippy passed locally after this test-only correction. Locked
packaging passed again from clean code commit `987b9ad` (105 files). Final local
CodeRabbit branch review reported no runtime/test findings and one documentation
correction: Jest's DataModel mount is lower-case `devPackages`, while its disk
directory is `DevPackages`. That correction and the last template-review wording
were inspected directly. Final-head CI, guarded merge, main CI and branch cleanup
are tracked in [PR #93](https://github.com/chatarabdelilah/rproj/pull/93). The focused
official-Wally Linux compatibility CI job is retained.
Version 0.20.4 is unchanged; release preparation and owner publication are separate
work.

## October 7: 0.20.4 publication and release alignment

The owner published 0.20.4 with `cargo publish --locked`. Crates.io records
`2026-10-07T09:46:49.376034Z`; the official archive checksum is
`a626cfb6726474a925e8c88b0f0f43268227cc96978a544c02ffee43ab2d67ad`.
Downloaded registry metadata and archive agree. The published manifest/root
lockfile are 0.20.4; `.cargo_vcs_info.json` identifies clean release commit
`1a046683275faa0e9c9214b030bc183aadf3722c`. All 106 archived files match the
verified clean-main package byte for byte. The compressed archive checksum
differs from the earlier local archive; its file contents and Git identity match.

Release preparation [PR #91](https://github.com/chatarabdelilah/rproj/pull/91)
passed final local CodeRabbit review with zero findings across all seven files.
The review's 24-minute service cooldown was respected before a single retry.
Exact-head CI [37600658551](https://github.com/chatarabdelilah/rproj/actions/runs/37600658551)
at `2c1c920d08e4ce58d8281360bf1e17a3b0a6f8c0` and merged-main CI
[37601126414](https://github.com/chatarabdelilah/rproj/actions/runs/37601126414)
passed all four required jobs on first attempt. Each Windows toolchain passed all
490 ordinary tests, with 25 ignored checks separate; the two locally blocked
setup checks passed in CI. Official Linux Wally compatibility and packaging also
passed. Reviewed/merged full trees matched, and the fully merged local/remote
release branch was removed with a verified remote lease. Clean-main locked
packaging compiled and the isolated binary reported 0.20.4.

The annotated `v0.20.4` tag identifies the published commit. The matching
[GitHub alpha prerelease](https://github.com/chatarabdelilah/rproj/releases/tag/v0.20.4)
uses the dedicated notes. Current publication documentation is corrected through
a documentation-only PR; runtime, tests, CI and Cargo versions remain unchanged.
No additional local CodeRabbit review is required for this status correction.

No known broken items. Fresh Windows, authenticated Open Cloud, full fresh-Linux,
UI Labs and Scribe Studio acceptance remain unverified. VM provisioning remains
deferred by the owner. No global manifest, historical patched cache, existing
project or active Watch was changed.

## October 7: 0.20.4 alpha candidate preparation

The candidate starts from clean reviewed main `adc56a3`, after [PR #90](https://github.com/chatarabdelilah/rproj/pull/90).
Its exact-head CI [37593781968](https://github.com/chatarabdelilah/rproj/actions/runs/37593781968)
and main CI [37594298177](https://github.com/chatarabdelilah/rproj/actions/runs/37594298177)
passed all four required jobs on their first attempts: Windows stable, Rust 1.89,
official Linux release compatibility and packaging. Both Windows jobs passed all
490 unique ordinary tests, with 25 ignored checks separate. The full merged tree
matched the checked head; both local CodeRabbit reviews reported zero issues.
Fully merged implementation branches were verified and removed.

Manifest and root lockfile versions are aligned at 0.20.4. Rust dependency versions,
runtime, tests and CI remain identical to reviewed main. Candidate notes and
current-state documentation distinguish this candidate from published 0.20.3;
owner publication remains pending.

The owner's existing background Watch remains active. It intentionally blocks
the Home and standalone machine-setup cancellation checks locally; neither is
counted as a local pass. It also holds the existing `target/debug/rproj.exe` open,
so a default-target build failed with Windows error 5. Candidate build/testing
uses a separate `target/release-0-20-4-target` directory without stopping Watch or
changing installed tools. Current release evidence is recorded as gates finish;
exact-head/main CI must execute the complete 490-test ordinary suite.

The isolated locked build reports `rproj 0.20.4`, and all 488 available unique
ordinary tests passed using that candidate target directory. The nested one-test
child is not counted twice. Strict all-target Clippy, formatting and diff checks
passed. The 25 ignored checks remain separate; only
`cargo test --locked --test wpt_release -- --ignored --exact released_wpt_preserves_valid_generic_types_and_parses_const --test-threads=1`
was explicitly invoked for the candidate, with the verified official Windows
1.7.0 binary supplied through `RPROJ_WPT_TEST_BIN`. It passed both fixtures; the
archive SHA-256 matched the official asset digest recorded below. No machine
configuration, global Rokit manifest, historical patched cache or existing
project was replaced.

Local CodeRabbit iteration review found one minor stale roadmap statement saying
no candidate was active. That contradiction was corrected in the current-state
and next-milestone paragraphs.

Clean `cargo package --locked --target-dir target/release-0-20-4-target` at
`af69bfd67d3696ae2b9a4419474657d21ece6a29` compiled the crate from its tarball.
The inspected archive has 106 paths: the published 0.20.3 inventory plus only
`tests/wpt_release.rs`. Manifest/root lockfile versions are 0.20.4, and
`.cargo_vcs_info.json` records that clean commit. Source/tests, README/license,
Cargo metadata and tracked root configuration are present; docs/workflows,
CodeGraph, target artifacts and credentials are absent. This evidence-only
update changes excluded documentation; packaged files remain identical.
Final branch review and exact-head/main CI remain required; their outcomes are
recorded in the candidate PR before owner publication. Publication is pending;
no 0.20.4 tag or GitHub release precedes archive verification.

## October 7 implementation, shipped in 0.20.4: official Wally type release

Official wally-package-types 1.7.0 includes the upstream generic-default ordering
and `const` parser fixes previously supplied by pinned commit `daf5c97`. Generated
Wally CI now invokes the Rokit-installed release; its Cargo build/cache workaround
and absolute Cargo binary path are removed. Shared/server/dev package arguments,
sourcemap selection and installation-before-check ordering remain intact.

Managed Wally CI upgrades validate the official stable project pin is at least
1.7.0, reading `rokit.toml` through the existing review snapshot. Missing, malformed,
older or unverifiable pins refuse all writes, including with `--yes`, and explain
explicit installation/update and retry. Upgrade never rewrites tool pins;
manifest changes during confirmation refuse the whole upgrade. Projects without
managed Wally CI bypass the preflight. Host global manifests, patched caches and
existing projects are untouched; Cargo remains 0.20.3 pending separate release
preparation.

All 22 focused upgrade tests and strict all-target Clippy passed. The explicit
official Windows binary test passed both synthetic generic-default and `const`
fixtures, preserving source bytes and restored exports. Its archive digest
`33e833440402d70d928a104de7325c8014ac5b87772725711f0946ad8246bf5f`
matches GitHub's release asset digest. A focused Ubuntu CI job verifies the pinned
Linux asset digest and runs the same isolated test; it does not establish full
fresh-Linux generated-project acceptance.

The initial ordinary suite stopped at Home's machine-setup cancellation check:
the owner's active background Watch intentionally prevents machine-wide setup.
The isolated retry reproduced that host condition; the owner's Watch was not
stopped. The subsequent run also reached the standalone setup cancellation check,
which printed the same intentional Watch guard. With both named checks explicitly
excluded, all 488 remaining unique ordinary tests passed; neither blocked check is
counted as a local pass. The nested one-test child is not an additional unique
test. The 25 ignored checks remain separate, including the explicitly invoked
official-release compatibility test. Exact-head/main CI must run all 490 ordinary
tests on their clean runners.

Formatting, diff checks and locked packaging passed; the 106-file archive compiled
from its tarball. CodeRabbit iteration review found zero issues across all eight
changed files. Final branch review and exact-head/main CI remain required; their
outcomes are recorded in the implementation PR as those gates finish. This repair
is unreleased on the published 0.20.3 baseline; no host provisioning or package
publication is part of this change.

## October 7: owner publication of 0.20.3 verified

The owner published 0.20.3 after [PR #88](https://github.com/chatarabdelilah/rproj/pull/88)
merged at `2932862af2cfc7a7670e54da06e133c799a53592`. Its exact final-head CI
[37573835354](https://github.com/chatarabdelilah/rproj/actions/runs/37573835354)
and merged-main CI [37574077034](https://github.com/chatarabdelilah/rproj/actions/runs/37574077034)
passed on their first attempts. Fully merged candidate branches were removed
after reviewed identities, ancestry and tree-equality checks; main was clean
for owner publication.

The official crates.io archive SHA-256 matches registry checksum
`f2403a0d54155e7ec3c37dee9a9f99d9adfee66f0a242cdc2d98f4bfb3ea013c`.
Archive manifest/root lockfile version is 0.20.3; `.cargo_vcs_info.json` identifies
the clean reviewed release commit `2932862af2cfc7a7670e54da06e133c799a53592`.
Annotated tag object `0c6c968f81ec1d11cd046629981e8575011bbb36` peels to that
same commit locally and remotely. The matching [GitHub release](https://github.com/chatarabdelilah/rproj/releases/tag/v0.20.3)
is an alpha prerelease. No shipped tag was moved.

This publication-state update changes documentation only: Cargo versions,
dependencies, runtime and tests remain identical to the shipped commit. The
diff is directly inspected under AGENTS.md's documentation-only review exception;
exact documentation-head/main CI outcomes are recorded in its PR before
completion. Fresh-Windows installation/retry and deferred Open Cloud/platform
Studio acceptance remain unverified. No further audit or feature is automatically
queued; the next planning decision concerns remaining acceptance priorities.

## October 7: 0.20.3 alpha candidate preparation

The candidate starts from clean main `bdd2ab6`, after the missing-Git repair
[PR #87](https://github.com/chatarabdelilah/rproj/pull/87) passed first-attempt
final-head CI [37569022721](https://github.com/chatarabdelilah/rproj/actions/runs/37569022721)
and exact-main CI [37569252616](https://github.com/chatarabdelilah/rproj/actions/runs/37569252616).
`Cargo.toml` and the root `Cargo.lock` package are aligned at 0.20.3. Dependency
versions and runtime/test source remain identical to verified main; notes and
current-state docs distinguish this candidate from published 0.20.2.

The candidate passed `cargo build --locked`, all 485 ordinary locked tests,
strict all-target Clippy, formatting and diff checks. Its 24 ignored tests
remain a separate inventory. The applicable live creation check passed serially
in 11.31 seconds with
`cargo test --locked --test live hub_creation_confirm_hands_the_reviewed_graph_to_the_existing_executor -- --ignored --exact --test-threads=1`,
using Git 2.52.0.windows.1, Rokit 1.2.0 and Rojo 7.7.0. It confirms creation,
saved-composition consistency and cancelled replay on an already configured PC.
The machine configuration hash was unchanged, and unique project/setup fixture
inventories returned to their baseline after the test.

Clean `cargo package --locked` at `a7491aa` verified the crate builds from its
tarball. All 105 package paths match the previously inspected manifest; required
source/tests, Cargo metadata, README/license and tracked root configuration are
present, while docs/workflows/CodeGraph/target caches and local credentials are
absent. Manifest/root lockfile version and clean archive Git identity were checked;
the rebuilt binary reports `rproj 0.20.3`.

Local CodeRabbit iteration review found one minor release-notes state mismatch:
wording implied pending checks were already recorded. That wording was corrected
before completed local evidence was added. Final branch review at `3ca97c7`
completed with zero findings across all seven files. This review-evidence update
changes excluded documentation only; packaged source, tests, dependencies,
manifest and README remain identical. Exact-head/main CI are required before
publication; their outcomes are recorded in the candidate PR as gates complete.
Ongoing owner consent covers tracked rproj review
diffs and public context. Owner publication remains pending; no 0.20.3 tag or
GitHub release precedes verified publication.

The live creation check uses already provisioned tools and uniquely named
temporary project/setup fixtures. It does not install machine applications or
replace existing projects. Fresh-Windows installation/retry and the deferred
Open Cloud/platform Studio gaps remain unverified; VM provisioning stays deferred.

## October 7: missing-Git project-creation recovery

The disposable audit exercised the same `new::execute_confirmed` execution
boundary used after New Project confirmation, with a temporary destination and
a child process whose PATH contains only an empty fixture directory. Before
creation, the child proved that `git --version` fails with `NotFound`. It did
not change the parent process or Windows PATH, uninstall Git, provision tools
or run a network-dependent scaffold.

On the published 0.20.2 behavior the regression failed: creation returned only
`failed to spawn git: program not found`. The executor also warned that the
newly claimed destination remains and will not be overwritten. The repair now
names Git's prerequisite, links to `https://git-scm.com/install/`, asks for PATH
and new-terminal verification with `git --version`, and instructs a retry in a
new destination after inspecting the failed attempt's folder.

The error chain retains the original OS error. Guidance applies only to
`NotFound` while the working directory still exists; permission failures,
nonzero Git commands and missing working directories retain their diagnostics.
The regression now passes, verifies unchanged configuration and existing
fixture bytes, and leaves the failed destination empty. A separate fixture
with `.git` still bypasses initialization even without Git on PATH. Parent
PATH remains unchanged. The test child runs serially to prevent libtest warning
output from contaminating its captured transcript.

Focused creation and Git error-boundary checks pass. The 485-test ordinary
locked suite, strict all-target Clippy, formatting and diff checks pass; the
24 ignored checks remain separate. Local CodeRabbit iteration and final branch
reviews reported zero findings across all four changed files; final review was
at `12401c7`. This evidence-only update changes no runtime or test content.
Exact final-head/main CI evidence is recorded in the repair PR before completion.
This is an unreleased runtime
diagnostic repair on 0.20.2; Cargo versions and dependencies are unchanged.
Successful installation/retry on fresh Windows and other deferred acceptance
gaps remain unverified. No new feature milestone or host provisioning is added.

## October 7: owner publication of 0.20.2 verified

The owner published 0.20.2 after [PR #85](https://github.com/chatarabdelilah/rproj/pull/85)
merged at `eaa0447d73751f147c2138b106f1ceb263617e10`. Its final-head CI
[37564015631](https://github.com/chatarabdelilah/rproj/actions/runs/37564015631)
and exact-main CI [37564268978](https://github.com/chatarabdelilah/rproj/actions/runs/37564268978)
passed on their first attempts. Fully merged candidate branches were removed
after identity, ancestry and tree-equality checks; main was clean for publication.

The official crates.io archive SHA-256 matches registry checksum
`260f61d5baa9b5e1ea422b4f85a4356d790bb2fd36c24fbc91a874f09c526745`.
Archive manifest/root lockfile version is 0.20.2; `.cargo_vcs_info.json` names
the clean reviewed commit `eaa0447d73751f147c2138b106f1ceb263617e10`. Annotated
tag object `a51104a601c7305c600c357dbd3a34c5c26acf1a` peels to that commit
locally and remotely. The matching [GitHub release](https://github.com/chatarabdelilah/rproj/releases/tag/v0.20.2)
is an alpha prerelease. No shipped tag was moved.

This publication-state documentation update does not change Cargo versions,
dependencies, runtime or tests. It is directly inspected under AGENTS.md's
documentation-only review exception; exact final-head/main CI evidence is
recorded in its documentation PR. Fresh-Windows/Open Cloud/platform Studio
acceptance remains unverified. Missing-Git recovery is a proposed bounded audit,
not a confirmed bug; its temporary child-PATH fixture leaves host Git untouched.

## October 7: 0.20.2 alpha candidate preparation

The candidate is based on clean main `481e33b`, after [PR #84](https://github.com/chatarabdelilah/rproj/pull/84)
passed exact final-head CI [37562556726](https://github.com/chatarabdelilah/rproj/actions/runs/37562556726)
and merged-main CI [37562809134](https://github.com/chatarabdelilah/rproj/actions/runs/37562809134)
on their first attempts. It packages the Watch acknowledgment runtime repair,
Template Explorer PTY synchronization and remaining UI child-output isolation.

`Cargo.toml` and the root `Cargo.lock` package are aligned at 0.20.2. Dependency
versions and runtime/test source are unchanged from verified main. Candidate
notes and current-state documentation distinguish the unpublished candidate
from immutable published 0.20.1.

The candidate passed `cargo build --locked`, all 481 ordinary locked tests,
`cargo clippy --locked --all-targets -- -D warnings`, formatting and diff checks.
Its 24 ignored tests remain a separate inventory. Both applicable installed-tool
Watch acceptances passed serially with
`cargo test --locked --bin rproj background_watch::windows::tests:: -- --ignored --test-threads=1`
in 6.32 seconds, using Rojo 7.7.0 and Wally 0.3.2.

Clean `cargo package --locked` at `a15099c` verified the crate builds from its
tarball. All 105 packaged entries were inspected: source/tests, Cargo metadata,
README/license and tracked root configuration are present; docs, GitHub workflows,
CodeGraph/cache/target paths and local credentials are absent. The manifest and
lockfile identify 0.20.2, and `.cargo_vcs_info.json` names the clean candidate
commit. The rebuilt binary reports `rproj 0.20.2`.

Local CodeRabbit iteration and final branch reviews both reported zero findings
across all seven files; final review was at `b2502ed`. This evidence-only update
does not change runtime/tests, dependencies or packaged content. Exact-head/main
CI evidence is recorded in the candidate PR before completion. Ongoing owner
approval covers tracked rproj review diffs
and public repository context. Publication remains the owner's action; no 0.20.2
tag or GitHub release is created before registry/archive verification.

Fresh-Windows/Open Cloud/platform Studio acceptance remains unverified. The
owner's Hyper-V/VM deferral remains in force. Missing-Git recovery through a
disposable child-PATH fixture is the next development audit after publication
alignment.

## October 7: Watch acknowledgement budgets and peer failure isolation

The follow-up to [PR #83](https://github.com/chatarabdelilah/rproj/pull/83) inspected
the first head-CI failure: the flood scenario's Stop request returned Windows
error 10060, while the supervisor log ended with an aborted connection (10053).
Source inspection found that Stop and engine-ready acknowledgements follow a
durable session-state flush but shared the 300 ms status-probe read deadline.
An acknowledgement write error also escaped the control loop and tore down the
owned engine rather than remaining a failed exchange with that peer.

Three stalled/fragmented-peer probes passed on the original code and are not
retained as regressions. A separate Windows socket probe observed immediate
WouldBlock on incomplete accepted peers; these checks do not support a server
concurrency rewrite. Their scratch source, patch and logs remain under ignored
`target/watch-control-*` paths.

Two targeted regressions failed before the repair: an authenticated Stop reply
delayed by 750 ms produced 10060, and a connection-aborted reply escaped as a
supervisor failure. Stop and engine-ready replies now have a two-second read
budget; status, connect, send, lock and ordinary-test deadlines remain unchanged.
Requests and replies serialize into one buffer before writing. Serialization,
state-persistence and ownership errors still propagate; a peer's reply-delivery
failure is logged without killing the engine. Version/nonce and token checks,
state-before-ack ordering, and process-tree cleanup remain intact.

Connection, send and acknowledgement failures now carry separate context. The
controlled tests prove the repaired response policies; they do not identify the
historic runner's exact disk/scheduler timing. A future timeout can therefore
be assigned to its actual phase rather than assumed to be a stopped owner.

All ten runnable Watch checks pass against the rebuilt supervisor (7.83 seconds).
The two ignored installed-tool acceptances pass explicitly (7.30 seconds): live
Rojo detachment/update/stop and Wally/Jest background recovery, using installed
Rojo 7.7.0 and Wally 0.3.2. The 481-test ordinary suite, strict Clippy, formatting
and diff checks pass; the 24 ignored checks remain a separate inventory.
Automatic approval review rejected the local CodeRabbit export because previous
consent named a different five-file payload; that command did not run. Explicit
ongoing owner approval now covers tracked rproj review diffs and public repository
context. Local CodeRabbit review completed with zero findings at `f1ba499` across
all four changed files. This review-evidence update is documentation-only; runtime
and test content remain identical to that reviewed head. [PR #84](https://github.com/chatarabdelilah/rproj/pull/84)
records exact final-head and merged-main CI evidence as those gates complete.

This is an unreleased runtime fix on the 0.20.1 baseline; Cargo versions and
dependencies are unchanged. Prepare 0.20.2 only after the runtime PR is complete.
Fresh-Windows/Open Cloud gaps remain unverified; VM provisioning stays deferred.

## October 7: remaining unit-test UI child output boundaries

The editor repair merged in [PR #82](https://github.com/chatarabdelilah/rproj/pull/82)
at `8e6a6ab`, after zero local CodeRabbit findings and first-attempt Windows
stable, Rust 1.89 and package CI on both the reviewed head and merged main.

CodeGraph exploration followed by a focused source inventory found nine PTY
launch sites that run the unit-test executable: two project-creation launches,
one editor launch, four Saved Setup launches, one capability-prompt launch and
one shared Machine Setup launch. Five already selected a single child test
thread. The other four exposed the same libtest parallel timer-output boundary
reproduced in the editor investigation; each now passes `--test-threads=1`.
Ordinary parent-suite parallelism, timeouts and UI assertions are unchanged.

Projects' selected-path command driver and Watch's process fixtures run through
ordinary subprocess paths, rather than a live libtest-hosted PTY UI. Captured
execution fixtures likewise do not write directly into the UI screen. They were
inspected and are outside this four-line repair.

The existing affected tests pass: six Machine Setup checks (2.10 seconds), one
capability-prompt check covering five workflows (1.28 seconds), and thirteen
Saved Setup checks (2.09 seconds). The Saved Setup baseline also passed before
the flag change; this audit does not claim a new ordinary-test failure or a new
slow reproduction. PR #82's controlled 72.46-second proof records the shared
child-runner mechanism.

The full ordinary suite, strict Clippy, formatting and diff checks pass.
The first reviewed-head CI attempt passed Windows stable and packaging, but
Rust 1.89 failed in the unchanged Watch flood scenario's stop request at
`src/background_watch/tests.rs:544` with Windows error 10060. Watch source and
fixture have no diff in this repair. The exact lifecycle acceptance passed
locally in 6.83 seconds; only the failed job was requested for retry on the same
reviewed head. This recurring Watch control timeout remains unresolved and is
the next development priority; final retry/main evidence belongs in the PR.

Automatic approval review blocked the local CodeRabbit export because the
previous consent did not clearly cover this changed five-file payload; that
review command did not run. The owner subsequently explicitly authorized the
five-file payload. Required local review/head/main CI evidence belongs in the
follow-up PR.
This is test-only work; published 0.20.1, runtime behavior,
dependencies and Cargo versions remain unchanged. Fresh-Windows/Open Cloud
acceptance gaps remain unverified, and VM provisioning stays deferred.

## October 6: editor PTY paste and child-harness output isolation

Investigation of PR #80's first stable-main failure initially considered lost
paste input. A character-acknowledged probe then exposed a different boundary:
the child libtest watchdog printed its 60-second warning into the live JSON
screen. The captured row contained `editor_pty_driver has been running for over
60 seconds` amid the JSON field; text/cursor assertions could no longer trust
the displayed row. This was harness output interference, not evidence that
the saved JSON or production editor had lost input.

The child runs one editor driver with `--test-threads=1`, avoiding libtest's
parallel timeout notification loop while leaving the ordinary parent suite
parallel. The same slow malformed-template probe then passed in 72.46 seconds.
Rust's [1.89 runner source](https://github.com/rust-lang/rust/blob/1.89.0/library/test/src/lib.rs#L371)
and 1.94 implementation both use blocking completion receive in that single-
thread branch rather than the parallel timer notification path. No timeout
was extended. The probe patch and logs remain in ignored `target/editor-paste-*`
artifacts; the slow character-pacing prototype is not retained in normal tests.

Final helpers send the complete bracketed-paste payload and wait for its final
row/cursor acknowledgment. They also wait for the complete JSON frame and the
cursor after the opening brace before inserting a field. A deterministic
regression rejects incomplete text, stale cursor position and text on another
row. It failed with a text-only acknowledgment and passes with the row/cursor
check. Existing byte/absence preservation, concurrent-edit refusal, locked-save
retry, resize recovery and terminal restoration assertions remain intact.

All 24 runnable editor tests pass (8.17 seconds); two installed-Rojo tests are
ignored in that run. The affected installed-Rojo editor PTY save/refusal check
passes explicitly (5.92 seconds). Ordinary locked validation, formatting,
clippy with warnings denied and diff checks pass. Required local CodeRabbit
review was blocked by automatic approval review because it treated source-diff
export as requiring payload-specific authorization; no rejected review ran.
The owner subsequently gave explicit CodeRabbit authorization in this chat.
Review and exact-head/main CI follow-through are recorded in the repair PR.

Changes are confined to test code, the shared PTY harness and documentation.
Published 0.20.1, Cargo versions, dependencies and production behavior remain
unchanged. Fresh-Windows/Open Cloud gaps remain unverified; VM provisioning
stays deferred.

## October 6: 0.20.1 owner publication and release alignment

Crates.io confirms owner publication of 0.20.1; it is current and not yanked.
The downloaded archive's SHA-256 matches registry checksum
`17085cc851e7d73cbdf3de2671380e86bde3ff664a5ecaebd3aa594b329fd0bf`.
Its manifest and root lockfile are 0.20.1, and `.cargo_vcs_info.json` identifies
clean reviewed release commit `07634cc90dc638abbea113574134d3ddc501df4b`.

[Candidate PR #80](https://github.com/chatarabdelilah/rproj/pull/80) passed both
local CodeRabbit reviews with zero findings and all exact-head checks at
`cdd69f6` ([run](https://github.com/chatarabdelilah/rproj/actions/runs/37445624015)).
Merged-main [run](https://github.com/chatarabdelilah/rproj/actions/runs/37446019202)
passed on attempt 2: its first stable attempt timed out waiting for pasted JSON
in three existing Template Explorer PTY tests. All 11 runnable editor PTY checks
passed locally (one installed-Rojo check ignored), and unchanged main passed
the failed-job retry. The watcher subsequently encountered GitHub HTTP 502;
a fresh run query confirmed success. Retry success did not repair the harness.
The next bounded task is to synchronize editor PTY paste/ready-state assertions
while preserving input and file-protection coverage.

The annotated `v0.20.1` tag was created only after published-archive verification.
Tag object `eacb94a2e96848f988f50734befb3511ff1924f7` resolves locally and remotely
to `07634cc`. The matching public GitHub release remains an alpha prerelease
at that commit; no shipped tag moved. Candidate branches were verified and
removed after green CI. README, roadmap, release instructions and dedicated
notes now describe published 0.20.1. This alignment changes documentation only;
Cargo versions, runtime, tests and dependencies retain the shipped identity.
No repeated local runtime tests or CodeRabbit review are required for the small
documentation correction; its PR records direct inspection and exact-head/main CI.

Fresh-Windows, authenticated Open Cloud and recorded platform/Studio acceptance
gaps remain unverified. Owner-deferred Hyper-V/VM provisioning stays deferred.

## October 6: 0.20.1 prerequisite-recovery candidate

The patch candidate packages the missing-Cargo recovery from PR #77 and
missing-WinGet recovery from PR #78, plus PR #79's physical-row assertion
repair. Cargo.toml and the root Cargo.lock package agree on 0.20.1; dependency
versions and runtime source are unchanged from reviewed main `3291c75`.
Dedicated candidate notes, README, roadmap and release instructions distinguish
the unpublished candidate from the immutable published 0.20.0 baseline.

Candidate validation passes 477 ordinary locked tests with 24 ignored,
formatting, clippy with warnings denied and diff checks. The ordinary suite
includes all ten Cargo/WinGet bootstrap regressions and the Projects/Catalog
terminal assertions. Both explicit installed-tool background Watch acceptances
pass serially (8.95 seconds): persistence after launcher exit, sourcemap updates,
clean stop and Wally/Jest recovery with preserved types and mounts. Command:
`cargo test --locked --bin rproj installed_ -- --ignored --test-threads=1`.
Tools: Rojo 7.7.0, Wally 0.3.2, wally-package-types 1.6.2, Rust/Cargo 1.94.0.
These checks exercise temporary project/session storage with installed tools.
No fresh-machine or authenticated Open Cloud check is counted as a pass.
Clean locked packaging and file inspection are recorded in the candidate PR.

PR #79's initial MSRV run timed out on a Watch stop request with Windows error
10060 at `src/background_watch/tests.rs:544`. Its focused local lifecycle test,
unchanged-head retry and first-attempt merged-main CI passed. This candidate
does not claim that retry fixed the timeout; retain the evidence and investigate
if it reproduces during applicable validation.

Whole-branch CodeRabbit review, exact-head CI, merged-main CI and branch cleanup
are recorded in the candidate PR. Owner publication remains pending. No 0.20.1
tag or GitHub release is created before the published archive is verified.
Fresh Windows, authenticated Open Cloud and the other recorded platform/Studio
acceptance gaps remain unverified; Hyper-V/VM provisioning stays deferred.

## October 6: Projects physical-row assertion recovery

PR #78's first MSRV CI attempt failed the Projects resize assertion. The
expected and actual screens contained identical characters but differed by
one newline at offset 600: `vt100::Screen::contents()` joins soft-wrapped
rows, and ConPTY can change wrap flags without changing visible cells. The
unchanged head passed the failed-job retry and merged-main CI; that retry
did not resolve the assertion defect.

Projects now captures and waits for complete physical-row snapshots through
the same shared helper as Catalog. The helper preserves row boundaries and
trims only trailing spaces within each row. Its deterministic regression
accepts equivalent wrapped/cursor-positioned screens and rejects a different
layout with the same characters. Filter, detail scrolling, Help, Home/End,
project opening and Home return remain covered by the real-terminal test.

Focused validation passes all 12 Hub and 4 Catalog tests. Ordinary locked
validation passes 477 tests with 24 ignored, plus formatting, clippy with
warnings denied and diff checks. Local CodeRabbit review and reviewed-head/main
CI are recorded in the repair PR. This is test-only work; published v0.20.0
and the two unreleased prerequisite diagnostic corrections remain unchanged.
Fresh Windows and authenticated Open Cloud acceptance remain unverified,
and VM provisioning remains deferred.

## October 6: missing WinGet bootstrap recovery

The next bounded prerequisite check reproduced the same generic spawn error
when WinGet was unavailable. Disposable nonexistent-executable fixtures cover
both plain capture and the reported execution used by Machine Setup; the
App Installer recovery assertion failed before the correction.

WinGet now names App Installer's Microsoft installation/repair documentation,
PATH verification, a new terminal, `winget --version` and a `rproj setup` retry.
The Cargo and WinGet paths share NotFound-only context handling and preserve
the underlying OS error. Regressions preserve permission errors and successful
or failed installer output, including the existing hash-mismatch payload.
All 10 bootstrap regressions pass, including the earlier Cargo checks. Ordinary
locked validation passes 476 tests with 24 ignored, plus formatting, clippy
with warnings denied and diff checks. Local whole-branch review and
exact-head/main CI evidence are recorded in the PR; ignored prerequisites
remain separate from these passing ordinary checks.

These fixtures install no tools, mutate no process-wide PATH and do not change
the owner's Windows features, drivers or applications. Fresh-Windows acceptance
remains unverified and VM provisioning stays deferred. Published 0.20.0 remains
unchanged; this compatible recovery correction is unreleased.

## October 6: deferred VM provisioning and Cargo bootstrap recovery

Read-only inventory found Windows 11 Pro with firmware virtualization enabled,
but no ready local VM manager or installed Hyper-V module. Hyper-V and Windows
Sandbox features were disabled. The owner has only the already-provisioned PC
and deferred the proposed VM setup; no host Windows feature, driver or
application was changed for this audit. Do not resume host provisioning on a
routine request to continue. Fresh-Windows acceptance remains unverified.

The next bounded existing-workflow check reproduced missing-Cargo recovery:
the Rokit bootstrap reported only a spawn error, with no Rust/PATH instructions.
An isolated nonexistent executable reproduced the underlying OS NotFound error;
the recovery assertion failed before the correction. The shared bootstrap now
adds the Rust installation URL, a new-terminal/PATH check and `rproj setup`
retry instructions for NotFound only, for both ordinary and reported execution.
Permission errors and Cargo build failures retain their original diagnostics.
No tool installation or machine-wide prerequisite provisioning is used by these
regressions. Both focused Cargo-bootstrap tests pass after the correction.
Ordinary locked validation passes all 474 runnable tests with 24 ignored;
formatting, clippy with warnings denied and diff checks also pass. The ignored
checks were not executed by this change and do not establish fresh-machine or
authenticated Open Cloud evidence.
Whole-branch review and exact-head/main CI are recorded in the corresponding
PR. Published 0.20.0 and its immutable tag remain unchanged; no candidate bump
is prepared by this fix.

## October 6: 0.20.0 owner publication and release alignment

Owner publication is confirmed on crates.io; current newest/max version is
0.20.0 and it is not yanked. The downloaded published archive's SHA-256 matches
registry checksum `175f1bb4b3b746960c623bfdaf29600c3069707bd3e70c253ff8deb421b2f190`.
Its manifest declares 0.20.0 and `.cargo_vcs_info.json` identifies clean reviewed
release commit `f3cdea8d1c88a8acb5831c00a3d7f44c2d6654b0`.

[Release PR #75](https://github.com/chatarabdelilah/rproj/pull/75) passed
whole-branch CodeRabbit with zero findings and all required CI on reviewed head
`41f352b` ([run](https://github.com/chatarabdelilah/rproj/actions/runs/37377921971))
and merged main `f3cdea8`
([run](https://github.com/chatarabdelilah/rproj/actions/runs/37378547740)).
Local verification passed 472 ordinary tests and 21 explicit checks; clean
locked packaging compiled its inspected 105-file archive. The release branch
was checked against the merge tree and main ancestry, then removed locally
and remotely before publication.

Annotated tag `v0.20.0` was created only after archive verification. Tag object
`0ce9fe32c1b592c713c128bc79680528a5f50728` resolves locally and remotely to
`f3cdea8`. The matching [GitHub release](https://github.com/chatarabdelilah/rproj/releases/tag/v0.20.0)
targets that exact commit, is public, and remains marked as a prerelease.
Publication dates here use Europe/Brussels: the registry's October 5 22:18 UTC
timestamp is October 6 locally. No shipped tag was moved.

README, roadmap, release instructions and dedicated release notes now describe
published 0.20.0. This follow-up changes documentation only; Cargo versions,
runtime, tests and dependencies retain the published identity. No repeat local
runtime tests or CodeRabbit review are required for this correction; its PR
records documentation inspection and required exact-head/main CI.

Fresh-Windows provisioning and authenticated Open Cloud execution remain
unverified. The environment inventory and subsequent owner decision are
recorded above. Package additions and further audits remain uncommitted backlog.

## October 5: 0.20.0 alpha candidate preparation

The candidate starts from clean main `ece51e5`, containing background Watch v1
and its reviewed follow-ups. [PR #74](https://github.com/chatarabdelilah/rproj/pull/74)
passed exact-head CI and merged-main CI on `ece51e5`
([final attempt](https://github.com/chatarabdelilah/rproj/actions/runs/37373801957/attempts/2)).
The first main attempt passed stable and packaging, but GitHub could not acquire
a hosted runner for Rust 1.89; it ran no test steps. Only that job was retried,
and its tests passed. All four merged follow-up branches were checked against
their squash-merge trees and main ancestry, then removed locally and remotely.

Branch `codex/release-0.20.0` aligns Cargo.toml/Cargo.lock to 0.20.0 and updates
README, roadmap, release instructions and dedicated release notes. Runtime,
tests and resolved dependency versions are unchanged from `ece51e5`. Published
0.19.1 remains the registry baseline; no 0.20.0 tag or publication occurs during
preparation. Final CodeRabbit review, clean locked packaging, exact-head/main CI
and publication readiness are recorded in the release PR.

The provisioned audit machine has Rojo 7.7.0, Wally 0.3.2, Rokit 1.2.0 and
Lute 1.0.0. No background Watch was active before validation. Explicit live
checks use isolated temporary projects and existing shared tool/package caches.
No machine-wide applications are provisioned to complete this audit.

The candidate reports `rproj 0.20.0`. Ordinary locked validation passed 472
tests with 24 ignored (496 discovered), formatting, clippy with warnings denied
and diff checks. Explicit serial validation passed all 14 live workflows
(138.33 seconds), four real-Rojo template checks (9.49 seconds), both installed
background Watch checks (5.77 seconds), and the installed Jest 0.4.1
missing-credential refusal check (4.46 seconds): 21 separately executed checks.
Live Jest passed three starter specs and reported the deliberate failure as
two passed/one failed with exit 1. Background Watch continued sourcemap updates
after launcher exit and preserved recovered Wally/Jest types and mounts.

The first credential-check invocation mistakenly selected `.rokit/bin`'s
Rokit proxy; copying it into the isolated fixture produced Windows error 50.
The proxy has the same hash as the Rojo proxy. Pointing `RPROJ_LIVE_JEST_CLI`
to the existing `tool-storage/christopher-buss/jest-roblox-cli/0.4.1` executable
passed without source changes. The test never supplies complete cloud
credentials and targets loopback; authenticated Open Cloud remains unverified.

The symlink-privilege, upstream-badge and broader eight-example Jest-stack
ignored checks were not executed for this candidate. Fresh Windows provisioning,
fresh Linux generated-project execution, UI Labs Studio stories and Scribe
Studio playtesting remain unverified. These are residual acceptance gaps,
not inferred passes; published 0.19.1 remains unchanged.

## October 5: Persistent background Watch v1 (unreleased)

Before this correction, main was `e543203`, containing reviewed [PR #73](https://github.com/chatarabdelilah/rproj/pull/73).
Its final reviewed head passed all required CI, but merged-main CI
([run](https://github.com/chatarabdelilah/rproj/actions/runs/37305627562)) failed
Rust 1.89 while stable, clippy, formatting and packaging passed. The new
diagnostics identify the crash fixture's Watching acknowledgement: persisted
and live state were Failed with Windows error 5 (Access denied), although both
Rojo sourcemap commands had started. This was an actual terminal failure rather
than a slow acknowledgement, blocking release acceptance at that revision.

Branch `codex/watch-state-persistence-diagnostics` initially preserved stage-specific
state-write errors and a failing regression. Eight concurrent metadata readers
reproduced error 5 on the first atomic `session.json` replacement locally.
Three coordination prototypes failed the bounded concurrency check (exclusive
state lock, writer-intent gate, then buffered decoding). The gate diagnostics
showed waiting for the active state reader; those unsuccessful prototypes were
removed. The doubtful assumption was that the state lock's lifetime and fairness
matched the intended read/replace interval. The resumed minimal probes confirmed
that dropping a handle releases its lock and individual snapshot reads finish
normally. A guarded 50-write probe passed, while a 500-write probe exposed reader
starvation when the lock also covered staging and disk flushing.

The correction uses shared snapshot-read locks and an exclusive replacement
lock, with a separate writer-intent gate preventing new readers from starving
replacement. Staging and flushing occur before acquiring these locks; decoding
uses a buffered reader. The lock deadline remains 300 ms. A held state lock
refuses reads/writes and preserves the snapshot; dropping it allows reading
again. The original eight-reader, 500-replacement regression passes, including
three further focused runs, with no read errors.

The first correction head's CI completed all 500 replacements and passed the
Watch lifecycle scenarios, but stable failed because the stress test counted
16 bounded busy responses as unexpected read errors
([run](https://github.com/chatarabdelilah/rproj/actions/runs/37370739903)).
Contention now returns a typed `WouldBlock` error with the same retry message
and unchanged deadline. The stress test retries that result, still rejects
missing or malformed snapshots and every other read error, requires successful
reads, and requires all 500 writes to succeed. The held-lock check verifies
the typed error for both reads and writes.

Ordinary validation passed 472 tests with 24 ignored (496 discovered). Both
explicit installed Rojo/Wally/Jest acceptances passed (7.72 seconds), along with
formatting, clippy with warnings denied and diff checks. Temporary probes were
removed. The final whole-branch local CodeRabbit review reported zero findings.
Locked packaging and final-head/main CI are recorded in
[the correction's PR #74](https://github.com/chatarabdelilah/rproj/pull/74).
Keep merged follow-up branches until corrected final-main
CI is green. Published 0.19.1 remains unchanged; 0.20.0 candidate preparation
follows successful acceptance.

Main `acf9c70` contains the completion-marker and Catalog visible-row corrections
from [PR #72](https://github.com/chatarabdelilah/rproj/pull/72). Its reviewed-head
CI passed; merged-main stable subsequently exposed a Stop control timeout after
output flooding ([run](https://github.com/chatarabdelilah/rproj/actions/runs/37302044628)).
The supervisor's single-thread runtime also ran synchronous disk writes and log
rotation. Those operations now run on Tokio's blocking pool, with each pipe
awaiting its current chunk to retain bounded buffering and propagate errors.
A deterministic regression holds the log mutex until a control-runtime timer
releases it; disk work must yield rather than block that timer.
The regression failed with the original synchronous drain (2.01 seconds), then
passed with blocking-pool writes (0.07 seconds). The ordinary suite passed
470 tests with 24 ignored (494 discovered), and both explicit installed-tool
checks passed (5.94 seconds). All seven lifecycle scenarios passed in three
further focused runs. Formatting, clippy with warnings denied and diff checks
passed. Locked packaging verified the 105-file archive and compiled it after
committing the runtime correction. Local review and final-head/main CI are
recorded with the correction's PR.

The correction is tracked in [PR #73](https://github.com/chatarabdelilah/rproj/pull/73),
branch `codex/watch-control-during-log-drain`. Whole-branch local CodeRabbit
completed with zero findings on `8b76826`. Exact-head CI
([run](https://github.com/chatarabdelilah/rproj/actions/runs/37303706265)) passed
Rust 1.89 and packaging, but stable timed out in the normal Watch lifecycle
scenario. The generic wait did not identify which phase timed out; this result
does not establish that the log-drain correction failed or that the remaining
failure is only a fixture defect. At that revision, the PR remained unmerged and
release acceptance was incomplete. After three corrections without clean final
CI, speculative fixes stopped in favor of phase/state/log diagnostics. Retain
all merged follow-up branches until final acceptance is green.

On resuming, documentation-only head `afa0388` had passed all required CI gates
with unchanged runtime/tests ([run](https://github.com/chatarabdelilah/rproj/actions/runs/37304172636)).
The lifecycle fixture now reports named wait phases and caller locations, plus
persisted/live state and bounded log tails before panic cleanup. Snapshot tokens
are excluded and log control characters are escaped. All seven scenarios passed
locally with these diagnostics (8.13 seconds); clippy and diff checks passed.
Timeouts and acceptance assertions remain unchanged. A recurrence must be
diagnosed from this evidence rather than attributed to an unspecified race.
Diagnostic head `054a470` passed all required CI gates
([run](https://github.com/chatarabdelilah/rproj/actions/runs/37304758537)); its
narrow follow-up CodeRabbit review reported zero findings. The normal timeout
did not recur locally or in these two subsequent CI runs. Its cause remains
unconfirmed; the diagnostics remain available if it recurs. Final documentation
head and merged-main CI are recorded in PR #73. Published 0.19.1 is unchanged;
the next release task remains 0.20.0 alpha candidate preparation.

Runtime implementation merged in [PR #70](https://github.com/chatarabdelilah/rproj/pull/70).
Both local whole-branch CodeRabbit reviews reported zero findings. CI passed on
reviewed head `8abc188` ([run](https://github.com/chatarabdelilah/rproj/actions/runs/37298235374))
and merged main `8b0b9d1` ([run](https://github.com/chatarabdelilah/rproj/actions/runs/37298724095)):
Windows stable, Rust 1.89, locked packaging, stable clippy and formatting.
Final local ordinary validation passed 469 tests with 24 explicitly ignored
(493 discovered: 418 unit and 75 integration). Locked packaging verified its
105-file archive and compiled it. The save-refusal regression preserves bytes
and the pending draft under lost Watch control, then saves successfully after
ownership clears.

A test-only correction restores actual competing-start coverage: the earlier
WMI wrapper waited for its first launcher before starting the second. Two
independent launchers now write distinct readiness markers, wait at a shared
barrier, then start together. Both results must identify the same supervisor
or explicitly report busy startup, and exactly one Rojo watcher may launch.
All seven lifecycle cases passed again (8.16 seconds); runtime code is unchanged.

Final main CI on `6b652d5` passed Rust 1.89 and packaging, but stable reproduced
a completion-marker fixture race: `Set-Content` created the file before closing
its exclusive handle, and `finish_launcher` read it immediately (Windows sharing
violation 32, [failed run](https://github.com/chatarabdelilah/rproj/actions/runs/37299861247)).
The test launcher now writes a sibling pending file with `File.WriteAllText`,
closes it, then atomically moves it to the completion path. Marker existence
means the complete result is readable; exit-code checks remain strict. Runtime
code is unchanged; this failure is not counted as passing evidence.
The corrected fixture passed the ordinary suite (469 passed, 24 ignored) and
both explicit installed Rojo/Wally/Jest checks (5.84 seconds), formatting and
clippy. The narrow local CodeRabbit retry was rate-limited for 25 minutes;
this small test-only diff was inspected directly, not recorded as a passed
CodeRabbit review. The runtime's two earlier local reviews remain applicable.

The correction's first final-head CI passed all Watch lifecycle cases, Rust 1.89
and packaging, but stable exposed an unrelated Catalog PTY comparison failure
([run](https://github.com/chatarabdelilah/rproj/actions/runs/37301012745)).
Its visible rows matched after resize; ConPTY's soft-wrap flags differed,
changing `Screen::contents()` line joins. The Catalog fixture now compares each
visible row with its original layout, retaining exact content and scroll checks
while excluding soft-wrap metadata. This changes only the test assertion.
Ten consecutive focused Catalog runs passed locally; formatting, clippy with
warnings denied and diff checks passed.

The owner approved one persistent Windows sourcemap Watch per user, manual
startup and stop after the active recovery command. Work starts from main
`5ee9809`; published 0.19.1 remains unchanged. Direct foreground Watch remains
available; the hub defaults to background Watch and includes status/logs/stop
and an explicit foreground action. Home refreshes the active project/state.

The hidden supervisor owns an engine and all tool descendants through
process-wrap 10.0.1's Tokio JobObject plus KillOnDrop. An exclusive lifetime file
lock is ownership authority; status readers share their probes. Restricted
storage and separately authenticated versioned loopback control prevent token
disclosure through normal UI/log output. Stale PIDs are never used for control.
Conflicting project writes/Machine Setup require stopping Watch; tests coexist
with Watching and are blocked during recovery/stopping or lost control.

Executed evidence:

- Seven ordinary lifecycle cases verify launcher exit/reopened control,
  idempotent and competing starts, different-project refusal, recovery stopping,
  watcher failure, supervisor crash and grandchild cleanup, blocked job
  breakaway, and output saturation. Invalid control tokens and held/corrupt or
  unresponsive ownership are refused. Mutation policy is checked separately.
- The lifecycle fixture found two defects in the new implementation before
  review: append-only Windows log access could not truncate during rotation;
  exclusive status probes could look like an owner to concurrent readers.
  Logs now retain write access and seek after truncation; probes share locks.
- `cargo test --locked`: final ordinary validation passed 469 tests. Both
  installed-tool acceptances are ignored by default and were executed explicitly;
  they do not replace ordinary evidence. Formatting, clippy with warnings denied,
  and diff checks passed.
- `cargo test --locked installed_rojo_keeps_updating_after_launcher_exit -- --ignored --nocapture`:
  passed with installed Rojo 7.7.0. A source added after the launcher exited
  appeared in the sourcemap; stopping completed and removed owned processes.
- `cargo test --locked installed_wally_and_jest_background_recovery -- --ignored --nocapture`:
  passed for both generated Wally and Jest compositions (6.93 seconds). Installed
  Wally and wally-package-types restored Charm exports; Jest's generated project
  retained its development-package mount; both watchers updated after launcher
  exit and stopped. Initial acceptance fixtures omitted generated package mounts
  and assumed Wally's alias extensions; those fixture errors were corrected.

Tests own unique temporary projects/session storage and verify child locks are
released. Codex's containing job denies breakaway, so ordinary positive fixtures
use a hidden WMI-created test launcher outside that job; a separate negative
case verifies the product's refusal. Production has no WMI escape fallback.
No applications or startup tasks were installed, and existing projects were
not replaced. Authenticated Open Cloud and fresh-machine provisioning remain
unverified. No automatic restart, Rojo Serve, beta date or publication is implied.

## October 5: Bounded New Project execution-boundary audit

Audited clean main `0e05001`, with successful merged-main CI and no open PRs.
CodeGraph traced Home confirmation, `creation::prepare`, `Prepared::execute`,
`new::execute_confirmed`, scaffolding and named-setup persistence. No actionable
defect was established in this bounded review; no runtime change or new test
was needed.

The creation model requires a separate Create confirmation. Preparation checks
the final destination and setup name before returning a prepared operation;
Home suspends the terminal before execution. The executor checks interruption
and claims the destination with `create_dir`, refusing an existing directory
even if it appeared after review. Project records and setup saving occur after
scaffolding; a new named setup is staged, synced and committed without clobbering
an existing file. Execution errors return failure and warn that partial project
files remain for inspection. These operations are not a multi-file transaction:
a later failure can leave a project and an already saved setup. The audit does
not claim rollback or exhaustive failure injection.

Executed existing checks on this revision:

- `cargo test --locked --test live hub_creation_ -- --ignored --test-threads=1 --nocapture`:
  all three passed on the provisioned machine (19.77 seconds). Cancellation
  creates nothing and restores the terminal; a concurrently created destination
  is preserved; confirmed creation hands off the reviewed graph, saves identical
  project/setup bytes, returns to project actions, and replay cancellation leaves
  the saved setup unchanged.
- Existing ordinary tests for destination refusal/missing-parent creation,
  explicit Create/Ctrl+C behavior and atomic no-clobber setup saving: four passed.

Live fixtures use reserved temporary setup names and unique project directories,
with their existing cleanup guards. No machine applications were installed;
this is not fresh-machine or hosted Open Cloud evidence. Broader partial-failure,
external-writer and installed-runner paths remain outside this bounded audit.

The audit is complete. Further resize matrices are coverage gaps rather than
automatic roadmap tasks; they should be selected for an observed defect or
specific acceptance requirement. Any next product feature needs a separately
scoped proposal, not another inferred test-only milestone.

## October 5: New Project strategy revision resize recovery

Work starts from clean main `9b040a5`, successful merged-main CI and no open
PRs. `new_project_resize_preserves_strategy_revision_and_testing_repair`
covers None/Git submodules crossed with TestEZ/disabled Testing. Each case
starts with a reviewed Wally/Open Cloud composition, explicit Signal package,
lint capability and optional-file exclusions.

The test filters Dependencies, preserves detail focus/scroll through 120 x 30,
80 x 24, 60 x 16, 40 x 10 and recovery, and closes Help to the identical
buffer. Escape at Dependencies restores the complete graph. Both cancellation
and acceptance passes resize again, including Git's cleared package selection,
then filter the required Testing decision and repeat the same resize/Help
checks. The incompatible-Jest warning and all graph state remain unchanged
during rendering. Escape at Testing repair restores the entire reviewed graph,
including packages, Open Cloud backend and exclusions.

Acceptance verifies the complete expected graph: the selected workflow/mode,
cleared explicit packages/exclusions, TestEZ's derived package or disabled
Testing, and unchanged lint. Reopening Dependencies retains the workflow;
Capabilities retains the repaired Testing choice and lint. A shared test-only
resize helper also continues to cover the existing capability revision.

This is model/render coverage, not strategy-path PTY input gating, preparation,
project creation or installed-runner execution. No runtime behavior or package
version changes; published 0.19.1 remains unchanged.

All three focused New Project TestBackend resize tests and all 464 ordinary
tests passed locally with normal Windows permissions. Twenty-two prerequisite
tests remain ignored (486 discovered). Formatting, diff checks and clippy with
warnings denied passed. CodeRabbit CLI 0.7.6 reviewed the final branch against
main with zero findings. Reviewed-head/merged-main CI evidence is recorded on
the accompanying PR after completion.

## October 5: New Project capability revision PTY resize recovery

Work starts from clean main `8a07456` with successful merged-main CI and no
open PRs. The isolated creation driver now also supports a reviewed expert/Wally
TestEZ composition, entering its capability revision without configuration,
template preparation or execution. Its first cancelled revision must restore
the entire reviewed graph; its final graph must match the Open Cloud composition
and raw mode must be restored.

`pty_new_project_resize_recovers_capability_revision_without_creating` filters
Capabilities, test implementation and Jest execution. On both cancellation and
acceptance passes, each screen retains its checked choices, filter, selection
and focused/scrolled details through 120 x 30, 80 x 24, 60 x 16, 40 x 10 and
recovery. Help closes to the complete expected screen at each size. Blocked
undersized focus/toggle/acceptance/text/paste/save inputs are followed by a Help
input barrier before recovery. Expected physical rows come from full TestBackend
renders at every size; leading/internal spacing and row boundaries are preserved.
Continued scrolling changes the recovered screen. The accepted Open Cloud
choice remains selected after reopening, cancelling that revision and exiting.

The temporary sentinel remains byte-identical, the root contains only that
sentinel and no project/setup is created. This covers production `next_effect`
input gating and terminal recovery, not Home dispatch, preparation, confirmed
creation, installed runners or authenticated Open Cloud execution. No runtime
or package-version change is needed; published 0.19.1 remains unchanged.

The focused PTY regression and all 463 ordinary tests passed locally with normal
Windows permissions; 22 prerequisite-dependent tests remain ignored (485
discovered). Formatting, diff checks and clippy with warnings denied passed.
Both package and capability PTYs passed 20 repetitions together with two test
threads. CodeRabbit CLI 0.7.6 reviewed the final test/documentation branch against
main with zero findings. Reviewed-head and merged-main CI evidence is recorded
on the accompanying PR after each run completes.

## October 5: New Project capability revision resize recovery

`new_project_resize_preserves_capability_revision_through_jest_execution`
starts with a reviewed expert/Wally TestEZ composition. It filters the checked
testing capability, chooses Jest Roblox and filters its Open Cloud execution
choice. At each screen it renders 120 x 30, 80 x 24, 60 x 16, undersized
40 x 10 and recovered 120 x 30; Help closes to the identical buffer, and
filter, selection, checked choices, detail focus/scroll and graph are retained.
One pass cancels the revision and restores the complete reviewed graph; another
accepts Open Cloud, verifies unrelated graph state and reopens the retained
execution choice.

The focused regression, formatting, diff checks, clippy with warnings denied
and all 462 ordinary tests passed locally with normal Windows permissions.
Twenty-two prerequisite-dependent tests remain ignored (484 discovered).
This is TestBackend model/render evidence, not a capability-path PTY run,
project creation or authenticated Open Cloud execution. Runtime behavior and
published 0.19.1 are unchanged.

CodeRabbit CLI 0.7.6 reviewed the final test/documentation branch against main
and reported zero findings. Reviewed-head and merged-main CI evidence belongs
in the accompanying PR; this audit does not infer a passing run before it finishes.

## October 5: Complete PTY resize expectations

PR #64's new creation regression passed both reviewed-head CI jobs and both
merged-main toolchains. Merged-main [CI at `7d3d0fd`](https://github.com/chatarabdelilah/rproj/actions/runs/37259045946)
nevertheless failed the existing Saved Setup recovery comparison on stable;
Rust 1.89 and packaging passed. That run is not counted as a passing main run.
The failing Capabilities screen matched a complete local baseline row-for-row.
CI did not capture its expected baseline, and 30 focused diagnostic repetitions
did not reproduce the mismatch, so a partial-baseline timing cause remains an
inference rather than a captured second root cause.

The follow-up removes that assumption: Saved Setup and New Project PTY tests
derive expected screens from completed TestBackend renders of their fixture
states. They wait for the entire expected screen before and after recovery,
after scrolling and, for Saved Setup, at Review. A first partial snapshot
matching a row label can no longer become the expected baseline. Physical
rows retain leading/internal spaces and row boundaries; trailing blank-cell
padding is normalized across backends. The existing regression also verifies
explicit trailing spaces do not change the visible-row comparison.

The first follow-up PR run passed both resize-baseline tests, stable and
packaging, but Rust 1.89 exposed a separate browser snapshot race: the
selected-row assertion at line 533 sampled an empty screen immediately after
fresh filter output. That assertion now waits for the selected name inside
the current list pane instead of assuming the filter redraw completes the
list. The failing run is retained on the PR and is not counted as a pass.

Both focused resize tests passed. The four concurrent Saved Setup PTYs and
the New Project PTY each passed 20 repetitions, with the two groups running
concurrently. All 461 ordinary tests passed with normal Windows permissions;
22 prerequisite tests remain ignored. Formatting, diff checks and clippy with
warnings denied passed. CodeRabbit CLI 0.7.6 reviewed the three-file diff with
zero findings. No runtime behavior, user storage or package version changes;
reviewed-head/merged-main CI evidence will be recorded on the follow-up PR.
After the browser correction, all four Saved Setup PTYs passed another 20
repetitions, all 461 ordinary tests passed again, and final whole-branch
CodeRabbit review reported zero findings.

## October 5: New Project package revision PTY resize recovery

Work started from clean main `c7ba7e2` (PR #63), with successful merged-main
CI and no open PRs, on `codex/new-project-pty-resize`. An ordinary PTY
regression starts an isolated expert/Wally draft at its package revision.
It filters `janitor`, checks it and focuses/scrolls details, then resizes
through 120 x 30, 80 x 24, 60 x 16, 40 x 10 and back, with Help at each size.

The unit-test driver calls production `creation::next_effect`, covering its
small-screen key/paste gate without configuration, template preparation or
execution. Undersized focus, toggle, acceptance and paste input cannot alter
the recovered screen. Physical-row snapshots retain the filtered/checked
revision and scrolled details; scrolling still changes the recovered screen.
The test accepts the revision, reopens the checked package, cancels that
revision and exits. The driver verifies the reviewed graph differs only by
Janitor and checks raw-mode restoration. A temporary sentinel stays
byte-identical; no project or setup is created and no tools are provisioned.
Home dispatch, preparation and confirmed creation remain separate coverage.

Initial harness failures identified two synchronization assumptions: ConPTY
diff output does not always repeat an unchanged filter label, and `Help` in
the undersized footer is not proof that the popup opened. Current-screen
predicates, the popup border title and Help input barriers address those
assumptions. No runtime defect was established.

All 24 creation tests passed, and the new PTY regression passed 20 consecutive
repetitions. Formatting, diff checks, clippy with warnings denied and all 461
ordinary tests passed with normal Windows permissions. Twenty-two
prerequisite-dependent tests remain ignored (483 discovered). CodeRabbit CLI
0.7.6 found one stale documentation total, now corrected and directly
inspected; no code findings were reported. Reviewed-head/merged-main CI
evidence will be recorded on the PR.
Published 0.19.1 remains unchanged; this test/documentation change needs no
version bump.

## October 5: PTY recovery comparisons preserve physical rows

Merged-main [CI at `12027c5`](https://github.com/chatarabdelilah/rproj/actions/runs/37243719351)
failed the Saved Setup editor's Capabilities recovery comparison on Windows
Rust 1.89. Concurrent local PTY runs reproduced the same timeout at Review
recovery. Temporary baseline diagnostics found identical visible text: the
expected string had 3,600 characters and no newlines, while the recovered
string had 3,601 characters and one newline. Removing only that newline made
the strings equal. The diagnostics were removed after capture.

`wait_screen` now compares physical rows from `vt100::Screen::rows`, instead
of `contents()` whose newline insertion depends on soft-wrap flags. Row
boundaries remain significant. A deterministic regression proves equal rows
with different wrap metadata compare equal, while a different layout with the
same flattened text compares unequal. Timeout panics now identify the caller.
This fixes a test assertion; no editor state-loss defect was established.

All 13 Saved Setups tests passed, followed by 20 consecutive repetitions of
the four PTY tests running together with four test threads. Formatting, diff
checks and clippy with warnings denied passed. All 459 ordinary tests passed
with normal Windows permissions; 22 prerequisite-dependent tests remain
ignored (481 discovered). CodeRabbit CLI 0.7.6 reviewed the two-file diff with
zero findings; reviewed-head/merged-main CI evidence will be recorded on the
PR. This test/documentation change needs no
version bump; published 0.19.1 remains unchanged.

## October 5: New Project package revision resize recovery

Work started from clean main `edb9f4b` (PR #61), with no open PRs, on
`codex/new-project-resize-coverage`. One ordinary TestBackend regression
opens an expert Wally composition's package revision, types `janitor`, checks
that package and focuses/scrolls details. The same app and terminal resize
through 120 x 30, 80 x 24, 60 x 16, 40 x 10 and back to 120 x 30.

The test retains the filter, selected/checked package and detail focus/scroll,
while leaving the reviewed graph unchanged until acceptance. It checks the
visible checked row at usable sizes, the resize message while undersized,
Help opening/closing at every size and identical recovered screen cells.
Scrolling still changes the recovered screen. Accepting the revision changes
only the package list; reopening and cancelling retains the accepted choice.
No project/setup is created and no tools are provisioned. This covers the
creation model/render boundary, not Home's input gate or a real-terminal run.

The focused regression, formatting, diff checks and clippy with warnings
denied passed. All 458 ordinary tests passed with normal Windows permissions;
22 prerequisite-dependent tests remain ignored (480 tests discovered).
The restricted-token full run denied existing Windows DACL and Bash checks;
that run is not counted as a pass. CodeRabbit CLI 0.7.6 reviewed the regression
with zero findings; the documentation diff was inspected directly.
Reviewed-head/merged-main CI evidence will be recorded on the PR.
Published 0.19.1 remains unchanged;
this test/documentation change needs no version bump.

## October 4: Saved Setup editor resize recovery

Work started from clean main `76d7c88` (PR #60), with no open PRs.
Three ordinary regressions cover the editor using disposable setup storage.
The TestBackend check retains a filtered capability revision, unchecked choice,
detail focus/scroll and Help through 120 x 30, 80 x 24, 60 x 16, 40 x 10
and back. It applies the revision, saves, makes a second revision and confirms
discard without changing the last successful save's bytes.

Two real-terminal tests recover the same filtered/scrolled revision, verify
continued scrolling, apply and save after recovery, and repeat a no-op save.
They cancel default-No discard and explicit No, then confirm Back and Home
while undersized. Reopening after Back restores the original capability;
discard leaves the commented source byte-identical. The driver checks raw-mode
restoration and each test verifies its temporary directory contains only the
owned setup. No user configuration is written and no tools are provisioned.

The isolated driver now mirrors Home's small-screen key/paste gate, including
`exit_confirmation_key`: edits and save are blocked while discard choices stay
usable. This is manager coverage, not execution of Home's dispatch loop or all
composition steps. PTY synchronization waits for completed Help redraws and
uses Help as an input barrier before resizing, so pending keys are not delivered
at the next size. The initial fixture failures exposed these synchronization
assumptions; no runtime defect was found.

All 12 Saved Setups tests passed, and all four manager PTY tests passed ten
consecutive repetitions. Formatting, diff checks, clippy with warnings denied
and all 457 ordinary tests passed; 22 prerequisite-dependent tests remain
ignored (479 Windows tests discovered). The full suite passed with normal
Windows permissions after a restricted-token run denied existing DACL and
Bash checks. Iterative CodeRabbit review reported zero findings. Final branch
review and reviewed-head/merged-main CI evidence will be recorded on the PR.
This is test/documentation work; published 0.19.1 remains unchanged.

## October 4: Saved Setups PTY resize recovery

Work started from clean main `bd77054` (PR #59), with no open PRs.
An ordinary unit regression drives the isolated `setup_pty_driver` in a real
terminal. Thirty-one matching disposable setups plus one excluded setup exercise
the complete `sample` filter, non-default `sample29` selection, scrolled list
and composition details. It resizes from 120 x 30 through 80 x 24, 60 x 16,
40 x 10 and back, opening/closing Help at each small size.

Terminal cell snapshots verify the selected name inside the list pane and
identical restored composition rows. Flattened text can join wrapped rows;
list offsets may adjust with the viewport and after Actions/Back. Home/End
still scrolls after recovery; Actions opens the same setup and Back restores
the filter and scrolled composition. Ctrl+C exits successfully, the driver
checks raw-mode restoration, and all 32 fixture files retain their original
bytes. Logging is disabled; no user setup storage or provisioning is involved.

The new regression passed ten consecutive repetitions. Formatting and the
full ordinary suite passed with normal Windows permissions: 454 passed and
22 prerequisite-dependent tests ignored, out of 476 discovered tests (401
unit and 75 integration). Ignored tests are not counted as passes. Clippy with
warnings denied and diff checks passed; final whole-branch CodeRabbit CLI
review reported zero findings across all five changed files. Reviewed-head/main
CI evidence will be recorded on the PR before closeout. This test-only change
needs no version bump; published 0.19.1 remains unchanged.

## October 4: Saved Setups browser resize recovery

Work started from clean main `30e2b7b` (PR #58), with no open PRs.
The existing Saved Setups rendering regression now types a filter over 31
matching disposable setups and one excluded setup, selects `sample29`, and
scrolls both its list and composition details. One app and TestBackend resize
through 120 x 30, 280 x 70, 80 x 24, 60 x 16, 40 x 10 and back.

The test checks the selected row and highlight within the list pane, selected
document, query, focus and detail scroll; Help closes to the exact prior buffer
at every size. Restored composition cells match the original buffer, scrolling
still works, and Actions/Back retains the selected setup and browser scroll.
Existing storage-error and editor Help checks remain covered. List offsets may
adjust with viewport size. This is TestBackend coverage, not a new PTY check.

Formatting, diff checks, clippy with warnings denied and all 453 ordinary tests
passed; 22 prerequisite-dependent tests remain ignored. The full suite ran with
normal Windows permissions. CodeRabbit CLI 0.7.6 connected after explicit owner
approval but returned a 12-minute quota cooldown; no local review result is
claimed. PR review and CI evidence will be recorded before closeout.
No runtime, dependency or version change is involved; published 0.19.1 remains
unchanged, and the test uses only temporary setup storage.

## October 4: Projects PTY resize recovery

Work started from clean main `429b4bd` (PR #57), with no open PRs.
An ordinary regression in `tests/hub.rs` runs the real binary in a PTY.
Its uniquely named temporary launch project supplies a long package summary;
filtering by its full name excludes unrelated projects without changing the
machine configuration or provisioning tools. Diagnostic logging is disabled.

The regression focuses and scrolls details at 120 x 30, resizes through
80 x 24, 60 x 16 and 40 x 10, and opens/closes Help at each small size.
It checks fresh resize output, restores the exact scrolled screen at 120 x 30,
then verifies Home/End scrolling, opening the filtered project, backing out
and returning Home with Ctrl+C before a clean exit. It does not certify
non-default browser selection or a scrolled project list; those remain covered
by the separate TestBackend regression below.

The new PTY test passed ten repetitions. Formatting, clippy with warnings
denied and all 453 ordinary tests passed; 22 prerequisite-dependent tests
remain ignored (475 Windows tests discovered). An initial restricted-token
full run failed existing Windows DACL preservation and Bash checks; rerunning
with normal permissions passed. No runtime, dependency or version change is
involved; published 0.19.1 remains unchanged. PR review and CI evidence are
recorded on the PR before closeout.

## October 4: Projects resize recovery coverage

Work started from clean main `11dcfb6` (PR #56), with no open PRs and successful
merged-main CI `37180824242`. The existing Projects rendering regression now
uses one app and TestBackend through 120 x 30, 80 x 24, 60 x 16, 40 x 10 and
back. Thirty matching temporary projects plus one excluded project exercise a
typed filter, non-default selection and a scrolled list. Injected discovery
warnings provide enough detail text to verify actual keyboard scrolling.

The regression checks filter and selected-path preservation, selected-row text
and highlight inside the list pane at usable sizes, detail focus/scroll, Help
open/close at every size, identical
restored detail cells and working Home/End scrolling after recovery. List offsets
may adjust to keep the selected row visible. Existing disabled-action reasons
and the 280 x 70 rendering check remain covered. No runtime change, provisioning,
dependency or version bump is involved; published 0.19.1 remains unchanged.

Local formatting, `cargo test --locked` (452 passed, 22 prerequisite-dependent
tests ignored), clippy with warnings denied and diff checks passed. The test
inventory remains 474 Windows tests (400 unit and 74 integration). CodeRabbit's
initial review identified a weak row-visibility assertion: the selected name
also appeared in the detail path. The test now checks the list pane and its
selection highlight. Final review and reviewed-head/main CI will be recorded
on the PR before closeout.
This is TestBackend coverage, not a real-terminal Projects resize test;
ignored live-tool tests are not counted as passes.

## October 4: Catalog resize recovery coverage

Work started from clean main `1af843e` (PR #55), with no open PRs. Two new
ordinary unit regressions resize the same Catalog and TestBackend through
120 x 30, 80 x 24, 60 x 16, 40 x 10 and back. They preserve a typed filter,
non-default selected entry, detail scroll and rendered detail contents;
verify a long list keeps its selected row visible and remains scrolled;
and exercise Help, Back history, keyboard and mouse scrolling after recovery.
List offsets may adjust with viewport size while retaining selection visibility.

A third ordinary regression runs standalone `rproj info` in the shared real
terminal harness, filters to reactRoblox, scrolls to its caveats, shrinks through
the same sizes, opens/closes Help while undersized, and restores the exact
scrolled screen. Ctrl+Home/End still work afterward and Esc exits successfully.
It waits for the complete filter and fresh resize output, uses a temporary
working directory and disables diagnostic logging. No runtime or dependency
change, machine provisioning or Cargo version bump is involved.

Local formatting, `cargo test --locked` (452 passed, 22 prerequisite-dependent
tests ignored), clippy with warnings denied and diff checks passed. There are
474 discovered Windows tests: 400 unit and 74 integration. The new PTY regression
also passed ten consecutive repetitions. CodeRabbit reported zero findings.
Ignored live-tool tests were not rerun and are not counted as passes. Final
reviewed-head/main CI and branch cleanup evidence will be recorded on the PR.
Published 0.19.1 and its tag remain unchanged.

## October 4: 0.19.1 publication verified

The owner published 0.19.1 to crates.io at `2026-10-04T00:38:42.3801Z`.
The version is current and non-yanked. The downloaded archive matches registry
SHA-256 `be193ef19a8277f78d788bb9494834d91ba3146d7d7a6daf1d32e7f854cdb210`.
Its `.cargo_vcs_info.json` identifies clean reviewed release commit
`957a0aa67fa679dca4194e6fd3c739594c409dc6`, exactly the main from
[release PR #53](https://github.com/chatarabdelilah/rproj/pull/53).
Both packaged Cargo files identify 0.19.1 and the manifest retains Rust 1.89.
No publication was performed by the agent.

PR #53's final head `61397d1` passed CI `37165190198`; merged release commit
`957a0aa` passed CI `37165309630`. Both passed Windows stable, Rust 1.89 and
locked packaging. Their trees matched exactly and the completed local/remote
release branch was removed. Badge freshness was skipped and is not a pass.
Final whole-branch CodeRabbit review reported zero findings after its service
cooldown; the corrected iterative README finding is documented below.

After archive verification, annotated tag `v0.19.1` was created at `957a0aa`.
Remote tag object `fd9083834e6a194824ea6c3e22e0a095ea154968` peels to the
same commit. The matching [GitHub alpha release](https://github.com/chatarabdelilah/rproj/releases/tag/v0.19.1)
was published at `2026-10-04T00:41:43Z`, is not a draft and remains marked as
a prerelease. Its body uses the dedicated 0.19.1 release notes.

Publication closeout initially contained only documentation. PR #54's first
CI run (`37165858860`, head `13be429`) passed stable and packaging but failed
the Rust 1.89 terminal resize regression with invalid JSON, not a save timeout.
The fixture waited for an address substring before resizing; ConPTY can deliver
paste as individual keys, leaving the trailing comma pending when the terminal
shrinks. The test now waits for the entire inserted field including its comma.
This changes test synchronization only; the published runtime, dependencies,
Cargo version and tag remain unchanged. Formatting, all 449 ordinary tests
(22 explicitly ignored) and clippy with warnings denied passed locally.
The corrected resize regression also passed ten consecutive local repetitions;
iterative CodeRabbit review reported zero findings.
Final whole-branch CodeRabbit review and exact-head/main CI are required for the expanded
closeout; their final evidence and completed branch cleanup are recorded on
PR #54. The successful release-candidate live gates were not repeated.
Publication closes release alignment; the acceptance limits below remain
unverified. No subsequent release candidate is active.

## October 4: 0.19.1 patch candidate preparation

Preparation started from clean main `32b4673` (PR #52), with successful
merged-main CI `37162300958` and no open PRs. Registry verification found
0.19.0 current and non-yanked; 0.19.1 was not present. The selected patch
contains PR #51's small-terminal exit fix and PR #52's external-edit save/reset
protection, plus PR #50's terminal recovery regressions. It changes no
generated-project schema and requires no project migration.

`codex/release-0-19-1` aligns Cargo.toml and the root Cargo.lock package entry
to 0.19.1 without changing dependency resolution. Dedicated candidate notes
describe the behavior and optimistic conflict-detection limit.

Candidate local gates passed on Windows with Cargo 1.94.0:

- Formatting, `cargo test --locked` (449 ordinary passes, 22 ignored), clippy
  with warnings denied and diff checks. Metadata confirms 0.19.1 and Rust 1.89.
- `RPROJ_TEST_TIMEOUT=180 cargo test --locked --test live -- --ignored --test-threads=1 --nocapture`:
  all 14 passed in 138.19 seconds, including actual negative-gate diagnostics,
  setup replay/refusal, Watch recovery and local Jest pass/fail.
- `cargo test --locked --bin rproj real_rojo -- --ignored --test-threads=1 --nocapture`:
  all four passed in 9.97 seconds, including the terminal save/refusal/repair
  boundary and ten generated template validation variants.
- With the actual installed Jest Roblox CLI 0.4.1 executable selected,
  `open_cloud_missing_credentials_report_names_and_preserve_exit_code` passed
  in 4.35 seconds. The first invocation incorrectly selected Rokit's dispatch
  shim; copying that shim into the fixture caused OS error 50 and exit 1.
  Selecting the installed executable corrected the audit invocation; no runtime
  fix was needed. Synthetic partial credentials exercise 14 refusals.

Installed tools included Rojo 7.7.0, Wally 0.3.2, Rokit 1.2.0, Git for Windows
2.52.0, Selene 0.31.0 and StyLua 2.5.2. Before/after checks confirmed unchanged
machine configuration, template, global Rokit manifest and existing project/setup
names; temporary fixture cleanup completed. The live scaffold refreshed
JestRobloxRunner rather than preserving its prior bytes. The resulting plugin
matches upstream v0.4.4 asset SHA-256
`7fa3e89d172ed8dc76bbbc57f6f36082818d41c4451a26217edae6c475bcabd8`.
This normal scaffold side effect is not an unchanged-plugin claim.

Headroom processed the approved Cargo version diff (zero tokens saved);
diagnostic logs stayed local. Iterative CodeRabbit found one minor README
overstatement; it now explains the optimistic conflict-detection limit.
Clean candidate `01c76f1` passed `cargo package --locked`: 101 files,
1.3 MiB (327.8 KiB compressed), with a successful extracted-package build.
The inspected archive excludes `docs/`, `.github/` and `.codegraph/`; both
packaged Cargo files identify 0.19.1, the manifest retains Rust 1.89 and its
Git identity matches that clean commit. Subsequent documentation-only evidence
updates do not alter runtime, dependencies, README or the packaged file set.

Final branch review and exact-head/main CI will be recorded on the release PR
before owner publication. Authenticated Open Cloud execution, fresh Windows
provisioning, fresh Linux generated-project execution, UI Labs Studio stories,
Scribe Studio playtesting and unavailable Windows symlink privileges remain
unverified. Unix permission-mode checks were not run on Windows.
At preparation closeout, owner publication and archive/tag/GitHub alignment
were pending; the publication section above records their completion.

## October 4: Template Explorer external-edit protection

Work started from clean main `3dc9e7e` (PR #51), with successful merged-main
CI `37154970455` and no open PRs. A disposable terminal regression reproduced
the defect: another process replaced the template after the editor opened,
but Ctrl+S overwrote that edit and reported success.

Both standalone and Home entry points now load the draft and persistence
baseline from the same `project_template::EditSession` read. Save and reset
refuse changed contents, external creation or deletion. Save checks before
Rojo validation and after permission-preserving sibling staging; the snapshot
advances only after successful persistence. Refusal retains the draft and
names reopening Template Explorer as the recovery action. This is optimistic
conflict detection, not a file lock: external writes between the final check
and replacement/removal remain possible.

Five new storage regressions cover external change/create/delete refusal for
save/reset, validation-time changes without write artifacts, successive saves
and reset/recreation, validation refusal/repair and non-file read refusal.
Two terminal regressions verify external save/reset refusal, bytes or absence
preservation, dirty-draft retention, retry after restoration of the original
snapshot, and raw-mode restoration. The child harness uses the same session
and production editor with disposable paths; no global template, machine
configuration, projects, plugins or authentication are changed.

Local locked gates passed: 449 ordinary Windows tests, with 22 ignored
prerequisite-dependent cases, formatting, clippy with warnings denied and
diff checks. There are 471 discovered tests (398 unit/73 integration), with
40 project-editor tests. An explicit installed-Rojo 7.7.0 run of
`pty_real_rojo_save_and_rejection_preserve_last_valid_template` passed in
5.31 seconds on the final source, exercising valid save, upstream refusal and corrected-draft
save through all ten generated variants. Other ignored live-tool tests were
not rerun and are not counted as passes.

Headroom processed the approved source diff and saved 47 tokens; diagnostic
logs stayed local. Iterative CodeRabbit review reported zero findings. Final
branch review and exact-head/main CI closeout are recorded on the change's PR.
This runtime protection and PR #51's resize fix await the next selected
release candidate; Cargo versions, published 0.19.0 and its tag are unchanged.

## October 3: Template Explorer resize recovery

Verification started from clean main `6ab9ef3` (PR #50). A disposable real
terminal regression reproduced an exit trap: shrinking an edited Explorer to
40 x 10 hid its discard confirmation and blocked its confirmation keys.
The editor now renders Exit confirmation below the 60 x 16 editing minimum
and accepts Enter/Y to discard or Esc/N to retain the draft. Help and Ctrl+C
remain available; editing, paste, save, reset and other confirmation actions
remain blocked at these sizes.

Three new ordinary terminal regressions verify edited-draft discard with
byte-identical persistence, JSON draft preservation through 80 x 24, 60 x 16,
40 x 10 and back, blocked small-screen typing/paste/save/reset, refusal of a
pending reset, and raw-mode restoration after exit. The PTY harness replays
recorded resize boundaries at their actual dimensions. Its child runs the
production editor with a disposable template path; no global template,
configuration, project, plugin or authentication state is changed.

Local locked gates passed: 442 ordinary Windows tests, with 22
prerequisite-dependent tests ignored, formatting and clippy with warnings
denied. There are 464 discovered tests (391 unit/73 integration), including
38 project-editor tests. Ignored live-tool tests were not rerun for this
terminal-only change and are not counted as passes. Headroom processed the
approved source diff and saved 33 tokens; diagnostic logs stayed local.
Iterative CodeRabbit review reported zero findings. Whole-branch review and
exact-head/main CI closeout are recorded on the change's PR.
This runtime fix is unreleased; the published 0.19.0 baseline and its tag
remain unchanged. Prepare a new version when the next candidate is selected.

## October 3: Template Explorer terminal recovery regressions

Verification started from clean main `729beb5` after the 0.19.0 publication
closeout. The new code is confined to the existing `#[cfg(test)]` editor
harness; runtime behavior, dependencies and Cargo versions are unchanged.

Four new ordinary terminal regressions exercise first-edit cancellation with
an existing or absent template, saving then discarding a later draft with
Esc/Ctrl+C, invalid-JSON refusal followed by repair, and a real Windows sharing
violation followed by retry. They preserve original bytes or file absence,
unknown fields at the editor/persistence boundary and unrelated settings,
confirm the draft survives a failed save, verify temporary-file cleanup,
reopen saved content and assert raw mode is disabled after the editor returns.

The child harness runs the production editor loop and existing atomic template
writer with an injected disposable path; it does not change the user's global
template, machine configuration, projects, plugins or authentication. Basic
Home/editor handoff remains covered by the existing hub integration tests;
these new cases exercise the isolated editor and save boundary.

With installed Rojo 7.7.0,
`cargo test --locked --bin rproj project_editor::app::tests::pty_real_rojo_save_and_rejection_preserve_last_valid_template -- --ignored --exact --nocapture`
passed in 5.38 seconds. Through the real terminal loop, valid edits passed all
ten generated sourcemap/build variants; invalid `servePlaceIds` was refused
before persistence; repairing the draft passed validation and saved. The last
valid file remained byte-identical during refusal. This installed-tool test
remains ignored without its prerequisite and is not an ordinary-CI pass.

Two fixture assumptions were corrected during verification: Rojo rejects an
invented unknown top-level field, so the live fixture uses supported fields;
ConPTY may deliver pasted characters separately, so the live repair uses
ordinary editing keys rather than assuming one Undo removes the whole paste.
Unknown-field preservation is tested separately without claiming upstream Rojo
acceptance. No production defect was reproduced.

Local gates passed: 439 ordinary Windows tests with 22 prerequisite-dependent
tests ignored, formatting, clippy with warnings denied and `git diff --check`.
Headroom processed the approved source diff and saved 16 tokens; diagnostic
logs stayed local. Iterative CodeRabbit review reported zero findings. The
whole-branch review of `d1651ab` found only a stale aggregate test count in
architecture; it is corrected to 461 discovered (388 unit/73 integration),
439 ordinary and 22 ignored. The documentation-only correction was inspected
directly without repeating successful runtime/live gates. Exact-head/main CI
closeout is recorded on [PR #50](https://github.com/chatarabdelilah/rproj/pull/50),
including final-head/main Git identities, CI runs and branch cleanup. No new
release candidate or publication is needed.

## October 3: 0.19.0 publication verified

The owner published 0.19.0 to crates.io at `2026-10-03T16:55:48.275821Z`.
The version is not yanked and crates.io reports it as current. The downloaded
archive's SHA-256 matches registry checksum
`f7711708ad0669085feedbe5f2ae7194c6e59a10eed53b36863ae253162047f8`.
Its `.cargo_vcs_info.json` identifies
`68fa47929fc9deea81a3a8447e01db9f8cd71301`, exactly the clean, reviewed release
commit from [PR #48](https://github.com/chatarabdelilah/rproj/pull/48).
Both packaged Cargo files identify 0.19.0. No publication was performed by
the agent.

PR #48's final head `fbd8052` passed CI `37135709267`; merged release commit
`68fa479` passed CI `37135909071`. Both passed Windows stable, Rust 1.89 and
locked packaging. Their trees matched exactly and the completed local/remote
release branch was removed. Badge freshness was skipped and is not a pass.

After archive verification, annotated tag `v0.19.0` was created at `68fa479`.
Remote tag object `64ca544de4f1880e24f60561f776acee9647ad1c` peels to that same
commit. The matching [GitHub alpha release](https://github.com/chatarabdelilah/rproj/releases/tag/v0.19.0)
was published at `2026-10-03T17:00:43Z`, is not a draft, and remains marked
as a prerelease. Its body uses the dedicated 0.19.0 release notes.

The documentation closeout changes no runtime, dependencies or version.
Publication closes release alignment; the acceptance gaps listed below remain
unverified. No subsequent release candidate is active.

## October 3: 0.19.0 alpha candidate preparation

Preparation started from clean main `a47d818` (PR #47), whose merged-main
CI `37108848149` passed. The owner approved preparing the next release.
`codex/release-0-19-0` aligns Cargo.toml and the root Cargo.lock entry to
0.19.0 without changing dependency resolution. Dedicated release notes cover
the merged changes in PRs #38–#47 and their remaining alpha limitations.

Candidate local gates passed on Windows with Cargo 1.94.0: 435 ordinary tests
passed and 21 prerequisite-dependent tests were ignored; formatting, clippy
with warnings denied, locked offline metadata and `git diff --check` passed.
The lockfile diff changes only the root package version.

Applicable ignored checks were then executed explicitly on the candidate:

- `RPROJ_TEST_TIMEOUT=180 cargo test --locked --test live -- --ignored --test-threads=1 --nocapture`:
  all 14 passed together in 141.82 seconds, including actual negative-gate
  diagnostics, setup replay/refusal, Watch recovery and local Jest pass/fail.
- `cargo test --locked --bin rproj real_rojo -- --ignored --test-threads=1 --nocapture`:
  all three passed in 4.93 seconds, covering ten built-in/guided validation
  variants and rejection of an invalid property.
- With `RPROJ_LIVE_JEST_CLI` selecting installed Jest Roblox CLI 0.4.1,
  `cargo test --locked --test test_workflow open_cloud_missing_credentials_report_names_and_preserve_exit_code -- --ignored --exact --test-threads=1 --nocapture`:
  passed in 5.01 seconds, exercising 14 refusals with synthetic partial values.

Installed tools included Rojo 7.7.0, Wally 0.3.2, Rokit 1.2.0, Git for Windows
2.52.0, Selene 0.31.0 and StyLua 2.5.2. Before/after comparisons confirmed
unchanged machine configuration, Jest Studio plugin bytes, existing project
names and saved setup names. Disposable fixture cleanup completed.
Headroom processed the approved Cargo version diff (zero tokens saved);
diagnostic logs stayed local.

Clean candidate `4c34500` passed `cargo package --locked`: 101 files,
1.3 MiB (324.0 KiB compressed), followed by a successful extracted-package build.
The file list excludes documentation, development metadata and user secrets.
Both packaged Cargo files use 0.19.0, retain Rust 1.89, and the package Git
identity matches the clean commit. Subsequent documentation-only updates do
not alter the tested source, dependencies or packaged file set.

The final `cr review --agent --base main` review of `41f1b18` reported zero
findings across all six changed files. The iterative review's pending-gate
advisory confused this candidate with prior PR #47 evidence; the sections now
explicitly distinguish them. Later documentation-only evidence updates are
inspected directly. Exact-head and merged-main CI must both pass and be recorded
on [release PR #48](https://github.com/chatarabdelilah/rproj/pull/48) before
owner publication. That PR's closeout records the final candidate Git identity,
both CI runs, tree preservation and completed branch cleanup.

At preparation closeout, owner publication and tag/GitHub release alignment
were pending; the publication section above records their subsequent completion.
Authenticated Open Cloud execution, fresh Windows
provisioning, fresh Linux generated-project execution, UI Labs Studio stories,
Scribe Studio playtesting and unavailable Windows symlink privileges remain
unverified; Unix permission-mode checks were not run on Windows.

## October 3: Open Cloud missing-credential regressions

The audit began on clean main `2bc6462` (PR #46), with merged-main CI
`37083248511` passing and no open PRs. Generated cloud CI already refused
missing credentials; local `rproj test` delegated credential resolution to
Jest Roblox. No production correction was needed.

The new ordinary regression executes the credential guard extracted from
generated CI with Git Bash, followed by an offline success marker. All seven
incomplete combinations are tested with absent and empty variables: each exits
1, names the required fields and prevents the marker from running. All-present
synthetic values reach the marker without launching a runner. No API-key value
is printed. This is an executed shell check, not only a workflow string check.

With `RPROJ_LIVE_JEST_CLI` pointing to installed Jest Roblox CLI 0.4.1,
`cargo test --locked --test test_workflow open_cloud_missing_credentials_report_names_and_preserve_exit_code -- --ignored --exact --test-threads=1 --nocapture`
passed in 4.10 seconds. It drives `rproj test` for all seven incomplete
credential combinations using standard variables and then `JEST_`-prefixed
variables: 14 refusals, each naming exactly the missing fields with environment
guidance and preserving runner exit code 2. Each case keeps the production
project, test source and installed package link byte-identical. Preparation
uses fixture tools; the existing package-reuse regression still passes.

The real-runner fixture clears all six credential variables only in each child
environment, supplies synthetic values for present fields, never supplies a
complete set, and sets the cloud base URL to loopback. It uses disposable
directories and does not provision applications or change user authentication,
projects, machine configuration or Studio plugins. This test remains ignored
without its explicit installed-runner prerequisite. Source excerpts were sent
to Headroom under the owner's existing approval; diagnostics stayed local.

This establishes missing-credential error handling, not authenticated cloud
execution or upload success. Fresh-machine provisioning and Windows symlink
execution also remain separate gaps. No runtime/dependency/version change,
publication, tag or release preparation. Review and CI evidence belong to the
implementing PR.

Local gates passed: 435 ordinary Windows tests, with 21 prerequisite-dependent
tests ignored; formatting, clippy with warnings denied and `git diff --check`
also passed. The new ordinary shell test requires Bash from the existing Git
for Windows installation. The restricted sandbox token could not create Bash's
signal pipe; the test passed under the normal host token used by this suite's
Windows permission regressions.

CodeRabbit identified an additional Git Bash installation layout; discovery now
accepts both `bin/bash.exe` and `usr/bin/bash.exe` beside Git's ancestor paths.
Its advisory to ignore or skip a missing Bash is not adopted: Windows CI provides
Git Bash, this guard should be exercised by ordinary CI, and unavailable tooling
must remain a reported prerequisite failure. The requirement is documented.

[PR #47](https://github.com/chatarabdelilah/rproj/pull/47) merged at `a47d818`.
Final CodeRabbit review of head `e1e40df` reported zero findings. PR CI
`37108674698` and merged-main CI `37108848149` passed Windows stable,
Rust 1.89 and locked packaging. The merged tree matched the reviewed head;
the completed local and remote branch was removed.

## October 3: Template Explorer validation with real Rojo

Clean main `845512a` (PR #45) passed
`cargo test --locked --bin rproj real_rojo -- --ignored --test-threads=1 --nocapture`
with installed Rojo 7.7.0: **three passed, zero failed, zero ignored**, in
4.24 seconds. This explicitly executes three prerequisite-dependent tests
that remain ignored in ordinary CI; it does not change the ordinary test count.

- The built-in template passed sourcemap generation and binary builds for all
  ten validation variants: plain, plain with tests, four Wally combinations,
  two submodule combinations, and two Jest combinations.
- Guided CFrame, UDim2, UDim, and Rect attribute values, plus guided GUI
  Size/SliceCenter/Padding properties, passed the same real-Rojo validation.
- An invalid Part Anchored value was refused with a Rojo rejection error.

These tests use disposable validation workspaces, and no matching temporary
workspace remained afterward. They do not save the user's global template or
modify existing projects, provision tools, or use cloud credentials. This
verifies the validation boundary, not a new interactive edit/save/cancel session.
Open Cloud, fresh-machine provisioning, and Windows symlink-privilege execution
remain separate gaps. No runtime, dependency, version, publication or tag change.

[PR #46](https://github.com/chatarabdelilah/rproj/pull/46) merged at `2bc6462`.
Its documentation diff was inspected directly. PR head `0c4a027` passed CI
`37083008971`; merged-main CI `37083248511` passed Windows stable, Rust 1.89,
and locked packaging. The merged tree matched the reviewed head and completed
local/remote branches were removed.

## October 2: combined live verification and negative-gate diagnostics

After the GitHub API quota reset, clean main `144c35a` (PR #44) passed
`cargo test --locked --test live -- --ignored --test-threads=1 --nocapture`:
**14 passed, zero failed, zero ignored**, in 138.22 seconds, with
`RPROJ_TEST_TIMEOUT=180`. This closes the previous combined-run gap for project
creation, cancellation/revision, saved setup replay/refusal, Wally/submodule
builds and checks, dependency recovery in Watch, and local Jest pass/fail.
Jest's three starter specs passed; its deliberate failure returned 1 with
two passing/one failing test and preserved the installed package files.

The captured negative-gate output exposed a fixture weakness despite that green
run: all three added defects followed the starter module's terminal `return`,
so syntax errors caused the failures instead of the intended diagnostics.
The fixture now inserts defects before `return`, preserving the strict-mode
header, requires each intended diagnostic (`TypeError:`, Selene's
`undefined_variable`, and StyLua's diff), and rejects syntax/parse errors.
It still requires a nonzero gate exit and a green gate after each restoration.
The undefined global may also fail type analysis; this is not a claim that
exactly one tool rejects each defect.

Adding diagnostic assertions first reproduced the old fixture's failure in
9.99 seconds. After correcting placement, the focused live test
`the_generated_gate_rejects_bad_code_one_step_at_a_time` passed in 17.08 seconds.
The full 14-test result above belongs to main `144c35a`; the focused result
verifies the only changed live test. No production behavior changed.

Local ordinary verification passed: `cargo test --locked --quiet` ran 434
passing tests with 20 prerequisite-dependent tests ignored. Formatting,
clippy with warnings denied, and `git diff --check` passed. Review and final-head
CI evidence belong to the implementing PR.

All temporary project/setup fixtures were removed after these runs. Machine
configuration stayed byte-identical; the existing project list and shared Jest
Studio plugin hash were unchanged. Existing tools, Studio and caches were used;
no machine applications, authentication or user projects were changed. Headroom
was used for authorized source compression; diagnostic logs stayed local.
Open Cloud, fresh-machine provisioning, and Windows symlink-privilege execution
remain separate gaps. No version bump, publication, tag or release is prepared.

[PR #45](https://github.com/chatarabdelilah/rproj/pull/45) merged at `845512a`.
CodeRabbit reported zero findings on reviewed head `5c92dbe`. PR CI
`37058950254` and merged-main CI `37059254161` passed Windows stable, Rust 1.89,
and locked packaging. The merged tree matched the reviewed head; completed
local and remote branches were removed.

## October 2: live workflow audit and repeated test preparation

The audit began on clean main `79f2dab` (PR #43), with merged-main CI
`36821611764` passing and no open PRs. This is unreleased source verification,
not a new package candidate. Installed prerequisites were Rokit 1.2.0, Rojo
7.7.0, Wally 0.3.2, Git 2.52.0.windows.1, Studio, and its Jest runner plugin.
Local Rust/Cargo were 1.94.0; Jest CLI 0.4.1 was available in tool storage.

The first serial live run was stopped after a Home test timed out: its helper
still expected Composition immediately after naming a project. The helper now
waits for Review and explicitly selects Start point. All three affected Home
tests passed in 19.98 seconds. A subsequent full run completed with 12 passing
and two failing tests in 101.90 seconds:

- The setup-refusal fixture used nested setup names, now forbidden by the
  existing single-name contract. It now reserves unique input/output filenames
  directly in the setup directory and retains byte-preservation/cleanup checks.
- The live Jest test failed before runner startup because `sync_for_test`
  reprocessed links already rewritten by `wally-package-types`. Complete Wally
  trees now reuse their existing links and types, regenerating only the sourcemap.
  Missing/stale dependencies retain the full install/sourcemap/retype path.

Focused verification after those fixes:

| Check | October 2 result |
| --- | --- |
| `cargo test --locked --test live invalid_saved_setups_refuse_before_reconfiguration_or_creation -- --ignored --exact --test-threads=1 --nocapture` | Passed all four refusal cases in 8.55 seconds. |
| `cargo test --locked --test live jest_starter_specs_pass_and_report_failure -- --ignored --exact --test-threads=1 --nocapture` | Passed in 26.99 seconds: three starter specs succeeded, then the deliberate failure returned 1 with two passing/one failing test. Both runs preserved every installed package file byte-for-byte. |
| Ordinary fixture CLI regression | Failed before the runtime fix; passed afterward. TestEZ and Jest each test complete-tree reuse twice, plus missing-package recovery followed by reuse. Fixture executables record ordering without network/provisioning. |
| `cargo test --locked --test live -- --ignored --test-threads=1 --nocapture` | Final combined attempt completed with nine passing/five failing tests in 79.31 seconds. GitHub rate-limit exhaustion prevented required Jest and saved-replay tool pins. Wally/submodule clean-gate and gate-failure checks also failed in this run; a complete rerun remains required. This is not a passing live suite. |
| Local ordinary gates | 434 ordinary Windows tests passed; 20 prerequisite-dependent tests ignored. Formatting and clippy with warnings denied passed. |

Live runs used `RPROJ_TEST_TIMEOUT=180` and serial execution. All owned project
and setup fixtures were removed, including the stopped run's fixtures; machine
configuration stayed byte-identical. Normal Jest scaffolding refreshed the
existing shared Studio runner plugin, changing its hash. Rokit/Wally caches and
trust state are shared. No machine applications were provisioned. Authentication
and existing user projects/setups were unchanged. The live gate helper
now prints captured tool output on failure rather than only an exit code.

CodeRabbit raised the case of complete bare links after a manual `wally install`.
The package-preserving test behavior is retained deliberately: runtime execution
does not require exported types, and type restoration belongs to the full
scaffolding/Watch sync. This boundary is documented and the ordinary CLI
regression covers both complete typed and bare links without mutating them.
Final CodeRabbit review on `9ddd80e` reported only a stale architecture test
total; it was corrected and the documentation-only diff was inspected directly.

The combined rerun after rate-limit reset is recorded above. Open Cloud,
fresh-machine provisioning, and Windows symlink-privilege evidence remain outside
this audit. No version bump, publication, tag, or release is prepared. Review,
package, and CI evidence belong to the implementing PR.

[PR #44](https://github.com/chatarabdelilah/rproj/pull/44) merged at `144c35a`.
Final CodeRabbit review covered runtime/tests on `9ddd80e`; the final head
`f46575c` corrected only documentation. Final-head CI `36991045623` and
merged-main CI `36991461179` passed. Locked packaging passed and completed
branches were removed.

## Unreleased: safe Jest refresh saves

Jest's production mount repair, generated test project, and merged runner config
now use the shared staging/permission helper. Replacement no longer removes or
uses existing `.rproj-old` files. Read-only and symbolic-link targets are refused;
failed replacement keeps its target intact and cleans up the temporary sibling.
Writes remain sequential, so an earlier successful production repair can remain
when a later target fails. There is no external-writer lock or crash guarantee.

The backup regression failed against the old writer and passed after the fix.
Focused Windows tests cover backup preservation and all three output targets'
read-only refusal and locked replacement failure, cleanup, and successful retry.
Shared permission regressions cover Windows DACL preservation; Unix mode and
Windows symlink execution retain the limitations recorded below. No dependency
or version change, machine provisioning, or live Studio/Open Cloud run.

October 1 local verification passed 433 ordinary Windows tests, with 20
prerequisite-dependent tests ignored, formatting, and clippy with warnings denied.
Review and CI evidence belong to the implementing PR.

[PR #43](https://github.com/chatarabdelilah/rproj/pull/43) merged at `79f2dab`.
CodeRabbit reported zero findings on `e847165`; reviewed-head CI `36821401624`
and merged-main CI `36821611764` passed. Completed branches were removed.

## Unreleased: staged upgrade replacements

Upgrade prepares and syncs every planned replacement before replacing any
target, using the shared staging/permission helper also used by Configure.
Preparation failure preserves target contents; newly created parent directories
may remain. The whole snapshot set is rechecked after staging. A failed
replacement preserves its target, cleans up temporary files, reports the number
of earlier targets saved, and asks the user to rerun after fixing the cause.
Earlier successful replacements are not rolled back. The separate additive
metadata merges remain outside this replacement boundary.

October 1 local verification passed 430 ordinary Windows tests, with 20
prerequisite-dependent tests ignored, formatting, and clippy with warnings denied.
The later read-only failure regression failed before the fix and passed after.
Actual Windows lock failures cover the first and second targets, cleanup,
progress reporting, and successful reruns. An injected partial staging write
preserves the original; Configure's read-only failure retains pending changes.
Existing Windows DACL regressions pass after moving the permission helper.
The Unix mode regression remains unexecuted locally.

CodeRabbit identified symbolic-link replacement as a behavior regression;
preparation now refuses linked targets without replacing the link or shared
configuration. The explicitly invoked Windows symlink regression was blocked
at fixture creation by OS error 1314 (missing privilege), not passed. It is
ignored on Windows by default and ordinary on Unix; no machine setting changed.

No dependency or version change; no provisioning or Studio/Open Cloud run.
Review and CI evidence belong to the implementing PR.

[PR #42](https://github.com/chatarabdelilah/rproj/pull/42) merged at `1e87f39`.
Final CodeRabbit review reported zero findings on `5c4405e`; reviewed-head CI
`36803293173` and merged-main CI `36803533721` passed. Completed branches were
removed.

## Unreleased: upgrade confirmation conflicts

Upgrade now retains the exact snapshots used for planning and merging. Recorded
rewrite targets and inputs, including the composition record and production
Rojo document, are checked after confirmation and before the first write. File
edit/create/delete conflicts refuse the reviewed plan without writes. Non-missing
read errors also fail safely. Cancellation preserves every fixture file and
directory; accepted upgrades retain custom settings and user-owned files.

September 30 local verification passed 426 ordinary Windows tests, with 19
prerequisite-dependent tests ignored, formatting, and clippy with warnings denied.
The stale-save regression failed before the fix and passed afterward. All 15
upgrade terminal regressions pass without network or tool provisioning.
Conflict detection is optimistic; later writes are not a multi-file transaction.
The separate `.gitignore`, `.luaurc`, and `tests/.luaurc` merge helpers read
current files after the planned writes; their targets are not snapshotted.
No version bump or publication is prepared. Review and CI evidence belong to
the implementing PR.

[PR #41](https://github.com/chatarabdelilah/rproj/pull/41) merged at `d720134`.
CodeRabbit reported no runtime findings; its documentation clarification was
corrected and directly inspected. Reviewed-head CI `36763458103` and merged-main
CI `36763842366` passed. Completed branches were removed.

## Unreleased: shared configuration saving

CLI configuration and Ratatui now share one `EditSession`: a single loaded
snapshot, current values, pending changes, checked merging, and staged file
replacement. Direct CLI saves now reject external edits (including valid
TOML/JSON, file creation, and deletion). No-op bytes, unknown/unsupported values,
and cancellation remain protected. Failed saves retain pending changes and the
original baseline for retry. Conflict detection is optimistic; this is not a
cross-process lock or a guarantee of crash durability.

September 30 local verification passed 421 ordinary tests on Windows, with 19
prerequisite-dependent tests ignored, formatting, and clippy with warnings denied.
New coverage includes both formats' no-op/reverted changes, external file
creation/deletion/replacement, failed replacement cleanup and retry, invalid
merges, missing projects, and real CLI cancellation/conflicts. Read-only and
locked destinations retain the original file and pending changes. Replacement
preserves Windows DACL entries and inheritance protection; the safe wrapper
adds one Windows-only dependency without introducing unsafe repository code.
A Unix-mode preservation regression was added but not exercised locally.
No machine provisioning or Studio/Open Cloud run was needed or performed.

No version bump or publication is prepared. Review and CI evidence belong to
the implementing PR.

[PR #40](https://github.com/chatarabdelilah/rproj/pull/40) merged at `efe6615`.
Final local CodeRabbit review reported zero findings on `096b004`; reviewed-head
CI `36751976197` and merged-main CI `36760836260` passed. Completed branches
were removed.

## Unreleased: shared creation choices and validation

Direct CLI and Ratatui creation now use the capability catalog's shared rules
for workflow-compatible implementation choices, ordering, prerequisites, and
Jest backend routing. Picker ordering does not change legacy TestEZ defaults.
Direct prompts retain skip-with-explanation behavior; Ratatui retains correction
before graph mutation. Unknown saved values retain their existing behavior.

New catalog and screen tests cover all workflows, implementations, both Jest
backends, missing prerequisites, and unknown saved values. A real terminal test
exercises direct prompts without provisioning tools or creating project files.
This is a behavior-preserving refactor; no version bump or publication is prepared.

September 30 local verification passed 410 ordinary tests, with 19
prerequisite-dependent tests deliberately ignored, plus formatting and clippy
with warnings denied. Live Studio/Open Cloud and fresh-machine provisioning
were not rerun for this refactor.

[PR #39](https://github.com/chatarabdelilah/rproj/pull/39) merged at `fa8e6be`.
Local CodeRabbit review reported zero findings; reviewed-head CI `36668710496`
and merged-main CI `36668894684` passed. Completed branches were removed.

## Unreleased: UI Labs package

The optional `uiLabs` utility uses `pepeeltoro41/ui-labs@2.4.2` and can be
selected alongside any UI framework. Catalog details include a controls example
and explain that the Studio plugin is installed separately. Git submodules use
the utility repository's `src` module, not the plugin repository.

The Wally index confirms version 2.4.2 is shared with no dependencies. Upstream
tag `v2.4.2` supplies the source layout and control API; its checked-in
`wally.toml` still says 2.4.1, so the registry is the version authority.
Sources: [registry](https://github.com/UpliftGames/wally-index/blob/main/pepeeltoro41/ui-labs),
[utility source](https://github.com/PepeElToro41/ui-labs-utils/tree/v2.4.2).
Studio story execution remains unverified. This addition is not yet published.

[PR #38](https://github.com/chatarabdelilah/rproj/pull/38) merged at `eb09aef`.
Local CodeRabbit review reported zero findings. Reviewed-head CI `36667964166`
and merged-main CI `36668163520` passed Windows stable, Rust 1.89, and packaging.
The completed feature branches were removed.

September 30 local verification passed: 404 ordinary tests (19 deliberately
ignored), formatting, clippy with warnings denied, locked packaging with
`--allow-dirty` for the reviewed source, and `cargo run --locked -- info uiLabs`.
The ordinary catalog checks cover guide imports, unique keys, dependency closure,
and package selection. No Studio plugin installation or story playtest was run.

## 0.18.1 published

Scope is the post-0.18.0 work from [PR #33](https://github.com/chatarabdelilah/rproj/pull/33),
[PR #34](https://github.com/chatarabdelilah/rproj/pull/34), and
[PR #35](https://github.com/chatarabdelilah/rproj/pull/35): compatible existing
Jest mounts, explicit setup choices, Scribe Studio, forced trusted global Rokit
adds, additive project composition, Catalog mouse scrolling, TestEZ companion
configuration, explicit StarterPlayer classes, and reliable Test/Watch Wally
synchronization. No dependency was added.

[Release notes](release-notes-0.18.1.md) record user-visible behavior and limits.
The owner published from clean commit
`9d136812d5d33d358b67a36e503db47466fc0760` on September 29, 2026. The
non-yanked crates.io archive checksum is
`e2080b8fef645191fdb7e6c0df5c33b11530a0678080a5efdac067bab285c7d3`;
its embedded Git identity, annotated tag, and
[GitHub alpha release](https://github.com/chatarabdelilah/rproj/releases/tag/v0.18.1)
match that commit.

Local candidate gates passed 404 ordinary tests with 19 prerequisite-dependent
tests ignored, formatting, clippy with warnings denied, and locked packaging
(96 files, 1.2 MiB uncompressed, 310.1 KiB compressed). The final clean package
also embedded `9d136812d5d33d358b67a36e503db47466fc0760`. Reviewed-head CI run
`36590433561` and merged-main CI run `36590794320` passed Windows stable,
Rust 1.89, and package jobs. Evidence is recorded in
[PR #36](https://github.com/chatarabdelilah/rproj/pull/36). Fresh Windows
provisioning and a Scribe Studio playtest remain unverified.

## 0.18.1: project composition maintenance and reliable test/watch coexistence

New Project starts at Review and retains Start point access to Guided, Expert,
and saved setups. Projects can add packages and capabilities through a Ratatui
editor for valid `rproj.toml` projects. Existing choices remain selected;
dependency-workflow changes and removals stay outside this bounded editor.
Wally updates preserve unrelated entries and comments, generated configuration,
test mounts, test folders, tool pins, and sourcemaps are refreshed, and an
external `rproj.toml` edit blocks saving.

Test execution reuses a complete selected Wally package tree, avoiding the
directory-replacement window that crashed a concurrent Rojo sourcemap watcher.
Missing aliases still run the existing Wally recovery path. TestEZ companion
configuration is always generated with TestEZ. Catalog details accept mouse-wheel
scrolling while keyboard scrolling remains available. Generated StarterPlayer
and StarterPlayerScripts nodes carry explicit class names.

The project editor is additive: stop an active Watch before applying package
additions. It may reformat `rproj.toml`; Wally entries outside rproj's catalog are
left untouched. On September 29, 2026, the locked ordinary suite passed 404 tests
with 19 prerequisite-dependent tests ignored. Formatting and clippy with warnings
denied passed, and the installed real Rojo validated every built-in template
variant. CodeRabbit identified ten applicable recovery, manifest-preservation,
and freshness findings; all were corrected, and the final complete-diff review
reported zero findings. CI evidence belongs to the implementing PR.

## 0.18.1: explicit choices, Scribe Studio, and Rokit setup trust

Fresh Machine Setup and New Project optional selections start unchecked;
saved choices are preserved. Removed catalog selection-default fields and
redundant initial-selection state. Scribe Studio uses the existing manual
Studio-plugin flow. Confirmed machine tool installation uses
`rokit add --global --force`, including for pinned sources; project-local pins
keep their existing behavior. A setup rerun reinstalls selected global tools.

The [duplication audit](duplication-audit.md) records measured baseline size,
shared execution already present, remaining duplicated policy/persistence,
and the recommended bounded consolidation. It is not a claim of a completed
Inquire migration. Tests, review, and CI evidence are recorded in the PR.
No live machine provisioning or Scribe playtest was performed.

## 0.18.0 published

Scope is the merged work from [PR #30](https://github.com/chatarabdelilah/rproj/pull/30)
and [PR #31](https://github.com/chatarabdelilah/rproj/pull/31), plus aligned manifest
versions and release documentation. No dependency or runtime behavior changed
during version preparation.

[Release notes](release-notes-0.18.0.md) include the DevPackages import migration,
Local Studio/Open Cloud selection, credentials, and verification limits. The owner
published from clean commit `ba1319b5ec0a681ffac0d773ae0fb893b21c0692` on
September 21, 2026. The non-yanked crates.io archive checksum is
`0882efd53f10ade38d6b46515a699b77ca972769383903c25a1f0e33ed27df23`;
its embedded Git identity, annotated tag, and
[GitHub alpha release](https://github.com/chatarabdelilah/rproj/releases/tag/v0.18.0)
match that commit.

Candidate verification results and reviewed-head/main CI are recorded in
[PR #32](https://github.com/chatarabdelilah/rproj/pull/32). Live Open Cloud,
fresh Linux generated-project execution, and fresh Windows provisioning remain
unverified; ignored tests are not counted as passes.

## 0.18.0: Jest execution selection and Ratatui refinements

The Jest picker now asks for Local Studio or Open Cloud. The choice is retained
in project records and saved setups, drives runner configuration, and determines
whether generated CI contains cloud tests. Local creation alone provisions the
Studio runner plugin. This supersedes PR #30's repository-variable switch.

Catalog rows contain names only; group explanations appear in Overview and
entry explanations remain in Details. Enter opens groups, never focuses an
entry; Page Up/Down and Ctrl+Home/End scroll details without a focus change.
Projects > Configure Tools stays in the shared Ratatui terminal, reviews changes,
preserves unrelated/unsupported settings, protects cancellation, checks external
edits, and atomically saves the selected tool's configuration. Direct CLI configure
retains its prompt interface.

Local verification: 388 ordinary tests passed, 19 prerequisite-dependent tests ignored; formatting and clippy passed. The PTY regression saves a tool setting and returns to the project with one terminal enter/leave and no suspension. Unit coverage checks settings preservation, cancellation, external edits, malformed files, JSON merging, four terminal sizes, Jest backend persistence, and both generated CI variants. The isolated installed Jest Roblox 0.3.24/Studio regression also passed eight package examples after real Wally installation and Rojo sourcemap regeneration. CodeRabbit reported one documentation clarification about atomic visibility versus crash durability; it was corrected. Reviewed-head/main CI are recorded in the implementing PR. Open
Cloud execution is not claimed without credentials; no applications are installed
by ordinary tests.

## 0.18.0: generated project CI and ignore coverage

Jest projects now expose `ReplicatedStorage.devPackages` in the ordinary
sourcemap as well as the Jest project. Disk casing remains `DevPackages`.
The test project omits Lighting and enables LoadStringEnabled. PR #30 initially generated CI that
opted into Open Cloud with `JEST_OPEN_CLOUD=true` (superseded above); enabled cloud tests still failed
on missing credentials. Ignore rules cover generated place files, place locks,
coverage, and local environment files while preserving `.env.example`.
See [the source review and migration notes](project-template-review.md).

Local verification: 378 ordinary tests passed, 19 ignored; formatting and clippy
with warnings denied passed. The isolated real-Wally/Rojo/Jest regression passed
using the installed pinned Jest Roblox 0.3.24 executable, including ordinary
sourcemap development-package resolution and eight Studio package examples.
The initial attempt through the Rokit shim failed because the temporary fixture
has no Jest tool manifest; direct use of the existing pinned binary resolved that
fixture prerequisite without installing applications. Open Cloud execution and
fresh Linux generated-project execution remain unverified. CodeRabbit review
and reviewed-head/merged-main CI evidence are recorded in the change's PR.

## T5: 0.17.0 Published

Owner publication is verified. The non-yanked crates.io archive checksum is `c95a02febad76d5956b2446dbf492ea78ddb888d248e34a5a0737fa266ef13a2`; its Git identity, annotated tag, and [GitHub alpha release](https://github.com/chatarabdelilah/rproj/releases/tag/v0.17.0) match clean commit `9ee8d5167175c7bacb58a61e9317b4fda5d443ea`. [Final reviewed-head CI](https://github.com/chatarabdelilah/rproj/actions/runs/35499542790) and [merged-main CI](https://github.com/chatarabdelilah/rproj/actions/runs/35499712898) passed Windows stable, Rust 1.89, and packaging. Badge freshness was intentionally skipped. All three merged implementation/fix branches were deleted locally and remotely.

A subsequent stable-Windows CI run timed out in the Catalog diagnostic-log PTY test. That test now waits for its filter to render and uses the documented direct Catalog exit instead of an unsynchronized Enter/Esc sequence. All eight diagnostics tests and formatting passed locally; this is test-only and does not change Catalog navigation.

Implementation merged through [PR #26](https://github.com/chatarabdelilah/rproj/pull/26), followed by test-only synchronization corrections.

Merged-main CI exposed a repeated-Watch PTY synchronization race. A checkpoint before launch was insufficient: ConPTY repainted the previous normal-screen acknowledgement when the next watcher left the alternate screen. The failed CI snapshot showed that watcher still running while the test expected Project actions. Each fixture launch now has a unique readiness marker, and completion waits start only after that marker is visible. Ten consecutive runs of all nine hub tests passed locally. This changes test synchronization only, not Watch runtime behavior. [PR #28](https://github.com/chatarabdelilah/rproj/pull/28) records the correction and final-head/main CI gates; owner publication followed successful checks.

- Local locked ordinary suite: **375 passed, 19 ignored**, 394 discovered (334 unit and 60 integration tests). Fixture PTY coverage exercises review/category cancellation, default-No confirmation, success and Back/Exit, worker panic, fatal failure, save failure, and stop-after-active-child without configuration saving or a subsequent item. Home cancellation remains in one terminal session.
- Real fixture subprocesses exercise concurrent stdout/stderr draining, Unicode/control-sequence handling, failed spawning, nonzero exits, and cooperative stopping. All roots/configuration writes are injected temporary fixtures; no applications are installed. Global-config replacement is staged, synced, and tested for preserving the old file when Windows denies replacement.
- Formatting, clippy, and locked packaging passed (94 packaged files, 1.1 MiB uncompressed, 294.8 KiB compressed after review fixes). CodeRabbit identified four applicable findings: completion lost after Back/Exit, CRLF doubling, mismatched Rokit trust guidance, and stale architecture documentation. All were corrected with regression coverage; follow-up module-map indentation was also corrected. Final local gates passed after runtime fixes. [Reviewed runtime CI](https://github.com/chatarabdelilah/rproj/actions/runs/35457787313) passed Windows stable, Rust 1.89, and packaging at `df102d0`. [PR #26](https://github.com/chatarabdelilah/rproj/pull/26) records final-head checks and release closeout; final merged-main verification and owner publication are recorded above.
- Missing live acceptance: fresh Windows provisioning, winget/vendor/UAC dialogs, real VS Code/Blender installation, and manual plugin/account linking were not exercised. Existing installers and project paths remain, but fixture parity does not establish fresh-machine installation success. No unrelated ignored live integrations were rerun.
- Limits: stop waits for the active item; no rollback or force-kill. A stalled installer may require external intervention. Display/output parsing are bounded with explicit truncation/failure behavior. Completed attempts record selection intent even when individual installations need repair; warnings are not proof of readiness.

## T4: 0.16.0 Published

Owner publication is verified. Crates.io reports non-yanked `0.16.0`; its archive SHA256 is `8031eb010130589d0b9ab74233845aabc53557e9d799f54d372ad04ee0373c3d`, matching the registry checksum. `.cargo_vcs_info.json` identifies clean commit `d54a9a874d8f23f5a57ebd573b6f1cd806506000`. The annotated `v0.16.0` tag and [GitHub alpha release](https://github.com/chatarabdelilah/rproj/releases/tag/v0.16.0) use that commit. [Main CI](https://github.com/chatarabdelilah/rproj/actions/runs/35426957745) passed, and the fully merged implementation branch was deleted locally and remotely.

- Local ordinary locked suite: 351 passed, 19 ignored (370 discovered: 312 unit and 58 integration tests). A stale Catalog navigation index was corrected after inserting the new Home destination.
- Storage checks cover byte preservation, unknown fields, unsupported choices, invalid Jest repair, conflicts, collisions, name restrictions, Windows junction rejection, locked replacement failure, duplication, deletion, and rename partial failure. Fixtures use temporary storage.
- Manager checks cover saved baselines, default-No discard/deletion, failed saves retaining drafts, browser Back state, four terminal sizes, and repeated-save PTY restoration. Home PTY coverage verifies the manager borrows the same terminal. Creation controls retain provenance and unknown exclusions in setup mode.
- Formatting and clippy passed. The saved-setup replay integration passed for Wally and Git submodules (one serial test, 22.45 seconds), including direct replacement semantics and source-byte preservation during replay. Unrelated live integrations are not rerun for this milestone.
- Locked packaging passed at runtime commit `f85e14d`: 89 files, 1.1 MiB uncompressed, 284.6 KiB compressed; archive compilation passed. CodeRabbit CLI 0.7.6 completed the committed branch's base-main review with zero reported findings. A preliminary working-tree review also reported none; the committed review was necessary to include new source files after staging. Subsequent evidence-only edits do not change runtime or dependencies. [PR #24](https://github.com/chatarabdelilah/rproj/pull/24) records the reviewed commit, CI run links, and merged-main verification; both Windows toolchains and packaging must pass before owner publication.

Limits: unsupported composition choices conservatively disable guided editing; file management remains available. Changed composition saves may drop comments/reformat TOML. External-writer checks are best effort, and rename is not a multi-file transaction. T5 remains Machine Setup; model import remains permanently dropped.

## T3: 0.15.0 Published (September 10, 2026)

Owner publication is verified: crates.io's archive SHA256 is `2dc75bbd10e4e11180995ce267f2b5a7f96b8eb60802fe1adf4e10ce50424a45`; `.cargo_vcs_info.json` records clean commit `b1c1664273bea2fce9bb0f4b954f554cc96f9dfb`. The annotated `v0.15.0` tag and [GitHub alpha release](https://github.com/chatarabdelilah/rproj/releases/tag/v0.15.0) match that identity. Version 0.15.0 is not yanked.

- Scope: Projects browser, selected-path command dispatch, creation handoff, and removal of Catalog Place Template presentation. No dependency or configuration-schema change; model import stays dropped.
- Local ordinary locked suite: 330 passed, 19 ignored (349 discovered: 292 unit and 57 integration tests). Coverage includes junction exclusion, malformed projects, missing roots, stale scans, Back state, four-size rendering, deleted targets, and A/B path isolation with fake tools and a fake clipboard sink.
- Eight Home PTY checks cover terminal continuity, selected-project actions, repeated Watch interruption, watcher failure, cancelled provisioning, and redirected output. Three live creation checks passed serially using installed tools: cancellation, concurrent-destination refusal, and successful creation/saved-setup handoff. Their scratch projects are removed; no machine applications were installed.
- Formatting, clippy, and locked packaging passed. Packaging at `c57221c` contained 86 files (1.0 MiB uncompressed, 272.1 KiB compressed) and compiled successfully from its archive. Local CodeRabbit CLI 0.7.6 completed the base-main review on September 10 with zero findings. [PR #22](https://github.com/chatarabdelilah/rproj/pull/22) records reviewed-head and merged-main Windows stable/Rust 1.89/package CI; publication is gated on both runs passing.
- Existing Open Cloud and fresh-machine provisioning limitations remain. Ordinary tests do not modify the real clipboard or provision machine applications.
- Reviewed head `2eb6b49` passed CI run `34426275526`; merged main `b1c1664` passed CI run `34426498712`. Both runs passed Windows stable, Rust 1.89, and package verification. The merged feature branch was deleted locally and remotely. Owner publication, annotated tag, and GitHub prerelease are complete; documentation-only alignment needs no further Cargo publication.

Previous baseline (September 9, 2026): **0.14.0**, alpha, with persistent Home, template save continuity, and Catalog clarity. Its crates.io archive records `f86a7bd06d0f5b637a41a0e0ca05792be66c42e5`, matching the annotated tag and [GitHub alpha release](https://github.com/chatarabdelilah/rproj/releases/tag/v0.14.0). SHA256: `b3469e9cf85ae5f1a88f6ec6aaf5065ce05301cd52172414b5adafc34974b95e`. Model import and embedded quality tools are permanently dropped. See [release notes](release-notes-0.14.0.md).

The published package and annotated `v0.12.2` tag correspond to commit `f71bf4e`; the [GitHub release](https://github.com/chatarabdelilah/rproj/releases/tag/v0.12.2) is a prerelease. The automated Jest regression was merged afterward in [PR #7](https://github.com/chatarabdelilah/rproj/pull/7), at `a855a2f`. It is present on main, not in the published 0.12.2 archive; no runtime code or version changed in that PR.

## Confirmed Defects And Fixes

1. **Project creation touched the destination before confirmation.** Escape could leave an empty directory, and explicit Cancel recursively removed it even if another process had added files. Creation now happens after confirmation, uses an exclusive directory creation, and never deletes the destination on cancellation. Unit tests cover an existing directory and missing parents; a live regression covers another writer creating a sentinel while the prompt is open, for both Create and Cancel.
2. **Some Inspector values did not match Rojo's serialization.** UDim/UDim2/Rect now use explicit nested representations with integer offsets; CFrame attributes use position/orientation fields. Reopening values flattens components for the existing input controls. Unit round trips and a real-Rojo validation test cover the fix. NaN/infinity and fractional/out-of-range offsets are rejected before changing the draft.
3. **Live tests were stale and unsafe to rerun against an occupied project root.** They selected TestEZ in the old package picker, assumed VS Code was selected, manually parsed the wrong global-config path, and deleted fixed-name directories. The harness now follows current capability/runner prompts, reads actual TOML via the configured directory API, checks provisioning up front, and uses unique scratch names without removing pre-existing directories.
4. **Jest Roblox projects could not run their selected test runner.** Rokit 1.2 rejected untrusted project-local sources, the generated runner invoked `jest-roblox` while Rokit installs `jest-roblox-cli`, and `test.projects` used DataModel-path strings that 0.3.24 interprets as configuration-file paths. rproj now trusts selected sources before every add, invokes the installed executable, and emits inline project entries with filesystem include globs.

## Evidence

### 0.14.0 T2 Published

Published at `f86a7bd` after PR #20 and owner publication; archive, annotated tag,
and alpha release are aligned. Scope: persistent Home, template save
baselines, foreground cancellation, Catalog hierarchy/readability, and bundled
package guidance. No new catalog packages or machine applications are installed.

September 9 local evidence:

- The ordinary locked suite passed: 320 passed, 19 explicitly ignored,
  339 discovered (282 unit and 57 integration). Formatting, clippy with warnings
  denied, and diff whitespace checks passed. Reviewed-head/main CI and package
  identities are recorded below when complete.
- Windows PTY fixtures passed repeated Watch interruption, unexpected Watch
  failure, Home re-entry, and interrupted Rokit restoration without starting Lute.
  An exclusively held child file proves the child has exited before Home returns.
- Template PTYs passed repeated saves without closing, confirmed reset, and
  malformed JSON repair. Unit tests cover saved-baseline undo/redo and atomic
  failure. A large paste exposed per-character full-screen redraw cost; queued
  input now redraws at a bounded interval. Float round-trip testing exposed
  serde_json's default parser rounding; float_roundtrip now preserves those values.
- All three real-Rojo template/editor checks passed with Rojo 7.7.0. All three
  live Home-creation regressions passed in 29.51 seconds: cancellation,
  concurrent-destination refusal, and confirmed creation/saved-setup replay.
  Initial sandbox denials were not counted as passes; the successful run used
  unique temporary projects and a unique saved setup outside the sandbox.
- Eight Catalog package snippets executed without errors through installed
  Jest Roblox CLI 0.3.24 and Studio, following Wally installation and sourcemap
  retyping. The fixture needed its missing source directories restored; its
  original count incorrectly assumed starter specs despite requesting none.
  The corrected complete live stack check passed in 12.83 seconds.
  [Example evidence](catalog-examples.md) separates source review from runtime coverage.

Locked packaging passed at `8389643`: 85 files, 1.0 MiB uncompressed and 263.9 KiB
compressed; the crate built from its archive. The package includes the new source
modules and PTY fixture, while retaining the existing documentation/CI exclusions.
After review fixes, locked packaging passed again at `4889a48`: the same 85 files,
1.0 MiB uncompressed and 264.2 KiB compressed, compiled from the archive.

Local CodeRabbit review identified recovery and cancellation improvements:
creation now names the retained destination on failure/cancellation, Blender's
temporary script has RAII cleanup, and VS Code/Blender discovery preserves
cancellation instead of a misleading missing-tool error. VS Code extension
cancellation is propagated, and README states the help/version logging exception.
The ordinary suite and clippy passed again after these runtime fixes. The full
local review completed with 11 findings; the narrow six-file follow-up completed
with zero findings. [Release PR #20](https://github.com/chatarabdelilah/rproj/pull/20)
records reviewed-head CI, any remote review follow-up, and merged-main verification
before the owner is asked to publish. Documentation-only evidence updates do not
change the tested runtime or dependency graph.

Remote CodeRabbit review at `13ffed0` added an explicit Jest JSON formatter and a
90-second failing watchdog for the PTY child fixture. Both were applied: all
eight Home PTYs passed, and all eight Catalog snippets passed again through
Studio in 37.88 seconds. Its prose-wrapping edge case was reproduced and fixed
with assertions in the existing Catalog test; the four Catalog tests pass.
The audit header date was corrected. Windows stable, Rust 1.89, and package CI
passed at both `13ffed0` and final correction commit `2ec83ba`, then on merged
main `f86a7bd`. The owner's `.codex/` and `.serena/` ignore rules keep local agent
configuration outside Git and Cargo archives; no such configuration is shipped.

Review decisions: do not force `process::exit` on a second interrupt because it
skips terminal restoration and child waiting. Do not record machine setup as
completed after cancellation. Do not relax the immediate child-lock assertion:
the subprocess wait must already have completed before acknowledgement. Catalog
Ctrl+C intentionally returns to Home (standalone Catalog exits); its separate
Quit/Back outcomes express that caller-dependent behavior. Acknowledgement remains
the documented Enter action: the suggested interrupted-read shortcut did not
work in a Windows PTY and was reverted, not counted as a passing Ctrl+C check.

Remaining limits: no fresh-machine installation/cancellation certification,
no Open Cloud run, and no full UI/data/replication lifecycle execution for every
package. Commands ignoring Ctrl+C can delay return until they finish; no forced
termination or detached supervisor is added.

### Unreleased Configuration Preservation

Three new regressions first reproduced destructive behavior in 0.13.0: an unlisted choice was reset to its catalog default, a structured JSON value was replaced with a boolean, and accepting handwritten TOML produced duplicate keys/tables. The fix omits unchanged answers, keeps unsupported values by default, and validates a proposed TOML merge against the expected parsed document before writing. Explicit replacement remains available. This does not introduce file locking or crash-atomic writes, and the legacy TOML writer may refuse unusual valid layouts rather than rewrite them unsafely. Shared upgrade/scaffolding writers are unchanged.

September 8: all 309 ordinary tests passed (19 explicitly ignored, 328 discovered). Eight regressions were added: two unit tests and six PTY tests. An initial new replacement test used the heading-wait helper for an inline confirmation; correcting that test synchronization produced the passing full run. Formatting, clippy, reviewed-head CI, and main CI passed on the fix PR. The 0.13.1 release candidate packaged 80 files, 965.4 KiB uncompressed / 249.0 KiB compressed; [release PR #17](https://github.com/chatarabdelilah/rproj/pull/17) and [merged-main CI](https://github.com/chatarabdelilah/rproj/actions/runs/34183475162) passed. CodeRabbit required manual review for this OSS release PR; that review found no issues. Live Studio/Rojo/provisioning checks are not rerun because their execution paths are unchanged; the dated 0.13.0 evidence below remains historical, not a new pass.

### Released Evidence

| Check | Result |
| --- | --- |
| 0.13.0 candidate local gates | September 8: 301 ordinary tests passed (19 explicitly ignored); formatting and clippy passed. All 14 serial live regressions passed in 183.89 seconds. The three real-Rojo checks passed in 5.22 seconds. Verified installed Rokit 1.2.0, Rojo 7.7.0, and Wally 0.3.2; the Jest runner was exercised through its project-local pin, not a global shim in this repository. Packaged verification and exact reviewed-head/main CI are recorded on the release PR before publication. |
| Creation merge/main CI | PR #14 merged at `a90037f`, tree-identical to reviewed `ef1d5df`. [Main CI](https://github.com/chatarabdelilah/rproj/actions/runs/34162680901) passed Windows stable, Rust 1.89, and package builds. Both CodeRabbit findings were resolved; the merged feature branch was deleted. |
| Creation source verification | 301 ordinary tests passed; 19 explicitly ignored; 320 discovered (273 unit + 47 integration). Formatting, clippy, and `cargo package --locked` passed (80 files). The three real-Rojo template/Inspector checks passed. Windows stable, Rust 1.89, and package CI passed on `ea07979`; [PR #14](https://github.com/chatarabdelilah/rproj/pull/14) records subsequent reviewed-head and merge/main verification. |
| Creation CodeRabbit review | Both actionable findings were verified and fixed: pasting into name/setup modals now preserves the Review selection (regression assertions added), and architecture live-test counts/breakdowns now match 14 live tests. Unfinished capability choices also have explicit back-navigation regressions. |
| Creation CI correction | The first Windows stable/1.89 run exposed an existing hub-name test that depended on this machine's saved setup. The fixture now explicitly marks setup ready; the separate disabled-action test covers the unconfigured state. No machine provisioning was added to CI. |
| Creation live regression | September 7: all 14 serial live tests passed in 125.57 seconds, including hub cancellation, real confirmation, concurrent destination refusal, named setup save/replay, direct saved-setup replay/refusal, Wally/submodules, generated quality gates, and Jest starter success/deliberate failure. The initial hub test used the wrong prompt label; corrected to the existing `Project folder name` before the passing run. Only unique temporary fixtures were removed. |
| Ordinary suite with saved-setup refusal regression | 273 passed; 16 deliberately ignored; 289 discovered (253 unit + 36 integration) |
| Diagnostic-logger source suite | 286 passed; 16 deliberately ignored; 302 discovered (258 unit + 44 integration). Five logger unit tests and eight integration tests cover unique bounded logs, redaction/control escaping, command outcomes, accepted settings, TUI navigation without text capture, opaque runner arguments, opt-out, and non-fatal logging failures. |
| Live regression rerun with diagnostic logging | Passed September 7: saved-setup refusal/replay together in 28.53 seconds; Jest starter specs passed, then the deliberate failure returned 1 with two passed / one failed in 28.93 seconds. Existing unique temporary fixtures were removed by the harness. |
| Formatting and clippy | Passed locally |
| Existing live project suite | Seven passed in 38.39 seconds on September 5 after harness corrections |
| Concurrent-destination live regression | Passed for both cancellation and confirmation |
| Existing real-Rojo template checks | Passed with Rojo 7.7.0 |
| Compound Inspector output | Passed real-Rojo validation across the template matrix |
| Live Jest Roblox execution | `jest-roblox-cli` 0.3.24, Studio CLI backend: three generated starter specs passed in 11.58 seconds on September 6 |
| Automated live Jest regression, PR #7 | Reviewed test passed in 26.84 seconds on September 6: scaffolded Wally + Jest in a unique temporary directory, verified project-local pins and three passing starter specs, then broke the shared spec and verified exit code 1, the assertion failure, and two passing / one failing test. Temporary project removed. |
| Automated saved-setup replay | Passed in 20.07 seconds on September 6: real `new --save-setup` then `new --like` for both Wally and Git submodules, with charm/promise, lint/format, and a dropped `.gitignore`. Verified exact graphs, no repeated choices, installed package links, selected tool pins, generated/omitted files, unchanged saved setup, and cleanup. |
| Automated saved-setup refusal | Passed in 7.78 seconds on September 6: missing setup, malformed TOML, and Jest with no manager or Git submodules each exited 1 with a case-specific error before explicit `--reconfigure`. No input was sent; no destination/parent was created; machine config, source fixtures, and an existing output-setup sentinel remained byte-identical. Temporary directories removed. |
| Upstream badge check | Passed September 6; advisory to review Matter's Active badge (last reported push December 31, 2024); not proof that the project is abandoned |
| Previous CodeRabbit findings, PR #4 | Both addressed in baseline commit 4737c84; plugin identity and ServerPackages exclusion regression present |
| Published 0.12.2 package | cargo package --locked passed at `f71bf4e`; 73 files, 852.5 KiB uncompressed; crates.io reports 0.12.2 |
| PR CI and CodeRabbit | Historical 0.12.1 review: [PR #6](https://github.com/chatarabdelilah/rproj/pull/6). Jest regression: [PR #7](https://github.com/chatarabdelilah/rproj/pull/7), CodeRabbit reported no actionable findings at `2496a42`; Windows stable, Rust 1.89, and package checks passed. [Post-merge main CI](https://github.com/chatarabdelilah/rproj/actions/runs/34038222655) passed at `a855a2f`. |

Reproduction commands:

```powershell
cargo fmt --all --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
$env:RPROJ_TEST_TIMEOUT = '180'
cargo test --locked --test live jest_starter_specs_pass_and_report_failure -- --ignored --test-threads=1 --nocapture
cargo test --locked --test live saved_setup_replays_workflow_packages_capabilities_and_dropped_files -- --ignored --test-threads=1 --nocapture
cargo test --locked --test live invalid_saved_setups_refuse_before_reconfiguration_or_creation -- --ignored --test-threads=1 --nocapture
cargo test --locked --test live -- --ignored --test-threads=1
cargo test --locked steps::rojo::tests:: -- --ignored --test-threads=1
cargo test --locked guided_compound_values_pass_real_rojo_validation -- --ignored
cargo test --locked badges_do_not_contradict_upstream -- --ignored --nocapture
cargo package --locked
```

The live project suite uses unique directories under the configured projects root and shares real Rokit/Wally caches. The Jest regression requires Studio and JestRobloxRunner to be installed; normal scaffolding refreshes that plugin. Only its temporary project is isolated and removed. Run serially on an explicitly provisioned machine, not as an unattended installer on a fresh host. The regression remains ignored by ordinary CI and must be invoked explicitly.

## Remaining Limits

The refusal regression is also ignored because it reads real machine configuration and uses the configured projects/setup roots. It owns unique fixture directories and never answers provisioning prompts; a newly created empty setup directory may remain. A saved global template can invoke Rojo validation before setup refusal. This is not fresh-machine provisioning evidence.

The saved-setup regression uses the real configured projects root and setup directory, reserving a unique setup filename before the CLI writes it. Both projects and the reserved setup are removed, including partial runs; a newly created empty setup directory may remain. Existing setups and machine configuration are not changed. Rokit/Wally caches and trust state remain shared; this test requires provisioned tools and network access, runs serially, and does not install Studio plugins. Like the Jest regression, it is ignored by ordinary CI and does not change the published 0.12.2 package.

- **Jest evidence covers the local Studio CLI backend.** The automated starter-spec pass/fail gap is closed by PR #7. This does not verify Open Cloud, an attached Studio session, or a fresh Windows installation. In T2, the ignored `real_jest_stack_installs_validates_retypes_and_executes` test now executes eight Catalog snippets instead of only no-test success; the live regression separately verifies starter execution through `rproj test`.
- **Open Cloud execution was not performed.** No audit universe/place and credentials were supplied. Generated preflight and workflow text are tested, but that is not equivalent to a current hosted run.
- **Fresh Windows provisioning was not performed.** winget/editor/Studio/plugin installation changes the machine. Existing unit coverage and inspection are not a substitute for a clean-machine acceptance test.
- **Saved-setup evidence is bounded.** Replay covers valid Wally and Git-submodule compositions. Refusal covers missing/malformed records and Jest without Wally, before explicit reconfiguration or creation. It does not cover every hand-edited/legacy setup or unknown-package fallback. Clipboard contents, machine provisioning/recovery, and every template keyboard path also remain gaps. No claim of beta/1.0 readiness is made.
- **Template validity remains Rojo-authoritative.** This patch repairs specific existing compound controls, not every possible explicit/future property representation. Unsupported advanced values and old malformed drafts are not automatically normalized.

## Handoff

Keep Ratatui and all external tool boundaries. The model-import experiment was permanently retired at the owner's request on September 9, 2026. The local branch and importer-only build artifacts were deleted; no remote branch, separate worktree, or stash existed. No importer code was merged. Historical commits are not rewritten.

Releases 0.13.0, 0.13.1, and 0.14.0 are aligned and complete. The agent handles review findings, merge, post-merge CI, merged-branch cleanup, and release alignment; the owner alone runs `cargo publish --locked` when a new package is ready. Test-only and documentation-only follow-ups do not require publication or moving an existing release tag.

Saved-setup replay/refusal, diagnostic logging (PR #11), shared confirmed execution (PR #12), and Ratatui creation (PR #14) shipped in 0.13.0 through release PR #15. [Merged-main CI](https://github.com/chatarabdelilah/rproj/actions/runs/34181012236) passed. Configuration preservation shipped in 0.13.1 through PRs #16 and #17. T2 shipped in 0.14.0 through [PR #20](https://github.com/chatarabdelilah/rproj/pull/20), with [merged-main CI](https://github.com/chatarabdelilah/rproj/actions/runs/34317249700) passing at `f86a7bd`. The owner published; archive identity, annotated tag, and alpha release are aligned. Completed release branches are deleted. Do not republish or move shipped tags. Remaining alpha audit gaps stay tracked.

## Post-Handoff Review: September 7, 2026

Reviewed the task history and changes from v0.12.1 through main at `fe8f4f4`.

- **Release identity:** crates.io 0.12.2 is not yanked; its archive records
  `f71bf4e103735a532b2e8de53cbbd560f5c2a3d3`, matching the annotated v0.12.2
  tag and GitHub alpha release. Do not undo or retag that publication.
- **Process correction:** 0.12.2 was pushed directly to main and Cargo-published
  by the agent after a short "publish 0.12.2" request. That bypassed the intended
  PR workflow and owner-operated publication handoff. Root `AGENTS.md` now makes
  the ownership rule and new-task startup requirements explicit.
- **Change assessment:** the Jest executable/configuration/trust fixes address
  observed failures. PRs #7-#10 add regression coverage and documentation;
  PR #11 adds the requested logger; PR #12 extracts existing confirmed execution.
  No runtime blocker was found in this review. No rollback is indicated.
- **Independent verification:** 286 ordinary tests passed, 16 deliberately
  ignored; formatting and clippy passed locally. GitHub records successful
  main CI for every post-0.12.1 merge, including
  [fe8f4f4](https://github.com/chatarabdelilah/rproj/actions/runs/34101827132).
  CodeRabbit's documentation findings in PRs #8/#10 are addressed; logger and
  execution-boundary reviews report no actionable findings. The existing live
  test evidence is historical, not a new live run during this review.
- **Remaining caution:** logs contain local paths and selected names, have no
  automatic retention cleanup, and are not a complete screen transcript. Review
  them before sharing. Open Cloud and fresh-machine acceptance remain unverified.

Update September 8: **0.13.0 is published and aligned** after
[release PR #15](https://github.com/chatarabdelilah/rproj/pull/15) and successful
merged-main CI. The owner performed publication; the agent verified the archive
identity before creating the annotated tag and GitHub alpha prerelease. The next
configuration-preservation fix is separate and unreleased.
