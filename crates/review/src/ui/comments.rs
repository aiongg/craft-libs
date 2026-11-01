//! The Comments panel (N5.01).

use std::sync::{Arc, LazyLock};

use craft_host::Host;
use craft_i18n::{DateFormat, DateStyle, DateTime, I18n, TimeZone};
use craft_theme::{ButtonVariant, TypeRole};
use craft_ui::controls::{Button, IconButton, TextField};
use craft_ui::edit::EditEvent;
use craft_ui::icons::{self, Icon, IconSize};
use craft_ui::localized::{AGE_SHOWN_AS_DATE, AgeStyle, Labels as _, fmt, relative_age};
use craft_ui::menu::{MenuEntry, MenuItem, PressMenu};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, UiTokens, corner_radius, role_format};
use egui::accesskit::Role;
use egui::text::LayoutJob;
use egui::{Align, AsIdSalt, Color32, Context, Frame, Id, IdSalt, Key, Layout, Margin, Modifiers, Rect, Response, Sense, Ui, UiBuilder, Vec2, pos2};

use super::ReviewStyle;
use crate::comments::{CommentSort, CommentThread, ReviewStatus, ThreadId, initials, matches, sort_threads};

craft_ui::words! {
    /// The panel's words, English by default (PdfCraft's, Acrobat's).
    #[non_exhaustive]
    pub struct CommentsLabels {
        title: "comments.title" = "Comments";
        replies: "comments.replies" = "{n} replies";
        search: "comments.search" = "Search comments";
        sort_by: "comments.sortBy" = "Sort by";
        sort_page: "comments.sortPage" = "Page";
        sort_author: "comments.sortAuthor" = "Author";
        sort_date: "comments.sortDate" = "Date";
        sort_type: "comments.sortType" = "Type";
        sort_color: "comments.sortColor" = "Color";
        add_comment: "comments.add" = "Add a comment";
        reply: "comments.reply" = "Add a reply";
        post: "comments.post" = "Post";
        cancel: "comments.cancel" = "Cancel";
        more: "comments.more" = "More";
        edit: "comments.edit" = "Edit";
        set_status: "comments.setStatus" = "Set status";
        delete: "comments.delete" = "Delete";
        copy_text: "comments.copyText" = "Copy text";
        mark: "comments.mark" = "Mark with checkmark";
        unmark: "comments.unmark" = "Remove checkmark";
        marked: "comments.marked" = "Marked";
        locked: "comments.locked" = "Locked";
        unknown_author: "comments.unknownAuthor" = "Unknown author";
        page: "comments.page" = "Page {n}";
        status_none: "comments.statusNone" = "None";
        accepted: "comments.accepted" = "Accepted";
        rejected: "comments.rejected" = "Rejected";
        cancelled: "comments.cancelled" = "Cancelled";
        completed: "comments.completed" = "Completed";
        empty: "comments.empty" = "No comments yet.";
    }
}

/// What the panel keeps between passes; the app keeps one per document window.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CommentsPanelState {
    pub search: String,
    pub sort: CommentSort,
    pub selected: Option<ThreadId>,
    /// The message being edited inline: (thread, message index, text).
    pub editing: Option<(ThreadId, usize, String)>,
    /// The selected card's reply draft.
    pub reply: String,
    /// The "Add a comment" box's draft.
    pub new_comment: String,
}

/// What the panel asks of the app.
#[derive(Clone, Debug, PartialEq)]
pub enum CommentEdit {
    /// "Add a comment": a sticky note on the current page.
    Add {
        text: String,
    },
    Reply {
        thread: ThreadId,
        text: String,
    },
    /// Message `index` (0: the comment) edited.
    Edit {
        thread: ThreadId,
        index: usize,
        text: String,
    },
    /// Message `index` deleted; 0 deletes the thread.
    Delete {
        thread: ThreadId,
        index: usize,
    },
    SetStatus {
        thread: ThreadId,
        status: ReviewStatus,
    },
    Mark {
        thread: ThreadId,
        marked: bool,
    },
    /// A card was selected: the app selects the annotation on the page and scrolls to it.
    Select(ThreadId),
    /// Copy text of message `index`; the app puts it on the clipboard.
    CopyText {
        thread: ThreadId,
        index: usize,
    },
}

