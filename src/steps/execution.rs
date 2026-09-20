use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::Arc;

use anyhow::{Context, Result, bail};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageKind {
    Detail,
    Already,
    Installed,
    Warning,
    Manual,
    Output,
}

#[derive(Clone)]
pub struct Reporter(pub Arc<MessageSink>);

type MessageSink = dyn Fn(MessageKind, &str) + Send + Sync;

impl Reporter {
    pub fn message(&self, kind: MessageKind, text: &str) {
        (self.0)(kind, text);
    }

    pub fn capture(&self, command: &mut Command) -> Result<super::Captured> {
        let program = command.get_program().to_string_lossy().into_owned();
        crate::diagnostics::event("setup.tool", &program);
        let mut child = command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("failed to spawn `{program}`"))?;
        // Both pipes must be drained even after the receiver stops rendering.
        let (stdout, stderr, status) = std::thread::scope(|scope| {
            let out = child.stdout.take().expect("piped stdout");
            let err = child.stderr.take().expect("piped stderr");
            let a = scope.spawn(|| self.drain(out));
            let b = scope.spawn(|| self.drain(err));
            let status = child.wait();
            let stdout = a
                .join()
                .map_err(|_| anyhow::anyhow!("stdout reader panicked"))?;
            let stderr = b
                .join()
                .map_err(|_| anyhow::anyhow!("stderr reader panicked"))?;
            Ok::<_, anyhow::Error>((stdout?, stderr?, status?))
        })?;
        crate::diagnostics::tool_exit(&program, status);
        Ok(super::Captured {
            stdout,
            stderr,
            success: status.success(),
        })
    }

    fn drain(&self, mut reader: impl Read) -> Result<String> {
        const LIMIT: usize = 8 * 1024 * 1024;
        let mut all = Vec::new();
        let mut chunk = [0; 4096];
        let mut overflow = false;
        let mut display = OutputText::default();
        loop {
            let count = reader.read(&mut chunk)?;
            if count == 0 {
                break;
            }
            self.message(MessageKind::Output, &display.feed(&chunk[..count], false));
            if all.len() + count <= LIMIT {
                all.extend_from_slice(&chunk[..count]);
            } else {
                overflow = true;
            }
        }
        self.message(MessageKind::Output, &display.feed(&[], true));
        if overflow {
            bail!(
                "installer output exceeded the 8 MiB parsing limit; output was drained, but its result cannot be safely interpreted"
            );
        }
        Ok(String::from_utf8_lossy(&all).into_owned())
    }

    pub fn run(&self, command: &mut Command) -> Result<()> {
        let output = self.capture(command)?;
        if !output.success {
            bail!(
                "installation command failed; see output (commands requiring terminal input must be completed manually)"
            );
        }
        Ok(())
    }
}

#[derive(Default)]
struct OutputText {
    pending: Vec<u8>,
    escape: u8,
    carriage: bool,
}

