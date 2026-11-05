//! Comment pins on a canvas, the thread popover and the composer (N5.02).

use std::ops::Range;

use craft_theme::{ButtonVariant, ControlSize};
use craft_ui::controls::{Button, IconButton, TextField, ToggleButton};
use craft_ui::edit::{CancelReason, EditEvent, GestureId};
use craft_ui::icons::Icon;
use craft_ui::localized::{Labels as _, fmt};
use craft_ui::menu::{MenuEntry, MenuItem, PressMenu};
use craft_ui::popover::{Popover, PopoverAnchor, PopoverClose};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, UiTokens, corner_radius, role_format};
use egui::text::{CCursor, CCursorRange, LayoutJob};
use egui::{
    Align, AsIdSalt, CornerRadius, Id, IdSalt, Key, Layout, Modifiers, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2, pos2, vec2,
};

use super::ReviewStyle;
use super::comments::{CommentEdit, age_text, avatar, text};
use crate::comments::{CommentThread, ThreadId, initials};

craft_ui::words! {
    /// The thread popover's and composer's words, English by default (FigCraft's).
    #[non_exhaustive]
    pub struct ThreadLabels {
        reply: "thread.reply" = "Reply";
        add_comment: "thread.addComment" = "Add a comment";
        post: "thread.post" = "Post";
        edit: "thread.edit" = "Edit";
        delete: "thread.delete" = "Delete";
        resolve: "thread.resolve" = "Resolve";
        reopen: "thread.reopen" = "Reopen";
        copy_link: "thread.copyLink" = "Copy link";
        mark_unread: "thread.markUnread" = "Mark as unread";
        react: "thread.react" = "Add reaction";
        close: "thread.close" = "Close";
        pin: "thread.pin" = "Comment by {author}";
        cluster: "thread.cluster" = "{n} comments";
    }
}

/// The mention list's rows.
const MENTION_ROWS: usize = 6;

/// The rows of the new comment's field.
const COMPOSER_ROWS: usize = 3;

/// The longest `@query` the picker follows.
const MENTION_QUERY_CHARS: usize = 40;

/// The reactions a message offers (FigCraft `comments.rs:26`).
pub const REACTIONS: [&str; 8] = ["👍", "❤", "😂", "🎉", "👀", "🙏", "🔥", "✅"];

/// A pin or a cluster of pins on screen.
#[derive(Clone, Debug, PartialEq)]
pub struct PinCluster {
    pub at: Pos2,
    pub threads: Vec<ThreadId>,
}

/// Groups of `points` (screen): in order, each point joins the first group whose centre (the mean
/// of its points so far) is closer than `radius`, else starts one. Indices per group.
// figcraft: crates/ui-egui/src/canvas/comments.rs:176 `cluster`.
pub fn cluster(points: &[Pos2], radius: f32) -> Vec<Vec<usize>> {
    let mut groups: Vec<(Vec<usize>, Vec2)> = Vec::new();
    for (index, point) in points.iter().enumerate() {
        let near = groups.iter_mut().find(|(members, sum)| (*sum / members.len() as f32).to_pos2().distance(*point) < radius);
        match near {
            Some((members, sum)) => {
                members.push(index);
                *sum += point.to_vec2();
            }
            None => groups.push((vec![index], point.to_vec2())),
        }
    }
    groups.into_iter().map(|(members, _)| members).collect()
}

/// The `@query` being typed just before `cursor` (chars) in `text`: its byte range from the `@`
/// to the cursor. `None` unless the `@` starts the text or follows a non-alphanumeric character,
/// and the query stays on one line, is at most 40 chars and doesn't end in whitespace.
// figcraft: crates/ui-egui/src/canvas/comments.rs:136 `mention_query`.
pub fn mention_query(text: &str, cursor: usize) -> Option<(usize, usize)> {
    let end = text.char_indices().nth(cursor).map_or(text.len(), |(byte, _)| byte);
    let before = text.get(..end)?;
    let at = before.rfind('@')?;
    let query = before.get(at + 1..)?;
    if query.contains('\n') || query.chars().count() > MENTION_QUERY_CHARS || query.ends_with(char::is_whitespace) {
        return None;
    }
    let starts = before.get(..at)?.chars().next_back().is_none_or(|c| !c.is_alphanumeric());
    starts.then_some((at, end))
}

