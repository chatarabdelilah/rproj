# rproj 0.23.2 — public alpha

- New Project checks prerequisites before claiming the destination folder.
  When tools are missing, set them up and resume at the same destination.
- Wally composition restores empty mount directories before generating the
  Rojo sourcemap so they are included in the project layout.
- The workspace lockfile updates dependencies flagged by RustSec. It also
  updates the test-only PTY dependency and preserves the Windows terminal
  tests through the ConPTY cursor handshake.

The CLI and core library use matching version 0.23.2. The portable Windows
desktop build is available through CI artifacts.

The dependency audit reports zero vulnerabilities and zero soundness
warnings. Nine maintenance warnings remain in the desktop dependency tree.