impl OutputText {
    fn feed(&mut self, bytes: &[u8], finished: bool) -> String {
        self.pending.extend_from_slice(bytes);
        let mut decoded = String::new();
        loop {
            match std::str::from_utf8(&self.pending) {
                Ok(text) => {
                    decoded.push_str(text);
                    self.pending.clear();
                    break;
                }
                Err(error) => {
                    let valid = error.valid_up_to();
                    decoded.push_str(
                        std::str::from_utf8(&self.pending[..valid]).expect("validated prefix"),
                    );
                    self.pending.drain(..valid);
                    if let Some(count) = error.error_len() {
                        self.pending.drain(..count);
                        decoded.push('\u{fffd}');
                    } else {
                        if finished {
                            decoded.push('\u{fffd}');
                            self.pending.clear();
                        }
                        break;
                    }
                }
            }
        }
        let mut clean = String::new();
        for character in decoded.chars() {
            match self.escape {
                1 => {
                    self.escape = match character {
                        '[' => 2,
                        ']' | 'P' | '^' | '_' => 3,
                        _ => 0,
                    }
                }
                2 => {
                    if ('@'..='~').contains(&character) {
                        self.escape = 0;
                    }
                }
                3 => {
                    if character == '\u{7}' {
                        self.escape = 0;
                    } else if character == '\u{1b}' {
                        self.escape = 4;
                    }
                }
                4 => self.escape = if character == '\\' { 0 } else { 3 },
                _ => match character {
                    '\u{1b}' => self.escape = 1,
                    '\r' => {
                        clean.push('\n');
                        self.carriage = true;
                    }
                    '\n' => {
                        if !self.carriage {
                            clean.push('\n');
                        }
                        self.carriage = false;
                    }
                    '\t' => {
                        clean.push(character);
                        self.carriage = false;
                    }
                    c if !c.is_control() => {
                        clean.push(c);
                        self.carriage = false;
                    }
                    _ => {}
                },
            }
        }
        clean
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_child() {
        use std::io::Write;
        let Some(root) = std::env::var_os("RPROJ_EXECUTION_FIXTURE_ROOT") else {
            return;
        };
        let root = std::path::PathBuf::from(root);
        match std::env::var("RPROJ_EXECUTION_FIXTURE_MODE").as_deref() {
            Ok("hold") => {
                println!("Fixture child active");
                std::io::stdout().flush().unwrap();
                let started = std::time::Instant::now();
                while !root.join("release").exists() {
                    assert!(
                        started.elapsed() < std::time::Duration::from_secs(30),
                        "fixture release timeout"
                    );
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
                std::fs::write(root.join("child-finished"), "").unwrap();
            }
            Ok("fail") => std::process::exit(9),
            _ => {
                let stderr = std::thread::spawn(|| {
                    for _ in 0..1024 {
                        eprintln!("{}", "stderr".repeat(128));
                    }
                });
                for _ in 0..1024 {
                    println!("{}", "stdout".repeat(128));
                }
                stderr.join().unwrap();
                println!("\u{1b}[31mUnicode \u{4e2d}\u{00e9}\u{1b}[0m");
            }
        }
    }

    #[test]
    fn fixture_process_drains_both_pipes_and_preserves_exit_status() {
        let root = tempfile::tempdir().unwrap();
        let reporter = Reporter(Arc::new(|_, _| {}));
        for mode in ["output", "fail"] {
            let output = reporter
                .capture(
                    Command::new(std::env::current_exe().unwrap())
                        .args([
                            "steps::execution::tests::fixture_child",
                            "--exact",
                            "--nocapture",
                        ])
                        .env("RPROJ_EXECUTION_FIXTURE_ROOT", root.path())
                        .env("RPROJ_EXECUTION_FIXTURE_MODE", mode),
                )
                .unwrap();
            assert_eq!(output.success, mode == "output");
            if output.success {
                assert!(output.stdout.contains("Unicode \u{4e2d}\u{00e9}"));
                assert!(output.stderr.len() > 500_000);
            }
        }
    }

    #[test]
    fn reader_retains_unicode_for_parsing() {
        let reporter = Reporter(Arc::new(|_, _| {}));
        let text = "hello \u{4e2d}\u{00e9}\n";
        assert_eq!(reporter.drain(text.as_bytes()).unwrap(), text);
    }

    #[test]
    fn streamed_unicode_and_control_sequences_cross_chunk_boundaries() {
        let mut output = OutputText::default();
        let bytes = "\u{1b}[31m\u{4e2d}\u{1b}[0m\u{1b}]0;hidden\u{7} done".as_bytes();
        let clean: String = bytes
            .iter()
            .map(|byte| output.feed(&[*byte], false))
            .collect();
        assert_eq!(clean, "\u{4e2d} done");
        assert!(output.feed(&[], true).is_empty());
        assert_eq!(output.feed(b"one\r", false), "one\n");
        assert_eq!(output.feed(b"\ntwo\rthree\r\n", true), "two\nthree\n");
    }

    #[test]
    fn failed_spawn_has_program_context() {
        let reporter = Reporter(Arc::new(|_, _| {}));
        let error = reporter
            .capture(&mut Command::new("rproj-nonexistent-fixture-executable"))
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("rproj-nonexistent-fixture-executable")
        );
    }
}
