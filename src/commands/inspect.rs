use std::io::Write;
use std::path::Path;

use anyhow::Result;
use rproj_core::{DiagnosticSeverity, PresenceState, ProjectSnapshot, ResolutionState};

pub fn run(path: Option<&Path>, json: bool) -> Result<()> {
    let current;
    let path = match path {
        Some(path) => path,
        None => {
            current = std::env::current_dir()?;
            &current
        }
    };
    let snapshot = rproj_core::inspect(path)?;
    let mut output = std::io::stdout().lock();
    if json {
        serde_json::to_writer_pretty(&mut output, &snapshot)?;
        writeln!(output)?;
    } else {
        render(&snapshot, &mut output)?;
    }
    Ok(())
}

fn render(snapshot: &ProjectSnapshot, output: &mut impl Write) -> Result<()> {
    writeln!(output, "Project: {}", snapshot.root.display())?;
    if let Some(name) = &snapshot.name {
        writeln!(output, "Name: {name}")?;
    }
    writeln!(output, "Kind: {:?}", snapshot.kind)?;
    writeln!(output, "Configuration:")?;
    for file in [
        &snapshot.files.rproj,
        &snapshot.files.rojo,
        &snapshot.files.wally,
        &snapshot.files.wally_lock,
        &snapshot.files.rokit,
        &snapshot.files.check_script,
    ] {
        let path = file.path.strip_prefix(&snapshot.root).unwrap_or(&file.path);
        writeln!(output, "  {}: {:?}", path.display(), file.state)?;
    }
    writeln!(output, "Packages: {} declared", snapshot.packages.len())?;
    for package in &snapshot.packages {
        let lock = match package.resolution {
            ResolutionState::Resolved => "resolved",
            ResolutionState::Unlocked => "unlocked",
            ResolutionState::Stale => "stale lock",
            ResolutionState::Unknown => "unknown resolution",
        };
        let presence = match package.presence.state {
            PresenceState::Present => "alias and index present",
            PresenceState::Missing => "package files missing",
            PresenceState::Unknown => "package files unknown",
        };
        let version = package
            .resolved
            .as_ref()
            .map(|resolved| format!(" {}", resolved.version))
            .unwrap_or_default();
        writeln!(
            output,
            "  {} [{}]: {} ({lock}{version}; {presence})",
            package.alias,
            package.realm.section(),
            package.declaration
        )?;
    }
    writeln!(output, "Tool pins: {}", snapshot.tools.len())?;
    for tool in &snapshot.tools {
        let detection = tool
            .executable
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "not found on PATH".into());
        writeln!(output, "  {}: {} ({detection})", tool.alias, tool.source)?;
    }
    writeln!(
        output,
        "Configured actions (runtime prerequisites checked when run):"
    )?;
    for (name, action) in [
        ("watch", &snapshot.actions.watch),
        ("check", &snapshot.actions.check),
        ("test", &snapshot.actions.test),
    ] {
        writeln!(
            output,
            "  {name}: {}",
            if action.available {
                "available"
            } else {
                action.reason.as_deref().unwrap_or("unavailable")
            }
        )?;
    }
    for diagnostic in snapshot
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity != DiagnosticSeverity::Info)
    {
        writeln!(
            output,
            "{:?}: {}: {}",
            diagnostic.severity,
            diagnostic.path.display(),
            diagnostic.message
        )?;
    }
    Ok(())
}
