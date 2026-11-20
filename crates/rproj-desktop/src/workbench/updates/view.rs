use super::*;
use gpui_kit::{component::button::*, prelude::FluentBuilder};

fn timestamp(time: u64) -> String {
    i64::try_from(time)
        .ok()
        .and_then(|seconds| chrono::DateTime::from_timestamp(seconds, 0))
        .map(|date| date.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| format!("{time} Unix seconds"))
}
fn comparison_text(local: &Local, c: &Comparison) -> String {
    let latest = c.latest_stable.as_deref().unwrap_or("unknown");
    let mut text = format!("{} · Latest stable: {latest}", c.status.label());
    if let Local::Package { .. } = local {
        let within = c.within_requirement.as_deref().unwrap_or("none known");
        text.push_str(&format!(" · Within requirement: {within}"));
        if c.latest_stable.is_some() && c.latest_stable != c.within_requirement {
            text.push_str(&format!(
                ". Moving to {latest} requires changing the declaration."
            ));
        }
    }
    if let Some(reason) = &c.reason {
        text.push_str(&format!(" · {reason}"));
    }
    text
}
fn details(local: &Local) -> Vec<String> {
    match local {
        Local::Package {
            realm,
            declaration,
            package,
            locked,
            presence,
            registry,
            ..
        } => vec![
            format!("Package · {realm}"),
            format!("Declared: {declaration}"),
            format!("Package: {}", package.as_deref().unwrap_or("unknown")),
            format!("Registry: {}", registry.as_deref().unwrap_or("unknown")),
            format!(
                "Locked: {} · Files: {presence:?}",
                locked.as_deref().unwrap_or("unknown")
            ),
        ],
        Local::Tool {
            declaration,
            executable,
            ..
        } => vec![
            "Tool · Rokit".into(),
            format!("Project pin: {declaration}"),
            format!(
                "Release repository: {}",
                declaration
                    .split_once('@')
                    .map_or("unknown", |(repo, _)| repo)
            ),
            format!(
                "Executable on PATH: {}",
                executable
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "not detected".into())
            ),
        ],
    }
}
impl Workbench {
    pub(in super::super) fn updates_entry(&self, cx: &mut Context<Self>) -> AnyElement {
        Button::new("view-updates")
            .label("View updates…")
            .icon(gpui_kit::assets::IconName::Package)
            .on_click(cx.listener(|this, _, window, cx| {
                let Some(path) = this.project.clone() else {
                    return;
                };
                let group = if this.section == Section::Tools {
                    Group::Tools
                } else {
                    Group::Packages
                };
                this.updates.projects.entry(path).or_default().filter(group);
                this.section = Section::Updates;
                this.inspect();
                this.updates_focus.focus(window, cx);
                cx.notify();
            }))
            .into_any_element()
    }
    pub(in super::super) fn render_updates(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let narrow = window.viewport_size().width
            - theme::SIDEBAR_WIDTH.to_pixels(window.rem_size())
            < px(600.);
        let state = self
            .project
            .as_ref()
            .and_then(|p| self.updates.projects.get(p));
        let loaded = state.is_some_and(|s| s.loaded) && self.snapshot.is_some();
        let pending = state.is_some_and(|s| s.pending.is_some());
        let support = match (&self.client, &self.updates.support) {
            (_, Some(Err(e))) => Some(e.as_str()),
            (None, _) => Some("Checking for updates requires the matching bundled CLI."),
            (_, None) => Some("Checking update discovery support…"),
            (_, Some(Ok(()))) => None,
        };
        let mut header =
            div()
                .v_flex()
                .gap_2()
                .child(div().text_lg().child("Updates"))
                .child(
                    Button::new("check-updates")
                        .label("Check for updates")
                        .primary()
                        .disabled(
                            !loaded
                                || pending
                                || support.is_some()
                                || self.updates.apply.active.as_ref().is_some_and(|(_, s)| {
                                    Some(&s.project) == self.project.as_ref()
                                }),
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.start_updates(cx))),
                )
                .when_some(support, |v, m| v.child(m.to_owned()));
        let Some(state) = state else {
            return header.child("Reading local inventory…").into_any_element();
        };
        header = header
            .child(match state.last_success {
                Some(t) => format!("Last successful check: {}", timestamp(t)),
                None if state.summary.is_empty() => "Never checked".into(),
                None => "No successful check yet".into(),
            })
            .child(if pending {
                "Checking packages and tools…".to_string()
            } else {
                state.summary.clone()
            });
        let groups = div().h_flex().flex_wrap().gap_2().children(
            [
                (Group::All, "All"),
                (Group::Packages, "Packages"),
                (Group::Tools, "Tools"),
            ]
            .into_iter()
            .map(|(group, label)| {
                Button::new(SharedString::from(format!("updates-{label}")))
                    .label(label)
                    .selected(state.group == group)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(s) = this
                            .project
                            .as_ref()
                            .and_then(|p| this.updates.projects.get_mut(p))
                        {
                            s.filter(group);
                        }
                        cx.notify();
                    }))
            }),
        );
        let mut list = div()
            .id("updates-list")
            .v_flex()
            .gap_1()
            .min_w_0()
            .flex_1()
            .role(Role::Group)
            .aria_label("Update entries")
            .test_support();
        let mut detail = div()
            .id("updates-detail")
            .v_flex()
            .gap_2()
            .min_w_0()
            .flex_1()
            .role(Role::Group)
            .aria_label("Release comparison")
            .test_support();
        for local in state.entries.iter().filter(|e| state.group.includes(e)) {
            let id = key(local);
            let selected = state.selected.as_ref() == Some(&id);
            let (observation, failure, stale) = state.current(local);
            let label = if failure.is_some() {
                "Unknown"
            } else if stale {
                "Stale"
            } else if let Some(o) = observation {
                o.entry.comparison.status.label()
            } else {
                "Unchecked"
            };
            let kind = match local {
                Local::Tool { .. } => "Tool",
                Local::Package { realm, .. } => realm.as_str(),
            };
            list = list.child(
                Button::new(SharedString::from(format!("update-row-{id}")))
                    .w_full()
                    .selected(selected)
                    .label(format!("{} · {kind} · {label}", local.alias()))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(s) = this
                            .project
                            .as_ref()
                            .and_then(|p| this.updates.projects.get_mut(p))
                        {
                            s.selected = Some(id.clone());
                        }
                        cx.notify();
                    })),
            );
            if selected {
                detail = detail
                    .child(div().text_lg().child(local.alias().to_owned()))
                    .children(details(local));
                if let Some(error) = failure {
                    detail = detail.child(format!("Unknown · {error}"));
                } else if stale {
                    detail = detail.child("The project input changed. Check for updates again.");
                }
                if let Some(o) = observation {
                    detail = detail.child(format!(
                        "Observed {} · {}",
                        timestamp(o.time),
                        comparison_text(local, &o.entry.comparison)
                    ));
                    if failure.is_none() && !stale {
                        detail = detail.child(self.update_review_controls(&o.entry, cx));
                    }
                } else if !stale && failure.is_none() {
                    detail = detail.child(
                        local
                            .source()
                            .err()
                            .unwrap_or_else(|| "Check for updates to compare releases.".into()),
                    );
                }
            }
        }
        if state.selected.is_none() {
            list = list.child(if loaded {
                "No entries in this group."
            } else {
                "Reading local inventory…"
            });
        }
        div()
            .id("updates-view")
            .track_focus(&self.updates_focus)
            .v_flex()
            .gap_4()
            .p_4()
            .child(header)
            .child(groups)
            .child(
                div()
                    .id(if narrow {
                        "updates-stacked"
                    } else {
                        "updates-adjacent"
                    })
                    .role(Role::Group)
                    .aria_label("Update list and detail")
                    .test_support()
                    .h_flex()
                    .when(narrow, |v| v.flex_col())
                    .gap_4()
                    .items_start()
                    .child(list)
                    .child(detail),
            )
            .into_any_element()
    }
}