/// Acrobat's right-hand comment list (PdfCraft).
///
/// - Header "Comments N" with "N replies" under it, search (filters by [`matches`](crate::comments::matches)),
///   the Sort by menu.
/// - "Add a comment" box: Enter (Cmd+Enter in a multi-line draft) posts a sticky note.
/// - Cards grouped by page under "Page N" headings (by [`sort_threads`](crate::comments::sort_threads)): an
///   avatar with the kind's icon in the annotation colour, author, modified date (relative
///   under a week with [`relative_age`](craft_ui::localized::relative_age) in
///   [`AgeStyle::Short`](craft_ui::localized::AgeStyle), then the date through craft-i18n's
///   `I18n::dates()`, as `AGE_SHOWN_AS_DATE` describes), the text; a status badge (Accepted:
///   thumbs-up; Rejected: thumbs-down; Cancelled: circle-x; Completed: check; colours
///   `review.status_*` of [`ReviewStyle`](super::ReviewStyle)); the checkmark (a toggle on the
///   selected card, a mark otherwise); a lock badge; replies under it, each with its own "…"
///   ([`PressMenu`](craft_ui::menu::PressMenu): Edit, Copy text, Delete).
/// - The selected card has the `review.card_accent` bar, the reply box and "…" (Edit, Set
///   status, Delete). Locked threads and `read_only` hide the editing items.
/// - Keyboard: Up / Down move between cards (selecting them), Enter focuses the reply box, Esc
///   leaves an inline edit; every button is a Tab stop. AccessKit: a list of cards, each named
///   by author and text.
pub struct CommentsPanel<'a> {
    id_salt: IdSalt,
    threads: &'a [CommentThread],
    state: &'a mut CommentsPanelState,
    #[expect(dead_code, reason = "the app signs new comments with it; the edits carry only the text")]
    me: &'a str,
    read_only: bool,
    labels: Option<CommentsLabels>,
}

impl<'a> CommentsPanel<'a> {
    /// `me`: the author name new comments and replies get (the app's identity setting).
    pub fn new(id_salt: impl AsIdSalt, threads: &'a [CommentThread], state: &'a mut CommentsPanelState, me: &'a str) -> Self {
        Self { id_salt: IdSalt::new(id_salt), threads, state, me, read_only: false, labels: None }
    }

    /// No posting or editing (a document that forbids comments).
    pub fn read_only(mut self, on: bool) -> Self {
        self.read_only = on;
        self
    }

    pub fn labels(mut self, labels: CommentsLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // pdfcraft: crates/ui-egui/src/panels/comments_panel.rs:115 `show`, `:311` `card`, `:541`
    // `inline_editor`, `:568` `reply_menu`, `:600` `avatar`, `:626` `header_controls`;
    // `tools/comments/menu.rs:368` `status_badge`; deckcraft: crates/ui-egui/src/panels/comments.rs:67.
    pub fn show(self, ui: &mut Ui) -> CommentsPanelOutput {
        let Self { id_salt, threads, state, me: _, read_only, labels } = self;
        let labels = labels.unwrap_or_else(|| CommentsLabels::of(ui));
        let tokens = Tokens::of(ui);
        let style = ReviewStyle::of(&tokens);
        let cx = Cx { tokens: &tokens, style: &style, labels: &labels, id: ui.make_persistent_id(id_salt), read_only };
        let mut edits = Vec::new();
        let response = ui
            .vertical(|ui| {
                ui.spacing_mut().item_spacing.y = tokens.space.gap.stack;
                header(ui, &cx, threads, state);
                if !read_only {
                    add_box(ui, &cx, state, &mut edits);
                }
                if threads.is_empty() {
                    text(ui, &tokens, TypeRole::Body, tokens.color.text.muted.to_egui(), &labels.empty);
                    return;
                }
                list(ui, &cx, threads, state, &mut edits);
            })
            .response;
        CommentsPanelOutput { response, edits }
    }
}

/// What the Comments panel did this pass.
#[must_use]
pub struct CommentsPanelOutput {
    pub response: Response,
    /// This pass's edits, in order.
    pub edits: Vec<CommentEdit>,
}

/// What every part of the panel reads.
struct Cx<'a> {
    tokens: &'a UiTokens,
    style: &'a ReviewStyle,
    labels: &'a CommentsLabels,
    id: Id,
    read_only: bool,
}