/// The people (not `me`) whose name, or a word of it, starts with `query` in any case, in list
/// order, at most six (the list's rows).
// figcraft: crates/ui-egui/src/canvas/comments.rs:149 `mention_matches`.
pub fn mention_matches(people: &[String], query: &str, me: &str) -> Vec<String> {
    let query = query.to_lowercase();
    people
        .iter()
        .filter(|name| name.as_str() != me)
        .filter(|name| {
            let lower = name.to_lowercase();
            lower.starts_with(&query) || lower.split_whitespace().any(|word| word.starts_with(&query))
        })
        .take(MENTION_ROWS)
        .cloned()
        .collect()
}

/// Replace the `@query` at byte `range` with `@name ` and return the cursor after it (chars); an
/// invalid range changes nothing and returns the text's length in chars.
// figcraft: crates/ui-egui/src/canvas/comments.rs:164 `insert_mention`.
pub fn insert_mention(text: &mut String, range: (usize, usize), name: &str) -> usize {
    let (start, end) = range;
    if start > end || end > text.len() || !text.is_char_boundary(start) || !text.is_char_boundary(end) {
        return text.chars().count();
    }
    let inserted = format!("@{name} ");
    text.replace_range(start..end, &inserted);
    text.get(..start).map_or(0, |before| before.chars().count()) + inserted.chars().count()
}

/// Comment pins over a canvas (Figma's, as FigCraft draws them): a `review.pin` pin per thread
/// ([`ReviewStyle`](super::ReviewStyle); the author's avatar in a speech-bubble shape with a
/// square bottom-left corner), pins within `review.cluster_radius` gathered into a cluster
/// showing the count; a click on a pin opens its thread, on a cluster zooms or lists it. Your own
/// pin can be dragged (past egui's drag threshold) to another spot, reported as a
/// [`PinMove`] gesture (Esc cancels). Resolved threads are hidden unless shown. Each pin is a
/// Tab stop in reading order, Enter opens it; AccessKit `Button`s named by author and the
/// thread's first words.
pub struct CommentPins<'a> {
    id_salt: IdSalt,
    pins: &'a [(ThreadId, Pos2, &'a str)],
    me: &'a str,
    open: Option<ThreadId>,
}

impl<'a> CommentPins<'a> {
    /// `pins`: each thread's screen point and author.
    pub fn new(id_salt: impl AsIdSalt, pins: &'a [(ThreadId, Pos2, &'a str)], me: &'a str) -> Self {
        Self { id_salt: IdSalt::new(id_salt), pins, me, open: None }
    }

    /// The thread whose popover is open (its pin is drawn raised).
    pub fn open(mut self, thread: Option<ThreadId>) -> Self {
        self.open = thread;
        self
    }

    // figcraft: crates/ui-egui/src/canvas/comments.rs:242 `pins`, `:352` `paint_pin`, `:370`
    // `paint_cluster`, `:385` `draw`, `:419` `input`, `:119` `PinDrag`.
    pub fn show(self, ui: &mut Ui) -> CommentPinsOutput {
        let Self { id_salt, pins, me, open } = self;
        let labels = ThreadLabels::of(ui);
        let tokens = Tokens::of(ui);
        let style = ReviewStyle::of(&tokens);
        let id = ui.make_persistent_id(id_salt);
        let points: Vec<Pos2> = pins.iter().map(|(_, at, _)| *at).collect();
        let mut spots: Vec<PinCluster> = cluster(&points, style.cluster_radius)
            .into_iter()
            .map(|members| {
                let sum = members.iter().fold(Vec2::ZERO, |sum, &i| sum + points[i].to_vec2());
                PinCluster { at: (sum / members.len() as f32).to_pos2(), threads: members.iter().map(|&i| pins[i].0).collect() }
            })
            .collect();
        spots.sort_by(|a, b| a.at.y.total_cmp(&b.at.y).then(a.at.x.total_cmp(&b.at.x)));
        let cx = PinCx { tokens: &tokens, style: &style, labels: &labels, id };
        let mut out = CommentPinsOutput { response: ui.interact(Rect::NOTHING, id, Sense::hover()), opened: None, moved: None };
        for spot in &spots {
            let one = match spot.threads.as_slice() {
                [thread] => pins.iter().find(|(id, ..)| id == thread),
                _ => None,
            };
            match one {
                Some((thread, at, author)) => {
                    let response = single_pin(ui, &cx, (*thread, *at, author), me, open == Some(*thread), &mut out);
                    out.response = out.response.union(response);
                }
                None => {
                    let response = cluster_pin(ui, &cx, spot, pins, &mut out);
                    out.response = out.response.union(response);
                }
            }
        }
        out
    }
}

