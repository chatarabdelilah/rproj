//! Local, best-effort diagnostic events. Never a terminal/keystroke transcript.

use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const MAX_BYTES: usize = 2 * 1024 * 1024;
const MAX_EVENT_CHARS: usize = 4096;
static LOG: OnceLock<Mutex<Option<Log>>> = OnceLock::new();
static QUIET_HUB: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn quiet_hub() {
    QUIET_HUB.store(true, std::sync::atomic::Ordering::Relaxed);
}

pub fn path_message() -> String {
    LOG.get()
        .and_then(|state| {
            state.lock().ok().and_then(|log| {
                log.as_ref()
                    .map(|log| format!("Diagnostic log: {}", log.path.display()))
            })
        })
        .unwrap_or_else(|| "Diagnostic logging is unavailable.".into())
}

struct Log {
    file: File,
    path: PathBuf,
    started: Instant,
    bytes: usize,
    capped: bool,
    failed: bool,
}

impl Log {
    fn create(dir: &Path) -> io::Result<Self> {
        std::fs::create_dir_all(dir)?;
        let prefix = format!("rproj-{}-{}-", timestamp(), std::process::id());
        let (file, path) = tempfile::Builder::new()
            .prefix(&prefix)
            .suffix(".txt")
            .tempfile_in(dir)?
            .keep()
            .map_err(|error| error.error)?;
        Ok(Self {
            file,
            path,
            started: Instant::now(),
            bytes: 0,
            capped: false,
            failed: false,
        })
    }

    fn event(&mut self, kind: &str, message: &str) {
        if self.failed || self.capped {
            return;
        }
        let mut line = format!(
            "{} +{}ms [{}] {}\n",
            timestamp(),
            self.started.elapsed().as_millis(),
            clean(kind),
            clean(message)
        );
        if self.bytes + line.len() > MAX_BYTES - 128 {
            line = "[log.limit] 2 MiB limit reached; subsequent events omitted.\n".into();
            self.capped = true;
        }
        self.bytes += line.len();
        if self
            .file
            .write_all(line.as_bytes())
            .and_then(|()| self.file.flush())
            .is_err()
        {
            self.failed = true;
        }
    }
}

fn timestamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

/// Redact recognizable sensitive fields before truncation, and prevent text
/// from injecting extra events or terminal control sequences into the log.
/// This is defense in depth: callers must omit opaque payloads at the source.
fn clean(message: &str) -> String {
    let lower = message.to_ascii_lowercase();
    if [
        "token",
        "secret",
        "password",
        "authorization",
        "api_key",
        "apikey",
        "credential",
        "cookie",
        "private key",
        "bearer ",
        "ghp_",
        "github_pat_",
        "sk-",
        "eyj",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
    {
        return "[redacted: potentially sensitive value]".into();
    }
    let mut result = String::new();
    for ch in message.chars().take(MAX_EVENT_CHARS) {
        if ch.is_control() {
            result.extend(ch.escape_default());
        } else {
            result.push(ch);
        }
    }
    if message.chars().count() > MAX_EVENT_CHARS {
        result.push_str(" [truncated]");
    }
    result
}

pub fn event(kind: &str, message: impl AsRef<str>) {
    if let Some(state) = LOG.get() {
        let mut state = state.lock().unwrap_or_else(|poison| poison.into_inner());
        if let Some(log) = state.as_mut() {
            log.event(kind, message.as_ref());
        }
    }
}

/// Error excerpts can contain arbitrary source/configuration contents. Keep
/// the operation-level first line of each cause, not parser source snippets.
pub fn error(error: &anyhow::Error) {
    for cause in error.chain() {
        event("error", cause.to_string().lines().next().unwrap_or("error"));
    }
}

pub fn command(program: &str, args: &[&str]) {
    // Runner arguments are an opaque user-controlled passthrough, not rproj
    // choices. They may contain cloud credentials even without a named flag.
    if matches!(program, "jest-roblox-cli" | "lute") {
        event(
            "tool.start",
            format!("{program}; {} arguments (values omitted)", args.len()),
        );
    } else {
        event("tool.start", format!("{program} {}", args.join(" ")));
    }
}

pub fn tool_exit(program: &str, status: std::process::ExitStatus) {
    event("tool.exit", format!("{program}: {status}"));
}

/// Owns the run footer and path announcement, after the TUI has restored the
/// terminal. Logging failures must never replace the command's exit status.
pub struct RunLog;

pub fn start() -> RunLog {
    if std::env::var_os("RPROJ_NO_LOG").is_some_and(|value| value == "1") {
        return RunLog;
    }
    let dir = std::env::var_os("RPROJ_LOG_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            directories::ProjectDirs::from("", "", "rproj")
                .map(|dirs| dirs.data_local_dir().join("logs"))
        });
    let result = dir
        .ok_or_else(|| io::Error::other("no local data directory"))
        .and_then(|dir| Log::create(&dir));
    match result {
        Ok(log) => {
            let _ = LOG.set(Mutex::new(Some(log)));
            event(
                "run.start",
                format!(
                    "rproj {}; {} {}; timestamps are Unix milliseconds",
                    env!("CARGO_PKG_VERSION"),
                    std::env::consts::OS,
                    std::env::consts::ARCH
                ),
            );
            if let Ok(cwd) = std::env::current_dir() {
                event("cwd", cwd.display().to_string());
            }
            let previous = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |info| {
                // No panic payload: it may be arbitrary user/file content.
                event(
                    "panic",
                    info.location().map(ToString::to_string).unwrap_or_default(),
                );
                previous(info);
            }));
        }
        Err(_) => eprintln!("rproj: diagnostic log unavailable; continuing without logging."),
    }
    RunLog
}

