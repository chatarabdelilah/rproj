use std::collections::BTreeSet;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::catalog::{
    artifacts, capabilities,
    wally_packages::{self, Category},
};
use crate::config::PackageWorkflow;
use crate::graph::{Node, ProjectGraph};
use crate::tui::{ConfirmState, InputState, PickerItem, PickerState};

use super::super::new;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Source,
    Strategy,
    Packages(Option<usize>),
    Capabilities,
    Implementation(&'static str),
    RepairTesting,
    Review,
    Files,
}

pub enum Modal {
    Help,
    Name(InputState),
    Setup(InputState),
    Discard(ConfirmState),
    Create(ConfirmState),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Effect {
    None,
    LoadSetup(String),
    Create,
    Cancel,
}

pub struct Draft {
    pub name: String,
    pub graph: ProjectGraph,
    pub step: Step,
    pub picker: PickerState<String>,
    pub checked: BTreeSet<String>,
    pub modal: Option<Modal>,
    pub status: String,
    pub details_focus: bool,
    pub scroll: u16,
    pub save_setup: Option<String>,
    pub apps: Vec<String>,
    pub extensions: Vec<String>,
    setups: Vec<String>,
    guided: bool,
    capabilities_answered: bool,
    capabilities_complete: bool,
    revision: Option<ProjectGraph>,
    pending_implementations: Vec<&'static str>,
}

fn item(key: &str, label: &str, detail: &str) -> PickerItem<String> {
    PickerItem {
        value: key.into(),
        label: label.into(),
        detail: detail.into(),
    }
}

impl Draft {
    pub fn new(
        name: &str,
        setups: Vec<String>,
        apps: Vec<String>,
        extensions: Vec<String>,
    ) -> Self {
        let mut draft = Self {
            name: name.into(),
            graph: ProjectGraph::default(),
            step: Step::Source,
            picker: PickerState::new(vec![]),
            checked: BTreeSet::new(),
            modal: None,
            status: String::new(),
            details_focus: false,
            scroll: 0,
            save_setup: None,
            apps,
            extensions,
            setups,
            guided: true,
            capabilities_answered: false,
            capabilities_complete: false,
            revision: None,
            pending_implementations: vec![],
        };
        draft.open(Step::Source);
        draft
    }

    pub fn title(&self) -> String {
        match self.step {
            Step::Source => "Composition".into(),
            Step::Strategy => "Dependencies".into(),
            Step::Packages(Some(index)) => Category::ALL[index].label().into(),
            Step::Packages(None) => "Packages".into(),
            Step::Capabilities => "Capabilities".into(),
            Step::Implementation(key) => format!("{key} implementation"),
            Step::RepairTesting => "Testing needs a decision".into(),
            Step::Review => "Review".into(),
            Step::Files => "Optional files".into(),
        }
    }

    pub fn multi(&self) -> bool {
        matches!(
            self.step,
            Step::Packages(None) | Step::Capabilities | Step::Files
        ) || matches!(self.step, Step::Packages(Some(i)) if Category::ALL[i].allows_multiple())
    }

