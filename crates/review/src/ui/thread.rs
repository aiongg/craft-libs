//! Comment pins on a canvas, the thread popover and the composer.
//!
//! ```ignore
//! let pins = CommentPins::new("Comment pins", &pins, "Ana").open(open).show(ui);
//! if let Some(thread) = pins.opened { open = Some(thread) }
//! ```

#[cfg(feature = "screen-testing")]
mod stories;

use std::ops::Range;

use craft_behavior::{IconId, Metrics, PressPhase, Proposal, Sense};
use craft_theme::variant::Tone;
use craft_ui::buttons::{Button, IconButton, ToggleButton};
use craft_ui::contract::overlay::Anchor;
use craft_ui::contract::{
    Bind, Common, Component, Cx, MeasureCx, NoValue, Outcome, PaintUi, Pattern, Pointer as _, Props as _, Remember as _, Role, Show as _,
    ShowContainer as _, Trigger, clicked,
};
use craft_ui::display::{Shade, Text};
use craft_ui::layout_components::{Row, Stack};
use craft_ui::menu::{Menu, MenuItem};
use craft_ui::popover::Popover;
use craft_ui::primitives::{Completer, Completion, HAlign, IconSpec, PaintShape, StrokeWidth, TextRun};
use craft_ui::runtime::env::{Env, Label};
use craft_ui::runtime::focus::Initial;
use craft_ui::style::{ComponentStyle, DataColor, StyleColor, StyleCx, TypeRole};
use craft_ui::text::TextField;
use egui::{Color32, CornerRadius, Pos2, Rect, Response, Ui, Vec2};

use super::avatar::{Avatar, paint_avatar, paint_face};
use super::comments::{CommentEdit, age_text};
use crate::comments::{CommentThread, ThreadId};

craft_ui::words! {
    /// The thread popover's and composer's words, English by default.
    #[non_exhaustive]
    pub struct ThreadWords {
        reply: "thread.reply" = "Reply";
        add_comment: "thread.addComment" = "Add a comment";
        post: "thread.post" = "Post";
        edit: "thread.edit" = "Edit";
        delete: "thread.delete" = "Delete";
        delete_thread: "thread.deleteThread" = "Delete thread";
        more: "thread.more" = "More";
        cancel: "thread.cancel" = "Cancel";
        close: "thread.close" = "Close";
        resolve: "thread.resolve" = "Resolve";
        reopen: "thread.reopen" = "Reopen";
        copy_link: "thread.copyLink" = "Copy link";
        mark_unread: "thread.markUnread" = "Mark as unread";
        react: "thread.react" = "Add reaction";
        pin: template "thread.pin" = "Comment by {author}";
        pin_text: template "thread.pinText" = "Comment by {author}: {words}";
        cluster: template "thread.cluster" = "{n} comments";
    }
}

/// The mention list's rows.
const MENTION_ROWS: usize = 6;

/// The rows of the new comment's field.
const COMPOSER_ROWS: usize = 3;

/// The longest `@query` the picker follows.
const MENTION_QUERY_CHARS: usize = 40;

/// The words of the thread's text a pin's accessible name carries.
const PIN_NAME_WORDS: usize = 8;

/// The reactions a message offers.
pub const REACTIONS: [&str; 8] = ["👍", "❤", "😂", "🎉", "👀", "🙏", "🔥", "✅"];

/// A pin or a cluster of pins on screen.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct PinCluster {
    /// Centre of the pin or cluster, in screen points.
    pub at: Pos2,
    /// The threads it stands for.
    pub threads: Vec<ThreadId>,
}

/// Groups of `points` (screen): in order, each point joins the first group whose centre (the mean
/// of its points so far) is closer than `radius`, else starts one. Indices per group.
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
pub fn insert_mention(text: &mut String, range: (usize, usize), name: &str) -> usize {
    let (start, end) = range;
    if start > end || end > text.len() || !text.is_char_boundary(start) || !text.is_char_boundary(end) {
        return text.chars().count();
    }
    let inserted = format!("@{name} ");
    text.replace_range(start..end, &inserted);
    text.get(..start).map_or(0, |before| before.chars().count()) + inserted.chars().count()
}

/// Completes `@name` for the people a thread's reply box offers.
pub(crate) struct MentionCompleter {
    people: Vec<String>,
    me: String,
}

