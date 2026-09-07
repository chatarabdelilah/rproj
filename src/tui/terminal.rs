use std::io;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

pub struct TerminalSession {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl TerminalSession {
    pub fn enter() -> Result<Self> {
        crate::diagnostics::event("tui.enter", "alternate screen");
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        if let Err(error) = execute!(stdout, EnterAlternateScreen, event::EnableBracketedPaste) {
            let _ = disable_raw_mode();
            return Err(error.into());
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
                return Err(error.into());
            }
        };
        Ok(Self { terminal })
    }

    pub fn draw(&mut self, render: impl FnOnce(&mut ratatui::Frame<'_>)) -> Result<()> {
        self.terminal.draw(render)?;
        Ok(())
    }

    pub fn read_event(&self) -> Result<Event> {
        let event = event::read()?;
        log_event(&event);
        Ok(event)
    }

    pub fn poll_event(&self, timeout: Duration) -> Result<Option<Event>> {
        if event::poll(timeout)? {
            self.read_event().map(Some)
        } else {
            Ok(None)
        }
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
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
    use crossterm::event::{KeyCode, KeyEventKind};
    match event {
        Event::Key(key) if key.kind != KeyEventKind::Release => {
            // Text and shortcut characters are not a keystroke transcript.
            // Their committed semantic actions are logged by each screen.
            if !matches!(key.code, KeyCode::Char(_)) {
                crate::diagnostics::event("tui.navigation", format!("{:?}", key.code));
            }
        }
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
