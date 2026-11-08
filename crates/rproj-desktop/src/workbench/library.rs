use super::*;
use gpui_kit::base::text::TextViewState;
use rproj_core::catalog::browser::{self, Entry, EntryId, Guide, Kind};
use std::collections::BTreeMap;

#[cfg(test)]
mod tests;
mod view;

const HISTORY_LIMIT: usize = 30;

struct Bookmark {
    selected: EntryId,
    query: String,
    scope: Option<Kind>,
    list_offset: Point<Pixels>,
    guide_offset: Point<Pixels>,
    page_offset: Point<Pixels>,
    focus: FocusHandle,
}

pub(super) struct Library {
    pub(super) visible: bool,
    entries: Vec<Entry>,
    indices: Vec<usize>,
    query: Entity<InputState>,
    query_text: String,
    scope: Option<Kind>,
    selected: EntryId,
    guide: Option<Guide>,
    text: Entity<TextViewState>,
    scroll: UniformListScrollHandle,
    guide_scroll: ScrollHandle,
    page_scroll: ScrollHandle,
    focus: FocusHandle,
    rows_focus: FocusHandle,
    related_focus: BTreeMap<EntryId, FocusHandle>,
    history: VecDeque<Bookmark>,
    copied: Option<usize>,
    _subscription: Subscription,
}

impl Library {
    pub(super) fn new(window: &mut Window, cx: &mut Context<Workbench>) -> Self {
        let entries = browser::entries();
        let selected = EntryId::new(Kind::Package, "scribe");
        let guide = browser::guide(&selected);
        let markdown = guide.as_ref().map(guide_markdown).unwrap_or_default();
        let text = cx.new(|cx| TextViewState::markdown(&markdown, cx));
        let query =
            cx.new(|cx| InputState::new(window, cx).placeholder("Search all catalog sections"));
        let subscription = cx.subscribe(&query, |this, input, event, cx| {
            if matches!(event, InputEvent::Change) {
                let query = input.read(cx).value().to_string();
                // Back restores the model before replacing input text. Its queued
                // Change event must not reset the restored scroll position.
                if this.library.query_text != query {
                    this.library.query_text = query;
                    this.library.filter();
                    cx.notify();
                }
            }
        });
        let related_focus = entries
            .iter()
            .map(|entry| (entry.id.clone(), cx.focus_handle().tab_stop(true)))
            .collect();
        Self {
            visible: false,
            indices: (0..entries.len()).collect(),
            entries,
            query,
            query_text: String::new(),
            scope: None,
            selected,
            guide,
            text,
            scroll: UniformListScrollHandle::new(),
            guide_scroll: ScrollHandle::default(),
            page_scroll: ScrollHandle::default(),
            focus: cx.focus_handle(),
            rows_focus: cx.focus_handle().tab_stop(true),
            related_focus,
            history: VecDeque::new(),
            copied: None,
            _subscription: subscription,
        }
    }

    fn filter(&mut self) {
        self.indices = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                self.scope.is_none_or(|kind| kind == entry.id.kind)
                    && entry.matches(&self.query_text)
            })
            .map(|(i, _)| i)
            .collect();
        self.scroll
            .0
            .borrow()
            .base_handle
            .set_offset(Point::default());
    }

    fn at(&self, index: usize) -> Option<&Entry> {
        self.entries.get(*self.indices.get(index)?)
    }

    fn selection_visible(&self) -> bool {
        self.indices
            .iter()
            .any(|&i| self.entries[i].id == self.selected)
    }

    fn bookmark(&self, focus: FocusHandle) -> Bookmark {
        Bookmark {
            selected: self.selected.clone(),
            query: self.query_text.clone(),
            scope: self.scope,
            list_offset: self.scroll.0.borrow().base_handle.offset(),
            guide_offset: self.guide_scroll.offset(),
            page_offset: self.page_scroll.offset(),
            focus,
        }
    }

    fn load(&mut self, id: EntryId, cx: &mut Context<Workbench>) {
        self.selected = id;
        self.guide = browser::guide(&self.selected);
        let markdown = self.guide.as_ref().map(guide_markdown).unwrap_or_default();
        self.text
            .update(cx, |text, cx| text.set_text(&markdown, cx));
        self.guide_scroll.set_offset(Point::default());
        self.copied = None;
    }
}

