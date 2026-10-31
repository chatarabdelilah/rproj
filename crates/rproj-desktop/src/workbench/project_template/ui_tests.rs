use super::{
    Draft, Page, PreparedTemplate, Response, Session, Task, TemplateAction, TemplateOutcome,
    TemplateStatus,
};
use crate::workbench::{Workbench, ui_tests::fixture as shell_fixture};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{AnyWindowHandle, AppContext, Entity, TestAppContext, px, size};
use rproj_core::{
    operations::Cancellation,
    project_template::{
        builtin_project_template, protocol::TemplateMutation, review::TemplateReview,
    },
};
use std::sync::Arc;

fn fixture(cx: &mut TestAppContext) -> (tempfile::TempDir, AnyWindowHandle, Entity<Workbench>) {
    let (root, handle, view) = shell_fixture(cx);
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.template = Session::default();
            this.template.visible = true;
            this.template.support = Some(Ok(()));
            this.template.load(super::tests::loaded(None));
            cx.notify();
        })
    });
    (root, handle, view)
}
fn edit(s: &mut Session) {
    let d = s.draft.as_mut().unwrap();
    d.model
        .as_mut()
        .unwrap()
        .set_setting("servePort", Some(40001.into()))
        .unwrap();
    s.changed();
}
fn reset_plan(s: &mut Session) -> PreparedTemplate {
    let original = serde_json::to_string(&builtin_project_template()).unwrap();
    s.load(super::tests::loaded(Some(&original)));
    edit(s);
    let source = s.draft.as_ref().unwrap().loaded.source.clone();
    PreparedTemplate {
        review: TemplateReview::prepare(source, TemplateAction::Reset).unwrap(),
        rojo: None,
    }
}
#[gpui_kit::test]
fn explorer_retains_selection_during_resize_and_reload_defaults_to_keep(cx: &mut TestAppContext) {
    let (_root, handle, view) = fixture(cx);
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            edit(&mut this.template);
            this.template.draft.as_mut().unwrap().selected = vec!["ReplicatedStorage".into()];
            cx.notify()
        })
    });
    for (width, layout) in [(1120., "template-adjacent"), (800., "template-stacked")] {
        cx.simulate_window_resize(handle, size(px(width), px(680.)));
        cx.update_window(handle, |_, w, cx| {
            w.render_frame(cx);
            assert!(w.try_find(layout).is_some());
            assert!(w.find("template-flow").visible());
            assert_eq!(
                view.read(cx).template.draft.as_ref().unwrap().selected,
                ["ReplicatedStorage"]
            );
        })
        .unwrap();
    }
    cx.update_window(handle, |_, w, cx| {
        w.click("template-reload", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        assert!(w.find("template-keep").visible());
        w.press("enter", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        assert_eq!(w.find("template-flow").focused(), Some(true));
        assert!(view.read(cx).template.draft.as_ref().unwrap().dirty());
    })
    .unwrap();
}

#[gpui_kit::test]
fn reset_review_keeps_raw_draft_and_confirm_dismissal_restores_focus(cx: &mut TestAppContext) {
    let (_root, handle, view) = fixture(cx);
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            let plan = reset_plan(&mut this.template);
            let d = this.template.draft.as_mut().unwrap();
            d.raw = "{unfinished".into();
            this.template.review_return = Page::Json;
            this.template.page = Page::Review;
            this.template.rows = Arc::new(plan.review.diff());
            this.template_inputs.list.reset(this.template.rows.len());
            this.template.prepared = Some(Box::new(plan));
            cx.notify();
        })
    });
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        assert!(w.find("template-diff").visible());
        w.click("template-execute", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        w.press("escape", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        assert_eq!(w.find("template-flow").focused(), Some(true));
        w.click("template-back", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| {
        let s = &view.read(cx).template;
        assert!(s.page == Page::Json);
        assert_eq!(s.draft.as_ref().unwrap().raw, "{unfinished");
    });
}

#[gpui_kit::test]
fn uncertain_completion_while_close_dialog_is_open_preserves_all_drafts(cx: &mut TestAppContext) {
    let (_root, handle, view) = fixture(cx);
    let (id, path) = cx.update(|cx| {
        view.update(cx, |this, cx| {
            let plan = reset_plan(&mut this.template);
            let path = plan.review.source.path.clone();
            let task = Task::Execute(Box::new(plan));
            let op = task.operation();
            let id = this.operations.begin(op, path.clone());
            this.running = Some((op, Cancellation::default()));
            this.template.pending = Some(task);
            this.creation.draft = Some(rproj_core::creation::CreationDraft::new(
                "Independent".into(),
            ));
            this.creation.dirty = true;
            cx.notify();
            (id, path)
        })
    });
    cx.update_window(handle, |_, w, cx| {
        view.update(cx, |this, cx| assert!(!this.request_close(w, cx)))
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.accept_template(id.wrapping_add(1), Err(anyhow::anyhow!("late event")), cx);
            assert!(this.template.pending.is_some());
            this.accept_template(
                id,
                Ok(Response::Executed(TemplateOutcome {
                    path,
                    status: TemplateStatus::Uncertain,
                    mutation: TemplateMutation::Unknown,
                    saved: None,
                    message: Some("Missing report".into()),
                })),
                cx,
            );
        })
    });
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        w.click("template-confirm", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        assert!(w.find("template-flow").visible());
        let this = view.read(cx);
        assert!(this.template.reload && this.creation.dirty);
        assert!(this.template.draft.as_ref().is_some_and(Draft::dirty));
        assert!(this.template.close_after.is_none());
        assert_eq!(this.template.last, Some((id, false)));
    })
    .unwrap();
}
