//! Comment pins on a canvas, the thread popover and the composer (N5.02).

use std::ops::Range;

use craft_theme::{ButtonVariant, ControlSize, Stroke as StrokeWidth};
use craft_ui::controls::{Autocomplete, Button, IconButton, Suggestion, TextField, ToggleButton};
use craft_ui::edit::{CancelReason, EditEvent, GestureId};
use craft_ui::icons::{self, Icon, IconSize};
use craft_ui::localized::{Labels as _, fmt};
use craft_ui::menu::{MenuEntry, MenuItem, PressMenu};
use craft_ui::popover::{Popover, PopoverAnchor, PopoverClose};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, UiTokens, corner_radius, role_format};
use egui::text::{CCursor, CCursorRange, LayoutJob};
use egui::{
    Align, AsIdSalt, Color32, CornerRadius, Id, IdSalt, Key, Layout, Modifiers, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2, pos2, vec2,
};

use super::ReviewStyle;
use super::comments::{CommentEdit, age_text, avatar, text};
use super::presence::paint_avatar;
use crate::comments::{CommentThread, ThreadId};

craft_ui::words! {
    /// The thread popover's and composer's words, English by default (FigCraft's).
    #[non_exhaustive]
    pub struct ThreadLabels {
        reply: "thread.reply" = "Reply";
        add_comment: "thread.addComment" = "Add a comment";
        post: "thread.post" = "Post";
        edit: "thread.edit" = "Edit";
        delete: "thread.delete" = "Delete";
        delete_thread: "thread.deleteThread" = "Delete thread";
        more: "thread.more" = "More";
        cancel: "thread.cancel" = "Cancel";
        resolve: "thread.resolve" = "Resolve";
        reopen: "thread.reopen" = "Reopen";
        copy_link: "thread.copyLink" = "Copy link";
        mark_unread: "thread.markUnread" = "Mark as unread";
        react: "thread.react" = "Add reaction";
        close: "thread.close" = "Close";
        pin: "thread.pin" = "Comment by {author}";
        pin_text: "thread.pinText" = "Comment by {author}: {words}";
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
    /// Centre of the pin or cluster, in screen points.
    pub at: Pos2,
    /// The threads it stands for.
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

/// One thread's pin: where it is (screen), who started it and how it looks (FigCraft
/// `canvas/comments.rs:352` `paint_pin`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CommentPin<'a> {
    /// The thread this pin belongs to.
    pub id: ThreadId,
    /// The pin's spot in screen coordinates.
    pub point: Pos2,
    /// The author of the thread's first message.
    pub author: &'a str,
    /// The thread's first message, for the pin's accessible name (its first words).
    pub text: &'a str,
    /// The author's colour (straight RGB), as the people bar shows them.
    pub color: [u8; 3],
    /// The viewer hasn't read the thread: a dot on the pin.
    pub unread: bool,
    /// The thread is resolved: a grey avatar and a check.
    pub resolved: bool,
}

/// Comment pins over a canvas (Figma's, as FigCraft draws them): a `review.pin` pin per thread
/// ([`ReviewStyle`]; the author's avatar in the author's colour in a
/// speech-bubble shape with a square bottom-left corner, grey with a check once resolved, a dot
/// while unread), pins within `review.cluster_radius` gathered into a cluster showing the count; a
/// click on a pin opens its thread, a click on the open pin closes it, a click on a cluster lists
/// it. Your own pin can be dragged (past egui's drag threshold) to another spot, reported as a
/// [`PinMove`] gesture (Esc cancels). Each pin is a Tab stop in reading order, Enter opens it;
/// AccessKit `Button`s named by author and the thread's first words.
pub struct CommentPins<'a> {
    id_salt: IdSalt,
    pins: &'a [CommentPin<'a>],
    me: &'a str,
    open: Option<ThreadId>,
    labels: Option<ThreadLabels>,
}

