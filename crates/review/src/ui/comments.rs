//! The Comments panel: a header with the count, search, sort and filter menus, the add box and the cards.
//!
//! ```ignore
//! let out = CommentsPanel::new("Comments", &threads, &mut state).now(unix_seconds).show(ui);
//! for edit in out.edits {
//!     match edit {
//!         CommentEdit::Add { text } => doc.add_note(text),
//!         CommentEdit::Select(thread) => doc.select(thread),
//!         _ => {}
//!     }
//! }
//! ```

#[cfg(feature = "screen-testing")]
mod stories;

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::Arc;

use craft_behavior::{DisplayOrder, IconId, ItemSource, Metrics, Proposal, Selection, SpanLayout};
use craft_i18n::{DateStyle, DateTime, TimeZone};
use craft_ui::buttons::{Button, IconButton};
use craft_ui::contract::overlay::Anchor;
use craft_ui::contract::{
    ActivateOn, Bind, BindMut, Collection, CollectionEvent, CollectionProps, CollectionResponse, Common, Component, ContainerProps, Cx, ItemOutput,
    MeasureCx, NoValue, Outcome, PaintUi, Pattern, Props as _, Role, SelectMode, Show as _, ShowCollection as _, ShowContainer as _, Sizing, Trigger,
    WordSetCx as _, clicked,
};
use craft_ui::display::{Divider, Shade, Text};
use craft_ui::layout_components::{Row, Stack};
use craft_ui::menu::{Menu, MenuItem};
use craft_ui::notices::{Notice, NoticeLayout};
use craft_ui::primitives::{HAlign, IconSpec, PaintShape, TextRun, Wrap};
use craft_ui::runtime::env::{AgeStyle, Env, EnvView, Label};
use craft_ui::style::{ComponentStyle, DataColor, Ink, StyleColor, StyleCx, TypeRole};
use craft_ui::text::TextField;
use egui::{CornerRadius, Pos2, Rect, Response, Ui, Vec2, vec2};

use super::annotate::{AnnotateWords, PALETTE};
use super::avatar::paint_face;
use crate::annotate::AnnotationKind;
use crate::comments::{CommentFilter, CommentSort, CommentThread, ReviewStatus, ThreadId, matches, sort_threads};

/// Ages under this many seconds (one week) read relative; older ones read as a date.
pub(crate) const AGE_SHOWN_AS_DATE: u64 = 7 * 24 * 60 * 60;

/// The rows of the add box and of a reply box.
const REPLY_LINES: usize = 2;

/// The rows of an inline editor.
const EDIT_LINES: usize = 3;

craft_ui::words! {
    /// The panel's words, English by default.
    #[non_exhaustive]
    pub struct CommentsWords {
        title: template "comments.title" = "Comments {n}";
        replies: count "comments.replies" = "{n} reply" | "{n} replies";
        search: "comments.search" = "Search comments";
        sort_by: template "comments.sortBy" = "Sort by: {sort}";
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
        page: template "comments.page" = "Page {label}";
        card_name: template "comments.cardName" = "{author}: {text}";
        plain_kind: "comments.plainKind" = "Comment";
        no_color: "comments.noColor" = "No color";
        no_match: "comments.noMatch" = "No comments match.";
        filter: "comments.filter" = "Filter comments";
        filter_type: "comments.filterType" = "Type";
        filter_author: "comments.filterAuthor" = "Author";
        filter_status: "comments.filterStatus" = "Status";
        filter_color: "comments.filterColor" = "Color";
        filter_check: "comments.filterCheck" = "Checkmark";
        checked: "comments.checked" = "Checked";
        unchecked: "comments.unchecked" = "Unchecked";
        show_all: "comments.showAll" = "Show all";
        pages_menu: "comments.pagesMenu" = "Pages";
        collapse_all: "comments.collapseAll" = "Collapse all pages";
        expand_all: "comments.expandAll" = "Expand all pages";
        status_none: "comments.statusNone" = "None";
        accepted: "comments.accepted" = "Accepted";
        rejected: "comments.rejected" = "Rejected";
        cancelled: "comments.cancelled" = "Cancelled";
        completed: "comments.completed" = "Completed";
        empty: "comments.empty" = "No comments yet.";
    }
}

/// Where a card's menu hangs: the thread, the message and the rect of the button that asked.
#[derive(Clone, Debug, PartialEq)]
struct OpenMenu {
    thread: ThreadId,
    index: usize,
    anchor: Rect,
}

/// What the panel keeps between passes; the app keeps one per document window.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct CommentsPanelState {
    /// The search box text; threads match through [`matches`](fn@matches).
    pub search: String,
    /// The active sort.
    pub sort: CommentSort,
    /// What the filter menu hides.
    pub filter: CommentFilter,
    /// The pages whose group is folded under the Page sort.
    pub collapsed_pages: Vec<usize>,
    /// The selected thread, if any.
    pub selected: Option<ThreadId>,
    /// The message being edited inline: (thread, message index, text).
    pub editing: Option<(ThreadId, usize, String)>,
    /// The selected card's reply draft.
    pub reply: String,
    /// The "Add a comment" box's draft.
    pub new_comment: String,
    menu: Option<OpenMenu>,
}

impl CommentsPanelState {
    /// Selects `thread` as a click on its card does, and unfolds its page group: for a comment
    /// the app selected elsewhere (on the page, or just added), whose card must be drawn.
    pub fn reveal(&mut self, thread: &CommentThread) {
        select_thread(self, thread.id);
        if let Some(page) = thread.anchor.page() {
            self.collapsed_pages.retain(|&folded| folded != page);
        }
    }
}

/// What the panel asks of the app.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum CommentEdit {
    /// "Add a comment": a sticky note on the current page.
    #[non_exhaustive]
    Add {
        /// The comment text.
        text: String,
    },
    /// Reply to a thread.
    #[non_exhaustive]
    Reply {
        #[allow(missing_docs)]
        thread: ThreadId,
        /// The reply text.
        text: String,
    },
    /// Message `index` (0: the comment) edited.
    #[non_exhaustive]
    Edit {
        #[allow(missing_docs)]
        thread: ThreadId,
        /// Message index; 0 is the comment.
        index: usize,
        /// The new text.
        text: String,
    },
    /// Message `index` deleted; 0 deletes the thread.
    #[non_exhaustive]
    Delete {
        #[allow(missing_docs)]
        thread: ThreadId,
        /// Message index; 0 deletes the whole thread.
        index: usize,
    },
    /// Set a thread's review status.
    #[non_exhaustive]
    SetStatus {
        #[allow(missing_docs)]
        thread: ThreadId,
        #[allow(missing_docs)]
        status: ReviewStatus,
    },
    /// Set or clear a thread's checkmark.
    #[non_exhaustive]
    Mark {
        #[allow(missing_docs)]
        thread: ThreadId,
        #[allow(missing_docs)]
        marked: bool,
    },
    /// A card was selected: the app selects the annotation on the page and scrolls to it.
    Select(ThreadId),
    /// Copy text of message `index`; the app puts it on the clipboard.
    #[non_exhaustive]
    CopyText {
        #[allow(missing_docs)]
        thread: ThreadId,
        /// Message index.
        index: usize,
    },
    /// The viewer's reaction `emoji` on message `index` toggled (the thread popover).
    #[non_exhaustive]
    React {
        #[allow(missing_docs)]
        thread: ThreadId,
        /// Message index.
        index: usize,
        /// The reaction emoji.
        emoji: String,
    },
    /// Figma's resolve (`true`) or reopen (`false`).
    #[non_exhaustive]
    Resolve {
        #[allow(missing_docs)]
        thread: ThreadId,
        #[allow(missing_docs)]
        resolved: bool,
    },
    /// The app puts a link to the thread on the clipboard.
    #[non_exhaustive]
    CopyLink {
        #[allow(missing_docs)]
        thread: ThreadId,
    },
    /// The viewer's read state of the thread cleared.
    #[non_exhaustive]
    MarkUnread {
        #[allow(missing_docs)]
        thread: ThreadId,
    },
}