/// What the "…" menu of a card or reply asks for.
#[derive(Clone, Copy)]
enum CardMenu {
    Edit,
    Status(ReviewStatus),
    Copy,
    Delete,
}

/// A label in a type role and colour that wraps.
pub(super) fn text(ui: &mut Ui, tokens: &UiTokens, role: TypeRole, color: Color32, text: impl Into<String>) -> Response {
    ui.add(egui::Label::new(LayoutJob::single_section(text.into(), role_format(tokens, role, color))).wrap())
}

/// Now in seconds since the Unix epoch.
fn unix_now() -> Option<i64> {
    let since = web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH).ok()?;
    i64::try_from(since.as_secs()).ok()
}

/// How dates are written in `ctx`'s Host: English (US) without one.
fn dates(ctx: &Context) -> Arc<DateFormat> {
    static ENGLISH: LazyLock<Arc<DateFormat>> = LazyLock::new(|| Arc::new(DateFormat::from_tag("en-US")));
    Host::try_of_ctx(ctx).and_then(|host| host.i18n().map(I18n::dates)).unwrap_or_else(|| Arc::clone(&ENGLISH))
}

/// "5 min ago" under a week, then the date.
pub(super) fn age_text(ctx: &Context, then: i64) -> String {
    let age = unix_now().map(|now| now.saturating_sub(then).max(0).unsigned_abs());
    match age {
        Some(age) if age < AGE_SHOWN_AS_DATE => relative_age(ctx, age, AgeStyle::Short),
        _ => dates(ctx).format(DateTime::from_unix(then, TimeZone::Local), DateStyle::DATE),
    }
}

/// A round badge with the initials of `name`.
pub(super) fn avatar(ui: &mut Ui, tokens: &UiTokens, name: &str) -> Response {
    let side = tokens.size.control.sm;
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(side), Sense::hover());
    let on = tokens.color.text.on_strong.to_egui();
    ui.painter().circle_filled(rect.center(), side / 2.0, tokens.color.accent.strong.to_egui());
    let galley = ui.painter().layout_job(LayoutJob::single_section(initials(name), role_format(tokens, TypeRole::Caption, on)));
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, on);
    response
}

/// The icon of a thread's annotation kind.
fn kind_icon(kind: &str) -> Icon {
    match kind {
        "Highlight" | "Underline" | "StrikeOut" | "Squiggly" => Icon::Brush,
        "TextBox" | "Callout" | "FreeText" => Icon::Text,
        "Ink" => Icon::Pencil,
        "Attach" | "FileAttachment" => Icon::Link,
        _ => Icon::Note,
    }
}

/// The icon and colour of a status badge; `None` has none.
fn status_badge(style: &ReviewStyle, status: ReviewStatus) -> Option<(Icon, Color32)> {
    match status {
        ReviewStatus::None => None,
        ReviewStatus::Accepted => Some((Icon::Success, style.status_accepted)),
        ReviewStatus::Rejected => Some((Icon::Close, style.status_rejected)),
        ReviewStatus::Cancelled => Some((Icon::Ban, style.status_cancelled)),
        ReviewStatus::Completed => Some((Icon::Check, style.status_completed)),
    }
}

