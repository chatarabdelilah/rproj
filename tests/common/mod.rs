//! Runs the real `rproj` binary attached to a real pseudo-terminal, so the
//! interactive commands can be tested by answering their prompts.
//!
//! Why a PTY and not piped stdin: inquire reads the console input handle
//! rather than stdin, so `printf '\n\n' | rproj configure stylua` renders
//! the first prompt and then hangs forever - verified, it doesn't error, it
//! just never returns.
//!
//! Synchronisation is by *expecting output*, never by sleeping. Every
//! prompt is preceded by a `println!` of the setting's name, which inquire
//! doesn't redraw (it only rewrites its own prompt area), so waiting for
//! that heading is a reliable "the prompt is up now" signal. Fixed sleeps
//! would make the suite flaky on a loaded machine and slow on an idle one.
//!
//! What the byte stream contains is *not* what the program printed. ConPTY
//! re-renders the screen and is free to express a line break either as
//! `\n` or as a cursor-position escape, and which one it picks varies with
//! machine load - string-matching the raw bytes passed when run alone and
//! failed three runs in five under a parallel `cargo test`. So the bytes go
//! through a terminal emulator and assertions run against the rendered
//! screen, where a line break is always a line break.

#![allow(dead_code)]

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};

pub const ENTER: &str = "\r";
pub const DOWN: &str = "\x1b[B";
pub const ESC: &str = "\x1b";

/// How long any single wait may take before the test fails. Generous: the
/// cost of a high value is only paid when something is already broken.
const TIMEOUT: Duration = Duration::from_secs(30);

/// Tall enough that no test's output ever scrolls. Anything that scrolls
/// off the top is gone, since only the visible screen is rendered.
const ROWS: u16 = 400;
const COLS: u16 = 160;

/// A scratch directory that cleans itself up.
pub struct TempProject {
    path: PathBuf,
}

impl TempProject {
    pub fn new(label: &str) -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir()
            .join(format!("rproj-test-{}-{label}-{unique}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create temp project");
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn write(&self, relative: &str, contents: &str) {
        let path = self.path.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create parent");
        }
        std::fs::write(path, contents).expect("write fixture");
    }

    pub fn read(&self, relative: &str) -> String {
        std::fs::read_to_string(self.path.join(relative)).expect("read back")
    }

    pub fn exists(&self, relative: &str) -> bool {
        self.path.join(relative).exists()
    }
}

impl Drop for TempProject {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

pub struct Outcome {
    pub text: String,
    pub code: u32,
}

impl Outcome {
    pub fn assert_contains(&self, needle: &str) {
        assert!(self.text.contains(needle), "expected {needle:?} in:\n{}", self.text);
    }

    pub fn assert_lacks(&self, needle: &str) {
        assert!(!self.text.contains(needle), "did not expect {needle:?} in:\n{}", self.text);
    }
}

pub struct Session {
    child: Box<dyn portable_pty::Child + Send + Sync>,
    writer: Box<dyn Write + Send>,
    raw: Arc<Mutex<Vec<u8>>>,
    // Held so the pty outlives the child; dropping it early terminates the
    // process with STATUS_CONTROL_C_EXIT instead of letting it finish.
    _master: Box<dyn MasterPty + Send>,
}

impl Session {
    pub fn start(cwd: &Path, args: &[&str]) -> Self {
        let pair = native_pty_system()
            .openpty(PtySize { rows: ROWS, cols: COLS, pixel_width: 0, pixel_height: 0 })
            .expect("open pty");

        let mut cmd = CommandBuilder::new(env!("CARGO_BIN_EXE_rproj"));
        for arg in args {
            cmd.arg(arg);
        }
        cmd.cwd(cwd);
        let child = pair.slave.spawn_command(cmd).expect("spawn rproj");
        // The slave has to go, or the reader below never sees EOF.
        drop(pair.slave);

        let raw = Arc::new(Mutex::new(Vec::new()));
        let mut reader = pair.master.try_clone_reader().expect("clone reader");
        {
            let raw = Arc::clone(&raw);
            std::thread::spawn(move || {
                let mut chunk = [0u8; 4096];
                while let Ok(n) = reader.read(&mut chunk) {
                    if n == 0 {
                        break;
                    }
                    raw.lock().unwrap().extend_from_slice(&chunk[..n]);
                }
            });
        }
        let writer = pair.master.take_writer().expect("take writer");
        Self { child, writer, raw, _master: pair.master }
    }

    /// The screen as it currently looks, rendered from everything written
    /// so far.
    ///
    /// Rendered rather than string-matched on the raw stream: ConPTY may
    /// express a line break as `\n` or as a cursor-position escape, and
    /// which one it picks varies with machine load. Feeding the bytes
    /// through a terminal emulator makes the text deterministic.
    pub fn text(&self) -> String {
        let raw = self.raw.lock().unwrap();
        let mut parser = vt100::Parser::new(ROWS, COLS, 0);
        parser.process(&raw);
        let contents = parser.screen().contents();
        // Trailing blank rows are padding, not output.
        contents.trim_end().to_string()
    }

    pub fn send(&mut self, keys: &str) {
        self.writer.write_all(keys.as_bytes()).expect("write to pty");
        self.writer.flush().expect("flush pty");
    }

    /// Blocks until `needle` appears in the output.
    pub fn wait_for(&self, needle: &str) {
        let deadline = Instant::now() + TIMEOUT;
        loop {
            let text = self.text();
            if text.contains(needle) {
                return;
            }
            assert!(Instant::now() < deadline, "timed out waiting for {needle:?} in:\n{text}");
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// Blocks until the walkthrough is asking about `heading`.
    ///
    /// Matches the heading plus the description's two-space indent on the
    /// next line, so a heading that happens to be a substring of some
    /// description can't satisfy it.
    pub fn wait_for_prompt(&self, heading: &str) {
        self.wait_for(&format!("{heading}\n  "));
    }

    /// Waits for each setting's prompt in turn and presses enter at it.
    ///
    /// Passing the headings in explicitly rather than deriving them is
    /// deliberate: it pins the order and naming of the walkthrough, so
    /// adding or renaming a setting fails a test rather than silently
    /// changing what users are asked.
    pub fn enter_through(&mut self, headings: &[&str]) {
        for heading in headings {
            self.wait_for_prompt(heading);
            self.send(ENTER);
        }
    }

    pub fn finish(mut self) -> Outcome {
        let deadline = Instant::now() + TIMEOUT;
        let status = loop {
            match self.child.try_wait().expect("wait on rproj") {
                Some(status) => break status,
                None if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(20)),
                None => {
                    let text = self.text();
                    let _ = self.child.kill();
                    panic!("rproj never exited. Output so far:\n{text}");
                }
            }
        };
        // The reader thread may still be draining the last write.
        std::thread::sleep(Duration::from_millis(150));
        Outcome { text: self.text(), code: status.exit_code() }
    }
}