/// "5 min ago" under a week, then the date.
pub(crate) fn age_text(env: &EnvView<'_>, then: i64, now: i64) -> String {
    let age = now.saturating_sub(then).max(0).unsigned_abs();
    if age < AGE_SHOWN_AS_DATE {
        env.relative_age(age, AgeStyle::Short)
    } else {
        env.dates().format(DateTime::from_unix(then, TimeZone::Local), DateStyle::DATE)
    }
}

/// Acrobat's right-hand comment list.
///
/// - Header "Comments N" with "N replies" under it, search, the Sort by menu, the funnel menu
///   ([`CommentFilter`]: Type, Author, Status, Color, Checkmark rows that hide what they name) and,
///   under the Page sort, a menu with Collapse all pages and Expand all pages. A search or filter
///   that leaves nothing shows "No comments match."
/// - "Add a comment" box: a multi-line field and Post.
/// - Cards in one collection, grouped under headings by [`sort_threads`]; under the Page sort a
///   heading names the page, shows the count of the page's comments before filtering and folds its
///   cards. A card shows the kind's icon, author, age (relative under a week, then the date), the
///   text, a status icon, the checkmark, a lock icon and the replies, each with its own "..." menu.
/// - The selected card has the accent bar, the reply box and "..." (Edit, Set status, Delete).
///   Locked threads and `read_only` hide the editing items.
/// - Keyboard: Up and Down move between cards (selecting them), Enter selects, Escape in a control
///   of a card returns to the card.
///
/// ```ignore
/// let out = CommentsPanel::new("Comments", &threads, &mut state).now(now).show(ui);
/// ```
#[must_use = "show the panel with `show`"]
#[derive(Pattern)]
pub struct CommentsPanel<'a> {
    label: Label,
    threads: &'a [CommentThread],
    state: &'a mut CommentsPanelState,
    read_only: bool,
    now: i64,
    page_label: Option<Box<dyn Fn(usize) -> String + 'a>>,
}

impl<'a> CommentsPanel<'a> {
    /// The panel named `label` over `threads`. The edits carry only the text; the app signs new
    /// comments with its own identity.
    pub fn new(label: impl Into<Label>, threads: &'a [CommentThread], state: &'a mut CommentsPanelState) -> Self {
        CommentsPanel { label: label.into(), threads, state, read_only: false, now: 0, page_label: None }
    }

    /// No posting or editing (a document that forbids comments).
    #[must_use]
    pub fn read_only(mut self, on: bool) -> Self {
        self.read_only = on;
        self
    }

    /// The current time in Unix seconds, which the ages count from.
    #[must_use]
    pub fn now(mut self, unix_seconds: i64) -> Self {
        self.now = unix_seconds;
        self
    }

    /// The text of a page heading's `{label}`, from the page stored in the anchor (page labels
    /// such as "iv" or "A-3"); that page itself by default.
    #[must_use]
    pub fn page_label(mut self, label: impl Fn(usize) -> String + 'a) -> Self {
        self.page_label = Some(Box::new(label));
        self
    }
}

/// What the Comments panel did this pass.
#[must_use]
#[non_exhaustive]
pub struct CommentsPanelOutput {
    /// The panel's response.
    pub response: Response,
    /// This pass's edits, in order.
    pub edits: Vec<CommentEdit>,
}

impl Outcome for CommentsPanelOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// What a row of the filter menu does.
#[derive(Clone)]
enum FilterRow {
    Type(Option<AnnotationKind>),
    Author(String),
    Status(ReviewStatus),
    Color(Option<[u8; 3]>),
    Check(bool),
    ShowAll,
}

/// What a row of a card's menu does.
#[derive(Clone, Copy)]
enum CardMenu {
    Edit,
    Status(ReviewStatus),
    Copy,
    Delete,
}

/// Adds `value` to `list`, or takes it out when it is there.
fn toggle<T: PartialEq>(list: &mut Vec<T>, value: T) {
    match list.iter().position(|item| *item == value) {
        Some(at) => {
            list.remove(at);
        }
        None => list.push(value),
    }
}

/// Makes `thread` the selected one; false when it already was.
fn select_thread(state: &mut CommentsPanelState, thread: ThreadId) -> bool {
    let changed = state.selected != Some(thread);
    if changed {
        state.selected = Some(thread);
        state.reply.clear();
        state.editing = None;
    }
    changed
}

fn status_label(words: &CommentsWords, status: ReviewStatus) -> &Label {
    match status {
        ReviewStatus::None => &words.status_none,
        ReviewStatus::Accepted => &words.accepted,
        ReviewStatus::Rejected => &words.rejected,
        ReviewStatus::Cancelled => &words.cancelled,
        ReviewStatus::Completed => &words.completed,
    }
}

/// A label colour's name when it is one of the toolbar's palette colours, else its hex.
fn color_name(kinds: &AnnotateWords, color: [u8; 3]) -> String {
    match PALETTE.iter().position(|swatch| *swatch == color) {
        Some(at) => kinds.palette_color(at).to_owned(),
        None => format!("#{:02X}{:02X}{:02X}", color[0], color[1], color[2]),
    }
}

/// The reader's name of a thread's kind.
fn kind_name(words: &CommentsWords, kinds: &AnnotateWords, kind: Option<AnnotationKind>) -> String {
    kind.map_or_else(|| words.plain_kind.as_str().to_owned(), |kind| kinds.kind(kind).to_owned())
}

/// The key of a row of the card collection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum CardKey {
    /// The heading at this position among the headings.
    Group(usize),
    /// The card of this thread.
    Thread(ThreadId),
}

/// One row of the card collection's source.
enum RowEntry<'a> {
    Group { title: String, count: String, page: Option<usize>, open: bool },
    Card { thread: &'a CommentThread, depth: usize },
}

/// The shown threads, under their headings, as the collection's source.
struct Rows<'a> {
    entries: Vec<(CardKey, RowEntry<'a>)>,
}