// Guidance is structured plain text. Escape Markdown punctuation instead of
// treating descriptions as executable links, images or markup.
fn literal(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        if "\\`*_{}[]<>()#+-.!|~".contains(c) {
            out.push('\\');
        }
        if c == '\n' {
            out.push_str("  ");
        }
        out.push(c);
    }
    out
}

fn guide_markdown(guide: &Guide) -> String {
    let mut out = format!(
        "# {}\n\n{}\n\n",
        literal(&guide.entry.name),
        literal(&guide.entry.description)
    );
    for section in &guide.sections {
        out.push_str(&format!(
            "## {}\n\n{}\n\n",
            literal(&section.heading),
            literal(&section.text)
        ));
    }
    for example in &guide.examples {
        // Fence longer than any run in the bundled example; code bytes are unchanged.
        let fence = "`".repeat(
            example
                .code
                .split(|c| c != '`')
                .map(str::len)
                .max()
                .unwrap_or(0)
                .max(2)
                + 1,
        );
        out.push_str(&format!(
            "## {}\n\n{fence}luau\n{}\n{fence}\n\n",
            literal(&example.label),
            example.code
        ));
    }
    out
}

impl Workbench {
    pub(super) fn show_catalog(
        &mut self,
        id: Option<EntryId>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.browser.leave();
        self.template.visible = false;
        self.saved.visible = false;
        self.setup.visible = false;
        self.settings.visible = false;
        self.upgrade.visible = false;
        self.composition.visible = false;
        self.creation.visible = false;
        self.library.visible = true;
        if let Some(id) = id {
            self.select_catalog(id, self.library.rows_focus.clone(), cx);
        }
        self.library
            .query
            .read(cx)
            .focus_handle(cx)
            .focus(window, cx);
        cx.notify();
    }

    fn select_catalog(&mut self, id: EntryId, return_focus: FocusHandle, cx: &mut Context<Self>) {
        if self.library.selected != id {
            let bookmark = self.library.bookmark(return_focus);
            self.library.history.push_back(bookmark);
            while self.library.history.len() > HISTORY_LIMIT {
                self.library.history.pop_front();
            }
            self.library.load(id, cx);
        }
        cx.notify();
    }

    fn catalog_back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(bookmark) = self.library.history.pop_back() else {
            return;
        };
        self.library.query_text = bookmark.query.clone();
        self.library.scope = bookmark.scope;
        self.library.filter();
        self.library.query.update(cx, |input, cx| {
            input.replace_all(&bookmark.query, window, cx)
        });
        self.library.load(bookmark.selected.clone(), cx);
        // Handles belong to the session, so the returning controls reuse them.
        // Restore synchronously with the selection instead of queuing a callback
        // which could steal focus or overwrite a later user scroll.
        bookmark.focus.focus(window, cx);
        self.library
            .scroll
            .0
            .borrow()
            .base_handle
            .set_offset(bookmark.list_offset);
        self.library.guide_scroll.set_offset(bookmark.guide_offset);
        self.library.page_scroll.set_offset(bookmark.page_offset);
        cx.notify();
    }

    fn copy_catalog_example(&mut self, id: &EntryId, index: usize, cx: &mut Context<Self>) {
        if let Some(guide) = &self.library.guide
            && &guide.entry.id == id
            && let Some(example) = guide.examples.get(index)
        {
            cx.write_to_clipboard(ClipboardItem::new_string(example.code.clone()));
            self.library.copied = Some(index);
            cx.notify();
        }
    }

    fn open_catalog_docs(&self, id: &EntryId, index: usize, cx: &App) {
        if let Some(guide) = &self.library.guide
            && &guide.entry.id == id
            && let Some(link) = guide.links.get(index)
        {
            cx.open_url(&link.url);
        }
    }

    pub(super) fn catalog_package(
        &self,
        package: &rproj_core::PackageDependency,
    ) -> Option<EntryId> {
        self.catalog
            .iter()
            .find(|entry| package.package.as_deref() == Some(entry.package.as_str()))
            .map(|entry| EntryId::new(Kind::Package, &entry.key))
    }
}
