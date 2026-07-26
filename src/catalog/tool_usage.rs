//! How to actually *use* each tool rproj installs.
//!
//! The catalog's `description` says what a tool is in one line, which is
//! enough to decide whether to install it and not nearly enough to use it.
//! This is the missing half: what problem it solves, when you'd reach for
//! it, the commands you'd actually type, and the gotchas.
//!
//! Keyed by catalog key, plus a few `TOPICS` for things that aren't
//! installable tools (CI, the quality gate). Data-driven: `rproj info`
//! renders whatever is here.
//!
//! Every command below was checked against the tool's own `--help` on an
//! installed binary, not recalled - several of these CLIs have changed
//! subcommands over time.

pub struct Usage {
    /// Catalog key, or a topic name for the non-tool entries.
    pub key: &'static str,
    /// What problem this solves, in one sentence, assuming no prior
    /// knowledge of the Roblox tooling ecosystem.
    pub what: &'static str,
    /// When you'd reach for it during real work.
    pub when: &'static str,
    /// `(command, what it does)`. Ordered by how often you'd run them.
    pub commands: &'static [(&'static str, &'static str)],
    /// Things that bite people. Kept short; only genuine gotchas.
    pub notes: &'static [&'static str],
}

pub const USAGE: &[Usage] = &[
    Usage {
        key: "rojo",
        what: "Syncs code from files on disk into Roblox Studio, so you can use a real editor and real version control instead of writing scripts inside Studio.",
        when: "Constantly - it's the backbone of the whole workflow. Leave `rojo serve` running while you work.",
        commands: &[
            ("rojo serve", "Start syncing. Then hit Connect in Studio's Rojo plugin. Changes on disk appear in Studio live."),
            ("rojo build -o game.rbxlx", "Build the whole project into a place file without Studio - what CI uses."),
            ("rojo sourcemap -o sourcemap.json", "Write the file->instance map that luau-lsp needs for autocomplete."),
            ("rojo plugin install", "Install the Studio-side plugin (rproj does this for you)."),
        ],
        notes: &[
            "`serve` syncs disk -> Studio. Edits made *in* Studio are not written back; treat the files as the source of truth.",
            "default.project.json decides where each folder lands in the DataModel.",
        ],
    },
    Usage {
        key: "wally",
        what: "Package manager for Roblox, like npm or cargo. Downloads libraries listed in wally.toml into packages/.",
        when: "Whenever you want a third-party library and you picked the Wally workflow.",
        commands: &[
            ("wally install", "Install everything in wally.toml. Run after cloning, or after editing dependencies."),
            ("wally update", "Re-resolve to newer allowed versions and update wally.lock."),
            ("wally search <query>", "Find packages in the registry."),
        ],
        notes: &[
            "Commit wally.lock (like Cargo.lock) so everyone resolves identical versions.",
            "packages/ is generated - it's gitignored and rebuilt by `wally install`.",
            "Every install rewrites packages/ *without* type information, so a plain `wally install` \
             silently turns all your package types back into `any`. Follow it with wally-package-types \
             - or just use `rproj watch`, which does both.",
        ],
    },
    Usage {
        key: "wally-package-types",
        what: "Fixes the Luau types of Wally packages. Wally installs small redirect files that lose the real type information; this rewrites them so autocomplete and type checking work.",
        when: "Every time after `wally install` or `wally update`. `rproj new` and `rproj watch` both run it for you; you only need it by hand after installing manually.",
        commands: &[
            ("wally-package-types --sourcemap sourcemap.json packages", "Restore types in packages/ (needs a current sourcemap)."),
        ],
        notes: &[
            "Generate the sourcemap first - `rojo sourcemap -o sourcemap.json` - or it has nothing to work from.",
            "Not a one-off: the next `wally install` undoes it, because wally regenerates those redirect files from scratch.",
        ],
    },
    Usage {
        key: "selene",
        what: "A linter: reads your code without running it and flags mistakes and suspicious patterns (typos in globals, unused variables, shadowed names).",
        when: "Before committing, and in CI. The VS Code extension shows the same warnings inline as you type.",
        commands: &[
            ("selene src", "Lint everything under src/."),
            ("selene .", "Lint the whole project."),
        ],
        notes: &[
            "selene.toml's `std` must be \"roblox\" or it flags every Roblox global as undefined.",
            "Tune which lints are errors/warnings with `rproj configure selene`.",
        ],
    },
    Usage {
        key: "stylua",
        what: "An autoformatter: rewrites your code into one consistent style, so nobody argues about spacing and diffs stay clean.",
        when: "On save (the VS Code extension does this), and checked in CI.",
        commands: &[
            ("stylua src", "Reformat everything under src/ in place."),
            ("stylua --check src", "Report what isn't formatted without changing files - what CI runs."),
        ],
        notes: &[
            "`--check` exits non-zero when something needs formatting; that's how CI fails the build.",
            "Change the style with `rproj configure stylua`.",
        ],
    },
    Usage {
        key: "lute",
        what: "A standalone Luau runtime - runs Luau outside Roblox. rproj uses it as the task runner for the project's quality gate.",
        when: "To run the checks before you push, and to write project scripts in Luau instead of shell.",
        commands: &[
            ("lute run check", "Run .lute/check.luau - the whole quality gate (types, lint, format)."),
            ("lute test", "Run tests found in *.test.luau / *.spec.luau files."),
            ("lute run <script.luau>", "Run any Luau script. `lute <name>` resolves .lute/<name>.luau."),
            ("lute setup --with-luaurc", "Generate type definitions and point .luaurc at them."),
        ],
        notes: &[
            "`lute run check` and CI run the identical script, so a green local run means a green CI run.",
            "It is not Roblox - there's no `game` or `workspace`. It's for tooling, not gameplay code.",
        ],
    },
    Usage {
        key: "luau-lsp-cli",
        what: "The type checker behind the editor's autocomplete and red squiggles, as a command you can run in CI.",
        when: "In the quality gate. The VS Code extension gives you the same checking while editing.",
        commands: &[
            ("luau-lsp analyze --sourcemap=sourcemap.json --definitions=roblox.d.luau src", "Type-check src/ the way CI does."),
        ],
        notes: &[
            "Needs Roblox's API definitions to know what an Instance is; .lute/check.luau downloads them each run.",
            "Without a current sourcemap it can't resolve requires and reports false errors.",
        ],
    },
    Usage {
        key: "tarmac",
        what: "Manages images and other assets. Uploads them to Roblox and generates Luau code holding the resulting asset IDs, so you reference `Assets.logo` instead of pasting rbxassetid numbers around.",
        when: "Once you have real art. Skip it until then.",
        commands: &[
            ("tarmac sync --target roblox", "Upload changed assets and update the generated ID file."),
            ("tarmac sync --target none", "Dry run - see what would upload, without uploading."),
            ("tarmac upload-image foo.png --name \"Foo\"", "Upload one image and print its asset ID."),
            ("tarmac asset-list --output asset-list.txt", "List every asset the project needs."),
        ],
        notes: &[
            "Configured by tarmac.toml in the project root; rproj doesn't create one, since it depends on your asset layout.",
            "Uploading needs auth: an Open Cloud key via --api-key, or the cookie from an installed Studio.",
            "Uploaded images go through Roblox moderation and aren't usable until approved.",
        ],
    },
    Usage {
        key: "mantle",
        what: "Infrastructure-as-code for Roblox places: describes your experience (places, badges, passes, icons, configuration) in a file so deploying is a repeatable command instead of manual clicking in the website.",
        when: "When you have a real published game with staging/production environments. Overkill for a first project.",
        commands: &[
            ("mantle diff", "Show what deploying would change - always run this first."),
            ("mantle deploy", "Apply your configuration to the target environment."),
            ("mantle outputs", "Print the resulting IDs (place IDs, badge IDs) for scripts to consume."),
            ("mantle destroy", "Tear the environment down. Destructive."),
        ],
        notes: &[
            "Unmaintained: the author states not to expect fixes or responses. Still the most complete option, but you own any problems.",
            "It keeps a state file mapping your config to real Roblox resources - losing it means Mantle no longer knows what it manages.",
            "Needs an Open Cloud API key.",
        ],
    },
    Usage {
        key: "hoarcekat",
        what: "A Studio plugin for previewing one UI component on its own, without playing the game to reach it.",
        when: "While building UI. Write a `.story.luau` next to a component and it shows up in the Hoarcekat panel.",
        commands: &[],
        notes: &["A story is a module returning a function that mounts your component into a given parent frame."],
    },
    Usage {
        key: "testez",
        what: "A test framework. You write files like `foo.spec.luau` describing what your code should do, and TestEZ runs them and reports which expectations held.",
        when: "Once you have logic worth protecting from future changes - data handling, gameplay rules, anything subtle.",
        commands: &[
            ("lute test", "Run tests outside Roblox, in CI and locally."),
        ],
        notes: &[
            "Tests need a Roblox DataModel for anything touching Instances, which is what the TestEZ Companion plugin is for - it runs them inside Studio.",
            "rproj sets selene's std to \"roblox+testez\" when you select it, so describe/it/expect aren't reported as undefined globals.",
            "Archived by Roblox in Sept 2024. Still the most widely used option, but it isn't gaining features.",
        ],
    },
    Usage {
        key: "testez-companion",
        what: "Runs your TestEZ tests from inside VS Code and shows pass/fail inline, by talking to a companion plugin running in Studio.",
        when: "While writing tests, so you don't have to switch to Studio and press play to see results.",
        commands: &[
            ("testez-companion.buildPlugin", "VS Code command palette: builds the Studio plugin. Save it into Studio via 'Save as Local Plugin'."),
            ("Ctrl+;", "Run the tests and show results."),
        ],
        notes: &[
            "Needs testez-companion.toml listing which DataModel paths hold .spec files - rproj writes one matching the tree it scaffolds.",
            "Rojo must be syncing, since the plugin runs the tests against what's actually in Studio.",
            "A root that doesn't match your tree finds no tests, which looks the same as everything passing.",
        ],
    },
    Usage {
        key: "roblox-ui",
        what: "Adds an explorer panel to VS Code showing your project as the Roblox instance tree it becomes, rather than as files.",
        when: "Whenever you're unsure where a file will end up in the DataModel.",
        commands: &[],
        notes: &["It reads the Rojo sourcemap, so it's only as current as the last sourcemap generation."],
    },
    Usage {
        key: "github-actions",
        what: "Shows your CI runs and their logs inside VS Code, and gives completion and validation when editing workflow YAML.",
        when: "After pushing, to see whether the quality gate passed without leaving the editor.",
        commands: &[],
        notes: &["See `rproj info ci` for what the generated workflow actually runs."],
    },
    Usage {
        key: "rokit",
        what: "Installs and pins the versions of the command-line tools above, per project, so everyone on a project (and CI) runs identical versions.",
        when: "Whenever you add a tool or clone a project.",
        commands: &[
            ("rokit install", "Install every tool pinned in rokit.toml. Run this after cloning."),
            ("rokit add <owner>/<repo>", "Add a tool to this project."),
            ("rokit add --global <owner>/<repo>", "Make a tool available everywhere, not just this project."),
            ("rokit update", "Update pinned tools to newer versions."),
            ("rokit authenticate github", "Provide a GitHub token to raise the 60-request/hour API limit."),
        ],
        notes: &[
            "It finds rokit.toml by walking up from the current directory, so tool versions depend on where you are.",
            "Hitting \"rate limit\" errors while installing? That's the 60/hour unauthenticated GitHub limit - `rokit authenticate github` fixes it.",
        ],
    },
];