impl DisplayOrder<CardKey> for Rows<'_> {
    fn len(&self) -> usize {
        self.entries.len()
    }

    fn key(&self, at: usize) -> CardKey {
        self.entries[at].0
    }

    fn index_of(&self, key: &CardKey) -> Option<usize> {
        self.entries.iter().position(|(held, _)| held == key)
    }

    fn depth(&self, at: usize) -> usize {
        match &self.entries[at].1 {
            RowEntry::Group { .. } => 0,
            RowEntry::Card { depth, .. } => *depth,
        }
    }

    fn has_children(&self, at: usize) -> bool {
        matches!(self.entries[at].1, RowEntry::Group { .. })
    }

    fn open(&self, at: usize) -> bool {
        matches!(self.entries[at].1, RowEntry::Group { open: true, .. })
    }

    fn text(&self, at: usize) -> Option<Cow<'_, str>> {
        match &self.entries[at].1 {
            RowEntry::Group { title, .. } => Some(Cow::Borrowed(title.as_str())),
            RowEntry::Card { thread, .. } => thread.messages.first().map(|message| Cow::Borrowed(message.text.as_str())),
        }
    }
}

impl ItemSource<CardKey> for Rows<'_> {}

/// What a card reports; the panel applies it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum CardEvent {
    /// A "..." button was pressed: the menu of message `index` hangs from `anchor`.
    Menu { thread: ThreadId, index: usize, anchor: Rect },
    /// The checkmark button was pressed.
    Mark { thread: ThreadId, marked: bool },
    /// The inline editor's text changed.
    EditText(String),
    /// The inline editor's Post.
    EditPost,
    /// The inline editor was cancelled.
    EditCancel,
    /// The reply box's text changed.
    ReplyText(String),
    /// The reply box's Post.
    ReplyPost,
}

/// What a card row returns.
#[non_exhaustive]
pub(crate) struct RowOut {
    response: Response,
    events: Vec<CardEvent>,
}

impl Outcome for RowOut {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

impl ItemOutput<Vec<CardEvent>> for RowOut {
    fn into_event(self) -> Option<Vec<CardEvent>> {
        (!self.events.is_empty()).then_some(self.events)
    }
}

/// The card collection: a tree whose headings fold their cards.
#[derive(craft_ui::contract::Component)]
#[component(collection)]
pub(crate) struct CommentList<'a> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    items: CollectionProps,
    selection: Bind<'a, Selection<CardKey>>,
    grouped: bool,
}

impl<'a> CommentList<'a> {
    /// The list named `label` over the caller's `selection`; `grouped` lists the cards under headings.
    pub(crate) fn new(label: impl Into<Label>, selection: Bind<'a, Selection<CardKey>>, grouped: bool) -> Self {
        CommentList {
            common: Common::new(),
            props: ContainerProps::new().sizing(Sizing::Fill),
            items: CollectionProps::new().select(SelectMode::Single).activate(ActivateOn::Click),
            selection,
            grouped,
        }
        .label(label)
    }
}

/// The style of the list: no look of its own.
#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub(crate) struct CommentListStyle {
    #[style(metrics)]
    metrics: Metrics,
}

impl Component for CommentList<'_> {
    const ROLE: Role = Role::TREE;
    type Style = CommentListStyle;
    type Output = CollectionResponse<CardKey, Vec<CardEvent>>;

    fn style(_: &StyleCx<'_>, _: CommentListStyleKey) -> CommentListStyle {
        CommentListStyle::default()
    }

    fn finish(self, mut cx: Cx<'_, Self>) -> Self::Output {
        cx.collection_response()
    }
}

impl Collection for CommentList<'_> {
    type Key = CardKey;
    type ItemLayout = SpanLayout;
    type Selection = Selection<CardKey>;
    type Item<'r> = CommentRow<'r>;
    type ItemEvent = Vec<CardEvent>;

    fn items(&self) -> &CollectionProps {
        &self.items
    }

    fn selection(&mut self) -> Option<BindMut<'_, Selection<CardKey>>> {
        Some(self.selection.reborrow())
    }

    fn item_layout(&self, source: &dyn ItemSource<CardKey>, width: f32, cx: &StyleCx<'_>) -> SpanLayout {
        let layout = SpanLayout::sized(source, width, cx.control().height);
        if self.grouped { layout.tree() } else { layout }
    }
}

/// The style of a card or heading row.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub(crate) struct CommentRowStyle {
    #[style(metrics)]
    metrics: Metrics,
    normal: StyleColor,
    muted: StyleColor,
    good: StyleColor,
    bad: StyleColor,
    faint: StyleColor,
    highlight: StyleColor,
    bar: StyleColor,
    avatar: StyleColor,
    avatar_ink: StyleColor,
    bar_width: f32,
    /// The width of a card when the room names none.
    column: f32,
    height: f32,
    pad: f32,
    gap: f32,
}

/// What a heading shows.
struct Heading {
    title: String,
    count: String,
    open: bool,
}

/// How a text or icon is inked.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tone {
    Normal,
    Muted,
    Good,
    Bad,
    Faint,
}

/// How a text is set.
#[derive(Clone, Copy)]
struct Spec {
    role: TypeRole,
    tone: Tone,
    wrap: Wrap,
    end: bool,
}

impl Spec {
    const fn new(role: TypeRole, tone: Tone, wrap: Wrap) -> Self {
        Spec { role, tone, wrap, end: false }
    }

    const fn end(mut self) -> Self {
        self.end = true;
        self
    }
}

/// A control of a row, which names its child component.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Slot {
    More(usize),
    Mark,
    EditField,
    EditPost,
    EditCancel,
    ReplyField,
    ReplyPost,
}