impl Completer for MentionCompleter {
    fn complete(&self, text: &str, caret: usize, out: &mut Vec<Completion>) -> Option<Range<usize>> {
        let cursor = text.get(..caret)?.chars().count();
        let range = mention_query(text, cursor)?;
        let names = mention_matches(&self.people, text.get(range.0 + 1..range.1).unwrap_or(""), &self.me);
        out.extend(names.iter().map(|name| Completion::new(name.clone(), format!("@{name} "))));
        (!names.is_empty()).then_some(range.0..range.1)
    }
}

/// One thread's pin: where it is (screen), who started it and how it looks.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
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

impl<'a> CommentPin<'a> {
    /// A read, unresolved pin of the thread `id` at `point`, started by `author` in their colour `color`; `text` is
    /// the thread's first message.
    pub fn new(id: ThreadId, point: Pos2, author: &'a str, text: &'a str, color: [u8; 3]) -> Self {
        CommentPin { id, point, author, text, color, unread: false, resolved: false }
    }

    /// Whether the viewer hasn't read the thread.
    #[must_use]
    pub fn unread(mut self, unread: bool) -> Self {
        self.unread = unread;
        self
    }

    /// Whether the thread is resolved.
    #[must_use]
    pub fn resolved(mut self, resolved: bool) -> Self {
        self.resolved = resolved;
        self
    }
}

/// A pin dragged to a new screen point; the app maps it to its anchor.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PinMove {
    /// The thread that was dragged.
    pub thread: ThreadId,
    /// Where it was dropped, in screen points.
    pub to: Pos2,
}

/// What a pin bubble shows.
#[derive(Clone, Debug, PartialEq)]
enum Face {
    Pin { author: String, color: [u8; 3], unread: bool, resolved: bool, open: bool, mine: bool },
    Cluster { count: usize },
}

/// What a pin bubble returns.
#[non_exhaustive]
pub(crate) struct PinOut {
    response: Response,
    activated: bool,
    dropped: Option<Pos2>,
}

impl Outcome for PinOut {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// The style of a pin bubble.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub(crate) struct PinStyle {
    #[style(metrics)]
    metrics: Metrics,
    fill: StyleColor,
    fill_hot: StyleColor,
    edge: StyleColor,
    dot: StyleColor,
    cluster: StyleColor,
    on_strong: StyleColor,
    resolved: StyleColor,
    resolved_ink: StyleColor,
    side: f32,
    inset: f32,
    dot_side: f32,
}

/// A speech-bubble pin or a cluster of them: round on three corners, square at the spot (bottom-left).
#[derive(craft_ui::contract::Component)]
pub(crate) struct PinBubble {
    #[common]
    common: Common,
    face: Face,
    line: f32,
    activated: bool,
    dropped: Option<Pos2>,
}

impl PinBubble {
    /// A bubble of `face`, named `name`.
    fn new(face: Face, name: String) -> Self {
        PinBubble { common: Common::new(), face, line: 0.0, activated: false, dropped: None }.label(name)
    }

    fn mine(&self) -> bool {
        matches!(self.face, Face::Pin { mine: true, .. })
    }

    /// How far the pointer has dragged your own pin from where it was pressed.
    fn lift(&self, cx: &Cx<'_, Self>) -> Vec2 {
        match cx.press().phase {
            PressPhase::DragStart { start, pos } | PressPhase::Drag { start, pos, .. } if self.mine() => pos - start.origin,
            _ => Vec2::ZERO,
        }
    }
}

impl Component for PinBubble {
    const ROLE: Role = Role::BUTTON;
    type Style = PinStyle;
    type Output = PinOut;

    fn style(cx: &StyleCx<'_>, _: PinBubbleStyleKey) -> PinStyle {
        let color = &cx.tokens.color;
        PinStyle {
            metrics: Metrics::new(),
            fill: cx.color(color.surface.raised),
            fill_hot: cx.color(color.accent.soft),
            edge: cx.color(color.line.border),
            dot: cx.color(color.accent.strong),
            cluster: cx.color(color.accent.strong),
            on_strong: cx.color(color.text.on_strong),
            resolved: cx.color(color.text.faint),
            resolved_ink: cx.ink_on(DataColor::new(cx.color(color.text.faint).egui())),
            side: cx.control().height,
            inset: cx.tokens.space.gap.inline,
            dot_side: cx.tokens.shape.radius.sm * 2.0,
        }
    }