    fn open(&mut self, step: Step) {
        self.step = step;
        self.scroll = 0;
        self.details_focus = false;
        self.checked.clear();
        let items = match step {
            Step::Source => {
                let mut items = vec![
                    item("guided", "Guided", "Choose packages by category"),
                    item("expert", "Expert", "Search the complete package list"),
                ];
                items.extend(self.setups.iter().map(|name| {
                    item(
                        &format!("saved:{name}"),
                        name,
                        "Saved setup; review before creating",
                    )
                }));
                items
            }
            Step::Strategy => vec![
                item(
                    "wally",
                    "Wally",
                    "Package manager; supports every catalog package",
                ),
                item(
                    "git",
                    "Git submodules",
                    "Vendor supported dependencies as source",
                ),
                item("none", "None", "No dependency manager"),
            ],
            Step::Packages(category) => {
                self.checked = self.graph.package_set();
                let mut items: Vec<_> = wally_packages::PACKAGES
                    .iter()
                    .filter(|p| new::offerable_package(p, self.graph.package_workflow))
                    .filter(|p| {
                        category.is_none_or(|i| p.primary_choice && p.category == Category::ALL[i])
                    })
                    .map(|p| {
                        item(
                            p.key,
                            p.key,
                            &format!(
                                "{} [{}] ({})",
                                p.description,
                                p.category.label(),
                                p.maintenance.short_badge()
                            ),
                        )
                    })
                    .collect();
                if category.is_some_and(|i| !Category::ALL[i].allows_multiple()) {
                    items.insert(0, item("", "None", "No package in this category"));
                }
                items
            }
            Step::Capabilities => {
                self.checked = if self.capabilities_answered {
                    self.graph.capability_keys().into_iter().collect()
                } else {
                    capabilities::CAPABILITIES
                        .iter()
                        .filter(|c| c.default_selected)
                        .map(|c| c.key.into())
                        .collect()
                };
                capabilities::CAPABILITIES
                    .iter()
                    .map(|c| item(c.key, c.key, c.outcome))
                    .collect()
            }
            Step::Implementation(key) => {
                let mut implementations = capabilities::find(key)
                    .unwrap()
                    .implementations_for(self.graph.package_workflow);
                implementations.sort_by_key(|i| i.display);
                implementations
                    .iter()
                    .map(|i| item(i.key, i.display, key))
                    .collect()
            }
            Step::RepairTesting => vec![
                item(
                    "testez",
                    "TestEZ",
                    "Keep Testing with the compatible runner",
                ),
                item(
                    "off",
                    "Disable Testing",
                    "Remove only the Testing capability",
                ),
            ],
            Step::Review => vec![
                item(
                    "create",
                    "Create",
                    "Create exactly this reviewed composition",
                ),
                item(
                    "strategy",
                    "Dependencies",
                    "Changing strategy reopens package selection",
                ),
                item("packages", "Packages", "Revise package selection"),
                item(
                    "capabilities",
                    "Capabilities",
                    "Revise capabilities and implementations",
                ),
                item(
                    "files",
                    "Optional files",
                    "Keep required wiring; omit optional artifacts",
                ),
                item("name", "Project name", "Change the destination name"),
                item(
                    "setup",
                    "Save named setup",
                    "Save this composition only when creating",
                ),
                item("cancel", "Cancel", "Create nothing"),
            ],
            Step::Files => self
                .graph
                .full_plan(&self.apps, &self.extensions)
                .into_iter()
                .filter(|p| {
                    artifacts::ARTIFACTS
                        .iter()
                        .any(|a| a.key == p.key && a.droppable())
                })
                .map(|p| {
                    if !self.graph.dropped.iter().any(|k| k == p.key) {
                        self.checked.insert(p.key.into());
                    }
                    item(p.key, p.key, &p.reason.describe())
                })
                .collect(),
        };
        self.picker = PickerState::new(items);
        let selected = match step {
            Step::Strategy => Some(match self.graph.package_workflow {
                PackageWorkflow::Wally => "wally",
                PackageWorkflow::GitSubmodules => "git",
                PackageWorkflow::None => "none",
            }),
            Step::Implementation(key) => self.graph.capabilities.get(key).map(String::as_str),
            Step::Packages(Some(i)) if !Category::ALL[i].allows_multiple() => self
                .picker
                .items
                .iter()
                .find(|p| self.checked.contains(&p.value))
                .map(|p| p.value.as_str()),
            _ => None,
        };
        if let Some(value) = selected {
            self.picker.selected = self
                .picker
                .items
                .iter()
                .position(|p| p.value == value)
                .unwrap_or(0);
        }
        crate::diagnostics::event("creation.screen", format!("{step:?}"));
    }

    pub fn loaded(&mut self, name: &str, mut graph: ProjectGraph, warnings: Vec<String>) {
        graph.mode = format!("like:{name}");
        self.guided = false;
        self.graph = graph;
        self.capabilities_answered = true;
        self.capabilities_complete = true;
        self.review();
        self.status = warnings.join("\n");
    }