/// Where `CommentRow::lay` puts the parts of a row: the measuring pass sizes them, the paint pass shows them.
/// Rects are relative to the row's top left, laid out left to right; the sink mirrors them.
trait Sink {
    /// Text set in `at`'s width from `at`'s top left; returns its size.
    fn text(&mut self, at: Rect, spec: Spec, text: &str) -> Vec2;
    /// The width `text` takes on one line.
    fn width(&mut self, spec: Spec, text: &str) -> f32;
    /// The icon `id` in `rect`.
    fn icon(&mut self, rect: Rect, id: &'static str, tone: Tone);
    /// The avatar of `name`.
    fn avatar(&mut self, rect: Rect, name: &str);
    /// A square icon button; it reports its press.
    fn button(&mut self, slot: Slot, rect: Rect, id: &'static str, name: &Label, on: bool);
    /// A multi-line field at `at`'s top left and width; returns its height.
    fn field(&mut self, slot: Slot, at: Rect, label: &Label, text: &str, lines: usize) -> f32;
    /// The size the push button `slot` with `label` measures.
    fn push_size(&mut self, slot: Slot, label: &Label) -> Vec2;
    /// A push button.
    fn push(&mut self, slot: Slot, rect: Rect, label: &Label, enabled: bool);
}

/// A card or a heading of the card collection.
#[derive(craft_ui::contract::Component)]
pub(crate) struct CommentRow<'r> {
    #[common]
    common: Common,
    thread: Option<&'r CommentThread>,
    heading: Option<Heading>,
    selected: bool,
    editable: bool,
    now: i64,
    editing: Option<(usize, &'r str)>,
    reply: &'r str,
    words: Option<Arc<CommentsWords>>,
    /// The heights the editor field and the reply field measured, for the paint of the same pass.
    heights: [f32; 2],
    /// The sizes the push buttons measured, for the paint of the same pass.
    buttons: [Vec2; 3],
    events: Vec<CardEvent>,
}

impl<'r> CommentRow<'r> {
    fn bare(thread: Option<&'r CommentThread>, heading: Option<Heading>, name: String) -> Self {
        CommentRow {
            common: Common::new(),
            thread,
            heading,
            selected: false,
            editable: false,
            now: 0,
            editing: None,
            reply: "",
            words: None,
            heights: [0.0; 2],
            buttons: [Vec2::ZERO; 3],
            events: Vec::new(),
        }
        .label(name)
    }

    /// A heading row.
    pub(crate) fn heading(title: String, count: String, open: bool) -> Self {
        let name = title.clone();
        Self::bare(None, Some(Heading { title, count, open }), name)
    }

    /// The card of `thread`, named `name`.
    pub(crate) fn card(thread: &'r CommentThread, name: String) -> Self {
        Self::bare(Some(thread), None, name)
    }

    /// Whether the card is selected, whether its editing items show and the time its ages count from.
    pub(crate) fn state(mut self, selected: bool, editable: bool, now: i64) -> Self {
        self.selected = selected;
        self.editable = editable;
        self.now = now;
        self
    }

    /// The message being edited and its draft, and the reply draft.
    pub(crate) fn drafts(mut self, editing: Option<(usize, &'r str)>, reply: &'r str) -> Self {
        self.editing = editing;
        self.reply = reply;
        self
    }
}

fn kind_icon(kind: Option<AnnotationKind>) -> &'static str {
    use AnnotationKind as K;
    match kind {
        Some(K::Highlight | K::Underline | K::StrikeOut | K::Squiggly) => "brush",
        Some(K::TextBox | K::Callout) => "text",
        Some(K::Ink) => "pencil",
        Some(K::Attach) => "link",
        _ => "note",
    }
}

fn status_icon(status: ReviewStatus) -> Option<(&'static str, Tone)> {
    match status {
        ReviewStatus::None => None,
        ReviewStatus::Accepted => Some(("thumbs_up", Tone::Good)),
        ReviewStatus::Rejected => Some(("thumbs_down", Tone::Bad)),
        ReviewStatus::Cancelled => Some(("x_circle", Tone::Faint)),
        ReviewStatus::Completed => Some(("check", Tone::Good)),
    }
}

fn square(at: Pos2, side: f32) -> Rect {
    Rect::from_min_size(at, Vec2::splat(side))
}

fn strip(at: Pos2, width: f32) -> Rect {
    Rect::from_min_size(at, vec2(width.max(0.0), 0.0))
}

impl CommentRow<'_> {
    /// Places every part of the row for `width` and returns the row's height. `age` writes an age.
    fn lay(&self, st: &CommentRowStyle, width: f32, age: &dyn Fn(i64) -> String, sink: &mut dyn Sink) -> f32 {
        let words = self.words.clone().unwrap_or_default();
        let (h, gap, pad) = (st.height, st.gap, st.pad);
        let room = (width - 2.0 * pad).max(h);
        if let Some(heading) = &self.heading {
            let chevron = if heading.open { "chevron_down" } else { "chevron_right" };
            sink.icon(square(Pos2::new(pad, pad), h), chevron, Tone::Muted);
            let count_spec = Spec::new(TypeRole::Caption, Tone::Muted, Wrap::Wrap).end();
            let count = sink.width(count_spec, &heading.count);
            let x = pad + h + gap;
            let title = Spec::new(TypeRole::BodyStrong, Tone::Muted, Wrap::Elide);
            sink.text(strip(Pos2::new(x, pad), width - pad - count - gap - x), title, &heading.title);
            sink.text(strip(Pos2::new(width - pad - count, pad), count), count_spec, &heading.count);
            return h + 2.0 * pad;
        }
        let Some(thread) = self.thread else { return h };
        let y = pad;
        let first = thread.messages.first();
        sink.icon(square(Pos2::new(pad, y), h), kind_icon(thread.kind), Tone::Normal);
        let mut end = width - pad;
        if self.selected && self.editable {
            end -= h;
            sink.button(Slot::More(0), square(Pos2::new(end, y), h), "more", &words.more, false);
        }
        if thread.locked {
            end -= h;
            sink.icon(square(Pos2::new(end, y), h), "lock", Tone::Muted);
        }
        if self.selected && self.editable {
            end -= h;
            let name = if thread.marked { &words.unmark } else { &words.mark };
            sink.button(Slot::Mark, square(Pos2::new(end, y), h), "check", name, thread.marked);
        } else if thread.marked {
            end -= h;
            sink.icon(square(Pos2::new(end, y), h), "check", Tone::Muted);
        }
        if let Some((icon, tone)) = status_icon(thread.status) {
            end -= h;
            sink.icon(square(Pos2::new(end, y), h), icon, tone);
        }
        let name_at = pad + h + gap;
        let name_room = end - gap - name_at;
        let author = first.map_or(words.unknown_author.as_str(), |message| message.author.as_str());
        let author_spec = Spec::new(TypeRole::BodyStrong, Tone::Normal, Wrap::Elide);
        let author_size = sink.text(strip(Pos2::new(name_at, y), name_room), author_spec, author);
        let mut head = author_size.y.max(h);
        if let Some(message) = first {
            let when = age(message.modified.unwrap_or(message.created));
            let caption = Spec::new(TypeRole::Caption, Tone::Muted, Wrap::Elide);
            let size = sink.text(strip(Pos2::new(name_at, y + author_size.y), name_room), caption, &when);
            head = head.max(author_size.y + size.y);
        }
        let mut y = y + head + gap;
        y = self.message(0, thread, st, Pos2::new(pad, y), room, &words, sink) + gap;
        for index in 1..thread.messages.len() {
            let Some(message) = thread.messages.get(index) else { break };
            sink.avatar(square(Pos2::new(pad, y), h), &message.author);
            let x = pad + h + gap;
            let name_room = width - pad - h - gap - x;
            let author_size = sink.text(strip(Pos2::new(x, y), name_room), author_spec, &message.author);
            let when = age(message.modified.unwrap_or(message.created));
            let caption = Spec::new(TypeRole::Caption, Tone::Muted, Wrap::Elide);
            let size = sink.text(strip(Pos2::new(x, y + author_size.y), name_room), caption, &when);
            sink.button(Slot::More(index), square(Pos2::new(width - pad - h, y), h), "more", &words.more, false);
            y += h.max(author_size.y + size.y) + gap;
            y = self.message(index, thread, st, Pos2::new(x, y), width - pad - x, &words, sink) + gap;
        }
        if self.selected && self.editable {
            let field = sink.field(Slot::ReplyField, strip(Pos2::new(pad, y), room), &words.reply, self.reply, REPLY_LINES);
            y += field + gap;
            let button = sink.push_size(Slot::ReplyPost, &words.post);
            let post = Rect::from_min_size(Pos2::new(width - pad - button.x, y), button);
            sink.push(Slot::ReplyPost, post, &words.post, !self.reply.trim().is_empty());
            y += button.y + gap;
        }
        y - gap + pad
    }

    /// Message `index`'s text, or its inline editor; returns the bottom.
    fn message(
        &self,
        index: usize,
        thread: &CommentThread,
        st: &CommentRowStyle,
        at: Pos2,
        room: f32,
        words: &CommentsWords,
        sink: &mut dyn Sink,
    ) -> f32 {
        let Some(message) = thread.messages.get(index) else { return at.y };
        match self.editing {
            Some((editing, draft)) if editing == index => {
                let field = sink.field(Slot::EditField, strip(at, room), &words.edit, draft, EDIT_LINES);
                let y = at.y + field + st.gap;
                let (post_size, cancel_size) = (sink.push_size(Slot::EditPost, &words.post), sink.push_size(Slot::EditCancel, &words.cancel));
                let right = at.x + room;
                let cancel = Rect::from_min_size(Pos2::new(right - cancel_size.x, y), cancel_size);
                let post = Rect::from_min_size(Pos2::new(cancel.left() - st.gap - post_size.x, y), post_size);
                sink.push(Slot::EditPost, post, &words.post, !draft.trim().is_empty());
                sink.push(Slot::EditCancel, cancel, &words.cancel, true);
                y + post_size.y.max(cancel_size.y)
            }
            _ => {
                let body = Spec::new(TypeRole::Body, Tone::Normal, Wrap::Wrap);
                at.y + sink.text(strip(at, room), body, &message.text).y
            }
        }
    }
}

/// Sizes the parts of a row.
struct Measure<'a, 'b, 'r> {
    cx: &'a MeasureCx<'b, CommentRow<'r>>,
    fields: [f32; 2],
    buttons: [Vec2; 3],
}

