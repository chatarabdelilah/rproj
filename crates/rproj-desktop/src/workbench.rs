mod view;

use gpui_kit::component::{
    input::{InputEvent, InputState},
    *,
};
use gpui_kit::*;
use rproj_core::{
    ProjectSnapshot,
    catalog::CatalogEntry,
    desktop_state::{Appearance, DesktopState},
    operations::{
        Cancellation, Operation, OperationEvent, OperationOutcome, WatchStatus, WorkbenchClient,
    },
};
use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
        mpsc::{Receiver, SyncSender, sync_channel},
    },
    time::{Duration, Instant},
};

const EVENT_CAPACITY: usize = 128;
const LOG_CAPACITY: usize = 2_000;
const LOG_BYTES: usize = 1_048_576;

pub fn worker_path() -> anyhow::Result<PathBuf> {
    Ok(std::env::current_exe()?.with_file_name(if cfg!(windows) { "rproj.exe" } else { "rproj" }))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    Overview,
    Dependencies,
    Tools,
    Checks,
}
impl Section {
    fn title(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Dependencies => "Dependencies",
            Self::Tools => "Tools",
            Self::Checks => "Checks",
        }
    }
}

enum Event {
    Connected(Result<WorkbenchClient, String>, Vec<CatalogEntry>),
    Preferences(Result<(DesktopState, PathBuf), String>),
    PreferenceError(String),
    Inspected(u64, Box<Result<(ProjectSnapshot, String), String>>),
    Status(Result<WatchStatus, String>),
    Progress(OperationEvent),
    Complete(Result<OperationOutcome, String>),
    WatchLogs(Result<String, String>),
}

#[derive(Default)]
struct Activity {
    lines: VecDeque<String>,
    bytes: usize,
    omitted: usize,
    partial: [String; 2],
}
impl Activity {
    fn append_stream(&mut self, stderr: bool, text: &str) {
        let stream = usize::from(stderr);
        for character in text.chars() {
            if character == '\n' {
                self.flush_stream(stream);
            } else {
                self.partial[stream].push(character);
                // Reserve the display prefix so the bounded line writer keeps every chunk.
                if self.partial[stream].len() >= 8_192 - "[stderr] ".len() {
                    self.flush_stream(stream);
                }
            }
        }
    }
    fn flush_stream(&mut self, stream: usize) {
        let pending = std::mem::take(&mut self.partial[stream]);
        let text = pending.trim_end_matches('\r');
        if stream == 1 {
            self.append(&format!("[stderr] {text}"));
        } else {
            self.append(text);
        }
    }
    fn flush(&mut self) {
        for stream in 0..2 {
            if !self.partial[stream].is_empty() {
                self.flush_stream(stream);
            }
        }
    }
    fn displayed(&self) -> impl Iterator<Item = String> + '_ {
        self.lines.iter().cloned().chain(
            self.partial
                .iter()
                .enumerate()
                .filter(|(_, text)| !text.is_empty())
                .map(|(stream, text)| {
                    if stream == 1 {
                        format!("[stderr] {text}")
                    } else {
                        text.clone()
                    }
                }),
        )
    }
    fn append(&mut self, text: &str) {
        for line in text.lines() {
            let line: String = line.chars().take(8_192).collect();
            self.bytes += line.len();
            self.lines.push_back(line);
            while self.lines.len() > LOG_CAPACITY || self.bytes > LOG_BYTES {
                if let Some(line) = self.lines.pop_front() {
                    self.bytes -= line.len();
                    self.omitted += 1;
                }
            }
        }
    }
}

pub struct Workbench {
    focus: FocusHandle,
    section: Section,
    project: Option<PathBuf>,
    snapshot: Option<ProjectSnapshot>,
    selected_package: Option<String>,
    generation: u64,
    inspecting: bool,
    inspection_fingerprint: String,
    error: Option<String>,
    preferences: DesktopState,
    preferences_path: Option<PathBuf>,
    preferences_sender: SyncSender<DesktopState>,
    preferences_dirty: bool,
    preferences_error: Option<String>,
    client: Option<WorkbenchClient>,
    client_error: Option<String>,
    catalog: Vec<CatalogEntry>,
    watch: Option<WatchStatus>,
    querying_status: bool,
    running: Option<(Operation, Cancellation)>,
    activity: Activity,
    filter: Entity<InputState>,
    sender: SyncSender<Event>,
    receiver: Receiver<Event>,
    dropped: Arc<AtomicUsize>,
    last_poll: Instant,
    _subscriptions: Vec<Subscription>,
    _pump: Task<()>,
}

