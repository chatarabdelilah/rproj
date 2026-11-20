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

The suite covers static startup, version and alias rules, pin mismatches, source invalidation, Watch acknowledgement, captured Activity, focus, close races, theme contrast and the 600-pixel layout rule. Inline rendering and real-window behavior remain unverified. Before native implementation, independently validate isolated Wally resolution and reproducibility against the pinned tool, and Rokit installation against disposable project fixtures.