impl Sink for Measure<'_, '_, '_> {
    fn text(&mut self, at: Rect, spec: Spec, text: &str) -> Vec2 {
        self.cx.layout_at(&TextRun::new(text).role(spec.role).wrap(spec.wrap), at.width()).size()
    }

    fn width(&mut self, spec: Spec, text: &str) -> f32 {
        self.cx.layout_at(&TextRun::new(text).role(spec.role).wrap(Wrap::Line), f32::INFINITY).size().x
    }

    fn icon(&mut self, _: Rect, _: &'static str, _: Tone) {}

    fn avatar(&mut self, _: Rect, _: &str) {}

    fn button(&mut self, _: Slot, _: Rect, _: &'static str, _: &Label, _: bool) {}

    fn field(&mut self, slot: Slot, at: Rect, label: &Label, text: &str, lines: usize) -> f32 {
        let mut field = TextField::new(label.clone(), Bind::given(text.to_owned())).lines(lines);
        let height = self.cx.child(slot, &mut field, Proposal::Width(at.width())).y;
        self.fields[usize::from(slot == Slot::ReplyField)] = height;
        height
    }

    fn push_size(&mut self, slot: Slot, label: &Label) -> Vec2 {
        let size = self.cx.child(slot, &mut Button::new(label.clone()), Proposal::Ideal);
        self.buttons[push_index(slot)] = size;
        size
    }

    fn push(&mut self, _: Slot, _: Rect, _: &Label, _: bool) {}
}

/// Where the push button `slot`'s measured size is kept.
fn push_index(slot: Slot) -> usize {
    match slot {
        Slot::EditPost => 0,
        Slot::EditCancel => 1,
        _ => 2,
    }
}

/// Shows the parts of a row.
struct Paint<'a, 'b> {
    ui: &'a mut PaintUi<'b>,
    st: &'a CommentRowStyle,
    origin: Pos2,
    width: f32,
    rtl: bool,
    thread: Option<ThreadId>,
    heights: [f32; 2],
    buttons: [Vec2; 3],
    events: &'a mut Vec<CardEvent>,
}

impl Paint<'_, '_> {
    /// `rect` from the row's left-to-right layout, on the screen.
    fn place(&self, rect: Rect) -> Rect {
        let moved = rect.translate(self.origin.to_vec2());
        if self.rtl {
            let right = self.origin.x + self.width;
            let left = self.origin.x;
            let (min_x, max_x) = (left + right - moved.max.x, left + right - moved.min.x);
            Rect::from_min_max(Pos2::new(min_x, moved.min.y), Pos2::new(max_x, moved.max.y))
        } else {
            moved
        }
    }

    fn color(&self, tone: Tone) -> StyleColor {
        match tone {
            Tone::Normal => self.st.normal,
            Tone::Muted => self.st.muted,
            Tone::Good => self.st.good,
            Tone::Bad => self.st.bad,
            Tone::Faint => self.st.faint,
        }
    }
}

impl Sink for Paint<'_, '_> {
    fn width(&mut self, spec: Spec, text: &str) -> f32 {
        self.ui.draw().layout(&TextRun::new(text).role(spec.role).wrap(Wrap::Line), f32::INFINITY).size().x
    }

    fn text(&mut self, at: Rect, spec: Spec, text: &str) -> Vec2 {
        let mut run = TextRun::new(text).role(spec.role).wrap(spec.wrap).color(self.color(spec.tone));
        if spec.end {
            run = run.align(HAlign::End);
        }
        self.ui.draw().text(self.place(at), &run).size()
    }

    fn icon(&mut self, rect: Rect, id: &'static str, tone: Tone) {
        let spec = IconSpec::svg(IconId::new(id)).color(self.color(tone));
        self.ui.draw().icon(self.place(rect), &spec);
    }

    fn avatar(&mut self, rect: Rect, name: &str) {
        let rect = self.place(rect);
        paint_face(&self.ui.draw(), rect, name, self.st.avatar, self.st.avatar_ink);
    }

    fn button(&mut self, slot: Slot, rect: Rect, id: &'static str, name: &Label, on: bool) {
        let button = IconButton::new(IconSpec::svg(IconId::new(id))).label(name.clone()).selected(on);
        let response = self.ui.child(slot, self.place(rect), button);
        let (Some(thread), true) = (self.thread, clicked(&response)) else { return };
        match slot {
            Slot::More(index) => self.events.push(CardEvent::Menu { thread, index, anchor: response.rect }),
            Slot::Mark => self.events.push(CardEvent::Mark { thread, marked: !on }),
            _ => {}
        }
    }

    fn field(&mut self, slot: Slot, at: Rect, label: &Label, text: &str, lines: usize) -> f32 {
        let height = self.heights[usize::from(slot == Slot::ReplyField)];
        let rect = self.place(Rect::from_min_size(at.min, vec2(at.width(), height)));
        let field = TextField::new(label.clone(), Bind::given(text.to_owned())).lines(lines).placeholder(label.clone());
        let out = self.ui.child(slot, rect, field);
        if let Some(value) = out.value() {
            self.events.push(if slot == Slot::EditField { CardEvent::EditText(value.clone()) } else { CardEvent::ReplyText(value.clone()) });
        }
        if out.escaped && slot == Slot::EditField {
            self.events.push(CardEvent::EditCancel);
        }
        height
    }

    fn push_size(&mut self, slot: Slot, _: &Label) -> Vec2 {
        self.buttons[push_index(slot)]
    }

    fn push(&mut self, slot: Slot, rect: Rect, label: &Label, enabled: bool) {
        let response = self.ui.child(slot, self.place(rect), Button::new(label.clone()).enabled(enabled));
        if clicked(&response) {
            self.events.push(match slot {
                Slot::EditPost => CardEvent::EditPost,
                Slot::EditCancel => CardEvent::EditCancel,
                _ => CardEvent::ReplyPost,
            });
        }
    }
}

impl Component for CommentRow<'_> {
    const ROLE: Role = Role::TREE_ITEM;
    type Style = CommentRowStyle;
    type Output = RowOut;

    fn style(cx: &StyleCx<'_>, _: CommentRowStyleKey) -> CommentRowStyle {
        let color = &cx.tokens.color;
        CommentRowStyle {
            metrics: Metrics::new(),
            normal: cx.ink(Ink::OnPanel),
            muted: cx.color(color.text.muted),
            good: cx.color(color.status.good.fg),
            bad: cx.color(color.status.bad.fg),
            faint: cx.color(color.text.faint),
            highlight: cx.color(color.state.selected),
            bar: cx.color(color.accent.solid),
            avatar: cx.color(color.accent.strong),
            avatar_ink: cx.ink_on(DataColor::new(cx.color(color.accent.strong).egui())),
            bar_width: cx.tokens.stroke.thick,
            column: cx.tokens.size.dock.column,
            height: cx.control().height,
            pad: cx.tokens.space.pad.control,
            gap: cx.tokens.space.gap.row,
        }
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let width = match cx.proposal {
            Proposal::Width(width) => width,
            _ => cx.room.unwrap_or(cx.style.column),
        };
        self.words = Some(cx.words::<CommentsWords>());
        let now = self.now;
        let age = |then: i64| age_text(&cx.env, then, now);
        let mut sink = Measure { cx, fields: [0.0; 2], buttons: [Vec2::ZERO; 3] };
        let height = self.lay(cx.style, width, &age, &mut sink);
        self.heights = sink.fields;
        self.buttons = sink.buttons;
        Vec2::new(width, height)
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        self.words = Some(cx.words::<CommentsWords>());
        let st = cx.style;
        let rect = cx.rect;
        let rtl = cx.env.direction().is_rtl();
        let now = self.now;
        let age = |then: i64| age_text(&cx.env, then, now);
        if self.selected {
            ui.draw().fill(&PaintShape::Rect(rect, CornerRadius::ZERO), st.highlight);
            let bar = if rtl {
                Rect::from_min_max(Pos2::new(rect.max.x - st.bar_width, rect.min.y), rect.max)
            } else {
                Rect::from_min_max(rect.min, Pos2::new(rect.min.x + st.bar_width, rect.max.y))
            };
            ui.draw().fill(&PaintShape::Rect(bar, CornerRadius::ZERO), st.bar);
        }
        let mut events = std::mem::take(&mut self.events);
        let mut sink = Paint {
            ui,
            st,
            origin: rect.min,
            width: rect.width(),
            rtl,
            thread: self.thread.map(|t| t.id),
            heights: self.heights,
            buttons: self.buttons,
            events: &mut events,
        };
        self.lay(st, rect.width(), &age, &mut sink);
        self.events = events;
    }

    fn finish(self, cx: Cx<'_, Self>) -> RowOut {
        RowOut { response: cx.response.clone(), events: self.events }
    }
}