impl Workbench {
    pub fn new(
        initial: Option<PathBuf>,
        preferences_override: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        let (sender, receiver) = sync_channel(EVENT_CAPACITY);
        let filter = cx.new(|cx| InputState::new(window, cx).placeholder("Search output"));
        let subscription = cx.subscribe(&filter, |_, _, event, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        });
        let appearance_subscription = cx.observe_window_appearance(window, |this, window, cx| {
            if this.preferences.appearance == Appearance::System {
                Theme::sync_system_appearance(Some(window), cx);
            }
        });
        let preferences = DesktopState::default();
        apply_appearance(preferences.appearance, window, cx);
        let (preferences_sender, preferences_receiver) = sync_channel::<DesktopState>(1);
        let preferences_events = sender.clone();
        std::thread::spawn(move || {
            let loaded = load_preferences(preferences_override);
            let path = loaded.as_ref().ok().map(|(_, path)| path.clone());
            if preferences_events.send(Event::Preferences(loaded)).is_err() {
                return;
            }
            while let Ok(state) = preferences_receiver.recv() {
                if let Some(path) = &path
                    && let Err(error) = state.save(path)
                {
                    let _ = preferences_events.send(Event::PreferenceError(format!(
                        "Could not save desktop preferences: {error:#}"
                    )));
                }
            }
        });
        let executor = cx.background_executor().clone();
        let pump = cx.spawn(async move |this, cx| {
            loop {
                executor.timer(Duration::from_millis(100)).await;
                if this.update(cx, |this, cx| this.drain(cx)).is_err() {
                    break;
                }
            }
        });
        let mut view = Self {
            focus,
            section: Section::Overview,
            project: None,
            snapshot: None,
            selected_package: None,
            generation: 0,
            inspecting: false,
            inspection_fingerprint: String::new(),
            error: None,
            preferences,
            preferences_path: None,
            preferences_sender,
            preferences_dirty: false,
            preferences_error: None,
            client: None,
            client_error: Some("Connecting to the bundled CLI…".into()),
            catalog: Vec::new(),
            watch: None,
            querying_status: false,
            running: None,
            activity: Activity::default(),
            filter,
            sender,
            receiver,
            dropped: Arc::new(AtomicUsize::new(0)),
            last_poll: Instant::now(),
            _subscriptions: vec![subscription, appearance_subscription],
            _pump: pump,
        };
        let sender = view.sender.clone();
        std::thread::spawn(move || {
            let client = worker_path()
                .and_then(WorkbenchClient::new)
                .map_err(|error| format!("{error:#}"));
            let catalog = client
                .as_ref()
                .ok()
                .and_then(|client| client.catalog().ok())
                .unwrap_or_default();
            let _ = sender.send(Event::Connected(client, catalog));
        });
        if let Some(path) = initial {
            view.open_project(path, cx);
        }
        view
    }

    fn open_project(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.generation = self.generation.wrapping_add(1);
        self.project = Some(path);
        self.inspecting = false;
        self.snapshot = None;
        self.selected_package = None;
        self.error = None;
        self.inspection_fingerprint.clear();
        self.inspect();
        cx.notify();
    }

    fn inspect(&mut self) {
        if self.inspecting {
            return;
        }
        let Some(path) = self.project.clone() else {
            return;
        };
        self.inspecting = true;
        self.generation = self.generation.wrapping_add(1);
        let generation = self.generation;
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let result = rproj_core::inspect(&path)
                .and_then(|snapshot| {
                    let fingerprint = serde_json::to_string(&snapshot)?;
                    Ok((snapshot, fingerprint))
                })
                .map_err(|error| format!("{error:#}"));
            let _ = sender.send(Event::Inspected(generation, Box::new(result)));
        });
    }

    fn choose_project(&mut self, cx: &mut Context<Self>) {
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Open project".into()),
        });
        cx.spawn(async move |this, cx| match paths.await {
            Ok(Ok(Some(paths))) => {
                if let Some(path) = paths.into_iter().next() {
                    let _ = this.update(cx, |this, cx| this.open_project(path, cx));
                }
            }
            Ok(Ok(None)) => {}
            error => {
                let _ = this.update(cx, |this, cx| {
                    this.error = Some(format!("Could not open the folder picker: {error:?}"));
                    cx.notify();
                });
            }
        })
        .detach();
    }

    fn poll_status(&mut self) {
        if self.querying_status {
            return;
        }
        let Some(client) = self.client.clone() else {
            return;
        };
        self.querying_status = true;
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let _ = sender.send(Event::Status(
                client.watch_status().map_err(|error| format!("{error:#}")),
            ));
        });
    }

    fn drain(&mut self, cx: &mut Context<Self>) {
        let mut changed = false;
        for _ in 0..EVENT_CAPACITY {
            let Ok(event) = self.receiver.try_recv() else {
                break;
            };
            match event {
                Event::Preferences(loaded) => match loaded {
                    Ok((state, path)) => {
                        self.preferences = state;
                        self.preferences_path = Some(path);
                        match self.preferences.appearance {
                            Appearance::System => Theme::sync_system_appearance(None, cx),
                            Appearance::Light => Theme::change(ThemeMode::Light, None, cx),
                            Appearance::Dark => Theme::change(ThemeMode::Dark, None, cx),
                        }
                        if let Some(snapshot) = &self.snapshot {
                            let root = snapshot.root.clone();
                            self.remember(root);
                        }
                    }
                    Err(error) => self.preferences_error = Some(error),
                },
                Event::PreferenceError(error) => {
                    self.activity.append(&error);
                    self.preferences_error = Some(error);
                }
                Event::Connected(client, catalog) => {
                    self.catalog = catalog;
                    match client {
                        Ok(client) => {
                            self.client = Some(client);
                            self.client_error = None;
                            self.poll_status();
                        }
                        Err(error) => self.client_error = Some(error),
                    }
                }
                Event::Inspected(generation, result) if generation == self.generation => {
                    self.inspecting = false;
                    match *result {
                        Ok((snapshot, fingerprint)) => {
                            if fingerprint == self.inspection_fingerprint
                                && self.snapshot.is_some()
                                && self.error.is_none()
                            {
                                continue;
                            }
                            self.inspection_fingerprint = fingerprint;
                            if self.snapshot.is_none() {
                                self.remember(snapshot.root.clone());
                            }
                            self.project = Some(snapshot.root.clone());
                            self.snapshot = Some(snapshot);
                            self.error = None;
                        }
                        Err(error) => {
                            self.snapshot = None;
                            self.error = Some(error);
                        }
                    }
                }
                Event::Inspected(_, _) => continue,
                Event::Status(status) => {
                    self.querying_status = false;
                    match status {
                        Ok(status) => {
                            self.watch = Some(status);
                            self.client_error = None;
                        }
                        Err(error) => {
                            self.watch = None;
                            self.client_error = Some(error);
                        }
                    }
                }
                Event::Progress(OperationEvent::Output { stderr, text }) => {
                    self.activity.append_stream(stderr, &text);
                }
                Event::Progress(_) => continue,
                Event::Complete(result) => {
                    self.running = None;
                    self.activity.flush();
                    self.activity.append(&match result {
                        Ok(OperationOutcome::Succeeded) => "Completed".into(),
                        Ok(OperationOutcome::Cancelled) => "Cancelled".into(),
                        Ok(OperationOutcome::Failed { code }) => {
                            format!("Failed (exit code {code:?})")
                        }
                        Err(error) => format!("Could not complete operation: {error}"),
                    });
                    self.poll_status();
                    if !self.inspecting {
                        self.inspect();
                    }
                }
                Event::WatchLogs(result) => match result {
                    Ok(logs) => self.activity.append(&logs),
                    Err(error) => self
                        .activity
                        .append(&format!("Could not read Watch output: {error}")),
                },
            }
            changed = true;
        }
        let dropped = self.dropped.swap(0, Ordering::Relaxed);
        if dropped > 0 {
            self.activity.append(&format!(
                "{dropped} output chunks omitted while the queue was full."
            ));
            changed = true;
        }
        if self.last_poll.elapsed() >= Duration::from_secs(3) {
            self.last_poll = Instant::now();
            self.poll_status();
            if !self.inspecting {
                self.inspect();
            }
        }
        if self.preferences_dirty {
            self.save_preferences();
        }
        if changed {
            cx.notify();
        }
    }

    fn save_preferences(&mut self) {
        if self.preferences_path.is_some() {
            self.preferences_dirty = self
                .preferences_sender
                .try_send(self.preferences.clone())
                .is_err();
        }
    }

    fn remember(&mut self, root: PathBuf) {
        if self.preferences_path.is_none() {
            return;
        }
        self.preferences
            .recent_projects
            .retain(|path| path != &root);
        self.preferences.recent_projects.insert(0, root);
        self.preferences.recent_projects.truncate(12);
        self.save_preferences();
    }

    fn start(&mut self, operation: Operation, cx: &mut Context<Self>) {
        if self.running.is_some() {
            return;
        }
        let Some(client) = self.client.clone() else {
            return;
        };
        let project = if operation == Operation::StopWatch {
            self.watch
                .as_ref()
                .and_then(|status| status.project.clone())
                .or(self.project.clone())
        } else {
            self.project.clone()
        };
        let Some(project) = project else { return };
        let cancellation = Cancellation::default();
        self.running = Some((operation, cancellation.clone()));
        self.activity.append(&format!(
            "{} — {}",
            operation_label(operation),
            project.display()
        ));
        let sender = self.sender.clone();
        let progress = sender.clone();
        let dropped = self.dropped.clone();
        std::thread::spawn(move || {
            let result = client
                .run(operation, &project, &cancellation, move |event| {
                    if progress.try_send(Event::Progress(event)).is_err() {
                        dropped.fetch_add(1, Ordering::Relaxed);
                    }
                })
                .map_err(|error| format!("{error:#}"));
            let _ = sender.send(Event::Complete(result));
        });
        cx.notify();
    }
}