/// A pin dragged to a new screen point; the app maps it to its anchor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PinMove {
    pub thread: ThreadId,
    pub to: Pos2,
}

/// What the pins did this pass.
#[must_use]
pub struct CommentPinsOutput {
    pub response: Response,
    /// The thread whose pin or cluster entry was clicked.
    pub opened: Option<ThreadId>,
    /// A drag of your own pin, as one gesture.
    pub moved: Option<EditEvent<PinMove>>,
}

/// The thread popover next to a pin (`review.popover_width` wide), on the kernel's
/// [`Popover`](craft_ui::popover): messages with avatar, author, age and
/// text (@mentions in the accent), reactions under each (click to toggle yours), your messages'
/// Edit and Delete, the thread's Resolve / Reopen, Copy link and Mark as unread, and the reply
/// composer at the bottom with @mention completion (up to six rows; Up / Down pick, Enter or Tab
/// takes one). Esc closes it (after closing an open mention list). Focus goes to the reply box
/// when it opens.
pub struct ThreadPopover<'a> {
    id_salt: IdSalt,
    thread: &'a CommentThread,
    anchor: Rect,
    me: &'a str,
    people: &'a [String],
    draft: &'a mut String,
    labels: Option<ThreadLabels>,
}

impl<'a> ThreadPopover<'a> {
    pub fn new(id_salt: impl AsIdSalt, thread: &'a CommentThread, anchor: Rect, me: &'a str, people: &'a [String], draft: &'a mut String) -> Self {
        Self { id_salt: IdSalt::new(id_salt), thread, anchor, me, people, draft, labels: None }
    }

    pub fn labels(mut self, labels: ThreadLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // figcraft: crates/ui-egui/src/canvas/comments.rs:497 `composer`, `:601` `Act`, the popover
    // after it.
    pub fn show(self, ui: &mut Ui) -> ThreadPopoverOutput {
        let Self { id_salt, thread, anchor, me, people, draft, labels } = self;
        let labels = labels.unwrap_or_else(|| ThreadLabels::of(ui));
        let tokens = Tokens::of(ui);
        let style = ReviewStyle::of(&tokens);
        let id = ui.make_persistent_id(id_salt);
        let cx = PopoverCx { tokens: &tokens, labels: &labels, id, thread, me, people };
        let mut edits = Vec::new();
        let mut open = true;
        let point = pos2(anchor.right() + tokens.space.gap.inline, anchor.top());
        let name = thread.messages.first().map_or_else(String::new, |m| fmt(&labels.pin, &[("author", &m.author)]));
        let shown = Popover::new(id.with("popover"), PopoverAnchor::Point(point)).close(PopoverClose::default().escape(false)).label(name).show(
            ui,
            &mut open,
            |ui, pop| {
                pop.initial_focus(reply_field(&cx));
                ui.set_width(style.popover_width);
                ui.spacing_mut().item_spacing.y = tokens.space.gap.row;
                popover_header(ui, &cx, pop, &mut edits);
                let mut escape_used = false;
                for index in 0..thread.messages.len() {
                    escape_used |= message_view(ui, &cx, index, &mut edits);
                }
                escape_used |= reply_composer(ui, &cx, draft, &mut edits);
                if !escape_used && ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape)) {
                    pop.close();
                }
            },
        );
        ThreadPopoverOutput { edits, closed: shown.is_none_or(|shown| shown.closed_by.is_some()) }
    }
}

/// What the thread popover did this pass.
#[must_use]
pub struct ThreadPopoverOutput {
    /// This pass's edits, in order.
    pub edits: Vec<CommentEdit>,
    /// It closed this pass (Esc, a click outside).
    pub closed: bool,
}