fn status_label(labels: &CommentsLabels, status: ReviewStatus) -> &str {
    match status {
        ReviewStatus::None => &labels.status_none,
        ReviewStatus::Accepted => &labels.accepted,
        ReviewStatus::Rejected => &labels.rejected,
        ReviewStatus::Cancelled => &labels.cancelled,
        ReviewStatus::Completed => &labels.completed,
    }
}

/// A small non-interactive icon with a name for AccessKit and the tooltip.
fn badge(ui: &mut Ui, tokens: &UiTokens, icon: Icon, color: Color32, name: &str) {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(tokens.size.icon.md), Sense::hover());
    response.widget_info(|| egui::WidgetInfo::labeled(Role::Label, true, name));
    icons::paint(ui, rect, icon, IconSize::Sm, color);
    response.on_hover_text(name);
}

fn header(ui: &mut Ui, cx: &Cx, threads: &[CommentThread], state: &mut CommentsPanelState) {
    let (tokens, labels) = (cx.tokens, cx.labels);
    let replies: usize = threads.iter().map(CommentThread::replies).sum();
    text(ui, tokens, TypeRole::Title, tokens.color.text.normal.to_egui(), format!("{} {}", labels.title, threads.len()));
    text(ui, tokens, TypeRole::Caption, tokens.color.text.muted.to_egui(), fmt(&labels.replies, &[("n", &replies.to_string())]));
    let search = TextField::new(cx.id.with("search"), &labels.search, &state.search).placeholder(&labels.search).leading(Icon::Search).show(ui);
    if let Some(value) = search.event.as_ref().and_then(EditEvent::value) {
        state.search.clone_from(value);
    }
    let sorts = [
        (CommentSort::Page, &labels.sort_page),
        (CommentSort::Author, &labels.sort_author),
        (CommentSort::Date, &labels.sort_date),
        (CommentSort::Type, &labels.sort_type),
        (CommentSort::Color, &labels.sort_color),
    ];
    let current = sorts.iter().find(|(sort, _)| *sort == state.sort).map_or("", |(_, label)| label.as_str());
    let entries: Vec<MenuEntry<CommentSort>> =
        sorts.iter().map(|(sort, label)| MenuItem::new((*label).clone(), *sort).radio(state.sort == *sort).into()).collect();
    let button = ui.add(Button::new(format!("{}: {current}", labels.sort_by)).trailing_icon(Icon::ChevronDown).id_salt("sort"));
    if let Some(chosen) = PressMenu::new(&entries).show(ui, &button).chosen {
        state.sort = chosen;
    }
}

fn add_box(ui: &mut Ui, cx: &Cx, state: &mut CommentsPanelState, edits: &mut Vec<CommentEdit>) {
    let labels = cx.labels;
    let field = TextField::new(cx.id.with("add"), &labels.add_comment, &state.new_comment).placeholder(&labels.add_comment).show(ui);
    if let Some(value) = field.event.as_ref().and_then(EditEvent::value) {
        state.new_comment.clone_from(value);
    }
    let posted = state.new_comment.trim();
    if field.submitted && !posted.is_empty() {
        edits.push(CommentEdit::Add { text: posted.to_owned() });
        state.new_comment.clear();
    }
}

/// The heading a thread falls under for `sort`; `None` for none.
fn group_of(cx: &Cx, thread: &CommentThread, sort: CommentSort) -> Option<String> {
    match sort {
        CommentSort::Page => thread.anchor.page().map(|n| fmt(&cx.labels.page, &[("n", &n.to_string())])),
        CommentSort::Author => Some(thread.messages.first().map_or_else(|| cx.labels.unknown_author.clone(), |m| m.author.clone())),
        CommentSort::Type => Some(thread.kind.clone()).filter(|kind| !kind.is_empty()),
        CommentSort::Color => thread.color.map(|[r, g, b]| format!("#{r:02X}{g:02X}{b:02X}")),
        CommentSort::Date => None,
    }
}

