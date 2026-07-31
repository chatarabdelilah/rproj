# rproj — the project graph

## 0. The sentence

> **rproj asks for architectural decisions about your project, derives every implementation detail from them, and always shows its work.**

Every feature has to satisfy it. It is not decoration — it decides things:

| Question | Verdict |
| --- | --- |
| Should `rproj new` ask about `.gitignore`? | No. Not architectural. Derived, and shown on the summary with its reason. |
| Should it ask which dependency workflow? | Yes. Architectural — with a recommendation, because a default does not make a question fake. |
| Should it ask "pin Selene?" | No. That is a consequence of wanting linting. |
| Should it ask what kind of project this is? | Yes, once there is more than one answer. |

"About your project" is load-bearing. Without it the sentence reads as licence to derive the *toolchain* too, which §1 of `plan.md` rejects: rproj orchestrates Rojo, Wally, Selene and the rest — it does not reimplement them. It derives your project's implementation, never the ecosystem's.

"Shows its work" is an obligation, not a nicety. It is why the dependency prompt carries descriptions and why every generated file is listed with a reason. A derived detail the user cannot see is indistinguishable from magic, and this tool exists to teach the ecosystem, not to hide it.

---

## 1. What was actually wrong

The received diagnosis was "the prompts are in the wrong order." That is a symptom. The defect is that **rproj asks one decision at three abstraction levels**, and never at the level the user thinks in:

| Level | What the user thinks | Did rproj ask it? |
| --- | --- | --- |
| **Intent** | "I want my code linted" | never |
| **Implementation** | `selene` | yes — "Tools to pin" |
| **Artifact** | `selene.toml` | yes — "Files to generate" |

The user only ever wants to answer the top row. The other two are derivable from it. rproj asked the bottom two and never the top, so the user had to reverse-engineer their own intent into tool names and filenames — twice — and any disagreement between those two answers became a contradiction the CLI then had to patch.

That patching is visible in the code. `entailed_by` (v0.4.0) reports artifacts an earlier answer already decided, with a reason, instead of offering them. It is the right fix for the model as it stood and the wrong model: it makes the contradiction *legible* rather than making it unrepresentable. The tell is the "always settled" state in `rproj info` — four artifacts whose entailment condition is also their requirement, i.e. entries with no independent existence. Those four are the model reporting that it has a level too many.

### The seven concrete failures

1. **Packages asked before dependency workflow, and the workflow then silently overruled.** `pick_package_workflow` inspects the selection, finds React (npm-only upstream), forces Wally, prints a note. The one place the user's architectural intent should be sovereign was the one place rproj overrode it without asking.
2. **Workflow chosen, then "pin Wally?"** Untick it and the scaffold still runs `wally install` from rokit's global manifest — so the answer was not merely contradictory, it was *inert*. The project just failed to record the version of the tool it depends on.
3. **"Tools to pin" was two questions in one costume.** *Do I want this project linted* (a workflow decision) and *should the version be pinned for teammates* (a reproducibility decision) have different answers, different audiences and different defaults.
4. **Testing asked twice, four prompts apart.** `testez` in the package step; `tests/` as a checkbox in the files step; `testez.yml` and `testez-companion.toml` as further consequences.
5. **CI, Blender, Figma and Tarmac presented as files.** Nobody thinks *"I want a `.github/workflows/ci.yml`"*.
6. **`--like <setup>` skipped one prompt and still asked three.** Reuse should mean reuse.
7. **`none` sits last in every guided category**, and `Select` highlights index 0 — a package. Enter-through hands a beginner five packages they never chose. The safe answer must be the resting position.

---

## 2. The model: a graph, not a wizard

The CLI is not asking questions. It is **constructing a model**, and each answer adds a node:

```
Project
├── Project Type          Game | Package | Studio plugin | Empty
├── Dependency Strategy   Wally | Git submodules | None
├── Packages              what code this project depends on
├── Capabilities          what workflows this project supports
│     └── each capability:
│           ├── Implementation   the thing that provides it
│           ├── Tools            what gets pinned
│           ├── Artifacts        what gets written
│           └── Commands         what you then run
└── Summary               the tree, rendered
```

Three consequences follow immediately, and they are the reason this framing is worth the rewrite:

**The summary is not a screen.** It is the tree rendered. It cannot drift from what gets created, because it is the same data the scaffolder walks.

**Revision is invalidation, not navigation.** Changing `Dependency Strategy` invalidates `Packages` (some are unvendorable), which invalidates `Summary`. Everything else stands. That is a far better mental model than "go back three screens" — and it is what makes a "change something" affordance implementable at all.