impl<'a> CommentPins<'a> {
    /// `pins`: each thread's pin.
    pub fn new(id_salt: impl AsIdSalt, pins: &'a [CommentPin<'a>], me: &'a str) -> Self {
        Self { id_salt: IdSalt::new(id_salt), pins, me, open: None, labels: None }
    }

    /// The thread whose popover is open (its pin is drawn raised).
    pub fn open(mut self, thread: Option<ThreadId>) -> Self {
        self.open = thread;
        self
    }

    /// Words other than the installed ones.
    pub fn labels(mut self, labels: ThreadLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // figcraft: crates/ui-egui/src/canvas/comments.rs:242 `pins`, `:352` `paint_pin`, `:370`
    // `paint_cluster`, `:385` `draw`, `:419` `input`, `:119` `PinDrag`.
    /// Shows the pins and reports clicks and drags.
    pub fn show(self, ui: &mut Ui) -> CommentPinsOutput {
        let Self { id_salt, pins, me, open, labels } = self;
        let labels = labels.unwrap_or_else(|| ThreadLabels::of(ui));
        let tokens = Tokens::of(ui);
        let style = ReviewStyle::of(&tokens);
        let id = ui.make_persistent_id(id_salt);
        let points: Vec<Pos2> = pins.iter().map(|pin| pin.point).collect();
        let mut spots: Vec<PinCluster> = cluster(&points, style.cluster_radius)
            .into_iter()
            .map(|members| {
                let sum = members.iter().fold(Vec2::ZERO, |sum, &i| sum + points[i].to_vec2());
                PinCluster { at: (sum / members.len() as f32).to_pos2(), threads: members.iter().map(|&i| pins[i].id).collect() }
            })
            .collect();
        spots.sort_by(|a, b| a.at.y.total_cmp(&b.at.y).then(a.at.x.total_cmp(&b.at.x)));
        let cx = PinCx { tokens: &tokens, style: &style, labels: &labels, id };
        let mut out = CommentPinsOutput { response: ui.interact(Rect::NOTHING, id, Sense::hover()), opened: None, closed: None, moved: None };
        for spot in &spots {
            let one = match spot.threads.as_slice() {
                [thread] => pins.iter().find(|pin| pin.id == *thread),
                _ => None,
            };
            let response = match one {
                Some(pin) => single_pin(ui, &cx, pin, me, open == Some(pin.id), &mut out),
                None => cluster_pin(ui, &cx, spot, pins, &mut out),
            };
            out.response = out.response.union(response);
        }
        out
    }
}

/// A pin dragged to a new screen point; the app maps it to its anchor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PinMove {
    /// The thread that was dragged.
    pub thread: ThreadId,
    /// Where it was dropped, in screen points.
    pub to: Pos2,
}

/// What the pins did this pass.
#[must_use]
pub struct CommentPinsOutput {
    /// The pins' response.
    pub response: Response,
    /// The thread whose pin or cluster entry was clicked while its popover was closed.
    pub opened: Option<ThreadId>,
    /// The thread whose open pin was clicked: its popover closes.
    pub closed: Option<ThreadId>,
    /// A drag of your own pin, as one gesture.
    pub moved: Option<EditEvent<PinMove>>,
}

