//! Pins the working-tree line endings a scaffolded project checks out with.
//!
//! Without this, a scaffolded project fails its own quality gate the moment
//! anyone clones it on Windows - in a tree nobody has touched.
//!
//! The chain: StyLua formats to Unix line endings by default, the repo
//! stores LF, and Git for Windows ships `core.autocrlf=true`, which
//! converts to CRLF on checkout. `stylua --check` then reports a diff for
//! every single file. Reproduced by cloning a scaffolded project fresh:
//! `CRLF=5, bare LF=0` in a three-line starter file, and `stylua --check
//! src tests` exiting 1 across the whole tree.
//!
//! Setting `line_endings = "Windows"` in stylua.toml would be the wrong
//! fix - CI runs on Linux, where a checkout is always LF, so it would just
//! move the failure from the contributor's machine to the build. Forcing
//! LF in the working tree is what makes local and CI agree.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::ui;

/// `eol=lf` for text, explicit `binary` for the formats a Roblox project
/// keeps in git. `text=auto` does guess at binary-ness, but the guess is
/// content-based and a corrupted `.blend` or place file is a bad thing to
/// leave to a heuristic.
const GITATTRIBUTES: &str = "\
# StyLua formats to Unix line endings and CI checks that on Linux, where a
# checkout is always LF. Git for Windows defaults to core.autocrlf=true and
# would otherwise hand a Windows clone CRLF files, failing `stylua --check`
# on every file in a project nobody has touched.
* text=auto eol=lf

# Never line-ending-convert these.
*.blend binary
*.blend1 binary
*.rbxl binary
*.rbxlx binary
*.rbxm binary
*.rbxmx binary
*.png binary
*.jpg binary
*.ogg binary
*.mp3 binary
";

pub fn ensure_gitattributes(project_dir: &Path) -> Result<()> {
    let path = project_dir.join(".gitattributes");
    if path.exists() {
        ui::ok(".gitattributes already exists");
        return Ok(());
    }
    fs::write(&path, GITATTRIBUTES)
        .with_context(|| format!("failed to write {}", path.display()))?;
    ui::ok("wrote .gitattributes");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The whole point is that a Windows checkout gets LF. Losing this line
    /// puts every fresh clone back to a failing `stylua --check`.
    #[test]
    fn forces_lf_in_the_working_tree() {
        assert!(GITATTRIBUTES.contains("* text=auto eol=lf"), "{GITATTRIBUTES}");
    }

    /// Line-ending conversion inside a .blend or a place file corrupts it.
    #[test]
    fn binary_formats_are_excluded_from_conversion() {
        for ext in ["blend", "rbxl", "rbxlx", "rbxm", "rbxmx", "png"] {
            assert!(
                GITATTRIBUTES.contains(&format!("*.{ext} binary")),
                "*.{ext} must be marked binary:\n{GITATTRIBUTES}"
            );
        }
    }
}
