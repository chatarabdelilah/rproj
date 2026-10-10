rproj desktop - Windows portable alpha

This archive is an unreleased development build. Its package version matching
a published CLI version does not make this desktop archive a published release.

1. Extract the entire ZIP into a folder you can keep. Rust is not required.
2. Open rproj-desktop.exe, then choose an existing project folder.
3. Keep rproj.exe beside the desktop. It runs the matching project operations.

The direct CLI remains available: rproj.exe inspect "project folder" --json.
rproj.exe app opens the desktop. Bare rproj.exe retains its terminal interface.

Watch is a persistent background sourcemap task. Closing the app does not stop
Watch. Use the desktop Stop control, or rproj.exe watch stop. Only one project
can own the background Watch at a time. Switching waits for Stop acknowledgement.

Opening and inspecting a project does not install tools, update dependencies,
rewrite configuration, or create rproj.toml. Running Watch, Check, or Test uses
your ordinary external tools and project configuration. Configure the required
tools separately; this ZIP does not install Blender, Roblox Studio, or other
machine applications.

Preferences and recent projects use the existing per-user rproj configuration
directory in a separate desktop.json file. Project configuration stays in its
ordinary project files. This alpha does not store API credentials.

build.json identifies both executable hashes and the source commit. A dirty
source flag means this is a local development build, not a published release.
LICENSE is rproj's license. THIRD-PARTY.md, dependencies.json, and licenses/
preserve dependency declarations, authors, available notices and provenance.
Read THIRD-PARTY.md for any explicitly recorded legacy notice omissions;
successful build/smoke checks do not certify legal-notice completeness.

Support and documentation: https://github.com/chatarabdelilah/rproj