    fn sense(&self) -> Sense {
        if self.mine() { Sense::CLICK | Sense::DRAG | Sense::FOCUS } else { Sense::CLICK | Sense::FOCUS }
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let line = cx.layout(&TextRun::new("0").role(TypeRole::Caption)).size().y;
        self.line = line;
        let width = match self.face {
            Face::Cluster { count } => {
                let digits = cx.layout(&TextRun::new(count.to_string()).role(TypeRole::Caption)).size().x;
                cx.style.side.max(digits + 2.0 * cx.style.inset)
            }
            _ => cx.style.side,
        };
        Vec2::new(width, cx.style.side)
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        self.activated = cx.activation().is_some();
        if let PressPhase::Release { start, pos, click: false, .. } = cx.press().phase
            && self.mine()
        {
            self.dropped = Some(cx.rect.left_bottom() + (pos - start.origin));
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let st = cx.style;
        let rect = cx.rect.translate(self.lift(cx));
        let radius = CornerRadius { sw: 0, ..CornerRadius::same((rect.height() / 2.0) as u8) };
        let hot = cx.response.hovered() || cx.response.has_focus() || matches!(self.face, Face::Pin { open: true, .. });
        let shape = PaintShape::Rect(rect, radius);
        let draw = ui.draw();
        draw.fill(&shape, if hot { st.fill_hot } else { st.fill });
        draw.stroke(&shape, StrokeWidth::Thin, st.edge);
        let inner = rect.shrink(st.inset);
        match &self.face {
            Face::Pin { author, color, unread, resolved, .. } => {
                if *resolved {
                    paint_face(&draw, inner, author, st.resolved, st.resolved_ink);
                } else {
                    let [r, g, b] = *color;
                    paint_avatar(ui, inner, author, DataColor::new(Color32::from_rgb(r, g, b)));
                }
                if *unread {
                    let centre = Pos2::new(rect.right() - st.inset, rect.top() + st.inset);
                    draw.fill(&PaintShape::Circle(centre, st.dot_side / 2.0), st.dot);
                }
                if *resolved {
                    let check = Rect::from_center_size(Pos2::new(rect.right() - st.inset, rect.bottom() - st.inset), Vec2::splat(st.dot_side * 2.0));
                    draw.icon(check, &IconSpec::svg(IconId::new("check")).color(st.dot));
                }
            }
            Face::Cluster { count } => {
                draw.fill(&PaintShape::Rect(inner, CornerRadius::same((inner.height() / 2.0) as u8)), st.cluster);
                let run = TextRun::new(count.to_string()).role(TypeRole::Caption).color(st.on_strong).align(HAlign::Center);
                let at = Rect::from_min_size(Pos2::new(inner.left(), inner.center().y - self.line / 2.0), Vec2::new(inner.width(), self.line));
                draw.text(at, &run);
            }
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> PinOut {
        PinOut { response: cx.response.clone(), activated: self.activated, dropped: self.dropped }
    }
}

/// Comment pins over a canvas: a pin per thread (the author's avatar in the author's colour in a speech
/// bubble with a square bottom-left corner, grey with a check once resolved, a dot while unread), pins
/// within one pin's width of each other gathered into a cluster showing the count. A click or Enter on a
/// pin opens its thread, on the open pin closes it, on a cluster lists its threads. Your own pin can be
/// dragged to another spot, reported as a [`PinMove`] on release. Each pin is a focus stop in reading order.
#[must_use = "show the pins with `show`"]
#[derive(Pattern)]
pub struct CommentPins<'a> {
    label: Label,
    pins: &'a [CommentPin<'a>],
    me: &'a str,
    open: Option<ThreadId>,
}

impl<'a> CommentPins<'a> {
    /// The pins named `label`, one per thread of `pins`; `me` is the viewer's name.
    pub fn new(label: impl Into<Label>, pins: &'a [CommentPin<'a>], me: &'a str) -> Self {
        CommentPins { label: label.into(), pins, me, open: None }
    }

    /// The thread whose popover is open (its pin is drawn raised).
    #[must_use]
    pub fn open(mut self, thread: Option<ThreadId>) -> Self {
        self.open = thread;
        self
    }
}