**Persisting the tree gives three features for free.** `rproj.toml` recording *decisions* rather than *outcomes* means `rproj upgrade` re-derives from intent (so a changed default reaches old projects), `--like` replays a tree straight to the summary with zero questions, and a saved setup captures the whole composition rather than a package list.

### One unit of choice per level

Nothing in the tree above is a tool, a file, a config, or a pin. Those are all derived. The user's decisions are exactly the labelled nodes, and each sits at a level the one above determines.

### Packages vs Capabilities

The distinction is **not** runtime vs development-time — that breaks immediately, since TestEZ is a development dependency and still a package. It is:

| | Answers | Examples |
| --- | --- | --- |
| **Packages** | "What code does my project depend on?" | Reflex, Promise, React, TestEZ |
| **Capabilities** | "What workflows does my project support?" | Linting, Formatting, Testing, CI, Asset pipeline |

**Testing is a capability. TestEZ is one implementation of it.** That resolves failure ④ structurally rather than by reordering: Testing leaves the package picker, which drops the guided path from five category prompts to four — by principle, not by trimming.

---

## 3. The capability graph

A capability is one node with four derived children:

```
Capability      Testing
    ↓
Implementation  TestEZ            (or jest-lua, or none)
    ↓
Requires        the testez package, which requires Wally
    ↓
Artifacts       tests/, tests/.luaurc, testez.yml, testez-companion.toml
    ↓
Commands        lute test
```

Swap the implementation and everything below re-derives. The capability never changes.

### The rule this gives for free

**An implementation prompt appears only when a capability has more than one implementation.** Same rule as everywhere else — never ask a question with one answer — and it says exactly when a new prompt is permitted:

| Capability | Implementations today | Sub-prompt? |
| --- | --- | --- |
| Linting | Selene | no — renders as `Linting (Selene)` |
| Formatting | StyLua | no |
| Type checking | luau-lsp | no |
| Quality gate | Lute | no |
| Continuous integration | GitHub Actions | no |
| Asset pipeline | Tarmac (Figma as source) | no |
| 3D assets | Blender | no |
| **Testing** | **TestEZ, jest-lua, none** | **yes — the first and only one** |

M4 (jest-lua) is therefore the first milestone that adds a prompt, and it adds it because the model says it may — not as a special case. This is the answer to the open question in `plan.md` §11 about how a second test runner should be offered.

### Implementations are not all the same kind of thing

TestEZ is a Wally package. Selene is a rokit tool. GitHub Actions is neither — it is a hosted service the artifact targets. The implementation node points at whichever, so the catalog stays a flat inventory and capabilities reference into it.

---

## 4. The flow

```
0.  Machine provisioning     first run only, unchanged
1.  Project type             Game | Package | Studio plugin | Empty     [see §6 — gated]
2.  Guided or Expert         different interaction models, not defaults
3.  Dependencies             Wally ● recommended | Git submodules | None
4.  Packages                 1 prompt expert, 4 guided
5.  Capabilities             lint (Selene), format (StyLua), CI (GitHub Actions)…
6.  Summary                  the tree, every line reasoned, anything revisable
```

**Ordering rationale, where it is not obvious:**

- **Dependencies before packages** is failure ①. Choosing submodules after picking React is what let rproj silently overrule the architecture. It is beginner-safe now in a way it was not before: pre-selected, marked recommended, one keystroke, and its descriptions are the only place a newcomer will ever be told what Wally *is*.
- **Under submodules**, unvendorable packages are shown **greyed with the reason**, never hidden — hiding makes the user wonder where React went. Selecting one offers a *forward correction*: "react needs Wally — switch this project to Wally?" The same fact rproj already knows, turned from a silent override into an explicit revision of a decision the user made knowingly.
- **Capabilities after packages**, because a package can imply one (picking a UI library makes the mixed-table lint waiver relevant).

### The summary

```
  MyGame

  Type          Game
  Dependencies  Wally                                        [change]
  Packages      reflex, promise                              [change]
  Does          Linting (Selene), Formatting (StyLua),
                Type checking (luau-lsp), CI (GitHub Actions)  [change]

  Creates
    src/, default.project.json    every Rojo project has these
    wally.toml                    reflex + 1 other install from it
    rokit.toml                    pins 4 tool versions so teammates get the same ones
    selene.toml                   you chose Linting
    stylua.toml                   you chose Formatting
    .lute/check.luau              you chose the quality gate
    .github/workflows/ci.yml      you chose CI
    .gitignore, .gitattributes, .luaurc, rproj.toml

  › Create it        Change something
```

Every file carries its reason. Files are the one thing a beginner will actually open and edit, so the summary is the last chance to say what each is for — the same obligation the dependency prompt's descriptions discharge, one level down.

---

## 5. What disappears

