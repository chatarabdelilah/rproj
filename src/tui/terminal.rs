use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

pub struct TerminalSession {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    active: bool,
}

static ACTIVE: AtomicBool = AtomicBool::new(false);

#[derive(Debug)]
pub struct TerminalFailure;

impl std::fmt::Display for TerminalFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("terminal session failed")
    }
}

pub fn active() -> bool {
    ACTIVE.load(Ordering::Relaxed)
}

impl TerminalSession {
    pub fn enter() -> Result<Self> {
        crate::diagnostics::event("tui.enter", "alternate screen");
        enable_raw_mode().context(TerminalFailure)?;
        let mut stdout = io::stdout();
        if let Err(error) = execute!(stdout, EnterAlternateScreen, event::EnableBracketedPaste) {
            let _ = disable_raw_mode();
            return Err(anyhow::Error::new(error).context(TerminalFailure));
        }
        let terminal = match Terminal::new(CrosstermBackend::new(stdout)) {
            Ok(terminal) => terminal,
            Err(error) => {
                let _ = disable_raw_mode();
                let _ = execute!(
                    io::stdout(),
                    event::DisableBracketedPaste,
                    LeaveAlternateScreen
                );
                return Err(anyhow::Error::new(error).context(TerminalFailure));
            }
        };
        ACTIVE.store(true, Ordering::Relaxed);
        Ok(Self {
            terminal,
            active: true,
        })
    }

    pub fn draw(&mut self, render: impl FnOnce(&mut ratatui::Frame<'_>)) -> Result<()> {
        self.terminal.draw(render).context(TerminalFailure)?;
        Ok(())
    }

    pub fn suspend(&mut self) -> Result<()> {
        disable_raw_mode().context(TerminalFailure)?;
        execute!(
            self.terminal.backend_mut(),
            event::DisableBracketedPaste,
            LeaveAlternateScreen
        )
        .context(TerminalFailure)?;
        self.terminal.show_cursor().context(TerminalFailure)?;
        self.active = false;
        ACTIVE.store(false, Ordering::Relaxed);
        crate::diagnostics::event("tui.suspend", "command output");
        Ok(())
    }

    pub fn resume(&mut self) -> Result<()> {
        // Mark active first so Drop restores modes if re-entry fails midway.
        self.active = true;
        enable_raw_mode().context(TerminalFailure)?;
        execute!(
            self.terminal.backend_mut(),
            EnterAlternateScreen,
            event::EnableBracketedPaste
        )
        .context(TerminalFailure)?;
        self.terminal.clear().context(TerminalFailure)?;
        ACTIVE.store(true, Ordering::Relaxed);
        crate::diagnostics::event("tui.resume", "Home");
        Ok(())
    }

    pub fn read_event(&self) -> Result<Event> {
        let event = event::read().context(TerminalFailure)?;
        log_event(&event);
        Ok(event)
    }

    pub fn poll_event(&self, timeout: Duration) -> Result<Option<Event>> {
        if event::poll(timeout).context(TerminalFailure)? {
            self.read_event().map(Some)
        } else {
            Ok(None)
        }
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        ACTIVE.store(false, Ordering::Relaxed);
        if !self.active {
            return;
        }
        crate::diagnostics::event("tui.leave", "restoring terminal");
        // This guard is the single owner of terminal mode, including during unwind.
        let _ = disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            event::DisableBracketedPaste,
            LeaveAlternateScreen
        );
        let _ = self.terminal.show_cursor();
    }
}

fn log_event(event: &Event) {
    match event {
        Event::Paste(text) => crate::diagnostics::event(
            "tui.paste",
            format!("{} bytes; content omitted", text.len()),
        ),
        Event::Resize(cols, rows) => {
            crate::diagnostics::event("tui.resize", format!("{cols}x{rows}"))
        }
        _ => {}
    }
}