/// What the pins did this pass.
#[must_use]
#[non_exhaustive]
pub struct CommentPinsOutput {
    /// The pins' response.
    pub response: Response,
    /// The thread whose pin or cluster entry was clicked while its popover was closed.
    pub opened: Option<ThreadId>,
    /// The thread whose open pin was clicked: its popover closes.
    pub closed: Option<ThreadId>,
    /// A drag of your own pin, reported when it is dropped.
    pub moved: Option<PinMove>,
}

impl Outcome for CommentPinsOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

impl CommentPins<'_> {
    /// Shows the pins and reports clicks and drags.
    pub fn show(self, ui: &mut Ui) -> CommentPinsOutput {
        let CommentPins { label, pins, me, open } = self;
        let words = Env::of(ui).words::<ThreadWords>();
        let side = PinBubble::new(Face::Cluster { count: 1 }, String::new()).measure_in(ui, Proposal::Ideal).y;
        let points: Vec<Pos2> = pins.iter().map(|pin| pin.point).collect();
        let mut spots: Vec<PinCluster> = cluster(&points, side)
            .into_iter()
            .map(|members| {
                let sum = members.iter().fold(Vec2::ZERO, |sum, &i| sum + points[i].to_vec2());
                PinCluster { at: (sum / members.len() as f32).to_pos2(), threads: members.iter().map(|&i| pins[i].id).collect() }
            })
            .collect();
        spots.sort_by(|a, b| a.at.y.total_cmp(&b.at.y).then(a.at.x.total_cmp(&b.at.x)));
        let (mut opened, mut closed, mut moved) = (None, None, None);
        let shown = Stack::new().label(label).show(ui, |ui, _| {
            for spot in &spots {
                let one = match spot.threads.as_slice() {
                    [thread] => pins.iter().find(|pin| pin.id == *thread),
                    _ => None,
                };
                if let Some(pin) = one {
                    let words_of = pin.text.split_whitespace().take(PIN_NAME_WORDS).collect::<Vec<_>>().join(" ");
                    let name = if words_of.is_empty() {
                        words.pin.fill(&[("author", pin.author)])
                    } else {
                        words.pin_text.fill(&[("author", pin.author), ("words", &words_of)])
                    };
                    let face = Face::Pin {
                        author: pin.author.to_owned(),
                        color: pin.color,
                        unread: pin.unread,
                        resolved: pin.resolved,
                        open: open == Some(pin.id),
                        mine: pin.author == me,
                    };
                    let rect = Rect::from_min_max(Pos2::new(spot.at.x, spot.at.y - side), Pos2::new(spot.at.x + side, spot.at.y));
                    let bubble = PinBubble::new(face, name).id_salt(("pin", pin.id.0)).show_at(ui, rect);
                    if bubble.activated {
                        if open == Some(pin.id) {
                            closed = Some(pin.id);
                        } else {
                            opened = Some(pin.id);
                        }
                    }
                    if let Some(to) = bubble.dropped {
                        moved = Some(PinMove { thread: pin.id, to });
                    }
                } else {
                    let name = words.cluster.fill(&[("n", &spot.threads.len().to_string())]);
                    let face = Face::Cluster { count: spot.threads.len() };
                    let width = PinBubble::new(face.clone(), String::new()).measure_in(ui, Proposal::Ideal).x;
                    let rect = Rect::from_min_max(Pos2::new(spot.at.x, spot.at.y - side), Pos2::new(spot.at.x + width, spot.at.y));
                    let first = spot.threads.first().map_or(0, |thread| thread.0);
                    let bubble = PinBubble::new(face, name.clone()).id_salt(("cluster", first)).show_at(ui, rect);
                    let mut chosen = None;
                    Menu::new(name).opened_by(&bubble.response, Trigger::Click).show(ui, |ui, _| {
                        for thread in spot.threads.iter().filter_map(|thread| pins.iter().find(|pin| pin.id == *thread)) {
                            if MenuItem::new(words.pin.fill(&[("author", thread.author)])).show(ui).ran {
                                chosen = Some(thread.id);
                            }
                        }
                    });
                    if chosen.is_some() {
                        opened = chosen;
                    }
                }
            }
        });
        CommentPinsOutput { response: shown.output, opened, closed, moved }
    }
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

/// The inline editor of the popover: the message and its draft.
#[derive(Clone, Default)]
struct Editing(Option<(usize, String)>);