fn list(ui: &mut Ui, cx: &Cx, threads: &[CommentThread], state: &mut CommentsPanelState, edits: &mut Vec<CommentEdit>) {
    let query = state.search.trim().to_owned();
    let shown: Vec<&CommentThread> =
        sort_threads(threads, state.sort).into_iter().map(|i| &threads[i]).filter(|t| query.is_empty() || matches(t, &query)).collect();
    let list = ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = cx.tokens.space.gap.row;
        let mut focused = None;
        let mut group = None;
        for (position, thread) in shown.iter().enumerate() {
            let heading = group_of(cx, thread, state.sort);
            if heading.is_some() && heading != group {
                text(ui, cx.tokens, TypeRole::BodyStrong, cx.tokens.color.text.muted.to_egui(), heading.clone().unwrap_or_default());
            }
            group = heading;
            if card(ui, cx, thread, state, edits) {
                focused = Some(position);
            }
        }
        focused
    });
    ui.ctx().accesskit_node_builder(list.response.id, |node| node.set_role(Role::List));
    let Some(position) = list.inner else { return };
    let step = ui.input_mut(|input| {
        if input.consume_key(Modifiers::NONE, Key::ArrowDown) {
            Some(1)
        } else if input.consume_key(Modifiers::NONE, Key::ArrowUp) {
            Some(-1)
        } else {
            None
        }
    });
    if let Some(step) = step
        && let Some(next) = position.checked_add_signed(step).and_then(|p| shown.get(p))
    {
        select(next.id, state, edits);
        ui.memory_mut(|memory| memory.request_focus(card_id(cx, next.id)));
    }
}

fn card_id(cx: &Cx, thread: ThreadId) -> Id {
    cx.id.with(("card", thread))
}

fn reply_id(cx: &Cx, thread: ThreadId) -> Id {
    cx.id.with(("reply", thread))
}

fn select(thread: ThreadId, state: &mut CommentsPanelState, edits: &mut Vec<CommentEdit>) {
    if state.selected != Some(thread) {
        state.selected = Some(thread);
        state.reply.clear();
        state.editing = None;
        edits.push(CommentEdit::Select(thread));
    }
}

/// Draws a card; true when the card itself has the keyboard focus.
fn card(ui: &mut Ui, cx: &Cx, thread: &CommentThread, state: &mut CommentsPanelState, edits: &mut Vec<CommentEdit>) -> bool {
    let (tokens, style) = (cx.tokens, cx.style);
    let selected = state.selected == Some(thread.id);
    let editable = !cx.read_only && !thread.locked;
    let id = card_id(cx, thread.id);
    let fill = if selected { tokens.color.state.selected.to_egui() } else { Color32::TRANSPARENT };
    let pad = tokens.space.pad.control;
    let shown = ui.scope_builder(UiBuilder::new().scope_id(id).sense(Sense::click()), |ui| {
        let frame = Frame::new().fill(fill).corner_radius(corner_radius(tokens.shape.radius.control)).inner_margin(Margin::same(pad as i8));
        frame
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing.y = tokens.space.gap.row;
                card_head(ui, cx, thread, selected, editable, state, edits);
                message_body(ui, cx, thread, 0, state, edits);
                for index in 1..thread.messages.len() {
                    reply(ui, cx, thread, index, editable, state, edits);
                }
                if selected && editable {
                    reply_box(ui, cx, thread, state, edits);
                }
            })
            .response
            .rect
    });
    let response = shown.response;
    let rect = shown.inner;
    if selected {
        let bar = Rect::from_min_max(rect.min, pos2(rect.min.x + tokens.space.gap.inline / 2.0, rect.max.y));
        ui.painter().rect_filled(bar, 0.0, style.card_accent);
    }
    let first = thread.messages.first();
    let name = first.map_or_else(String::new, |m| format!("{}: {}", m.author, m.text));
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_role(Role::ListItem);
        node.set_label(name);
        node.set_selected(selected);
    });
    if response.clicked() {
        select(thread.id, state, edits);
    }
    if response.has_focus() && ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Enter)) {
        select(thread.id, state, edits);
        if editable {
            ui.memory_mut(|memory| memory.request_focus(reply_id(cx, thread.id)));
        }
    }
    response.has_focus()
}

