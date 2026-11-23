# Native Machine Setup proposal

## Native implementation approval — October 24, 2026

The owner approved this design against clean main `a56533a` and three sequential
reviewed PRs: shared planning/checked execution, private cooperative-stop worker,
then native Soft controls. Preserve the terminal catalog, selection/persistence
semantics and installation order. Stop finishes the active item; it must not use
the existing process-killing cancellation path. Native previews and real
installations remain paused. Validation uses isolated storage and fake installers;
no version bump, publication, OS troubleshooting or VM provisioning is included.

Approved design scope, October 23, 2026, against clean main `3519bab`:
an independent Soft conversation wireframe for **Selections → Review → Results**.
This approves the prototype scope, not native implementation or visual acceptance.

## Settled behavior

Machine Setup is an app-level action below project navigation, also reachable
from New Project's prerequisite explanation. It retains one machine-owned draft
across browsing, separately from creation, composition, settings and Upgrade.
The projects folder is read-only. Six existing catalog categories retain their
selection rules. Unknown choices survive without execution; unchecking never
uninstalls. Recorded choices and detected software are different facts.

Review includes required Rokit, selected apps, projects-folder creation, tools,
plugins/add-ons and extensions in executor order. Apply defaults to Keep reviewing.
Any observed Watch blocks Apply until the separate Stop control acknowledges
completion. Setup never stops or restarts Watch automatically.

Stop waits for the active item. Close defaults to Keep open; Stop and close waits
for confirmed termination, then protects other dirty drafts. Completed software
can remain. Successful and warning runs save selections; fatal/stopped runs do not.
Configuration-save failure is distinct from installation failure. Retry returns
to Review for explicit Apply. Unconfirmed termination keeps the sample open.

## Boundaries

Only sample data and simulated actions: no installation, real project access,
Rust, API, worker, persisted-format, version or release changes. Previous working
wireframes stay unchanged. Native previews remain paused because memory stability
is unresolved. Native implementation follows owner design approval.

Validation and delivery evidence: [acceptance](desktop-machine-setup-acceptance.md).

## Conversation prototype

The separate `rproj-machine-setup-wireframe.html` starts on useful sample
Selections, with complete fallback content before optional host helpers or icons.
All 36 existing setup catalog entries retain descriptions, documentation links
and source identifiers. Search changes visibility, not selections. Details sit
beside the list at 600 pixels of main-content width and stack below; the shell
also reflows below 620 pixels. Light/Dark/System uses the approved Soft palette,
platform font, labeled Lucide placeholders and visible focus.

The root path uses the existing configured/default behavior; setup does not
offer a folder editor. Existing/absent folders are separate sample states.
Detected software is labeled as sample evidence or not verified. Blender and
VS Code dependent selections remain retained while inactive. Manual plugin
instructions stay manual rather than becoming false installation successes.

Activity retains machine ownership and bounded output across project browsing
and closure. Start and new failures expand it; ordinary output and successful or
manual-only completion respect collapse. Its height and search remain independent
of navigation. Another operation blocks Apply; a running setup freezes submitted
selections. A changed configuration before Apply requires explicit reload, and a
change during installation refuses the final selection save in the simulation.

Simulation controls expose fresh/returning and missing/malformed/unreadable
configuration, long paths, missing folders, Watch conflicts, another operation,
other dirty drafts and outcome modes. They are test controls, not proposed native
product controls. Unconfirmed termination needs a separate simulated acknowledgement;
it never offers an immediate force close. Explicit reload is still required before
retry because the file state remains uncertain. Completion racing a close dialog
does not bypass default-Keep or other-draft protection.

Surrounding project pages and other drafts are intentionally lightweight sample
stand-ins; existing native workflows are not reproduced or changed. The New Project
shortcut demonstrates a retained draft and machine-level navigation. The sample
folder picker demonstrates cancellation only; it does not access the filesystem.

## Next implementation boundary

After owner design approval, plan shared machine selection/planning, a private
owned execution boundary and native controls separately. Existing terminal Stop
waits for its active installer, and warning runs may persist selections despite
item failures. These constraints must not become false success or rollback claims.
The proposed any-Watch guard and retained desktop draft are design decisions,
not assertions that the terminal already enforces them. No implementation approval
or Windows memory-stability acceptance is implied by this prototype.