fn operation_label(operation: Operation) -> &'static str {
    match operation {
        Operation::StartWatch => "Starting Watch",
        Operation::StopWatch => "Stopping Watch",
        Operation::SwitchWatch => "Switching Watch",
        Operation::Check => "Running checks",
        Operation::Test => "Running tests",
    }
}
fn apply_appearance(appearance: Appearance, window: &mut Window, cx: &mut App) {
    match appearance {
        Appearance::System => Theme::sync_system_appearance(Some(window), cx),
        Appearance::Light => Theme::change(ThemeMode::Light, Some(window), cx),
        Appearance::Dark => Theme::change(ThemeMode::Dark, Some(window), cx),
    }
}
fn load_preferences(override_path: Option<PathBuf>) -> Result<(DesktopState, PathBuf), String> {
    (|| -> anyhow::Result<_> {
        let path = match override_path {
            Some(path) => path,
            None => DesktopState::path()?,
        };
        let mut state = DesktopState::load(&path)?;
        let executable = std::env::current_exe()?;
        if state.desktop_executable.as_ref() != Some(&executable) {
            state.desktop_executable = Some(executable);
            state.save(&path)?;
        }
        Ok((state, path))
    })()
    .map_err(|error| format!("{error:#}"))
}

#[cfg(test)]
mod tests {
    use super::{Activity, DesktopState, Event, Section, Workbench, load_preferences};
    use gpui_kit::test::TestWindowExt;
    use gpui_kit::{
        AppContext, Bounds, Point, TestAppContext, WindowBounds, WindowOptions, px, size,
    };