/// The thread popover next to a pin: messages with avatar, author, age and text (@mentions in the accent),
/// reactions under each (click to toggle yours), your messages' Edit and Delete, the thread's Resolve /
/// Reopen button and a "..." menu (Mark as unread, Copy link and, on your own thread, Delete thread), and the
/// reply field at the bottom with @mention completion. Escape closes the mention list first, then an inline
/// edit, then the popover. Focus goes to the reply field when it opens.
#[must_use = "show the popover with `show`"]
#[derive(Pattern)]
pub struct ThreadPopover<'a> {
    thread: &'a CommentThread,
    opener: &'a Response,
    anchor: Option<Rect>,
    me: &'a str,
    people: &'a [String],
    draft: &'a mut String,
    now: i64,
}

impl<'a> ThreadPopover<'a> {
    /// A popover for `thread` hanging from `opener` (the pin's response). `me` is the viewer's name and
    /// `people` the names offered for @mentions; `draft` holds the reply text.
    pub fn new(thread: &'a CommentThread, opener: &'a Response, me: &'a str, people: &'a [String], draft: &'a mut String) -> Self {
        ThreadPopover { thread, opener, anchor: None, me, people, draft, now: 0 }
    }

    /// Hangs the popover from `rect` (in the opener's layer space) instead of from the opener's rect: for a
    /// popover opened from the Comments panel.
    #[must_use]
    pub fn anchor(mut self, rect: Rect) -> Self {
        self.anchor = Some(rect);
        self
    }

    /// The current time in Unix seconds, which the ages count from.
    #[must_use]
    pub fn now(mut self, unix_seconds: i64) -> Self {
        self.now = unix_seconds;
        self
    }
}

/// What the thread popover did this pass.
#[must_use]
#[non_exhaustive]
pub struct ThreadPopoverOutput {
    /// This pass's edits, in order.
    pub edits: Vec<CommentEdit>,
    /// It closed this pass (Escape, a click outside).
    pub closed: bool,
}

/// The words of `text` as runs: a mention in the accent, the rest plain.
fn mention_runs(text: &str, names: &[&str]) -> Vec<(String, bool)> {
    let mut runs = Vec::new();
    let mut at = 0;
    for range in mention_ranges(text, names) {
        if let Some(plain) = text.get(at..range.start) {
            runs.push((plain.to_owned(), false));
        }
        runs.push((text.get(range.clone()).unwrap_or_default().to_owned(), true));
        at = range.end;
    }
    runs.push((text.get(at..).unwrap_or_default().to_owned(), false));
    runs
}

/// The runs of a message split at its line breaks, one list of runs per line; an empty line keeps one blank run so it
/// takes a line's height.
fn message_lines(runs: Vec<(String, bool)>) -> Vec<Vec<(String, bool)>> {
    let mut lines = vec![Vec::new()];
    for (run, mention) in runs {
        for (index, part) in run.split('\n').enumerate() {
            if index > 0 {
                lines.push(Vec::new());
            }
            if !part.is_empty() {
                lines.last_mut().into_iter().for_each(|line| line.push((part.to_owned(), mention)));
            }
        }
    }
    for line in &mut lines {
        if line.is_empty() {
            line.push((" ".to_owned(), false));
        }
    }
    lines
}