/// The floating composer card for a new comment at a point (PdfCraft's new note / text box,
/// FigCraft's new comment): a multi-line field with the focus, Post (Cmd+Enter) and Cancel (Esc).
/// When its anchor scrolls out of view the draft is kept and the card hidden until it is back.
pub struct CommentComposer<'a> {
    id_salt: IdSalt,
    anchor: Option<Pos2>,
    draft: &'a mut String,
    labels: Option<ThreadLabels>,
}

impl<'a> CommentComposer<'a> {
    pub fn new(id_salt: impl AsIdSalt, anchor: Option<Pos2>, draft: &'a mut String) -> Self {
        Self { id_salt: IdSalt::new(id_salt), anchor, draft, labels: None }
    }

    // pdfcraft: crates/ui-egui/src/tools/comments/composer.rs:9 `composer`.
    pub fn show(self, ui: &mut Ui) -> CommentComposerResponse {
        let Self { id_salt, anchor, draft, labels } = self;
        let mut response = CommentComposerResponse { posted: None, closed: false };
        let Some(anchor) = anchor else { return response };
        let labels = labels.unwrap_or_else(|| ThreadLabels::of(ui));
        let tokens = Tokens::of(ui);
        let style = ReviewStyle::of(&tokens);
        let id = ui.make_persistent_id(id_salt);
        let field_id = id.with("field");
        let mut open = true;
        Popover::new(id.with("card"), PopoverAnchor::Point(anchor))
            .close(PopoverClose::default().outside(false).escape(false).focus_left(false))
            .label(&labels.add_comment)
            .show(ui, &mut open, |ui, pop| {
                pop.initial_focus(field_id);
                ui.set_width(style.popover_width);
                ui.spacing_mut().item_spacing.y = tokens.space.gap.row;
                let field = TextField::new(id.with("text"), &labels.add_comment, draft.as_str())
                    .id(field_id)
                    .multiline(COMPOSER_ROWS)
                    .placeholder(&labels.add_comment)
                    .show(ui);
                if let Some(value) = field.event.as_ref().and_then(EditEvent::value) {
                    draft.clone_from(value);
                }
                let text = draft.trim().to_owned();
                let (post, cancel) = ui
                    .horizontal(|ui| {
                        let post =
                            ui.add_enabled(!text.is_empty(), Button::new(&labels.post).variant(ButtonVariant::Primary).id_salt("post")).clicked();
                        (post, ui.add(Button::new(&labels.close).id_salt("cancel")).clicked())
                    })
                    .inner;
                if (post || field.submitted) && !text.is_empty() {
                    response.posted = Some(text);
                    response.closed = true;
                } else if cancel || field.escaped || ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape)) {
                    response.closed = true;
                }
            });
        if response.closed {
            draft.clear();
        }
        response
    }
}

/// What the composer did this pass.
#[must_use]
pub struct CommentComposerResponse {
    /// The text on Post.
    pub posted: Option<String>,
    /// It closed this pass (Post or Cancel).
    pub closed: bool,
}

/// A pin's bubble: round on three corners, square at the spot (bottom-left).
fn bubble(style: &ReviewStyle, rect: Rect) -> CornerRadius {
    let radius = corner_radius(rect.height().min(style.pin) / 2.0);
    CornerRadius { sw: 0, ..radius }
}

fn pin_rect(style: &ReviewStyle, at: Pos2, width: f32) -> Rect {
    Rect::from_min_max(pos2(at.x, at.y - style.pin), pos2(at.x + width, at.y))
}

/// What a pin being dragged keeps between passes.
#[derive(Clone, Copy)]
struct PinDrag {
    gesture: GestureId,
    grab: Vec2,
    cancelled: bool,
}

/// What the pins read.
#[derive(Clone, Copy)]
struct PinCx<'a> {
    tokens: &'a UiTokens,
    style: &'a ReviewStyle,
    labels: &'a ThreadLabels,
    id: Id,
}