| Prompt | Fate | Why |
| --- | --- | --- |
| **Tools to pin** | deleted — derived from capabilities | Never a question about versions. The genuine reproducibility question becomes `--no-pin` plus one explanatory line on the summary. |
| **Files to generate** | deleted — becomes the summary | Every entry is implied by a capability or a strategy. The picker survives only behind "Change something". |
| **`tests` checkbox** | deleted — folded into the Testing capability | Failure ④. |
| **The Wally-forcing note** | deleted — becomes a forward correction | Same fact, offered before the decision rather than announced after it. |

And `entailed_by` shrinks: with artifacts owned by capabilities, the per-entry `requires`/`entailed_by` pair collapses to `provided_by`. Entailment survives only for genuine cross-level implications (strategy → `wally.toml`) — a handful of edges instead of a field on every entry.

**Honest accounting: this is 8 questions on the guided path against today's 9.** The redesign's value was never the count. It is that no prompt asks about a consequence, every prompt names its implementation, and the two that vanished were the two asking the user to think at the wrong level.

Which redirects the compression question to where it belongs: the **four remaining guided package prompts** are now the largest block, and most are "none" for a beginner — with `none` currently last in each list (failure ⑦). That is the part worth optimising, not the architectural questions.

---

## 6. Rejected alternatives

Recorded because the reasoning for *not* doing these is the part that gets lost.

**Hide the dependency strategy behind `--submodules`.** Technically correct — Wally is the answer for anyone who does not already know otherwise — and rejected. rproj exists partly to teach the ecosystem, and a summary line reading `✓ via Wally` is a receipt, not an explanation: it arrives after the decision, so nothing is learned. A prompt with a recommendation costs one keystroke and is the only place a newcomer meets the concept. **A default does not make a question fake.**

The distinction that makes this compatible with "never ask about a consequence": a *fork* with real live alternatives is a decision; a *consequence* of a decision already made is not. "Pin Wally?" after packages is the second kind. "How should dependencies work?" is the first.

**Replace Guided/Expert with an `--expert` flag.** Rejected: they are different *interaction models* (category-by-category vs search-anything), not different defaults. Revisit only if the guided path gets short enough that what experts skip stops being worth a prompt. `--expert` should exist regardless, so a repeat user does not answer it daily.

**Ship Project Type now.** Rejected as premature — see §7. A prompt reading `What are you building? [Game] [Empty]` is a question whose answer is predetermined, which is the exact defect being removed, relocated to the top of the flow.

**Presets / archetypes.** Already cut in `plan.md`, not reopened. Worth stating the cost, which is paid entirely by the first-time user: they must assemble a composition before knowing what the pieces do. If ever revisited, the cheap version is not a curated catalog — it is `--like` seeded from your own last project, which needs no curation and never goes stale.

---

## 7. Project Type is a feature, not a prompt

It is the layer the original sketch called "Architecture" and left empty, and it is genuinely not a preset: a preset bundles package choices, while project type determines *structure* — tree shape, build target, whether a place file exists at all.

Three of its four values do not exist yet. `scaffold_project_json` takes no type parameter and the place template is unconditional:

| Type | State today |
| --- | --- |
| **Game** | done |
| **Empty** | nearly free — skip the place template and the client/server split |
| **Package** | ~60% — `wally.toml` already writes `[package]` metadata (name/version/registry/realm); needs a single-module project file, no place template, and a `wally publish` story |
| **Studio plugin** | mostly new — different build target and install path |

So the prompt is gated on the build targets being real. Adding it before then would break the rule this document exists to enforce.

---

## 8. Sequencing

| | Work | Depends on | Notes |
| --- | --- | ---: | --- |
| **R1** | Capability catalog; collapse "tools" + "files" into capabilities + summary; `provided_by` replaces the `requires`/`entailed_by` pair | — | Pure re-levelling of logic that already exists |
| **R2** | The tree as a real type: ordered nodes, invalidation on edit, `rproj.toml` stores decisions | R1 | Unlocks "change something", and re-derivation for `upgrade`/`--like` |
| **R3** | `rproj info <capability>` pages; `rproj configure` hints on the summary | R1 | The "shows its work" half |
| **R4** | Project type — after Package and Studio-plugin build targets exist | new scaffolding | §7 |

R1 makes **M4 (jest-lua) cheaper**: "which test runner" stops needing its own gate step and becomes the implementation node of the Testing capability.

### One decision this reopens

`plan.md` §11 defers `rproj-core` on the grounds that *"one front-end means extraction is cost without benefit, and it would forfeit the dead-code guarantee."* If the tree is the core, that reasoning weakens: the tree earns its keep from revision and upgrade alone, before any second frontend exists. The CLI, a future TUI and a GUI would all be frontends rendering the same project graph.

Not reversed here — flagged for revisit once R2 lands and the tree's real shape is known rather than sketched.