    fn review(&mut self) {
        new::apply_derived_packages(&mut self.graph);
        self.revision = None;
        self.open(Step::Review);
    }

    fn packages(&mut self, from: usize) {
        if from == 0 {
            self.graph.mode = if self.graph.package_workflow == PackageWorkflow::None {
                "none"
            } else if self.guided {
                "guided"
            } else {
                "expert"
            }
            .into();
            if self.guided {
                self.graph
                    .packages
                    .retain(|key| wally_packages::find(key).is_some_and(|p| p.primary_choice));
            }
        }
        if self.graph.package_workflow != PackageWorkflow::None {
            if !self.guided {
                self.open(Step::Packages(None));
                return;
            }
            for index in from..Category::ALL.len() {
                if wally_packages::PACKAGES.iter().any(|p| {
                    p.primary_choice
                        && p.category == Category::ALL[index]
                        && new::offerable_package(p, self.graph.package_workflow)
                }) {
                    self.open(Step::Packages(Some(index)));
                    return;
                }
            }
        }
        self.after_packages();
    }

    fn after_packages(&mut self) {
        let mut packages = self.graph.package_set();
        if self.guided {
            new::add_companions(&mut packages);
        }
        if self.graph.package_workflow == PackageWorkflow::GitSubmodules {
            let blocked = wally_packages::unvendorable_in_closure(&packages);
            if !blocked.is_empty() {
                self.status = format!(
                    "Cannot vendor {} with Git submodules. Revise packages or choose Wally.",
                    blocked
                        .iter()
                        .map(|(k, via)| via.unwrap_or(k))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                self.open(Step::Packages(None));
                return;
            }
            packages = wally_packages::with_dependencies(&packages);
        }
        self.graph.packages = packages.into_iter().collect();
        if !self.graph.testing_is_compatible() {
            self.status = "Jest Roblox requires Wally. Other capabilities are unchanged.".into();
            self.open(Step::RepairTesting);
        } else if self.capabilities_complete {
            self.review();
        } else {
            self.open(Step::Capabilities);
        }
    }

    fn next_implementation(&mut self) {
        if self.pending_implementations.is_empty() {
            self.capabilities_complete = true;
            self.review();
        } else {
            let key = self.pending_implementations.remove(0);
            self.open(Step::Implementation(key));
        }
    }

    pub fn key(&mut self, key: KeyEvent) -> Effect {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Effect::Cancel;
        }
        if self.modal.is_some() {
            return self.modal_key(key);
        }
        match key.code {
            KeyCode::Char('?') => self.modal = Some(Modal::Help),
            KeyCode::Tab | KeyCode::BackTab => self.details_focus = !self.details_focus,
            KeyCode::PageDown => self.scroll = self.scroll.saturating_add(8),
            KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(8),
            KeyCode::Down if self.details_focus => self.scroll = self.scroll.saturating_add(1),
            KeyCode::Up if self.details_focus => self.scroll = self.scroll.saturating_sub(1),
            KeyCode::Esc => self.back(),
            KeyCode::Char(' ') if self.multi() && !self.details_focus => {
                if let Some(value) = self.picker.selected_value().cloned()
                    && !self.checked.remove(&value)
                {
                    self.checked.insert(value);
                }
            }
            KeyCode::Enter if !self.details_focus => return self.accept(),
            _ if !self.details_focus => {
                self.picker.handle_key(key);
            }
            _ => {}
        }
        Effect::None
    }