/// The heading a thread falls under for `sort`, with its page; `None` for none.
fn group_of(
    words: &CommentsWords,
    kinds: &AnnotateWords,
    page_label: &dyn Fn(usize) -> String,
    thread: &CommentThread,
    sort: CommentSort,
) -> Option<(String, Option<usize>)> {
    match sort {
        CommentSort::Page => thread.anchor.page().map(|page| (words.page.fill(&[("label", &page_label(page))]), Some(page))),
        CommentSort::Author => {
            Some((thread.messages.first().map_or_else(|| words.unknown_author.as_str().to_owned(), |first| first.author.clone()), None))
        }
        CommentSort::Type => Some((kind_name(words, kinds, thread.kind), None)),
        CommentSort::Color => Some((thread.color.map_or_else(|| words.no_color.as_str().to_owned(), |color| color_name(kinds, color)), None)),
        CommentSort::Date => None,
    }
}

/// The Sort by button and its menu.
fn sort_menu(ui: &mut Ui, words: &CommentsWords, state: &mut CommentsPanelState) {
    let sorts = [
        (CommentSort::Page, &words.sort_page),
        (CommentSort::Author, &words.sort_author),
        (CommentSort::Date, &words.sort_date),
        (CommentSort::Type, &words.sort_type),
        (CommentSort::Color, &words.sort_color),
    ];
    let current = sorts.iter().find(|(sort, _)| *sort == state.sort).map_or("", |(_, label)| label.as_str());
    let name = words.sort_by.fill(&[("sort", current)]);
    let button = Button::new(name.clone()).trailing_icon(IconSpec::svg(IconId::new("chevron_down"))).show(ui);
    let mut chosen = None;
    let active = state.sort;
    Menu::new(name).opened_by(&button, Trigger::Click).show(ui, |ui, _| {
        for (sort, label) in &sorts {
            if MenuItem::new(label.as_str().to_owned()).radio_mark(active == *sort).show(ui).ran {
                chosen = Some(*sort);
            }
        }
    });
    if let Some(sort) = chosen {
        state.sort = sort;
    }
}