fn single_pin(ui: &mut Ui, cx: &PinCx, (thread, at, author): (ThreadId, Pos2, &str), me: &str, open: bool, out: &mut CommentPinsOutput) -> Response {
    let PinCx { tokens, style, labels, id } = *cx;
    let key = id.with(("pin", thread));
    let drag_key = key.with("drag");
    let mine = author == me;
    let dragging: Option<PinDrag> = ui.data(|d| d.get_temp(drag_key));
    let pointer = ui.input(|i| i.pointer.interact_pos());
    let shown_at = match (dragging, pointer) {
        (Some(drag), Some(pointer)) if !drag.cancelled => pointer - drag.grab + vec2(0.0, style.pin),
        _ => at,
    };
    let rect = pin_rect(style, shown_at, style.pin);
    let sense = if mine { Sense::click_and_drag() } else { Sense::click() };
    let response = ui.interact(rect, key, sense);
    let name = fmt(&labels.pin, &[("author", author)]);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &name));
    if response.clicked() || (response.has_focus() && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter))) {
        out.opened = Some(thread);
    }
    if mine {
        drag_pin(ui, &response, thread, rect, drag_key, out);
    }
    paint_bubble(ui, tokens, style, rect, open || response.hovered() || response.has_focus(), |ui, inner| {
        let color = tokens.color.text.on_strong.to_egui();
        ui.painter().circle_filled(inner.center(), inner.height() / 2.0, tokens.color.accent.strong.to_egui());
        let galley = ui.painter().layout_job(LayoutJob::single_section(initials(author), role_format(tokens, style.caption_text, color)));
        ui.painter().galley(inner.center() - galley.size() / 2.0, galley, color);
    });
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn drag_pin(ui: &mut Ui, response: &Response, thread: ThreadId, rect: Rect, drag_key: Id, out: &mut CommentPinsOutput) {
    let dragging: Option<PinDrag> = ui.data(|d| d.get_temp(drag_key));
    let pointer = ui.input(|i| i.pointer.interact_pos());
    let modifiers = ui.input(|i| i.modifiers);
    let spot = |pointer: Pos2, grab: Vec2| pointer - grab + vec2(0.0, rect.height());
    if response.drag_started()
        && let Some(pointer) = pointer
    {
        let grab = pointer - rect.min;
        let gesture = GestureId::next();
        ui.data_mut(|d| d.insert_temp(drag_key, PinDrag { gesture, grab, cancelled: false }));
        out.moved = Some(EditEvent::begin_with(gesture, PinMove { thread, to: spot(pointer, grab) }));
        return;
    }
    let Some(drag) = dragging else { return };
    if drag.cancelled {
        if !response.dragged() {
            ui.data_mut(|d| d.remove::<PinDrag>(drag_key));
        }
        return;
    }
    if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
        ui.data_mut(|d| d.insert_temp(drag_key, PinDrag { cancelled: true, ..drag }));
        out.moved = Some(EditEvent::cancel(drag.gesture, CancelReason::Escape));
    } else if response.drag_stopped() {
        ui.data_mut(|d| d.remove::<PinDrag>(drag_key));
        let to = pointer.map_or(rect.left_bottom(), |pointer| spot(pointer, drag.grab));
        out.moved = Some(EditEvent::commit(drag.gesture, PinMove { thread, to }, true, modifiers));
    } else if response.dragged()
        && let Some(pointer) = pointer
    {
        out.moved = Some(EditEvent::update(drag.gesture, PinMove { thread, to: spot(pointer, drag.grab) }, modifiers));
    }
}

fn cluster_pin(ui: &mut Ui, cx: &PinCx, spot: &PinCluster, pins: &[(ThreadId, Pos2, &str)], out: &mut CommentPinsOutput) -> Response {
    let PinCx { tokens, style, labels, id } = *cx;
    let color = tokens.color.text.on_strong.to_egui();
    let count = spot.threads.len().to_string();
    let galley = ui.painter().layout_job(LayoutJob::single_section(count, role_format(tokens, style.caption_text, color)));
    let width = style.pin.max(galley.size().x + 2.0 * tokens.space.pad.control);
    let rect = pin_rect(style, spot.at, width);
    let key = id.with(("cluster", spot.threads.first()));
    let response = ui.interact(rect, key, Sense::click());
    let name = fmt(&labels.cluster, &[("n", &spot.threads.len().to_string())]);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &name));
    let entries: Vec<MenuEntry<ThreadId>> = spot
        .threads
        .iter()
        .filter_map(|thread| pins.iter().find(|(id, ..)| id == thread))
        .map(|(thread, _, author)| MenuItem::new(fmt(&labels.pin, &[("author", author)]), *thread).into())
        .collect();
    if let Some(thread) = PressMenu::new(&entries).show(ui, &response).chosen {
        out.opened = Some(thread);
    }
    paint_bubble(ui, tokens, style, rect, response.hovered() || response.has_focus(), |ui, inner| {
        ui.painter().rect_filled(inner, corner_radius(inner.height() / 2.0), tokens.color.accent.strong.to_egui());
        ui.painter().galley(inner.center() - galley.size() / 2.0, galley.clone(), color);
    });
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A speech-bubble pin: raised and accented while `hot`; `inner` paints inside it.
fn paint_bubble(ui: &mut Ui, tokens: &UiTokens, style: &ReviewStyle, rect: Rect, hot: bool, inner: impl FnOnce(&mut Ui, Rect)) {
    let radius = bubble(style, rect);
    let fill = if hot { tokens.color.accent.soft } else { tokens.color.surface.raised }.to_egui();
    ui.painter().rect_filled(rect, radius, fill);
    ui.painter().rect_stroke(rect, radius, Stroke::new(1.0, tokens.color.line.border.to_egui()), StrokeKind::Inside);
    inner(ui, rect.shrink(style.pin_inset));
}

