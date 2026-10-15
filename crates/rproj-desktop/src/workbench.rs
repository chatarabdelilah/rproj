mod operation;
mod theme;
#[cfg(test)]
mod ui_tests;
mod view;
use operation::{OperationPresentation, RunStatus, WatchSwitch};

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
    fn icon(self) -> gpui_kit::assets::IconName {
        use gpui_kit::assets::IconName;
        match self {
            Self::Overview => IconName::LayoutDashboard,
            Self::Dependencies => IconName::Package,
            Self::Tools => IconName::Wrench,
            Self::Checks => IconName::FlaskConical,
        }
    }
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
    Connected(
        Result<WorkbenchClient, String>,
        Option<Result<Vec<CatalogEntry>, String>>,
    ),
    Preferences(Result<(DesktopState, PathBuf), String>),
    PreferenceError(String),
    Inspected(u64, Box<Result<(ProjectSnapshot, String), String>>),
    Status(Result<WatchStatus, String>),
    Progress(u64, OperationEvent),
    Complete(u64, Result<OperationOutcome, String>),
    WatchLogs(u64, Result<String, String>),
}

fn connection_event(worker: anyhow::Result<PathBuf>) -> Event {
    let client = worker
        .and_then(WorkbenchClient::new)
        .map_err(|error| format!("{error:#}"));
    let catalog = client.as_ref().ok().map(|client| {
        client
            .catalog()
            .map_err(|error| format!("Could not load package catalog: {error:#}"))
    });
    Event::Connected(client, catalog)
}

#[derive(Default)]
struct WatchLogRequest {
    context: u64,
    next_id: u64,
    pending: Option<(u64, u64)>,
}

impl WatchLogRequest {
    fn begin(&mut self) -> Option<u64> {
        if self.pending.is_some() {
            return None;
        }
        self.next_id = self.next_id.wrapping_add(1);
        self.pending = Some((self.next_id, self.context));
        Some(self.next_id)
    }

    fn invalidate(&mut self) {
        self.context = self.context.wrapping_add(1);
        // Keep the guard until the old worker finishes, even across project switches.
    }

    fn finish(&mut self, id: u64) -> bool {
        let Some((pending_id, context)) = self.pending else {
            return false;
        };
        if pending_id != id {
            return false;
        }
        self.pending = None;
        context == self.context
    }
}

#[derive(Default)]
struct Activity {
    lines: VecDeque<String>,
    origins: VecDeque<Option<Arc<str>>>,
    origin: Option<Arc<str>>,
    bytes: usize,
    omitted: usize,
    partial: [String; 2],
}
impl Activity {
    fn set_origin(&mut self, origin: Option<String>) {
        if self.origin.as_deref() != origin.as_deref() {
            self.flush();
            self.origin = origin.map(Arc::from);
        }
    }

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
        self.lines
            .iter()
            .zip(&self.origins)
            .map(|(line, origin)| {
                origin
                    .as_ref()
                    .map(|origin| format!("[{origin}] {line}"))
                    .unwrap_or_else(|| line.clone())
            })
            .chain(
                self.partial
                    .iter()
                    .enumerate()
                    .filter(|(_, text)| !text.is_empty())
                    .map(|(stream, text)| {
                        let text = if stream == 1 {
                            format!("[stderr] {text}")
                        } else {
                            text.clone()
                        };
                        self.origin
                            .as_ref()
                            .map(|origin| format!("[{origin}] {text}"))
                            .unwrap_or(text)
                    }),
            )
    }
    fn append(&mut self, text: &str) {
        for line in text.lines() {
            let line: String = line.chars().take(8_192).collect();
            self.bytes += line.len();
            self.lines.push_back(line);
            self.origins.push_back(self.origin.clone());
            while self.lines.len() > LOG_CAPACITY || self.bytes > LOG_BYTES {
                if let Some(line) = self.lines.pop_front() {
                    self.origins.pop_front();
                    self.bytes -= line.len();
                    self.omitted += 1;
                }
            }
        }
    }
}