/// The funnel and its menu: a submenu per property that lists its values as checks (checked = shown), and
/// Show all while anything is hidden.
fn filter_menu(ui: &mut Ui, words: &CommentsWords, kinds: &AnnotateWords, threads: &[CommentThread], state: &mut CommentsPanelState) {
    let active = state.filter.is_active();
    let button = IconButton::new(IconSpec::svg(IconId::new("funnel"))).label(words.filter.clone()).selected(active).show(ui);
    let mut kind_rows: Vec<Option<AnnotationKind>> = threads.iter().map(|thread| thread.kind).collect();
    kind_rows.sort_by_cached_key(|kind| kind_name(words, kinds, *kind));
    kind_rows.dedup();
    let mut authors: Vec<&str> = threads.iter().map(|thread| thread.messages.first().map_or("", |first| first.author.as_str())).collect();
    authors.sort_unstable();
    authors.dedup();
    let mut colors: Vec<Option<[u8; 3]>> = threads.iter().map(|thread| thread.color).collect();
    colors.sort_unstable();
    colors.dedup();
    let filter = state.filter.clone();
    let mut ran: Option<FilterRow> = None;
    let check = |ui: &mut Ui, ran: &mut Option<FilterRow>, label: String, row: FilterRow, hidden: bool| {
        if MenuItem::new(label).check(!hidden).keep_open(true).show(ui).ran {
            *ran = Some(row);
        }
    };
    Menu::new(words.filter.clone()).opened_by(&button, Trigger::Click).show(ui, |ui, _| {
        let _ = Menu::submenu_row(ui, words.filter_type.clone(), |ui, _| {
            for &kind in &kind_rows {
                check(ui, &mut ran, kind_name(words, kinds, kind), FilterRow::Type(kind), filter.hidden_types.contains(&kind));
            }
        });
        let _ = Menu::submenu_row(ui, words.filter_author.clone(), |ui, _| {
            for &author in &authors {
                let name = if author.is_empty() { words.unknown_author.as_str().to_owned() } else { author.to_owned() };
                check(ui, &mut ran, name, FilterRow::Author(author.to_owned()), filter.hidden_authors.iter().any(|hidden| hidden == author));
            }
        });
        let _ = Menu::submenu_row(ui, words.filter_status.clone(), |ui, _| {
            for status in ReviewStatus::MENU {
                check(
                    ui,
                    &mut ran,
                    status_label(words, status).as_str().to_owned(),
                    FilterRow::Status(status),
                    filter.hidden_statuses.contains(&status),
                );
            }
        });
        let _ = Menu::submenu_row(ui, words.filter_color.clone(), |ui, _| {
            for &color in &colors {
                let name = color.map_or_else(|| words.no_color.as_str().to_owned(), |color| color_name(kinds, color));
                check(ui, &mut ran, name, FilterRow::Color(color), filter.hidden_colors.contains(&color));
            }
        });
        let _ = Menu::submenu_row(ui, words.filter_check.clone(), |ui, _| {
            for (marked, name) in [(true, &words.checked), (false, &words.unchecked)] {
                check(ui, &mut ran, name.as_str().to_owned(), FilterRow::Check(marked), filter.hidden_checks.contains(&marked));
            }
        });
        if active {
            Divider::horizontal().show(ui);
            if MenuItem::new(words.show_all.as_str().to_owned()).show(ui).ran {
                ran = Some(FilterRow::ShowAll);
            }
        }
    });
    let filter = &mut state.filter;
    match ran {
        Some(FilterRow::Type(kind)) => toggle(&mut filter.hidden_types, kind),
        Some(FilterRow::Author(author)) => toggle(&mut filter.hidden_authors, author),
        Some(FilterRow::Status(status)) => toggle(&mut filter.hidden_statuses, status),
        Some(FilterRow::Color(color)) => toggle(&mut filter.hidden_colors, color),
        Some(FilterRow::Check(marked)) => toggle(&mut filter.hidden_checks, marked),
        Some(FilterRow::ShowAll) => filter.clear(),
        None => {}
    }
}

/// Collapse all pages and Expand all pages, shown under the Page sort.
fn pages_menu(ui: &mut Ui, words: &CommentsWords, threads: &[CommentThread], state: &mut CommentsPanelState) {
    let button = IconButton::new(IconSpec::svg(IconId::new("more"))).label(words.pages_menu.clone()).show(ui);
    let mut collapse = None;
    Menu::new(words.pages_menu.clone()).opened_by(&button, Trigger::Click).show(ui, |ui, _| {
        if MenuItem::new(words.collapse_all.as_str().to_owned()).show(ui).ran {
            collapse = Some(true);
        }
        if MenuItem::new(words.expand_all.as_str().to_owned()).show(ui).ran {
            collapse = Some(false);
        }
    });
    match collapse {
        Some(true) => {
            let mut pages: Vec<usize> = threads.iter().filter_map(|thread| thread.anchor.page()).collect();
            pages.sort_unstable();
            pages.dedup();
            state.collapsed_pages = pages;
        }
        Some(false) => state.collapsed_pages.clear(),
        None => {}
    }
}

/// The menu of message `menu.index` of the card whose "..." was pressed. It hangs from the button's rect.
fn card_menu(
    ui: &mut Ui,
    words: &CommentsWords,
    list: &Response,
    menu: &OpenMenu,
    thread: &CommentThread,
    editable: bool,
) -> (bool, Option<CardMenu>) {
    let mut open = true;
    let ran = std::cell::Cell::new(None);
    Menu::new(words.more.clone()).opened_by(list, Trigger::Manual).anchor(Anchor::Rect(menu.anchor)).open(Bind::bound(&mut open)).show(
        ui,
        |ui, _| {
            let row = |ui: &mut Ui, label: &Label, action: CardMenu| {
                if MenuItem::new(label.as_str().to_owned()).show(ui).ran {
                    ran.set(Some(action));
                }
            };
            if menu.index == 0 {
                row(ui, &words.edit, CardMenu::Edit);
                let _ = Menu::submenu_row(ui, words.set_status.clone(), |ui, _| {
                    for status in ReviewStatus::MENU {
                        if MenuItem::new(status_label(words, status).as_str().to_owned()).radio_mark(thread.status == status).show(ui).ran {
                            ran.set(Some(CardMenu::Status(status)));
                        }
                    }
                });
                Divider::horizontal().show(ui);
                row(ui, &words.delete, CardMenu::Delete);
            } else {
                if editable {
                    row(ui, &words.edit, CardMenu::Edit);
                }
                row(ui, &words.copy_text, CardMenu::Copy);
                if editable {
                    Divider::horizontal().show(ui);
                    row(ui, &words.delete, CardMenu::Delete);
                }
            }
        },
    );
    (open, ran.into_inner())
}