fn card_head(
    ui: &mut Ui,
    cx: &Cx,
    thread: &CommentThread,
    selected: bool,
    editable: bool,
    state: &mut CommentsPanelState,
    edits: &mut Vec<CommentEdit>,
) {
    let (tokens, labels, style) = (cx.tokens, cx.labels, cx.style);
    ui.horizontal(|ui| {
        let side = tokens.size.control.sm;
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(side), Sense::hover());
        ui.painter().circle_filled(rect.center(), side / 2.0, tokens.color.surface.sunken.to_egui());
        let tint = thread.color.map_or(tokens.color.accent.solid.to_egui(), |[r, g, b]| Color32::from_rgb(r, g, b));
        icons::paint(ui, rect, kind_icon(&thread.kind), IconSize::Sm, tint);
        let first = thread.messages.first();
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let author = first.map_or(labels.unknown_author.as_str(), |m| m.author.as_str());
            text(ui, tokens, TypeRole::BodyStrong, tokens.color.text.normal.to_egui(), author);
            if let Some(m) = first {
                let then = m.modified.unwrap_or(m.created);
                text(ui, tokens, TypeRole::Caption, tokens.color.text.muted.to_egui(), age_text(ui.ctx(), then));
            }
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if selected && editable {
                let entries: Vec<MenuEntry<CardMenu>> = vec![
                    MenuItem::new(labels.edit.clone(), CardMenu::Edit).into(),
                    MenuItem::submenu(
                        labels.set_status.clone(),
                        ReviewStatus::MENU
                            .iter()
                            .map(|&s| MenuItem::new(status_label(labels, s), CardMenu::Status(s)).radio(thread.status == s).into())
                            .collect(),
                    )
                    .into(),
                    MenuEntry::Separator,
                    MenuItem::new(labels.delete.clone(), CardMenu::Delete).into(),
                ];
                let more = ui.add(IconButton::new(Icon::More, &labels.more).id_salt("more"));
                match PressMenu::new(&entries).show(ui, &more).chosen {
                    Some(CardMenu::Edit) => {
                        let text = thread.messages.first().map(|m| m.text.clone()).unwrap_or_default();
                        state.editing = Some((thread.id, 0, text));
                    }
                    Some(CardMenu::Status(status)) => edits.push(CommentEdit::SetStatus { thread: thread.id, status }),
                    Some(CardMenu::Delete) => edits.push(CommentEdit::Delete { thread: thread.id, index: 0 }),
                    Some(CardMenu::Copy) | None => {}
                }
            }
            if thread.locked {
                badge(ui, tokens, Icon::Lock, tokens.color.text.muted.to_egui(), &labels.locked);
            }
            if selected && editable {
                let mut on = thread.marked;
                let name = if thread.marked { &labels.unmark } else { &labels.mark };
                if ui.add(IconButton::new(Icon::Check, name).toggle(&mut on).id_salt("mark")).changed() {
                    edits.push(CommentEdit::Mark { thread: thread.id, marked: on });
                }
            } else if thread.marked {
                badge(ui, tokens, Icon::Check, tokens.color.text.muted.to_egui(), &labels.marked);
            }
            if let Some((icon, color)) = status_badge(style, thread.status) {
                badge(ui, tokens, icon, color, status_label(labels, thread.status));
            }
        });
    });
}