/// What the popover's parts read.
struct PopoverCx<'a> {
    tokens: &'a UiTokens,
    labels: &'a ThreadLabels,
    id: Id,
    thread: &'a CommentThread,
    me: &'a str,
    people: &'a [String],
}

fn reply_field(cx: &PopoverCx) -> Id {
    cx.id.with("reply")
}

/// Resolve, copy link, mark as unread and close.
fn popover_header(ui: &mut Ui, cx: &PopoverCx, pop: &craft_ui::popover::PopoverCtx, edits: &mut Vec<CommentEdit>) {
    let (labels, thread) = (cx.labels, cx.thread.id);
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        if ui.add(IconButton::new(Icon::Close, &labels.close).id_salt("close")).clicked() {
            pop.close();
        }
        let resolved = cx.thread.resolved;
        let name = if resolved { &labels.reopen } else { &labels.resolve };
        if ui.add(IconButton::new(Icon::Check, name).selected(resolved).id_salt("resolve")).clicked() {
            edits.push(CommentEdit::Resolve { thread, resolved: !resolved });
        }
        if ui.add(IconButton::new(Icon::EyeOff, &labels.mark_unread).id_salt("unread")).clicked() {
            edits.push(CommentEdit::MarkUnread { thread });
        }
        if ui.add(IconButton::new(Icon::Link, &labels.copy_link).id_salt("link")).clicked() {
            edits.push(CommentEdit::CopyLink { thread });
        }
    });
}

/// Where each `@name` of `names` stands in `text`: an `@` at the start or after a
/// non-alphanumeric character, then a whole name.
fn mention_ranges(text: &str, names: &[&str]) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut from = 0;
    while let Some(offset) = text[from..].find('@') {
        let at = from + offset;
        let starts = text[..at].chars().next_back().is_none_or(|c| !c.is_alphanumeric());
        let rest = &text[at + 1..];
        let name = names
            .iter()
            .filter(|name| !name.is_empty() && rest.starts_with(**name) && rest[name.len()..].chars().next().is_none_or(|c| !c.is_alphanumeric()))
            .max_by_key(|name| name.len());
        match name {
            Some(name) if starts => {
                ranges.push(at..at + 1 + name.len());
                from = at + 1 + name.len();
            }
            _ => from = at + 1,
        }
    }
    ranges
}

/// A message's text with its @mentions in the accent.
fn mention_text(ui: &mut Ui, cx: &PopoverCx, message: &str) {
    let tokens = cx.tokens;
    let mut names: Vec<&str> = cx.people.iter().map(String::as_str).collect();
    names.push(cx.me);
    let plain = role_format(tokens, ReviewStyle::of(tokens).body_text, tokens.color.text.normal.to_egui());
    let mention = role_format(tokens, ReviewStyle::of(tokens).strong_text, tokens.color.accent.text.to_egui());
    let mut job = LayoutJob::default();
    let mut at = 0;
    for range in mention_ranges(message, &names) {
        job.append(&message[at..range.start], 0.0, plain.clone());
        job.append(&message[range.clone()], 0.0, mention.clone());
        at = range.end;
    }
    job.append(&message[at..], 0.0, plain);
    ui.add(egui::Label::new(job).wrap());
}

