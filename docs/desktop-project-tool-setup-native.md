# Native Project Tool Setup implementation

Approved November 17, 2026 against clean main `bd1782a`. Published version
remains 0.22.0. The approved Soft design is Tool → Review → Results, one
captured project/tool per review, with wait-for-completion closing.

## Delivery sequence

1. Shared read-only planning and checked single-tool execution, preserving
   existing pins/files and reporting required tool failures accurately.
2. Versioned private worker/client operations, bounded atomic reports and
   owned process-tree completion without a cancellation action.
3. Native Soft controls, retained ownership and default-Keep confirmations,
   Activity integration and close-after-completion protections.

Existing CLI ancestor discovery remains available; native requests capture an
explicit project. Setup requires a Rojo project file and actual prerequisites,
not an rproj record or a completed global setup flag. Shared batch setup
semantics remain outside the single-tool failure correction.

Preparation captures relevant existing and missing paths without writes.
Execution revalidates sources, prerequisites and Watch ownership. Existing
pins/configuration stay intact; missing text configuration uses the existing
generators. External trust/cache/download operations and partial effects must
be reported explicitly. No rollback is promised.

## Evidence

The pre-extraction CLI fixtures reproduced success after failed tool addition
and unnecessary external calls for an existing custom pin. The original
missing-manifest/config fixture passed. The initial sandboxed run was blocked
by Windows access denial before reaching setup; its results were not counted
as defect reproductions. Running the disposable suite with access to the Watch
mutex reproduced both defects. The new checked executor passes all four CLI
fixtures, including misleading failure output, missing result pins, failed
trust/init and unexpected changes to other pins. Generator text remains exact.

Rokit [Add](https://github.com/rojo-rbx/rokit/blob/main/src/cli/add.rs) documents
that `--force` also overwrites existing pins. This path therefore invokes
[Trust](https://github.com/rojo-rbx/rokit/blob/main/src/cli/trust.rs) separately,
revalidates the review, then adds without force using an explicit executable
alias. Existing pins never invoke an installer. Source/output checks are
optimistic checks, not a transaction against arbitrary external writers.

Implementation is in progress on `codex/project-tool-setup-shared`. The first
full CLI/core run passed 830 tests with 25 prerequisite-dependent cases ignored;
165 headless desktop tests passed. Strict CLI/core and desktop Clippy, formatting,
portable archive/static-runtime checks and matching-CLI smoke checks passed.
The archive preserves notices for 590 packages; three existing legacy archive
notice-completeness gaps remain. Native previews, real installation and real-window acceptance remain
paused/unverified because native memory stability is unresolved.

CodeRabbit 0.7.6 reviewed eleven files and raised two questions. Upstream
[Rojo accepts JSONC](https://github.com/rojo-rbx/rojo/blob/master/src/json.rs),
so the newly introduced strict JSON parse was removed: tool setup captures and
preserves the readable project file, but does not certify its contents or require
Rojo installation. A comments/trailing-comma regression covers this boundary.
`Reporter::run` already rejects nonzero exits; an additional fake-Rokit case now
covers failure after a valid pin was written, without losing the failed outcome.
The final CodeRabbit rerun reviewed the same eleven files with zero findings.
Focused reruns passed four CLI fixtures and thirteen setup unit tests (one
Windows symlink-prerequisite case ignored); strict CLI/core Clippy passed again.
The reviewed runtime source is ready for head/main CI; delivery remains pending.

Each PR requires applicable formatting/tests/Clippy, portable packaging checks,
local CodeRabbit review, final diff inspection, reviewed-head and merged-main
CI, and merged-branch cleanup. No version bump, publication or installation.
