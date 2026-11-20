# Review one project update

The [Soft proposal](prototypes/review-update.html) extends the [on-demand Updates report](desktop-update-discovery-design.md). Open it on Skyline, choose a Signals release and select **Prepare review**. The sample compares the captured declaration and lock entries before enabling **Apply update…**. All observations, file changes and operation outcomes are local fixtures.

## Package and tool decisions

| Selection | What the review shows | When Apply is available |
| --- | --- | --- |
| Signals 1.3.0 | The existing caret requirement still fits. Review the selected lock entry and the transitive change to sample/task. | Other direct dependencies retain their versions in every realm. |
| Signals 2.0.0 | The declaration must change. Review the old and new requirements along with the lock changes. | The sample can express the new caret or exact requirement. A compound requirement offers only a release it already admits. |
| Rojo 7.5.0 | Review the exact repository and version in the project pin, plus installation and cache effects. | The prepared pin matches and every unrelated pin stays intact. Detection on PATH is displayed separately. |

**Sample scenarios** includes two collateral-change cases. One changes a different server package. The other changes a dev alias for the selected package. Both show the affected declaration and block Apply. Changes to transitive packages are visible and reviewable. Current, local-ahead, unknown, unsupported and stale observations each explain why preparation is unavailable.

## Operation behavior

The confirmation names the project, path, target version and effects. **Keep reviewing** receives initial focus. Same-project Watch must acknowledge Stop before Apply. Lantern can keep watching while Skyline updates. Changed inputs invalidate the prepared review and require a fresh check.

Activity labels output with the captured project and target across navigation. Verified success updates the sample observation. A failed execution warns that files may have changed and requires fresh preparation. **Close after update** waits for confirmed completion. An uncertain result keeps the sample open until the termination fixture supplies a confirmed outcome.

## Acceptance

Run the deterministic DOM checks with an existing jsdom installation:

```powershell
node docs/prototypes/review-update.test.cjs target/new-project-design/dom-runtime/node_modules/jsdom
```

The suite covers static startup, version and alias rules, pin mismatches, source invalidation, Watch acknowledgement, captured Activity, focus, close races, theme contrast and the 600-pixel layout rule. Inline rendering and real-window behavior remain unverified.

## Released-tool proofs

The owner approved desktop Apply with a private worker and protection for local files. The public discovery command stays read-only. Run each live proof separately with Python 3.11 or later and the executables on PATH. The flags `--wally`, `--rojo`, `--wally-package-types` and `--rokit` also accept absolute executable paths.

```powershell
python scripts/prove-project-updates.py wally
python scripts/prove-project-updates.py rokit
```

Both ran successfully on Windows on November 20. Wally 0.3.2 resolved Knit 1.5.3 → 1.6.0 in a temporary project. Independent installs reproduced the reviewed lockfile bytes and generated package files with Rojo 7.7.0 and wally-package-types 1.7.0. Comm changed transitively. A second direct dev alias also moved in the two-alias fixture. Comparing the old installation to a clean reconstruction identified the edited link and the unfamiliar file. The probe downloads public packages and Wally uses its normal registry index cache.

Rokit 1.2.0 installed Rojo 7.7.1 under a custom alias in separate tool storage. The executable in the exact version directory reported 7.7.1. A scratch manifest allowed normal trust checks without force-overwriting an existing alias. The source project's other pin and comment survived the reviewed edit. Only the fixture's storage received explicit trust. These probes supply external-tool evidence independently of offline worker tests.

## Matching worker verification

The private update worker prepares one observed selection. Apply repeats the preparation under the project Watch mutation guard and checks the captured inputs and proposed output. A distinct installed package directory may be absent. An edited or unfamiliar path in an existing installation blocks replacement. Wally staging uses a JSON Rojo project with project-relative mounts. Unsupported mount layouts explain why a review cannot be prepared.

Run `python scripts/prove-reviewed-update-worker.py target/debug/rproj.exe wally` or substitute `rokit`. These live checks exercise refusal and confirmed execution in Unicode-named disposable folders. The first fixture run exposed Windows default-codepage decoding of its JSON report and a missing isolated Rokit directory. Both corrected fixtures passed, including a stale manifest between Prepare and Apply.

After files move, a failure keeps the recovery directory and identifies its path. The worker rechecks displaced originals and refuses to overwrite a destination created by an intervening save. Reports and prerequisite version probes have size limits. The initial combined local CLI/core run timed out in the existing Watch flood fixture. That fixture passed in isolation on the same commit, followed by the remaining suites. Environment-dependent ignored tests remain unverified.

On Windows, prerequisite executables stay open against writes or replacement during verification and launch. The worker compares the installed tool with a fresh download in temporary storage after the ordinary trust-checked install. The disposable Rokit fixture supplies a wrong executable at the expected cache path and checks that the project pin stays intact.

## Native workbench

From **Dependencies** or **Tools**, choose **View updates…**, then **Check for updates**. Select an observed release and choose **Prepare review**. Review lists blockers first and pages through every proposed effect. **Apply update…** confirms the captured project, alias and release. A failed check invalidates eligibility until another check succeeds. The matching worker rechecks the review under the Watch mutation guard before applying it.

Headless controls exercise default-Keep focus, a Watch change after confirmation opens, retained Activity ownership after navigation, uncertain completion during close and a 51-change review navigated through all seven pages. The pagination test initially expected an off-screen control to be visible. It now scrolls the project pane and exercises the buttons. Real-window interaction remains unverified.

The local desktop suite passed all 198 tests when run serially with access for the disposable Windows junction fixture. An earlier parallel sandbox run failed the existing project-tool layout and junction fixtures. Strict desktop Clippy passed. Portable packaging checks the matching binaries and bundled notices; two legacy notice omissions remain recorded in the distribution.
