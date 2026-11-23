# rproj 0.23.1 — public alpha

On Windows x64, Machine Setup downloads the pinned Rokit 1.2.0 executable,
verifies the archive's SHA-256, and runs self-install. This removes the need
to compile Rokit with Cargo. Setup can use the newly installed Rokit and
Rojo in the same session.

If the required Visual C++ runtime is missing, Setup shows the recovery
command. Run it, then retry `rproj setup`.

```powershell
winget install --id Microsoft.VCRedist.2015+.x64 --exact --source winget
```

The Windows first-install test verified Git 2.56.1.windows.1, Rokit 1.2.0
and Rojo 7.7.1 in a fresh PowerShell after installing the runtime. A separate
regression test covers the missing-runtime diagnostic.

The lockfile also refreshes two previously yanked transitive dependencies.
The CLI and core library use matching version 0.23.1. The portable desktop
build remains available through CI artifacts.