/// One message with its buttons, reactions and inline editor. True when Esc was used.
fn message_view(ui: &mut Ui, cx: &PopoverCx, index: usize, edits: &mut Vec<CommentEdit>) -> bool {
    let (tokens, labels, thread) = (cx.tokens, cx.labels, cx.thread.id);
    let Some(message) = cx.thread.messages.get(index) else { return false };
    let edit_key = cx.id.with(("editing", index));
    let mut escape_used = false;
    ui.horizontal_top(|ui| {
        avatar(ui, tokens, &message.author);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = tokens.space.gap.inline;
            ui.horizontal(|ui| {
                text(ui, tokens, ReviewStyle::of(tokens).strong_text, tokens.color.text.normal.to_egui(), &message.author);
                text(
                    ui,
                    tokens,
                    ReviewStyle::of(tokens).caption_text,
                    tokens.color.text.muted.to_egui(),
                    age_text(ui.ctx(), message.modified.unwrap_or(message.created)),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if message.author == cx.me {
                        if ui.add(IconButton::new(Icon::Trash, &labels.delete).id_salt(("delete", index))).clicked() {
                            edits.push(CommentEdit::Delete { thread, index });
                        }
                        if ui.add(IconButton::new(Icon::Pencil, &labels.edit).id_salt(("edit", index))).clicked() {
                            ui.data_mut(|d| d.insert_temp(edit_key, message.text.clone()));
                        }
                    }
                    let entries: Vec<MenuEntry<String>> = REACTIONS
                        .iter()
                        .map(|emoji| {
                            let mine = message.reactions.iter().any(|r| r.emoji == *emoji && r.people.iter().any(|p| p == cx.me));
                            MenuItem::new(*emoji, (*emoji).to_owned()).check(mine).into()
                        })
                        .collect();
                    let react = ui.add(IconButton::new(Icon::Plus, &labels.react).id_salt(("react", index)));
                    if let Some(emoji) = PressMenu::new(&entries).show(ui, &react).chosen {
                        edits.push(CommentEdit::React { thread, index, emoji });
                    }
                });
            });
            let draft: Option<String> = ui.data(|d| d.get_temp(edit_key));
            match draft {
                Some(draft) => escape_used = inline_editor(ui, cx, index, draft, edit_key, edits),
                None => mention_text(ui, cx, &message.text),
            }
            ui.horizontal_wrapped(|ui| {
                for reaction in &message.reactions {
                    let mut on = reaction.people.iter().any(|p| p == cx.me);
                    let chip = ToggleButton::new(format!("{} {}", reaction.emoji, reaction.people.len()), &mut on).size(ControlSize::Sm).id_salt((
                        "reaction",
                        index,
                        &reaction.emoji,
                    ));
                    let response = ui.add(chip);
                    if response.clicked() {
                        edits.push(CommentEdit::React { thread, index, emoji: reaction.emoji.clone() });
                    }
                    response.on_hover_text(reaction.people.join(", "));
                }
            });
        });
    });
    escape_used
}

/// A message being edited; Cmd+Enter or Post saves, Esc leaves. True when Esc was used.
fn inline_editor(ui: &mut Ui, cx: &PopoverCx, index: usize, mut draft: String, key: Id, edits: &mut Vec<CommentEdit>) -> bool {
    let labels = cx.labels;
    let field = TextField::new(cx.id.with(("edit-field", index)), &labels.edit, &draft).multiline(COMPOSER_ROWS).show(ui);
    if let Some(value) = field.event.as_ref().and_then(EditEvent::value) {
        draft.clone_from(value);
    }
    let text = draft.trim().to_owned();
    let post = ui.add_enabled(!text.is_empty(), Button::new(&labels.post).variant(ButtonVariant::Primary).id_salt(("save", index))).clicked();
    if (post || field.submitted) && !text.is_empty() {
        edits.push(CommentEdit::Edit { thread: cx.thread.id, index, text });
        ui.data_mut(|d| d.remove::<String>(key));
    } else if field.escaped {
        ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape));
        ui.data_mut(|d| d.remove::<String>(key));
        return true;
    } else {
        ui.data_mut(|d| d.insert_temp(key, draft));
    }
    false
}