impl ThreadPopover<'_> {
    /// Shows the popover and reports the edits made.
    pub fn show(self, ui: &mut Ui) -> ThreadPopoverOutput {
        let ThreadPopover { thread, opener, anchor, me, people, draft, now } = self;
        let env = Env::of(ui);
        let words = env.words::<ThreadWords>();
        let view = env.view();
        let name = thread.messages.first().map_or_else(String::new, |first| words.pin.fill(&[("author", &first.author)]));
        let mut names: Vec<&str> = people.iter().map(String::as_str).collect();
        names.push(me);
        let mut edits = Vec::new();
        let mut open = true;
        let mut escaped = false;
        let mut close = false;
        let mut popover = Popover::new(name, Bind::bound(&mut open)).opened_by(opener).initial_focus(Initial::Default);
        if let Some(rect) = anchor {
            popover = popover.overlay(|overlay| overlay.opener_anchor(Anchor::Rect(rect)));
        }
        let shown = popover.show(ui, |ui, cx| {
            let Editing(mut editing) = cx.keep::<Editing>().clone();
            Stack::new().show(ui, |ui, _| {
                Row::new().show(ui, |ui, _| {
                    let resolved = thread.resolved;
                    let label = if resolved { &words.reopen } else { &words.resolve };
                    let toggle = ToggleButton::new(label.clone(), Bind::given(resolved)).show(ui);
                    if let Some(&now_resolved) = toggle.value() {
                        edits.push(CommentEdit::Resolve { thread: thread.id, resolved: now_resolved });
                    }
                    let more = IconButton::new(IconSpec::svg(IconId::new("more"))).label(words.more.clone()).show(ui);
                    let mut ran = None;
                    Menu::new(words.more.clone()).opened_by(&more, Trigger::Click).show(ui, |ui, _| {
                        if MenuItem::new(words.mark_unread.as_str().to_owned()).icon(IconSpec::svg(IconId::new("eye_off"))).show(ui).ran {
                            ran = Some(CommentEdit::MarkUnread { thread: thread.id });
                        }
                        if MenuItem::new(words.copy_link.as_str().to_owned()).icon(IconSpec::svg(IconId::new("link"))).show(ui).ran {
                            ran = Some(CommentEdit::CopyLink { thread: thread.id });
                        }
                        if thread.messages.first().is_some_and(|first| first.author == me)
                            && MenuItem::new(words.delete_thread.as_str().to_owned()).icon(IconSpec::svg(IconId::new("trash"))).show(ui).ran
                        {
                            ran = Some(CommentEdit::Delete { thread: thread.id, index: 0 });
                        }
                    });
                    edits.extend(ran);
                    close |= clicked(&IconButton::new(IconSpec::svg(IconId::new("close"))).label(words.close.clone()).show(ui));
                });
                for (index, message) in thread.messages.iter().enumerate() {
                    Row::new().show(ui, |ui, _| {
                        let _ = Avatar::new(message.author.clone()).show(ui);
                        Stack::new().show(ui, |ui, _| {
                            Row::new().show(ui, |ui, _| {
                                Text::new(message.author.clone()).role(TypeRole::BodyStrong).show(ui);
                                let age = age_text(&view, message.modified.unwrap_or(message.created), now);
                                Text::new(age).role(TypeRole::Caption).shade(Shade::Muted).show(ui);
                                if message.author == me {
                                    let delete = IconButton::new(IconSpec::svg(IconId::new("trash"))).label(words.delete.clone()).show(ui);
                                    if clicked(&delete) {
                                        edits.push(CommentEdit::Delete { thread: thread.id, index });
                                    }
                                    let edit = IconButton::new(IconSpec::svg(IconId::new("pencil"))).label(words.edit.clone()).show(ui);
                                    if clicked(&edit) {
                                        editing = Some((index, message.text.clone()));
                                    }
                                }
                                let react = IconButton::new(IconSpec::svg(IconId::new("plus"))).label(words.react.clone()).show(ui);
                                Menu::new(words.react.clone()).opened_by(&react, Trigger::Click).show(ui, |ui, _| {
                                    for emoji in REACTIONS {
                                        let mine = message.reactions.iter().any(|r| r.emoji == emoji && r.people.iter().any(|p| p == me));
                                        if MenuItem::new(emoji.to_owned()).check(mine).show(ui).ran {
                                            edits.push(CommentEdit::React { thread: thread.id, index, emoji: emoji.to_owned() });
                                        }
                                    }
                                });
                            });
                            match editing.as_mut().filter(|(editing, _)| *editing == index) {
                                Some((_, text)) => {
                                    let field = TextField::new(words.edit.clone(), Bind::given(text.clone())).lines(COMPOSER_ROWS).show(ui);
                                    if let Some(value) = field.value() {
                                        text.clone_from(value);
                                    }
                                    let escaped = field.escaped;
                                    let trimmed = text.trim().to_owned();
                                    let post = Button::new(words.post.clone()).enabled(!trimmed.is_empty()).show(ui);
                                    let post_clicked = clicked(&post);
                                    if post_clicked && !trimmed.is_empty() {
                                        edits.push(CommentEdit::Edit { thread: thread.id, index, text: trimmed });
                                    }
                                    if post_clicked || escaped {
                                        editing = None;
                                    }
                                }
                                None => {
                                    Stack::new().show(ui, |ui, _| {
                                        for line in message_lines(mention_runs(&message.text, &names)) {
                                            Row::new().wrap(true).show(ui, |ui, _| {
                                                for (run, mention) in &line {
                                                    for word in run.split_inclusive(' ') {
                                                        let text = Text::new(word.to_owned());
                                                        if *mention {
                                                            text.role(TypeRole::BodyStrong).tone(Tone::Accent).show(ui)
                                                        } else {
                                                            text.show(ui)
                                                        };
                                                    }
                                                }
                                            });
                                        }
                                    });
                                }
                            }
                            Row::new().wrap(true).show(ui, |ui, _| {
                                for reaction in &message.reactions {
                                    let on = reaction.people.iter().any(|p| p == me);
                                    let chip = ToggleButton::new(format!("{} {}", reaction.emoji, reaction.people.len()), Bind::given(on));
                                    if chip.tooltip(reaction.people.join(", ")).show(ui).value().is_some() {
                                        edits.push(CommentEdit::React { thread: thread.id, index, emoji: reaction.emoji.clone() });
                                    }
                                }
                            });
                        });
                    });
                }
                let field = TextField::new(words.reply.clone(), Bind::bound(&mut *draft))
                    .lines(COMPOSER_ROWS)
                    .placeholder(words.reply.clone())
                    .complete(MentionCompleter { people: people.to_vec(), me: me.to_owned() })
                    .submits()
                    .default_stop()
                    .show(ui);
                escaped |= field.escaped;
                let text = draft.trim().to_owned();
                let post = Button::new(words.post.clone()).enabled(!text.is_empty()).show(ui);
                if (clicked(&post) || field.submitted) && !text.is_empty() {
                    edits.push(CommentEdit::Reply { thread: thread.id, text });
                    draft.clear();
                }
            });
            *cx.keep::<Editing>() = Editing(editing);
        });
        ThreadPopoverOutput { edits, closed: !open || escaped || close || shown.output.closed.is_some() }
    }
}