    fn back(&mut self) {
        if let Some(graph) = self.revision.take() {
            self.graph = graph;
            self.pending_implementations.clear();
            self.capabilities_complete = true;
            self.review();
            self.status = "Revision cancelled.".into();
            return;
        }
        match self.step {
            Step::Source | Step::Review => {
                self.modal = Some(Modal::Discard(ConfirmState::new(
                    "Discard this draft? No project or setup will be created.",
                )))
            }
            Step::Strategy => self.open(Step::Source),
            Step::Packages(Some(i)) if i > 0 => {
                let previous = (0..i).rev().find(|&index| {
                    wally_packages::PACKAGES.iter().any(|p| {
                        p.primary_choice
                            && p.category == Category::ALL[index]
                            && new::offerable_package(p, self.graph.package_workflow)
                    })
                });
                self.open(previous.map_or(Step::Strategy, |i| Step::Packages(Some(i))));
            }
            Step::Packages(_) | Step::RepairTesting => self.open(Step::Strategy),
            Step::Capabilities => self.open(Step::Strategy),
            Step::Implementation(_) => self.open(Step::Capabilities),
            Step::Files => self.review(),
        }
    }

    fn accept(&mut self) -> Effect {
        let value = self.picker.selected_value().cloned().unwrap_or_default();
        if !self.multi() && self.picker.selected_value().is_none() {
            return Effect::None;
        }
        self.status.clear();
        crate::diagnostics::event(
            "creation.accept",
            format!(
                "step={:?}; choice={value}; checked={:?}",
                self.step, self.checked
            ),
        );
        match self.step {
            Step::Source => {
                if let Some(name) = value.strip_prefix("saved:") {
                    return Effect::LoadSetup(name.into());
                }
                self.guided = value == "guided";
                self.graph = ProjectGraph::default();
                self.capabilities_answered = false;
                self.capabilities_complete = false;
                self.graph.mode = value;
                self.open(Step::Strategy);
            }
            Step::Strategy => {
                let workflow = match value.as_str() {
                    "git" => PackageWorkflow::GitSubmodules,
                    "none" => PackageWorkflow::None,
                    _ => PackageWorkflow::Wally,
                };
                if workflow != self.graph.package_workflow {
                    self.graph.invalidate(Node::Strategy);
                }
                self.graph.package_workflow = workflow;
                self.packages(0);
            }
            Step::Packages(category) => {
                let mut packages = self.graph.package_set();
                for p in &self.picker.items {
                    packages.remove(&p.value);
                }
                if self.multi() {
                    packages.extend(
                        self.checked
                            .iter()
                            .filter(|key| self.picker.items.iter().any(|p| &p.value == *key))
                            .cloned(),
                    );
                } else if !value.is_empty() {
                    packages.insert(value);
                }
                let next: Vec<_> = packages.into_iter().collect();
                if next != self.graph.packages {
                    self.graph.invalidate(Node::Packages);
                }
                self.graph.packages = next;
                if let Some(i) = category {
                    self.packages(i + 1);
                } else {
                    self.after_packages();
                }
            }
            Step::Capabilities => {
                for capability in capabilities::CAPABILITIES {
                    if self.checked.contains(capability.key)
                        && capability
                            .requires
                            .iter()
                            .any(|key| !self.checked.contains(*key))
                    {
                        self.status = format!(
                            "{} requires {}. Select its prerequisite or disable it.",
                            capability.key,
                            capability.requires.join(", ")
                        );
                        return Effect::None;
                    }
                }
                let previous = self.graph.capabilities.clone();
                self.graph
                    .capabilities
                    .retain(|key, _| self.checked.contains(key));
                self.pending_implementations.clear();
                self.capabilities_complete = false;
                for capability in capabilities::CAPABILITIES
                    .iter()
                    .filter(|c| self.checked.contains(c.key))
                {
                    if capability.needs_an_implementation_prompt(self.graph.package_workflow) {
                        if !self.graph.capabilities.contains_key(capability.key) {
                            self.graph.choose(capability.key, None);
                        }
                        self.pending_implementations.push(capability.key);
                    } else if let Some(implementation) = capability
                        .implementations_for(self.graph.package_workflow)
                        .first()
                    {
                        self.graph.choose(capability.key, Some(implementation.key));
                    }
                }
                if previous != self.graph.capabilities {
                    self.graph.invalidate(Node::Capabilities);
                }
                self.capabilities_answered = true;
                self.next_implementation();
            }
            Step::Implementation(key) => {
                if self.graph.capabilities.get(key) != Some(&value) {
                    self.graph.invalidate(Node::Capabilities);
                }
                self.graph.choose(key, Some(&value));
                self.next_implementation();
            }
            Step::RepairTesting => {
                self.graph.remove_incompatible_testing();
                if value == "testez" {
                    self.graph.choose("test", Some("testez"));
                }
                self.review();
            }
            Step::Files => {
                self.graph.dropped = self
                    .picker
                    .items
                    .iter()
                    .filter(|p| !self.checked.contains(&p.value))
                    .map(|p| p.value.clone())
                    .collect();
                self.review();
            }
            Step::Review => match value.as_str() {
                "create" => {
                    self.modal = Some(Modal::Create(ConfirmState::new(format!(
                        "Create {} with the reviewed files?",
                        self.name
                    ))))
                }
                "name" => self.modal = Some(Modal::Name(InputState::new(&self.name))),
                "setup" => {
                    self.modal = Some(Modal::Setup(InputState::new(
                        self.save_setup.as_deref().unwrap_or(""),
                    )))
                }
                "cancel" => self.back(),
                "files" => self.open(Step::Files),
                "strategy" | "packages" | "capabilities" => {
                    self.revision = Some(self.graph.clone());
                    match value.as_str() {
                        "strategy" => self.open(Step::Strategy),
                        "packages" => self.packages(0),
                        _ => self.open(Step::Capabilities),
                    }
                }
                _ => {}
            },
        }
        Effect::None
    }