/// Concepts that aren't installable tools but that a newcomer still has to
/// understand to work in a scaffolded project.
pub const TOPICS: &[Usage] = &[
    Usage {
        key: "ci",
        what: "GitHub Actions runs your quality gate automatically on every push and pull request, on GitHub's machines. The config lives at .github/workflows/ci.yml.",
        when: "Automatic once the project is on GitHub - you don't run it yourself.",
        commands: &[
            ("git push", "Triggers a run. Results appear in the repo's Actions tab and on the PR."),
            ("lute run check", "The same checks, locally - run this before pushing to avoid a red build."),
        ],
        notes: &[
            "The workflow installs the exact tool versions from rokit.toml, so CI matches your machine.",
            "It checks out submodules too, since vendored packages are part of the build.",
            "It never rewrites your code: formatting is checked with --check, not applied.",
        ],
    },
    Usage {
        key: "check",
        what: "The project's quality gate: one script (.lute/check.luau) that type-checks, lints, and verifies formatting. rproj generates it from the tools you installed.",
        when: "Before every commit, and automatically in CI.",
        commands: &[
            ("lute run check", "Run all of it. Exits non-zero if anything failed."),
            ("stylua src", "Fix the formatting failures it reported."),
        ],
        notes: &[
            "Every check runs before it exits, so one run shows you every problem rather than only the first.",
            "CI runs this same file, so green locally means green in CI.",
        ],
    },
];

pub fn find(key: &str) -> Option<&'static Usage> {
    USAGE.iter().chain(TOPICS).find(|u| u.key == key)
}