/// The floating composer card for a new comment at a point: a multi-line field with the focus, Post and
/// Cancel (Escape).
#[must_use = "show the composer with `show`"]
#[derive(Pattern)]
pub struct CommentComposer<'a> {
    anchor: Option<Pos2>,
    draft: &'a mut String,
}

impl<'a> CommentComposer<'a> {
    /// A composer at `anchor` (screen points; `None` hides it) that edits `draft`.
    pub fn new(anchor: Option<Pos2>, draft: &'a mut String) -> Self {
        CommentComposer { anchor, draft }
    }
}

/// What the composer did this pass.
#[must_use]
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct CommentComposerOutput {
    /// The text on Post.
    pub posted: Option<String>,
    /// It closed without posting (Cancel, Escape, a click outside).
    pub cancelled: bool,
}

impl CommentComposer<'_> {
    /// Shows the composer and reports whether the user posted or closed it.
    pub fn show(self, ui: &mut Ui) -> CommentComposerOutput {
        let CommentComposer { anchor, draft } = self;
        let mut output = CommentComposerOutput::default();
        let Some(anchor) = anchor else { return output };
        let words = Env::of(ui).words::<ThreadWords>();
        let mut open = true;
        let mut posted = None;
        let mut cancel = false;
        let shown =
            Popover::new(words.add_comment.clone(), Bind::bound(&mut open)).at_point(anchor).initial_focus(Initial::Default).show(ui, |ui, _| {
                Stack::new().show(ui, |ui, _| {
                    let field = TextField::new(words.add_comment.clone(), Bind::bound(&mut *draft))
                        .lines(COMPOSER_ROWS)
                        .placeholder(words.add_comment.clone())
                        .submits()
                        .default_stop()
                        .show(ui);
                    cancel |= field.escaped;
                    let text = draft.trim().to_owned();
                    if field.submitted && !text.is_empty() {
                        posted = Some(text.clone());
                    }
                    Row::new().show(ui, |ui, _| {
                        let post = Button::new(words.post.clone()).enabled(!text.is_empty()).show(ui);
                        if clicked(&post) && !text.is_empty() {
                            posted = Some(text.clone());
                        }
                        let close = Button::new(words.cancel.clone()).show(ui);
                        cancel |= clicked(&close);
                    });
                });
            });
        let closed = !open || shown.output.closed.is_some();
        if posted.is_some() || cancel || closed {
            draft.clear();
        }
        output.cancelled = posted.is_none() && (cancel || closed);
        output.posted = posted;
        output
    }
}