/// The thread popover next to a pin (`review.popover_width` wide), on the kernel's
/// [`Popover`](craft_ui::popover): messages with avatar, author, age and
/// text (@mentions in the accent), reactions under each (click to toggle yours), your messages'
/// Edit and Delete, the thread's Resolve / Reopen check button and a "…" menu (Mark as unread,
/// Copy link and, on your own thread, Delete thread), and the reply composer at the bottom with
/// @mention completion through [`Autocomplete`] (up to six rows; Up / Down pick, Enter or Tab
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
    /// A popover for `thread` next to `anchor` (the pin's rectangle, in screen points). `me` is the viewer's name and `people` the names offered for @mentions; `draft` holds the reply text.
    pub fn new(id_salt: impl AsIdSalt, thread: &'a CommentThread, anchor: Rect, me: &'a str, people: &'a [String], draft: &'a mut String) -> Self {
        Self { id_salt: IdSalt::new(id_salt), thread, anchor, me, people, draft, labels: None }
    }

    /// Uses these label texts instead of the ones read from the active locale's catalog.
    pub fn labels(mut self, labels: ThreadLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // figcraft: crates/ui-egui/src/canvas/comments.rs:497 `composer`, `:601` `Act`, the popover
    // after it.
    /// Shows the popover and reports the edits made.
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
        let shown = Popover::new(id.with("popover"), PopoverAnchor::Point(point))
            .close(PopoverClose::default().outside(false).escape(false))
            .label(name)
            .show(ui, &mut open, |ui, pop| {
                pop.initial_focus(reply_field(&cx));
                ui.set_width(style.popover_width);
                ui.spacing_mut().item_spacing.y = tokens.space.gap.row;
                popover_header(ui, &cx, pop, &mut edits);
                let mut escape_used = false;
                for index in 0..thread.messages.len() {
                    escape_used |= message_view(ui, &cx, index, &mut edits);
                }
                escape_used |= reply_composer(ui, &cx, pop, draft, &mut edits);
                if !escape_used && ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape)) {
                    pop.close();
                }
            });
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
    /// A composer at `anchor` (screen points; `None` hides it) that edits `draft`.
    pub fn new(id_salt: impl AsIdSalt, anchor: Option<Pos2>, draft: &'a mut String) -> Self {
        Self { id_salt: IdSalt::new(id_salt), anchor, draft, labels: None }
    }

    /// Words other than the installed ones.
    pub fn labels(mut self, labels: ThreadLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // pdfcraft: crates/ui-egui/src/tools/comments/composer.rs:9 `composer`.
    /// Shows the composer and reports whether the user posted or closed it.
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
                        (post, ui.add(Button::new(&labels.cancel).id_salt("cancel")).clicked())
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

/// The words of the thread's text a pin's accessible name carries.
const PIN_NAME_WORDS: usize = 8;

fn single_pin(ui: &mut Ui, cx: &PinCx, pin: &CommentPin, me: &str, open: bool, out: &mut CommentPinsOutput) -> Response {
    let PinCx { tokens, style, labels, id } = *cx;
    let CommentPin { id: thread, point: at, author, text, color, unread, resolved } = *pin;
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
    let words = text.split_whitespace().take(PIN_NAME_WORDS).collect::<Vec<_>>().join(" ");
    let name =
        if words.is_empty() { fmt(&labels.pin, &[("author", author)]) } else { fmt(&labels.pin_text, &[("author", author), ("words", &words)]) };
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &name));
    if response.clicked() || (response.has_focus() && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter))) {
        if open {
            out.closed = Some(thread);
        } else {
            out.opened = Some(thread);
        }
    }
    if mine {
        drag_pin(ui, &response, thread, rect, drag_key, out);
    }
    paint_bubble(ui, tokens, style, rect, open || response.hovered() || response.has_focus(), |ui, inner| {
        let fill = if resolved { tokens.color.text.faint.to_egui() } else { Color32::from_rgb(color[0], color[1], color[2]) };
        paint_avatar(ui.painter(), tokens, inner.center(), inner.height() / 2.0, author, fill);
        let accent = tokens.color.accent.strong.to_egui();
        if unread {
            let centre = rect.right_top() + vec2(-style.pin_dot_inset, style.pin_dot_inset);
            let ring = StrokeWidth::W1.width(ui.pixels_per_point());
            ui.painter().circle_filled(centre, style.pin_dot_radius + ring, tokens.color.surface.raised.to_egui());
            ui.painter().circle_filled(centre, style.pin_dot_radius, accent);
        }
        if resolved {
            let centre = rect.right_bottom() - Vec2::splat(style.pin_check_inset);
            icons::paint(ui, Rect::from_center_size(centre, Vec2::splat(tokens.size.icon.sm)), Icon::Check, IconSize::Sm, accent);
        }
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

fn cluster_pin(ui: &mut Ui, cx: &PinCx, spot: &PinCluster, pins: &[CommentPin], out: &mut CommentPinsOutput) -> Response {
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
        .filter_map(|thread| pins.iter().find(|pin| pin.id == *thread))
        .map(|pin| MenuItem::new(fmt(&labels.pin, &[("author", pin.author)]), pin.id).into())
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
    ui.painter().rect_stroke(
        rect,
        radius,
        Stroke::new(StrokeWidth::W1.width(ui.pixels_per_point()), tokens.color.line.border.to_egui()),
        StrokeKind::Inside,
    );
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

/// What the "…" menu offers.
#[derive(Clone, Copy)]
enum More {
    MarkUnread,
    CopyLink,
    DeleteThread,
}

/// Resolve (a check button), the "…" menu (Mark as unread, Copy link, Delete thread when it is
/// yours: Figma's) and close.
fn popover_header(ui: &mut Ui, cx: &PopoverCx, pop: &craft_ui::popover::PopoverCtx, edits: &mut Vec<CommentEdit>) {
    let (labels, thread) = (cx.labels, cx.thread.id);
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        if ui.add(IconButton::new(Icon::Close, &labels.close).id_salt("close")).clicked() {
            pop.close();
        }
        let mut entries: Vec<MenuEntry<More>> = vec![
            MenuItem::new(&labels.mark_unread, More::MarkUnread).icon(Icon::EyeOff).into(),
            MenuItem::new(&labels.copy_link, More::CopyLink).icon(Icon::Link).into(),
        ];
        if cx.thread.messages.first().is_some_and(|first| first.author == cx.me) {
            entries.push(MenuItem::new(&labels.delete_thread, More::DeleteThread).icon(Icon::Trash).into());
        }
        let more = ui.add(IconButton::new(Icon::More, &labels.more).id_salt("more"));
        match PressMenu::new(&entries).show(ui, &more).chosen {
            Some(More::MarkUnread) => edits.push(CommentEdit::MarkUnread { thread }),
            Some(More::CopyLink) => edits.push(CommentEdit::CopyLink { thread }),
            Some(More::DeleteThread) => edits.push(CommentEdit::Delete { thread, index: 0 }),
            None => {}
        }
        let resolved = cx.thread.resolved;
        let name = if resolved { &labels.reopen } else { &labels.resolve };
        if ui.add(IconButton::new(Icon::Check, name).selected(resolved).id_salt("resolve")).clicked() {
            edits.push(CommentEdit::Resolve { thread, resolved: !resolved });
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

fn set_cursor(ctx: &egui::Context, id: Id, at: usize) {
    let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
    state.cursor.set_char_range(Some(CCursorRange::one(CCursor::new(at))));
    egui::TextEdit::store_state(ctx, id, state);
}

/// The `@query` at the reply field's cursor and the people it matches; `None` when there is no
/// query or nobody matches. The list itself shows only while the field or the list has the focus,
/// and a press on a row moves the focus to the row, so the cursor is read without a focus test.
fn mention_at_cursor(ui: &Ui, cx: &PopoverCx, field_id: Id, draft: &str) -> Option<((usize, usize), Vec<String>)> {
    let cursor = egui::TextEdit::load_state(ui.ctx(), field_id)?.cursor.char_range()?.primary.index.0;
    let range = mention_query(draft, cursor)?;
    let names = mention_matches(cx.people, draft.get(range.0 + 1..range.1).unwrap_or(""), cx.me);
    (!names.is_empty()).then_some((range, names))
}

fn suggestions(names: &[String]) -> Vec<Suggestion> {
    names.iter().map(|name| Suggestion::new(name.as_str())).collect()
}

/// The reply field with @mention completion (`Autocomplete`, Enter accepts) and Post. True when
/// Esc was used. `pop` closes on a press outside the popover and the list.
fn reply_composer(ui: &mut Ui, cx: &PopoverCx, pop: &craft_ui::popover::PopoverCtx, draft: &mut String, edits: &mut Vec<CommentEdit>) -> bool {
    let labels = cx.labels;
    let field_id = reply_field(cx);
    let list_id = cx.id.with("mentions");
    let before = mention_at_cursor(ui, cx, field_id, draft).map(|(_, names)| suggestions(&names)).unwrap_or_default();
    let keys = Autocomplete::new(list_id, field_id, &before).enter_accepts(true).take_keys(ui);
    let field = TextField::new(cx.id.with("reply-text"), &labels.reply, draft.as_str())
        .id(field_id)
        .multiline(COMPOSER_ROWS)
        .placeholder(&labels.reply)
        .show(ui);
    if let Some(value) = field.event.as_ref().and_then(EditEvent::value) {
        draft.clone_from(value);
    }
    let after = mention_at_cursor(ui, cx, field_id, draft);
    let shown = after.as_ref().map(|(_, names)| suggestions(names)).unwrap_or_default();
    let list = Autocomplete::new(list_id, field_id, &shown).enter_accepts(true).show(ui, field.response.rect, keys);
    // The suggestion list is drawn on the tooltip layer, so the popover doesn't count it as its
    // own: a press outside closes the popover unless it landed in the list.
    let in_list = list.popup.as_ref().zip(ui.input(|i| i.pointer.interact_pos())).is_some_and(|(popup, at)| popup.rect.contains(at));
    if pop.pressed_outside() && !in_list {
        pop.close();
    }
    if let Some(((range, names), picked)) = after.zip(list.accepted)
        && let Some(name) = names.get(picked)
    {
        let at = insert_mention(draft, range, name);
        set_cursor(ui.ctx(), field_id, at);
        ui.memory_mut(|m| m.request_focus(field_id));
    }
    let escape_used = keys.dismissed;
    let text = draft.trim().to_owned();
    let post = ui.add_enabled(!text.is_empty(), Button::new(&labels.post).variant(ButtonVariant::Primary).id_salt("reply-post")).clicked();
    if (post || field.submitted) && !text.is_empty() {
        edits.push(CommentEdit::Reply { thread: cx.thread.id, text });
        draft.clear();
    }
    escape_used
}