impl RunLog {
    pub fn finish(self, code: u8) {
        event("run.end", format!("exit_code={code}"));
        if code != 0 {
            QUIET_HUB.store(false, std::sync::atomic::Ordering::Relaxed);
        }
    }
}

impl Drop for RunLog {
    fn drop(&mut self) {
        if let Some(state) = LOG.get() {
            let mut state = state.lock().unwrap_or_else(|poison| poison.into_inner());
            if let Some(log) = state.take() {
                if QUIET_HUB.load(std::sync::atomic::Ordering::Relaxed) {
                    return;
                }
                eprintln!(
                    "Diagnostic log: {}{}",
                    log.path.display(),
                    if log.failed {
                        " (incomplete: write failed)"
                    } else if log.capped {
                        " (size limit reached)"
                    } else {
                        ""
                    }
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secrets_are_redacted_before_truncation() {
        for value in [
            "--api_key=abc",
            "Authorization: Bearer abc",
            "ghp_abc",
            "password abc",
            "sk-abc",
            "github_pat_abc",
            "cookie=x",
        ] {
            assert_eq!(clean(value), "[redacted: potentially sensitive value]");
        }
        assert!(
            clean(&format!("{} token=secret", "x".repeat(MAX_EVENT_CHARS + 1)))
                .starts_with("[redacted")
        );
    }

    #[test]
    fn events_escape_controls_and_bound_long_unicode() {
        assert_eq!(clean("a\n\r\x1b[31m"), "a\\n\\r\\u{1b}[31m");
        assert!(clean(&"界".repeat(MAX_EVENT_CHARS + 1)).ends_with(" [truncated]"));
    }

    #[test]
    fn concurrent_sessions_use_distinct_files_without_overwriting() {
        let dir = tempfile::tempdir().unwrap();
        let mut first = Log::create(dir.path()).unwrap();
        let second = Log::create(dir.path()).unwrap();
        assert_ne!(first.path, second.path);
        first.event("choice", "wally");
        assert!(
            std::fs::read_to_string(&first.path)
                .unwrap()
                .contains("[choice] wally")
        );
    }

    #[test]
    fn write_failure_disables_further_events_without_panicking() {
        let dir = tempfile::tempdir().unwrap();
        let mut log = Log::create(dir.path()).unwrap();
        log.file = File::open(&log.path).unwrap(); // Deliberately read-only.
        log.event("event", "cannot write");
        assert!(log.failed);
        let bytes = log.bytes;
        log.event("event", "ignored after failure");
        assert_eq!(log.bytes, bytes);
    }

    #[test]
    fn logfile_stops_at_a_bounded_size() {
        let dir = tempfile::tempdir().unwrap();
        let mut log = Log::create(dir.path()).unwrap();
        let text = "x".repeat(MAX_EVENT_CHARS);
        for _ in 0..1000 {
            log.event("event", &text);
        }
        assert!(log.capped);
        assert!(std::fs::metadata(&log.path).unwrap().len() <= MAX_BYTES as u64);
        let text = std::fs::read_to_string(&log.path).unwrap();
        assert_eq!(text.matches("[log.limit]").count(), 1);
    }
}