    #[test]
    fn matching_registration_preserves_preferences_bytes() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("desktop.json");
        let executable = std::env::current_exe().unwrap();
        let text = format!(
            "{{\n  \"schema_version\": 1, \"appearance\": \"dark\",\n  \"recent_projects\": [],\n  \"desktop_executable\": {},\n  \"extra_metadata\": {{\"retained\": true}}\n}}\n",
            serde_json::to_string(&executable).unwrap()
        );
        std::fs::write(&path, &text).unwrap();

        let (loaded, loaded_path) = load_preferences(Some(path.clone())).unwrap();

        assert_eq!(loaded_path, path);
        assert_eq!(loaded.desktop_executable, Some(executable));
        assert_eq!(std::fs::read(path).unwrap(), text.as_bytes());
    }

    #[test]
    fn moved_registration_updates_preferences() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("desktop.json");
        let previous = DesktopState {
            desktop_executable: Some(directory.path().join("previous-desktop.exe")),
            appearance: super::Appearance::Dark,
            recent_projects: vec![directory.path().join("project")],
            ..DesktopState::default()
        };
        previous.save(&path).unwrap();

        let (loaded, _) = load_preferences(Some(path.clone())).unwrap();
        let saved = DesktopState::load(&path).unwrap();

        assert_eq!(
            loaded.desktop_executable,
            Some(std::env::current_exe().unwrap())
        );
        assert_eq!(saved.desktop_executable, loaded.desktop_executable);
        assert_eq!(saved.appearance, previous.appearance);
        assert_eq!(saved.recent_projects, previous.recent_projects);
    }

    #[test]
    fn output_stays_bounded_and_retains_recent_lines() {
        let mut output = Activity::default();
        for ix in 0..10_000 {
            output.append(&format!("line {ix}"));
        }
        assert_eq!(output.lines.len(), 2_000);
        assert_eq!(output.lines.back().unwrap(), "line 9999");
        assert_eq!(output.omitted, 8_000);
        output.append(&"🦀".repeat(200_000));
        assert!(output.bytes <= super::LOG_BYTES);
    }

    #[test]
    fn streamed_chunks_preserve_searchable_lines_and_separate_stderr() {
        let mut output = Activity::default();
        output.append_stream(false, "test ");
        output.append_stream(true, "warning ");
        output.append_stream(false, "failed\r\nnext");
        output.append_stream(true, "details\n");
        assert_eq!(output.lines[0], "test failed");
        assert_eq!(output.lines[1], "[stderr] warning details");
        assert_eq!(output.displayed().last().unwrap(), "next");
        output.flush();
        assert_eq!(output.lines.back().unwrap(), "next");
    }

    #[test]
    fn long_stream_lines_keep_every_character_with_bounded_segments() {
        for stderr in [false, true] {
            let mut output = Activity::default();
            output.append_stream(stderr, &"a".repeat(40_000));
            output.flush();
            assert_eq!(
                output
                    .lines
                    .iter()
                    .map(|line| line.trim_start_matches("[stderr] ").len())
                    .sum::<usize>(),
                40_000
            );
            assert!(output.lines.iter().all(|line| line.len() <= 8_192));
        }
    }

    #[gpui_kit::test]
    fn inspection_recovers_and_dependencies_are_keyboard_accessible(cx: &mut TestAppContext) {
        let project = tempfile::tempdir().unwrap();
        std::fs::write(
            project.path().join("default.project.json"),
            r#"{"name":"Fixture","tree":{"$className":"DataModel"}}"#,
        )
        .unwrap();
        std::fs::write(
            project.path().join("wally.toml"),
            "[dependencies]\nSignal = \"sleitnick/signal@2.0.1\"\n",
        )
        .unwrap();
        let snapshot = rproj_core::inspect(project.path()).unwrap();
        let fingerprint = serde_json::to_string(&snapshot).unwrap();
        let preferences = project.path().join("desktop-state.json");
        let (handle, view) = cx.update(|cx| {
            gpui_kit::init(cx);
            gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds {
                        origin: Point::default(),
                        size: size(px(1120.), px(780.)),
                    })),
                    ..Default::default()
                },
                cx,
                |window, cx| cx.new(|cx| Workbench::new(None, Some(preferences), window, cx)),
            )
            .unwrap()
        });
        cx.update(|cx| {
            view.update(cx, |this, cx| {
                this.sender
                    .send(Event::Inspected(
                        0,
                        Box::new(Ok((snapshot.clone(), fingerprint.clone()))),
                    ))
                    .unwrap();
                this.drain(cx);
                assert!(this.snapshot.is_some());
                this.sender
                    .send(Event::Inspected(
                        0,
                        Box::new(Err("Project folder is unavailable".into())),
                    ))
                    .unwrap();
                this.drain(cx);
                assert!(this.snapshot.is_none());
                this.sender
                    .send(Event::Inspected(0, Box::new(Ok((snapshot, fingerprint)))))
                    .unwrap();
                this.drain(cx);
                assert!(
                    this.snapshot.is_some(),
                    "identical restored project must recover after an error"
                );
                assert!(this.error.is_none());
                this.preferences_error = Some("Existing invalid preferences preserved".into());
            })
        });
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            window.press("tab", cx);
            assert_eq!(window.find("open-project").focused(), Some(true));
            window.click("Dependencies", cx);
            window.click("dependencies:Signal", cx);
            assert_eq!(
                window.find("dependencies:Signal").label(),
                Some("Signal  ·  dependencies")
            );
            window.click("Checks", cx);
            window.click("run-checks", cx);
            window.click("run-tests", cx);
        })
        .unwrap();
        cx.update(|cx| {
            assert_eq!(
                view.read(cx).selected_package.as_deref(),
                Some("dependencies:Signal")
            );
            assert!(view.read(cx).section == Section::Checks);
            assert!(
                view.read(cx).running.is_none(),
                "unavailable actions must not launch an operation"
            );
            assert_eq!(
                view.read(cx).preferences_error.as_deref(),
                Some("Existing invalid preferences preserved")
            );
        });
    }
}