pub struct Workbench {
    focus: FocusHandle,
    watch_focus: FocusHandle,
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
    catalog_error: Option<String>,
    watch: Option<WatchStatus>,
    querying_status: bool,
    watch_log_request: WatchLogRequest,
    running: Option<(Operation, Cancellation)>,
    operations: OperationPresentation,
    pending_switch: Option<WatchSwitch>,
    watch_generation: u64,
    activity_height: Pixels,
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
        theme::install(cx);
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
            watch_focus: cx.focus_handle(),
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
            catalog_error: None,
            watch: None,
            querying_status: false,
            watch_log_request: WatchLogRequest::default(),
            running: None,
            operations: OperationPresentation::default(),
            pending_switch: None,
            watch_generation: 0,
            activity_height: px(180.),
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
            let _ = sender.send(connection_event(worker_path()));
        });
        if let Some(path) = initial {
            view.open_project(path, cx);
        }
        view
    }

    fn open_project(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.pending_switch = None;
        self.watch_log_request.invalidate();
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

    fn close_project(&mut self, cx: &mut Context<Self>) {
        self.pending_switch = None;
        self.watch_log_request.invalidate();
        self.generation = self.generation.wrapping_add(1);
        self.project = None;
        self.snapshot = None;
        self.selected_package = None;
        self.inspecting = false;
        self.error = None;
        self.inspection_fingerprint.clear();
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

    fn request_watch_logs(&mut self, cx: &mut Context<Self>) {
        if self.changing_watch() {
            return;
        }
        let Some(client) = self.client.clone() else {
            return;
        };
        let Some(request) = self.watch_log_request.begin() else {
            return;
        };
        self.operations.expanded = true;
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let result = client.watch_logs().map_err(|error| format!("{error:#}"));
            let _ = sender.send(Event::WatchLogs(request, result));
        });
        cx.notify();
    }

    fn changing_watch(&self) -> bool {
        self.running
            .as_ref()
            .is_some_and(|(operation, _)| !operation.cancellable())
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
                    self.operations.expanded = true;
                    self.activity.append(&error);
                    self.preferences_error = Some(error);
                }
                Event::Connected(client, catalog) => {
                    self.watch_log_request.invalidate();
                    self.watch_generation = self.watch_generation.wrapping_add(1);
                    self.pending_switch = None;
                    match catalog {
                        Some(Ok(catalog)) => {
                            self.catalog = catalog;
                            self.catalog_error = None;
                        }
                        Some(Err(error)) => self.catalog_error = Some(error),
                        None => {}
                    }
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
                    if self
                        .watch
                        .as_ref()
                        .map(|watch| (&watch.project, watch.state))
                        != status
                            .as_ref()
                            .ok()
                            .map(|watch| (&watch.project, watch.state))
                    {
                        self.watch_log_request.invalidate();
                        self.watch_generation = self.watch_generation.wrapping_add(1);
                        self.pending_switch = None;
                        if status.as_ref().is_ok_and(|watch| {
                            matches!(
                                watch.state,
                                rproj_core::operations::WatchState::Failed
                                    | rproj_core::operations::WatchState::Unresponsive
                            )
                        }) {
                            self.operations.expanded = true;
                        }
                    }
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
                Event::Progress(id, OperationEvent::Output { stderr, text })
                    if self.operations.accepts(id) =>
                {
                    self.activity
                        .set_origin(self.operations.latest.as_ref().map(|run| {
                            format!(
                                "{} · {}",
                                run.project.display(),
                                operation_label(run.operation)
                            )
                        }));
                    self.activity.append_stream(stderr, &text);
                }
                Event::Progress(_, _) => continue,
                Event::Complete(id, result) => {
                    if !self.operations.finish(id, &result) {
                        continue;
                    }
                    self.running = None;
                    self.activity
                        .set_origin(self.operations.latest.as_ref().map(|run| {
                            format!(
                                "{} · {}",
                                run.project.display(),
                                operation_label(run.operation)
                            )
                        }));
                    self.activity.flush();
                    self.activity.append(&self.operations.summary());
                    self.activity.set_origin(None);
                    self.poll_status();
                    if !self.inspecting {
                        self.inspect();
                    }
                }
                Event::WatchLogs(request, result) => {
                    if self.watch_log_request.finish(request) {
                        match result {
                            Ok(logs) => {
                                let origin = self
                                    .watch
                                    .as_ref()
                                    .and_then(|watch| watch.project.as_ref())
                                    .map(|path| path.display().to_string())
                                    .unwrap_or_else(|| "No active project".into());
                                self.activity.flush();
                                self.activity.set_origin(Some(format!("{origin} · Watch")));
                                self.activity.append(&format!("Watch output — {origin}"));
                                self.activity.append(if logs.is_empty() {
                                    "No Watch output yet."
                                } else {
                                    &logs
                                });
                                self.activity.set_origin(None);
                            }
                            Err(error) => {
                                self.operations.expanded = true;
                                self.activity
                                    .append(&format!("Could not read Watch output: {error}"));
                            }
                        }
                    }
                }
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
        let project = if operation == Operation::StopWatch {
            self.watch
                .as_ref()
                .and_then(|status| status.project.clone())
                .or(self.project.clone())
        } else {
            self.project.clone()
        };
        let Some(project) = project else { return };
        self.start_for(operation, project, cx);
    }

    fn confirm_watch_switch(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(from) = self.watch.as_ref().and_then(|watch| watch.project.clone()) else {
            return;
        };
        let Some(to) = self.project.clone() else {
            return;
        };
        if self.running.is_some() || from == to {
            return;
        }
        let pending = WatchSwitch {
            from,
            to,
            generation: self.watch_generation,
        };
        if !pending.matches(
            self.project.as_deref(),
            self.watch.as_ref(),
            self.watch_generation,
        ) {
            return;
        }
        self.pending_switch = Some(pending.clone());
        // Component Button owns its focus handle and suppresses pointer focus.
        // Enter its native tab stop through the preceding, non-tab-stop anchor.
        self.watch_focus.focus(window, cx);
        window.focus_next(cx);
        let owner = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |dialog, _, _| {
            let confirm = owner.clone();
            let close = owner.clone();
            let pending = pending.clone();
            dialog.confirm().title("Switch the active Watch?")
                .child(div().v_flex().gap_3()
                    .child(format!("Stop: {}", pending.from.display()))
                    .child(format!("Then start: {}", pending.to.display()))
                    .child("The new Watch starts only after the current Watch acknowledges Stop."))
                .button_props(dialog::DialogButtonProps::default().ok_text("Stop and switch").cancel_text("Keep current Watch").show_cancel(true))
                .on_ok(move |_, _, cx| {
                    let _ = confirm.update(cx, |this, cx| {
                        let valid = this.pending_switch.as_ref().is_some_and(|current|
                            current.from == pending.from && current.to == pending.to
                                && current.generation == pending.generation
                                && current.matches(this.project.as_deref(), this.watch.as_ref(), this.watch_generation));
                        if valid && this.running.is_none() {
                            this.start_for(Operation::SwitchWatch, pending.to.clone(), cx);
                        } else {
                            this.operations.expanded = true;
                            this.activity.append("Watch switch expired because the project or Watch changed. Open Switch Watch again.");
                            cx.notify();
                        }
                    });
                    true
                })
                .on_close(move |_, _, cx| { let _ = close.update(cx, |this, _| this.pending_switch = None); })
        });
    }

    fn start_for(&mut self, operation: Operation, project: PathBuf, cx: &mut Context<Self>) {
        if self.running.is_some() {
            return;
        }
        let Some(client) = self.client.clone() else {
            return;
        };
        self.pending_switch = None;
        if !operation.cancellable() {
            self.watch_log_request.invalidate();
        }
        let cancellation = Cancellation::default();
        let id = self.operations.begin(operation, project.clone());
        self.running = Some((operation, cancellation.clone()));
        self.activity.flush();
        self.activity.set_origin(Some(format!(
            "{} · {}",
            project.display(),
            operation_label(operation)
        )));
        let sender = self.sender.clone();
        let progress = sender.clone();
        let dropped = self.dropped.clone();
        std::thread::spawn(move || {
            let result = client
                .run(operation, &project, &cancellation, move |event| {
                    if progress.try_send(Event::Progress(id, event)).is_err() {
                        dropped.fetch_add(1, Ordering::Relaxed);
                    }
                })
                .map_err(|error| format!("{error:#}"));
            let _ = sender.send(Event::Complete(id, result));
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
    use super::{
        Activity, DesktopState, Event, Section, Workbench, connection_event, load_preferences,
    };
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
    fn output_origins_survive_eviction_and_interleaved_sources() {
        let mut output = Activity::default();
        output.set_origin(Some("first project · Check".into()));
        output.append_stream(false, "partial");
        output.set_origin(Some("other project · Watch".into()));
        output.append("watch line");
        assert!(
            output
                .displayed()
                .next()
                .unwrap()
                .starts_with("[first project · Check] partial")
        );
        assert!(
            output
                .displayed()
                .last()
                .unwrap()
                .starts_with("[other project · Watch] watch line")
        );
        for _ in 0..super::LOG_CAPACITY {
            output.append("bounded output");
        }
        assert_eq!(output.origins.len(), output.lines.len());
        assert!(
            output
                .displayed()
                .all(|line| line.starts_with("[other project · Watch]"))
        );
        output.set_origin(None);
        output.append("workbench message");
        assert_eq!(output.displayed().last().unwrap(), "workbench message");
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
    fn watch_output_is_bounded_and_rejects_stale_contexts(cx: &mut TestAppContext) {
        let fixture = tempfile::tempdir().unwrap();
        let preferences = fixture.path().join("desktop-state.json");
        let (_, view) = cx.update(|cx| {
            gpui_kit::init(cx);
            gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
                cx.new(|cx| Workbench::new(None, Some(preferences), window, cx))
            })
            .unwrap()
        });
        cx.update(|cx| {
            view.update(cx, |this, cx| {
                (this.sender, this.receiver) = std::sync::mpsc::sync_channel(super::EVENT_CAPACITY);
                let first = this.watch_log_request.begin().unwrap();
                assert!(this.watch_log_request.begin().is_none());
                this.open_project(fixture.path().to_path_buf(), cx);
                assert!(
                    this.watch_log_request.begin().is_none(),
                    "switching projects must not release an unfinished worker"
                );
                this.sender
                    .send(Event::WatchLogs(first, Ok("old project output".into())))
                    .unwrap();
                this.drain(cx);
                assert!(this.activity.lines.is_empty());

                let second = this.watch_log_request.begin().unwrap();
                this.sender
                    .send(Event::Connected(Err("Worker replaced".into()), None))
                    .unwrap();
                this.sender
                    .send(Event::WatchLogs(second, Err("old worker error".into())))
                    .unwrap();
                this.drain(cx);
                assert!(this.activity.lines.is_empty());

                let third = this.watch_log_request.begin().unwrap();
                this.sender
                    .send(Event::Status(Ok(super::WatchStatus {
                        supported: true,
                        project: Some(fixture.path().to_path_buf()),
                        state: rproj_core::operations::WatchState::Watching,
                        message: "Watching another project".into(),
                    })))
                    .unwrap();
                this.sender
                    .send(Event::WatchLogs(third, Ok("old Watch output".into())))
                    .unwrap();
                this.drain(cx);
                assert!(this.activity.lines.is_empty());

                let current = this.watch_log_request.begin().unwrap();
                this.sender
                    .send(Event::WatchLogs(first, Ok("duplicate stale event".into())))
                    .unwrap();
                this.drain(cx);
                assert!(this.watch_log_request.begin().is_none());
                this.sender
                    .send(Event::WatchLogs(current, Ok("current output".into())))
                    .unwrap();
                this.drain(cx);
                assert_eq!(this.activity.lines.back().unwrap(), "current output");
                assert_eq!(this.activity.lines.len(), 2);
                assert!(
                    this.activity
                        .lines
                        .front()
                        .unwrap()
                        .contains(&fixture.path().display().to_string())
                );
                assert!(this.watch_log_request.pending.is_none());
            });
        });
    }

    #[gpui_kit::test]
    fn catalog_failure_remains_visible_after_connection_and_inspection(cx: &mut TestAppContext) {
        let fixture = tempfile::tempdir().unwrap();
        let source = fixture.path().join("worker.rs");
        std::fs::write(
            &source,
            format!(
                r#"fn main() {{
                    if std::env::args().nth(1).as_deref() == Some("--version") {{
                        println!("rproj {}");
                    }} else {{
                        println!("invalid catalog response");
                    }}
                }}"#,
                env!("CARGO_PKG_VERSION")
            ),
        )
        .unwrap();
        let worker = fixture
            .path()
            .join(format!("worker{}", std::env::consts::EXE_SUFFIX));
        let compiled = std::process::Command::new("rustc")
            .arg(source)
            .arg("-o")
            .arg(&worker)
            .output()
            .unwrap();
        assert!(
            compiled.status.success(),
            "{}",
            String::from_utf8_lossy(&compiled.stderr)
        );
        let connected = connection_event(Ok(worker));
        assert!(matches!(&connected, Event::Connected(Ok(_), Some(Err(_)))));
        std::fs::write(
            fixture.path().join("default.project.json"),
            r#"{"name":"Catalog fixture","tree":{"$className":"DataModel"}}"#,
        )
        .unwrap();
        let snapshot = rproj_core::inspect(fixture.path()).unwrap();
        let fingerprint = serde_json::to_string(&snapshot).unwrap();
        let preferences = fixture.path().join("desktop-state.json");
        let (handle, view) = cx.update(|cx| {
            gpui_kit::init(cx);
            gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
                cx.new(|cx| Workbench::new(None, Some(preferences), window, cx))
            })
            .unwrap()
        });
        cx.update(|cx| {
            view.update(cx, |this, cx| {
                // Isolate this event sequence from startup background workers.
                (this.sender, this.receiver) = std::sync::mpsc::sync_channel(super::EVENT_CAPACITY);
                this.querying_status = true;
                this.sender.send(connected).unwrap();
                this.drain(cx);
                assert!(
                    this.client.is_some(),
                    "catalog failure must keep the matching worker"
                );
                assert!(this.client_error.is_none());
                let error = this
                    .catalog_error
                    .clone()
                    .expect("catalog failure is visible");
                assert!(error.contains("invalid package catalog"));
                this.sender
                    .send(Event::Inspected(0, Box::new(Ok((snapshot, fingerprint)))))
                    .unwrap();
                this.sender
                    .send(Event::Status(Ok(super::WatchStatus {
                        supported: true,
                        project: None,
                        state: rproj_core::operations::WatchState::Stopped,
                        message: "Stopped".into(),
                    })))
                    .unwrap();
                this.drain(cx);
                assert!(this.snapshot.is_some());
                assert!(this.error.is_none());
                assert!(this.client_error.is_none());
                assert_eq!(this.catalog_error.as_deref(), Some(error.as_str()));
            });
        });
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            let alert = window.find("catalog-error");
            assert!(alert.visible());
            assert!(alert.label().unwrap().contains("invalid package catalog"));
        })
        .unwrap();
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
            assert_eq!(window.find("project-switcher").focused(), Some(true));
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