impl CommentsPanel<'_> {
    /// Shows the panel and reports what the user did.
    pub fn show(self, ui: &mut Ui) -> CommentsPanelOutput {
        let CommentsPanel { label, threads, state, read_only, now, page_label } = self;
        let env = Env::of(ui);
        let words = env.words::<CommentsWords>();
        let kinds = env.words::<AnnotateWords>();
        let folder = env.folder();
        let format = env.format();
        let page_label = page_label.unwrap_or_else(|| Box::new(|page: usize| page.to_string()));
        let mut edits = Vec::new();
        let body = |ui: &mut Ui, _: &mut craft_ui::contract::BodyCx<'_>| -> Response {
            let digits = |n: usize| format.integer(n as u64);
            let replies: usize = threads.iter().map(CommentThread::replies).sum();
            Text::new(words.title.fill(&[("n", &digits(threads.len()))])).role(TypeRole::Title).show(ui);
            Text::new(words.replies.text(replies as u64, &digits(replies))).role(TypeRole::Caption).shade(Shade::Muted).show(ui);
            TextField::search(words.search.clone(), Bind::bound(&mut state.search)).placeholder(words.search.clone()).show(ui);
            Row::new().show(ui, |ui, _| {
                sort_menu(ui, &words, state);
                filter_menu(ui, &words, &kinds, threads, state);
                if state.sort == CommentSort::Page {
                    pages_menu(ui, &words, threads, state);
                }
            });
            if !read_only {
                TextField::new(words.add_comment.clone(), Bind::bound(&mut state.new_comment))
                    .lines(REPLY_LINES)
                    .placeholder(words.add_comment.clone())
                    .show(ui);
                let text = state.new_comment.trim().to_owned();
                let post = Button::new(words.post.clone()).enabled(!text.is_empty()).show(ui);
                if clicked(&post) {
                    edits.push(CommentEdit::Add { text });
                    state.new_comment.clear();
                }
            }
            if threads.is_empty() {
                return Notice::new(craft_theme::variant::Tone::Neutral, words.empty.clone()).layout(NoticeLayout::Centred).show(ui).response;
            }
            let query = state.search.trim().to_owned();
            let kind_of = |kind: AnnotationKind| kind_name(&words, &kinds, Some(kind));
            let shown: Vec<&CommentThread> = sort_threads(threads, state.sort, kind_of)
                .into_iter()
                .map(|i| &threads[i])
                .filter(|thread| state.filter.shows(thread))
                .filter(|thread| query.is_empty() || matches(thread, &query, &folder, kind_of))
                .collect();
            if shown.is_empty() {
                return Notice::new(craft_theme::variant::Tone::Neutral, words.no_match.clone()).layout(NoticeLayout::Centred).show(ui).response;
            }
            let by_page = state.sort == CommentSort::Page;
            let grouped = state.sort != CommentSort::Date;
            let mut page_counts: BTreeMap<usize, usize> = BTreeMap::new();
            for page in threads.iter().filter_map(|thread| thread.anchor.page()) {
                *page_counts.entry(page).or_default() += 1;
            }
            let heading_of = |thread: &CommentThread| group_of(&words, &kinds, &*page_label, thread, state.sort);
            let mut shown_counts: BTreeMap<String, usize> = BTreeMap::new();
            for thread in &shown {
                if let Some((title, _)) = heading_of(thread) {
                    *shown_counts.entry(title).or_default() += 1;
                }
            }
            let mut entries: Vec<(CardKey, RowEntry<'_>)> = Vec::new();
            let mut last: Option<String> = None;
            let mut groups = 0;
            // The source lists the rows on show: a folded group keeps its heading and drops its cards.
            let mut folded = false;
            for thread in &shown {
                let heading = if grouped { heading_of(thread) } else { None };
                let depth = usize::from(heading.is_some());
                if heading.is_none() {
                    folded = false;
                }
                if let Some((title, page)) = heading {
                    if last.as_deref() != Some(title.as_str()) {
                        let count = match page {
                            Some(number) if by_page => page_counts.get(&number).copied().unwrap_or_default(),
                            _ => shown_counts.get(&title).copied().unwrap_or_default(),
                        };
                        let open = !page.is_some_and(|number| state.collapsed_pages.contains(&number));
                        folded = !open;
                        entries.push((CardKey::Group(groups), RowEntry::Group { title: title.clone(), count: digits(count), page, open }));
                        groups += 1;
                        last = Some(title);
                    }
                }
                if !folded {
                    entries.push((CardKey::Thread(thread.id), RowEntry::Card { thread, depth }));
                }
            }
            let source = Rows { entries };
            let mut selection = state.selected.map_or_else(Selection::default, |thread| Selection::single(CardKey::Thread(thread)));
            let selected = state.selected;
            let reply: &str = &state.reply;
            let editing = state.editing.as_ref().map(|(thread, index, text)| (*thread, *index, text.as_str()));
            let out = CommentList::new(label.clone(), Bind::bound(&mut selection), grouped).show_items(ui, &source, &mut |_, item| match &source
                .entries[item.at]
                .1
            {
                RowEntry::Group { title, count, open, .. } => CommentRow::heading(title.clone(), count.clone(), *open),
                RowEntry::Card { thread, .. } => {
                    let name = thread
                        .messages
                        .first()
                        .map_or_else(String::new, |first| words.card_name.fill(&[("author", &first.author), ("text", &first.text)]));
                    let is_selected = selected == Some(thread.id);
                    let editing = editing.filter(|(id, ..)| *id == thread.id).map(|(_, index, text)| (index, text));
                    CommentRow::card(thread, name)
                        .state(is_selected, !read_only && !thread.locked, now)
                        .drafts(editing, if is_selected { reply } else { "" })
                }
            });
            let list = out.response.clone();
            let group_page = |at: usize| match source.entries.iter().find(|(key, _)| *key == CardKey::Group(at)) {
                Some((_, RowEntry::Group { page, .. })) => *page,
                _ => None,
            };
            for event in out.events {
                match event {
                    CollectionEvent::Item(_, card_events) => {
                        for event in card_events {
                            apply(event, state, &mut edits);
                        }
                    }
                    // A click or Enter on a heading folds or unfolds its page; Left and Right set it.
                    CollectionEvent::Activate(CardKey::Group(at)) => {
                        if let Some(page) = group_page(at) {
                            toggle(&mut state.collapsed_pages, page);
                        }
                    }
                    CollectionEvent::Open(change) => {
                        if let CardKey::Group(at) = change.key
                            && let Some(page) = group_page(at)
                        {
                            state.collapsed_pages.retain(|&folded| folded != page);
                            if !change.open {
                                state.collapsed_pages.push(page);
                            }
                        }
                    }
                    _ => {}
                }
            }
            if let Some(CardKey::Thread(thread)) = selection.items.first().copied()
                && select_thread(state, thread)
            {
                edits.push(CommentEdit::Select(thread));
            }
            if let Some(menu) = state.menu.clone() {
                if let Some(thread) = threads.iter().find(|thread| thread.id == menu.thread) {
                    let editable = !read_only && !thread.locked;
                    let (open, ran) = card_menu(ui, &words, &list, &menu, thread, editable);
                    if let Some(action) = ran {
                        apply_menu(action, thread, menu.index, state, &mut edits);
                    }
                    if !open || ran.is_some() {
                        state.menu = None;
                    }
                } else {
                    state.menu = None;
                }
            }
            list
        };
        let shown = Stack::new().fill().show(ui, body);
        let response = shown.inner.unwrap_or(shown.output);
        CommentsPanelOutput { response, edits }
    }
}

/// Applies what a card reported.
fn apply(event: CardEvent, state: &mut CommentsPanelState, edits: &mut Vec<CommentEdit>) {
    match event {
        CardEvent::Menu { thread, index, anchor } => state.menu = Some(OpenMenu { thread, index, anchor }),
        CardEvent::Mark { thread, marked } => edits.push(CommentEdit::Mark { thread, marked }),
        CardEvent::EditText(text) => {
            if let Some((_, _, draft)) = state.editing.as_mut() {
                *draft = text;
            }
        }
        CardEvent::EditPost => {
            if let Some((thread, index, text)) = state.editing.take() {
                let text = text.trim().to_owned();
                if text.is_empty() {
                    state.editing = Some((thread, index, text));
                } else {
                    edits.push(CommentEdit::Edit { thread, index, text });
                }
            }
        }
        CardEvent::EditCancel => state.editing = None,
        CardEvent::ReplyText(text) => state.reply = text,
        CardEvent::ReplyPost => {
            let text = state.reply.trim().to_owned();
            if let (Some(thread), false) = (state.selected, text.is_empty()) {
                edits.push(CommentEdit::Reply { thread, text });
                state.reply.clear();
            }
        }
    }
}

/// Applies the row of a card's menu that ran.
fn apply_menu(action: CardMenu, thread: &CommentThread, index: usize, state: &mut CommentsPanelState, edits: &mut Vec<CommentEdit>) {
    let id = thread.id;
    match action {
        CardMenu::Edit => {
            let text = thread.messages.get(index).map(|message| message.text.clone()).unwrap_or_default();
            state.editing = Some((id, index, text));
        }
        CardMenu::Status(status) => edits.push(CommentEdit::SetStatus { thread: id, status }),
        CardMenu::Copy => edits.push(CommentEdit::CopyText { thread: id, index }),
        CardMenu::Delete => edits.push(CommentEdit::Delete { thread: id, index }),
    }
}