    fn modal_key(&mut self, key: KeyEvent) -> Effect {
        let modal = self.modal.take().unwrap();
        let is_name = matches!(modal, Modal::Name(_));
        if key.code == KeyCode::Esc {
            return Effect::None;
        }
        let confirm = matches!(
            key.code,
            KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y')
        );
        match modal {
            Modal::Help => {
                if key.code != KeyCode::Char('?') {
                    self.modal = Some(Modal::Help);
                }
            }
            Modal::Discard(_) if confirm => return Effect::Cancel,
            Modal::Create(_) if confirm => return Effect::Create,
            Modal::Discard(_) | Modal::Create(_)
                if matches!(key.code, KeyCode::Char('n') | KeyCode::Char('N')) => {}
            Modal::Name(mut input) | Modal::Setup(mut input) => {
                if key.code == KeyCode::Enter {
                    let value = input.text().trim();
                    if !is_name && value.is_empty() {
                        self.save_setup = None;
                        return Effect::None;
                    }
                    if let Err(error) = validate_name(value) {
                        input.error = Some(error.into());
                    } else if !is_name
                        && self
                            .setups
                            .iter()
                            .any(|name| name.eq_ignore_ascii_case(value))
                    {
                        input.error =
                            Some("That setup exists; choose a new name to preserve it.".into());
                    } else {
                        if is_name {
                            self.name = value.into();
                        } else {
                            self.save_setup = Some(value.into());
                        }
                        return Effect::None;
                    }
                } else {
                    input.handle_key(key);
                }
                self.modal = Some(if is_name {
                    Modal::Name(input)
                } else {
                    Modal::Setup(input)
                });
            }
            modal => self.modal = Some(modal),
        }
        Effect::None
    }

    pub fn paste(&mut self, text: &str) {
        let input = match &mut self.modal {
            Some(Modal::Name(input) | Modal::Setup(input)) => input,
            None if !self.details_focus => &mut self.picker.query,
            _ => return,
        };
        for ch in text.chars().filter(|ch| !ch.is_control()) {
            input.handle_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
        }
        self.picker.selected = 0;
    }
}

pub fn validate_name(name: &str) -> Result<(), &'static str> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.ends_with([' ', '.'])
        || name
            .chars()
            .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
    {
        return Err("Use a single folder name without reserved path characters.");
    }
    let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    if ["CON", "PRN", "AUX", "NUL"].contains(&stem.as_str())
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && matches!(stem.as_bytes()[3], b'1'..=b'9'))
    {
        return Err("This name is reserved by Windows.");
    }
    Ok(())
}