/// The @mention list of a reply field, kept between passes.
#[derive(Clone, Default)]
struct Mention {
    range: (usize, usize),
    names: Vec<String>,
    selected: usize,
    /// The `@` the viewer dismissed the list for with Esc.
    dismissed: Option<usize>,
}

fn set_cursor(ctx: &egui::Context, id: Id, at: usize) {
    let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
    state.cursor.set_char_range(Some(CCursorRange::one(CCursor::new(at))));
    egui::TextEdit::store_state(ctx, id, state);
}

/// The reply field with @mention completion and Post. True when Esc was used.
fn reply_composer(ui: &mut Ui, cx: &PopoverCx, draft: &mut String, edits: &mut Vec<CommentEdit>) -> bool {
    let labels = cx.labels;
    let field_id = reply_field(cx);
    let key = field_id.with("mention");
    let mut mention: Mention = ui.data(|d| d.get_temp(key)).unwrap_or_default();
    let mut escape_used = false;
    if ui.memory(|m| m.has_focus(field_id)) && !mention.names.is_empty() {
        let count = mention.names.len();
        ui.input_mut(|input| {
            if input.consume_key(Modifiers::NONE, Key::ArrowDown) {
                mention.selected = (mention.selected + 1) % count;
            }
            if input.consume_key(Modifiers::NONE, Key::ArrowUp) {
                mention.selected = (mention.selected + count - 1) % count;
            }
        });
        if ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape)) {
            mention.dismissed = Some(mention.range.0);
            mention.names.clear();
            escape_used = true;
        } else if ui.input_mut(|input| {
            (!input.modifiers.shift && input.consume_key(Modifiers::NONE, Key::Enter)) || input.consume_key(Modifiers::NONE, Key::Tab)
        }) && let Some(name) = mention.names.get(mention.selected).cloned()
        {
            let at = insert_mention(draft, mention.range, &name);
            set_cursor(ui.ctx(), field_id, at);
            mention.names.clear();
        }
    }
    let field = TextField::new(cx.id.with("reply-text"), &labels.reply, draft.as_str())
        .id(field_id)
        .multiline(COMPOSER_ROWS)
        .placeholder(&labels.reply)
        .show(ui);
    if let Some(value) = field.event.as_ref().and_then(EditEvent::value) {
        draft.clone_from(value);
    }
    let cursor = ui
        .memory(|m| m.has_focus(field_id))
        .then(|| egui::TextEdit::load_state(ui.ctx(), field_id))
        .flatten()
        .and_then(|state| state.cursor.char_range())
        .map(|range| range.primary.index.0);
    let found = cursor.and_then(|c| mention_query(draft, c)).filter(|range| mention.dismissed != Some(range.0));
    let names = found.map(|range| mention_matches(cx.people, draft.get(range.0 + 1..range.1).unwrap_or(""), cx.me)).unwrap_or_default();
    match found {
        Some(range) if !names.is_empty() => {
            if mention.range.0 != range.0 || mention.names.is_empty() {
                mention.selected = 0;
            }
            mention.selected = mention.selected.min(names.len() - 1);
            mention.range = range;
            mention.names = names;
        }
        _ if cursor.is_some() => mention.names.clear(),
        _ => {}
    }
    let mut chosen = None;
    for (row, name) in mention.names.iter().enumerate() {
        let mut on = row == mention.selected;
        let response = ui.add(ToggleButton::new(name, &mut on).size(ControlSize::Sm).id_salt(("mention", row)));
        if response.clicked() {
            chosen = Some(name.clone());
        }
    }
    if let Some(name) = chosen {
        let at = insert_mention(draft, mention.range, &name);
        set_cursor(ui.ctx(), field_id, at);
        ui.memory_mut(|m| m.request_focus(field_id));
        mention.names.clear();
    }
    ui.data_mut(|d| d.insert_temp(key, mention));
    let text = draft.trim().to_owned();
    let post = ui.add_enabled(!text.is_empty(), Button::new(&labels.post).variant(ButtonVariant::Primary).id_salt("reply-post")).clicked();
    if (post || field.submitted) && !text.is_empty() {
        edits.push(CommentEdit::Reply { thread: cx.thread.id, text });
        draft.clear();
    }
    escape_used
}