/// Message `index`: its text, or the inline editor while it is being edited.
fn message_body(ui: &mut Ui, cx: &Cx, thread: &CommentThread, index: usize, state: &mut CommentsPanelState, edits: &mut Vec<CommentEdit>) {
    let Some(message) = thread.messages.get(index) else { return };
    let editing = state.editing.as_ref().is_some_and(|(t, i, _)| *t == thread.id && *i == index);
    if !editing {
        text(ui, cx.tokens, TypeRole::Body, cx.tokens.color.text.normal.to_egui(), &message.text);
        return;
    }
    let draft = state.editing.as_ref().map(|(_, _, text)| text.clone()).unwrap_or_default();
    let field = TextField::new(cx.id.with(("edit", thread.id, index)), &cx.labels.edit, &draft).multiline(3).show(ui);
    if let Some(value) = field.event.as_ref().and_then(EditEvent::value)
        && let Some((_, _, text)) = state.editing.as_mut()
    {
        text.clone_from(value);
    }
    let text = state.editing.as_ref().map(|(_, _, text)| text.trim().to_owned()).unwrap_or_default();
    let mut post = field.submitted;
    let mut cancel = field.escaped;
    ui.horizontal(|ui| {
        post |= ui.add(Button::new(&cx.labels.post).variant(ButtonVariant::Primary).id_salt("post")).clicked();
        cancel |= ui.add(Button::new(&cx.labels.cancel).id_salt("cancel")).clicked();
    });
    if cancel {
        state.editing = None;
    } else if post && !text.is_empty() {
        edits.push(CommentEdit::Edit { thread: thread.id, index, text });
        state.editing = None;
    }
}

fn reply(
    ui: &mut Ui,
    cx: &Cx,
    thread: &CommentThread,
    index: usize,
    editable: bool,
    state: &mut CommentsPanelState,
    edits: &mut Vec<CommentEdit>,
) {
    let (tokens, labels) = (cx.tokens, cx.labels);
    let Some(message) = thread.messages.get(index) else { return };
    ui.horizontal_top(|ui| {
        avatar(ui, tokens, &message.author);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.horizontal(|ui| {
                text(ui, tokens, TypeRole::BodyStrong, tokens.color.text.normal.to_egui(), &message.author);
                text(ui, tokens, TypeRole::Caption, tokens.color.text.muted.to_egui(), age_text(ui.ctx(), message.modified.unwrap_or(message.created)));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let mut entries: Vec<MenuEntry<CardMenu>> = Vec::new();
                    if editable {
                        entries.push(MenuItem::new(labels.edit.clone(), CardMenu::Edit).into());
                    }
                    entries.push(MenuItem::new(labels.copy_text.clone(), CardMenu::Copy).into());
                    if editable {
                        entries.push(MenuEntry::Separator);
                        entries.push(MenuItem::new(labels.delete.clone(), CardMenu::Delete).into());
                    }
                    let more = ui.add(IconButton::new(Icon::More, &labels.more).id_salt(("more", index)));
                    match PressMenu::new(&entries).show(ui, &more).chosen {
                        Some(CardMenu::Edit) => state.editing = Some((thread.id, index, message.text.clone())),
                        Some(CardMenu::Copy) => edits.push(CommentEdit::CopyText { thread: thread.id, index }),
                        Some(CardMenu::Delete) => edits.push(CommentEdit::Delete { thread: thread.id, index }),
                        Some(CardMenu::Status(_)) | None => {}
                    }
                });
            });
            message_body(ui, cx, thread, index, state, edits);
        });
    });
}

fn reply_box(ui: &mut Ui, cx: &Cx, thread: &CommentThread, state: &mut CommentsPanelState, edits: &mut Vec<CommentEdit>) {
    let labels = cx.labels;
    let field = TextField::new(cx.id.with(("reply-field", thread.id)), &labels.reply, &state.reply)
        .placeholder(&labels.reply)
        .multiline(2)
        .id(reply_id(cx, thread.id))
        .show(ui);
    if let Some(value) = field.event.as_ref().and_then(EditEvent::value) {
        state.reply.clone_from(value);
    }
    let text = state.reply.trim().to_owned();
    let post = ui.add_enabled(!text.is_empty(), Button::new(&labels.post).variant(ButtonVariant::Primary).id_salt("reply-post")).clicked();
    if (post || field.submitted) && !text.is_empty() {
        edits.push(CommentEdit::Reply { thread: thread.id, text });
        state.reply.clear();
    }
}
