//! The Comments panel (N5.01).

use std::collections::BTreeMap;
use std::sync::{Arc, LazyLock};

use craft_host::Host;
use craft_i18n::{DateFormat, DateStyle, DateTime, Folder, I18n, TimeZone};
use craft_theme::{ButtonVariant, TypeRole};
use craft_ui::controls::{Button, IconButton, SearchField, TextField};
use craft_ui::edit::EditEvent;
use craft_ui::icons::{self, Icon, IconSize};
use craft_ui::localized::{AGE_SHOWN_AS_DATE, AgeStyle, Labels as _, LocaleNumbers, Words, fmt, relative_age};
use craft_ui::menu::{MenuEntry, MenuItem, PressMenu};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, UiTokens, corner_radius, role_format};
use egui::accesskit::Role;
use egui::text::LayoutJob;
use egui::{
    Align, AsIdSalt, Color32, Context, EventFilter, Frame, Id, IdSalt, Key, Layout, Margin, Modifiers, Rect, Response, Sense, Ui, UiBuilder, Vec2,
    pos2,
};

use super::ReviewStyle;
use super::annotate::{AnnotateLabels, PALETTE};
use crate::annotate::AnnotationKind;
use crate::collab::initials;
use crate::comments::{CommentFilter, CommentSort, CommentThread, ReviewStatus, ThreadId, matches, sort_threads};

/// The panel's count words, for the facade's `ui::COUNTS`.
pub mod counts {
    use craft_ui::localized::words::Counted;

    pub const REPLIES: Counted = Counted::new("comments.replies", "{n} reply", "{n} replies");

    /// Every count word of the Comments panel.
    pub const ALL: &[Counted] = &[REPLIES];
}

craft_ui::words! {
    /// The panel's words, English by default (PdfCraft's, Acrobat's).
    #[non_exhaustive]
    pub struct CommentsLabels {
        /// The header: the number of comments.
        title: "comments.title" = "Comments {n}";
        /// The replies under the header; the English plural of [`counts::REPLIES`].
        replies: "comments.replies" = "{n} replies";
        search: "comments.search" = "Search comments";
        clear_search: "comments.clearSearch" = "Clear search";
        /// The Sort by button: the chosen order.
        sort_by: "comments.sortBy" = "Sort by: {sort}";
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
        /// A page heading; `{label}` is the page's label.
        page: "comments.page" = "Page {label}";
        /// A card's accessible name.
        card_name: "comments.cardName" = "{author}: {text}";
        /// The kind of a comment that hangs on no annotation: its heading and filter row.
        plain_kind: "comments.plainKind" = "Comment";
        no_color: "comments.noColor" = "No color";
        /// The names of the toolbar's palette colours, in `annotate::PALETTE` order.
        color_yellow: "comments.colorYellow" = "Yellow";
        color_orange: "comments.colorOrange" = "Orange";
        color_red: "comments.colorRed" = "Red";
        color_pink: "comments.colorPink" = "Pink";
        color_purple: "comments.colorPurple" = "Purple";
        color_blue: "comments.colorBlue" = "Blue";
        color_light_blue: "comments.colorLightBlue" = "Light blue";
        color_green: "comments.colorGreen" = "Green";
        color_grey: "comments.colorGrey" = "Grey";
        color_black: "comments.colorBlack" = "Black";
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
        /// The menu beside the filter: Collapse all pages, Expand all pages.
        pages_menu: "comments.pagesMenu" = "Pages";
        collapse_all: "comments.collapseAll" = "Collapse all pages";
        expand_all: "comments.expandAll" = "Expand all pages";
        collapse: "comments.collapse" = "Collapse";
        expand: "comments.expand" = "Expand";
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
    /// What the filter menu hides.
    pub filter: CommentFilter,
    /// The pages whose group is folded under the Page sort.
    pub collapsed_pages: Vec<usize>,
    pub selected: Option<ThreadId>,
    /// The message being edited inline: (thread, message index, text).
    pub editing: Option<(ThreadId, usize, String)>,
    /// The selected card's reply draft.
    pub reply: String,
    /// The "Add a comment" box's draft.
    pub new_comment: String,
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
    /// The viewer's reaction `emoji` on message `index` toggled (the thread popover).
    React {
        thread: ThreadId,
        index: usize,
        emoji: String,
    },
    /// Figma's resolve (`true`) or reopen (`false`).
    Resolve {
        thread: ThreadId,
        resolved: bool,
    },
    /// The app puts a link to the thread on the clipboard.
    CopyLink {
        thread: ThreadId,
    },
    /// The viewer's read state of the thread cleared.
    MarkUnread {
        thread: ThreadId,
    },
}

/// Acrobat's right-hand comment list (PdfCraft).
///
/// - Header "Comments N" with "N replies" under it, search (a [`SearchField`]; filters by
///   [`matches`](crate::comments::matches), ignoring case as PdfCraft does), the Sort by menu, the
///   funnel menu ([`CommentFilter`]: Type, Author, Status, Color, Checkmark rows that hide what
///   they name, as PdfCraft's Filter comments) and, under the Page sort, a menu with Collapse
///   all pages and Expand all pages. A search or filter that leaves nothing shows "No comments
///   match."
/// - "Add a comment" box: Enter (Cmd+Enter in a multi-line draft) posts a sticky note.
/// - Cards grouped under headings (by [`sort_threads`](crate::comments::sort_threads)); under
///   the Page sort a heading names the page ([`page_label`](CommentsPanel::page_label)), shows the
///   count of the page's comments before filtering, and folds its cards on a click. Each card has: an
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
    read_only: bool,
    labels: Option<CommentsLabels>,
    annotate_labels: Option<AnnotateLabels>,
    page_label: Option<PageLabel<'a>>,
}

/// The label of a page, from the app's page index (or number) stored in the anchor.
type PageLabel<'a> = Box<dyn Fn(usize) -> String + 'a>;

impl<'a> CommentsPanel<'a> {
    /// The edits carry only the text; the app signs new comments with its own identity.
    pub fn new(id_salt: impl AsIdSalt, threads: &'a [CommentThread], state: &'a mut CommentsPanelState) -> Self {
        Self { id_salt: IdSalt::new(id_salt), threads, state, read_only: false, labels: None, annotate_labels: None, page_label: None }
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

    /// The reader's names of the annotation kinds, for headings, the filter and the search
    /// ([`AnnotateLabels::kind`]); the installed words by default.
    pub fn annotate_labels(mut self, labels: AnnotateLabels) -> Self {
        self.annotate_labels = Some(labels);
        self
    }

    /// The text of a page heading's `{label}`, from the page stored in the anchor (PdfCraft's
    /// page labels: "iv", "A-3"); that page itself by default.
    pub fn page_label(mut self, label: impl Fn(usize) -> String + 'a) -> Self {
        self.page_label = Some(Box::new(label));
        self
    }

    // pdfcraft: crates/ui-egui/src/panels/comments_panel.rs:115 `show`, `:311` `card`, `:541`
    // `inline_editor`, `:568` `reply_menu`, `:600` `avatar`, `:626` `header_controls`;
    // `tools/comments/menu.rs:368` `status_badge`; deckcraft: crates/ui-egui/src/panels/comments.rs:67.
    pub fn show(self, ui: &mut Ui) -> CommentsPanelOutput {
        let Self { id_salt, threads, state, read_only, labels, annotate_labels, page_label } = self;
        let labels = labels.unwrap_or_else(|| CommentsLabels::of(ui));
        let kinds = annotate_labels.unwrap_or_else(|| AnnotateLabels::of(ui));
        let page_label = page_label.unwrap_or_else(|| Box::new(|page: usize| page.to_string()));
        let tokens = Tokens::of(ui);
        let style = ReviewStyle::of(&tokens);
        let numbers = LocaleNumbers::of(ui);
        let cx = Cx {
            tokens: &tokens,
            style: &style,
            labels: &labels,
            kinds: &kinds,
            page_label: &*page_label,
            numbers: &numbers,
            id: ui.make_persistent_id(id_salt),
            read_only,
        };
        let mut edits = Vec::new();
        let response = ui
            .vertical(|ui| {
                ui.spacing_mut().item_spacing.y = tokens.space.gap.stack;
                header(ui, &cx, threads, state);
                if !read_only {
                    add_box(ui, &cx, state, &mut edits);
                }
                if threads.is_empty() {
                    text(ui, &tokens, style.body_text, tokens.color.text.muted.to_egui(), &labels.empty);
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
    kinds: &'a AnnotateLabels,
    page_label: &'a dyn Fn(usize) -> String,
    numbers: &'a LocaleNumbers,
    id: Id,
    read_only: bool,
}

impl Cx<'_> {
    /// A count written in the locale's digits.
    fn number(&self, n: usize) -> String {
        self.numbers.format(f64::from(u32::try_from(n).unwrap_or(u32::MAX)), 0..=0)
    }

    /// The reader's name of a thread's kind.
    fn kind_name(&self, kind: Option<AnnotationKind>) -> String {
        kind.map_or_else(|| self.labels.plain_kind.clone(), |kind| self.kinds.kind(kind).to_owned())
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

/// What a row of the pages menu does.
#[derive(Clone, Copy)]
enum PagesRow {
    CollapseAll,
    ExpandAll,
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
    let galley = ui.painter().layout_job(LayoutJob::single_section(initials(name), role_format(tokens, ReviewStyle::of(tokens).caption_text, on)));
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, on);
    response
}

/// The icon of a thread's annotation kind.
fn kind_icon(kind: Option<AnnotationKind>) -> Icon {
    use AnnotationKind as K;
    match kind {
        Some(K::Highlight | K::Underline | K::StrikeOut | K::Squiggly) => Icon::Brush,
        Some(K::TextBox | K::Callout) => Icon::Text,
        Some(K::Ink) => Icon::Pencil,
        Some(K::Attach) => Icon::Link,
        _ => Icon::Note,
    }
}

/// The icon and colour of a status badge; `None` has none.
fn status_badge(style: &ReviewStyle, status: ReviewStatus) -> Option<(Icon, Color32)> {
    match status {
        ReviewStatus::None => None,
        ReviewStatus::Accepted => Some((Icon::ThumbsUp, style.status_accepted)),
        ReviewStatus::Rejected => Some((Icon::ThumbsDown, style.status_rejected)),
        ReviewStatus::Cancelled => Some((Icon::XCircle, style.status_cancelled)),
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

/// A status badge: the icon in the status colour on its own `surface.panel` chip, so the
/// colour keeps its contrast on a selected card too.
fn status_chip(ui: &mut Ui, tokens: &UiTokens, icon: Icon, color: Color32, name: &str) {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(tokens.size.control.sm), Sense::hover());
    response.widget_info(|| egui::WidgetInfo::labeled(Role::Label, true, name));
    ui.painter().rect_filled(rect, corner_radius(tokens.shape.radius.control), tokens.color.surface.panel.to_egui());
    icons::paint(ui, rect, icon, IconSize::Sm, color);
    response.on_hover_text(name);
}

fn header(ui: &mut Ui, cx: &Cx, threads: &[CommentThread], state: &mut CommentsPanelState) {
    let (tokens, labels) = (cx.tokens, cx.labels);
    let replies: usize = threads.iter().map(CommentThread::replies).sum();
    text(ui, tokens, cx.style.title_text, tokens.color.text.normal.to_egui(), fmt(&labels.title, &[("n", &cx.number(threads.len()))]));
    let replies = Words::of(ui).counted(ui.ctx(), counts::REPLIES, &labels.replies, u64::try_from(replies).unwrap_or(u64::MAX));
    text(ui, tokens, cx.style.caption_text, tokens.color.text.muted.to_egui(), replies);
    let search = SearchField::new(cx.id.with("search"), &labels.search, &state.search, &labels.clear_search).placeholder(&labels.search).show(ui);
    if let Some(value) = search.event.as_ref().and_then(EditEvent::value) {
        state.search.clone_from(value);
    }
    ui.horizontal(|ui| {
        sort_menu(ui, cx, state);
        filter_menu(ui, cx, threads, state);
        if state.sort == CommentSort::Page {
            pages_menu(ui, cx, threads, state);
        }
    });
}

fn sort_menu(ui: &mut Ui, cx: &Cx, state: &mut CommentsPanelState) {
    let labels = cx.labels;
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
    let button = ui.add(Button::new(fmt(&labels.sort_by, &[("sort", current)])).trailing_icon(Icon::ChevronDown).id_salt(cx.id.with("sort")));
    if let Some(chosen) = PressMenu::new(&entries).show(ui, &button).chosen {
        state.sort = chosen;
    }
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

/// The funnel: a submenu per property that lists its values as checks (checked = shown), and
/// Show all while anything is hidden (PdfCraft `header_controls`, `:626`).
fn filter_menu(ui: &mut Ui, cx: &Cx, threads: &[CommentThread], state: &mut CommentsPanelState) {
    let labels = cx.labels;
    let filter = &state.filter;
    let check = |label: String, row: FilterRow, hidden: bool| MenuItem::new(label, row).check(!hidden).keep_open(true);
    let mut kinds: Vec<Option<AnnotationKind>> = threads.iter().map(|t| t.kind).collect();
    kinds.sort_by_cached_key(|kind| cx.kind_name(*kind));
    kinds.dedup();
    let mut authors: Vec<&str> = threads.iter().map(|t| t.messages.first().map_or("", |m| m.author.as_str())).collect();
    authors.sort_unstable();
    authors.dedup();
    let mut colors: Vec<Option<[u8; 3]>> = threads.iter().map(|t| t.color).collect();
    colors.sort_unstable();
    colors.dedup();
    let types = kinds.iter().map(|&kind| check(cx.kind_name(kind), FilterRow::Type(kind), filter.hidden_types.contains(&kind)).into()).collect();
    let by_author = authors
        .iter()
        .map(|&author| {
            let name = if author.is_empty() { labels.unknown_author.clone() } else { author.to_owned() };
            check(name, FilterRow::Author(author.to_owned()), filter.hidden_authors.iter().any(|hidden| hidden == author)).into()
        })
        .collect();
    let statuses = ReviewStatus::MENU
        .iter()
        .map(|&status| check(status_label(labels, status).to_owned(), FilterRow::Status(status), filter.hidden_statuses.contains(&status)).into())
        .collect();
    let by_color = colors
        .iter()
        .map(|&color| {
            let name = color.map_or_else(|| labels.no_color.clone(), |color| color_name(labels, color));
            let row = check(name, FilterRow::Color(color), filter.hidden_colors.contains(&color));
            match color {
                Some([r, g, b]) => row.swatch(Color32::from_rgb(r, g, b)).into(),
                None => row.into(),
            }
        })
        .collect();
    let checks = [(true, &labels.checked), (false, &labels.unchecked)]
        .map(|(marked, name)| check(name.clone(), FilterRow::Check(marked), filter.hidden_checks.contains(&marked)).into())
        .into();
    let mut entries: Vec<MenuEntry<FilterRow>> = vec![
        MenuItem::submenu(labels.filter_type.clone(), types).into(),
        MenuItem::submenu(labels.filter_author.clone(), by_author).into(),
        MenuItem::submenu(labels.filter_status.clone(), statuses).into(),
        MenuItem::submenu(labels.filter_color.clone(), by_color).into(),
        MenuItem::submenu(labels.filter_check.clone(), checks).into(),
    ];
    let active = state.filter.is_active();
    if active {
        entries.push(MenuEntry::Separator);
        entries.push(MenuItem::new(labels.show_all.clone(), FilterRow::ShowAll).into());
    }
    let button = ui.add(IconButton::new(Icon::Funnel, &labels.filter).selected(active).id_salt(cx.id.with("filter")));
    let filter = &mut state.filter;
    match PressMenu::new(&entries).show(ui, &button).chosen {
        Some(FilterRow::Type(kind)) => toggle(&mut filter.hidden_types, kind),
        Some(FilterRow::Author(author)) => toggle(&mut filter.hidden_authors, author),
        Some(FilterRow::Status(status)) => toggle(&mut filter.hidden_statuses, status),
        Some(FilterRow::Color(color)) => toggle(&mut filter.hidden_colors, color),
        Some(FilterRow::Check(marked)) => toggle(&mut filter.hidden_checks, marked),
        Some(FilterRow::ShowAll) => filter.clear(),
        None => {}
    }
}

/// Collapse all pages and Expand all pages (PdfCraft's "…" menu, shown under the Page sort).
fn pages_menu(ui: &mut Ui, cx: &Cx, threads: &[CommentThread], state: &mut CommentsPanelState) {
    let labels = cx.labels;
    let entries: Vec<MenuEntry<PagesRow>> = vec![
        MenuItem::new(labels.collapse_all.clone(), PagesRow::CollapseAll).into(),
        MenuItem::new(labels.expand_all.clone(), PagesRow::ExpandAll).into(),
    ];
    let button = ui.add(IconButton::new(Icon::More, &labels.pages_menu).id_salt(cx.id.with("pages")));
    match PressMenu::new(&entries).show(ui, &button).chosen {
        Some(PagesRow::CollapseAll) => {
            let mut pages: Vec<usize> = threads.iter().filter_map(|t| t.anchor.page()).collect();
            pages.sort_unstable();
            pages.dedup();
            state.collapsed_pages = pages;
        }
        Some(PagesRow::ExpandAll) => state.collapsed_pages.clear(),
        None => {}
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

/// A label colour's name when it is one of the toolbar's palette colours, else its hex.
fn color_name(labels: &CommentsLabels, color: [u8; 3]) -> String {
    let names = [
        &labels.color_yellow,
        &labels.color_orange,
        &labels.color_red,
        &labels.color_pink,
        &labels.color_purple,
        &labels.color_blue,
        &labels.color_light_blue,
        &labels.color_green,
        &labels.color_grey,
        &labels.color_black,
    ];
    const _: () = assert!(PALETTE.len() == 10);
    match PALETTE.iter().position(|swatch| *swatch == color).and_then(|at| names.get(at)) {
        Some(name) => (*name).clone(),
        None => format!("#{:02X}{:02X}{:02X}", color[0], color[1], color[2]),
    }
}

/// The heading a thread falls under for `sort`; `None` for none.
fn group_of(cx: &Cx, thread: &CommentThread, sort: CommentSort) -> Option<String> {
    match sort {
        CommentSort::Page => thread.anchor.page().map(|page| fmt(&cx.labels.page, &[("label", &(cx.page_label)(page))])),
        CommentSort::Author => Some(thread.messages.first().map_or_else(|| cx.labels.unknown_author.clone(), |m| m.author.clone())),
        CommentSort::Type => Some(cx.kind_name(thread.kind)),
        CommentSort::Color => Some(thread.color.map_or_else(|| cx.labels.no_color.clone(), |color| color_name(cx.labels, color))),
        CommentSort::Date => None,
    }
}

/// A page's heading under the Page sort: the fold chevron, the page, and the number of its
/// comments before filtering (PdfCraft `:246-275`). Returns true when the group was toggled.
fn page_heading(ui: &mut Ui, cx: &Cx, heading: String, page: usize, count: usize, folded: bool) -> bool {
    let (tokens, labels) = (cx.tokens, cx.labels);
    ui.horizontal(|ui| {
        let (icon, name) = if folded { (Icon::ChevronRight, &labels.expand) } else { (Icon::ChevronDown, &labels.collapse) };
        let mut toggled = ui.add(IconButton::new(icon, name).id_salt(cx.id.with(("fold", page)))).clicked();
        let title = LayoutJob::single_section(heading, role_format(tokens, cx.style.strong_text, tokens.color.text.muted.to_egui()));
        toggled |= ui.add(egui::Label::new(title).sense(Sense::click())).clicked();
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            text(ui, tokens, cx.style.caption_text, tokens.color.text.muted.to_egui(), cx.number(count));
        });
        toggled
    })
    .inner
}

fn list(ui: &mut Ui, cx: &Cx, threads: &[CommentThread], state: &mut CommentsPanelState, edits: &mut Vec<CommentEdit>) {
    let query = state.search.trim().to_owned();
    let folder = Folder::new("");
    let kind_name = |kind| cx.kind_name(Some(kind));
    let shown: Vec<&CommentThread> = sort_threads(threads, state.sort, kind_name)
        .into_iter()
        .map(|i| &threads[i])
        .filter(|t| state.filter.shows(t))
        .filter(|t| query.is_empty() || matches(t, &query, &folder, kind_name))
        .collect();
    if shown.is_empty() {
        text(ui, cx.tokens, cx.style.body_text, cx.tokens.color.text.muted.to_egui(), &cx.labels.no_match);
        return;
    }
    let mut page_counts: BTreeMap<usize, usize> = BTreeMap::new();
    for page in threads.iter().filter_map(|t| t.anchor.page()) {
        *page_counts.entry(page).or_default() += 1;
    }
    let by_page = state.sort == CommentSort::Page;
    let list = ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = cx.tokens.space.gap.row;
        let mut drawn: Vec<&CommentThread> = Vec::new();
        let mut focused = None;
        let mut group = None;
        let mut page = None;
        for thread in &shown {
            if by_page {
                let current = thread.anchor.page();
                if current != page {
                    page = current;
                    if let (Some(number), Some(heading)) = (current, group_of(cx, thread, state.sort)) {
                        let count = page_counts.get(&number).copied().unwrap_or_default();
                        let folded = state.collapsed_pages.contains(&number);
                        if page_heading(ui, cx, heading, number, count, folded) {
                            if folded {
                                state.collapsed_pages.retain(|&p| p != number);
                            } else {
                                state.collapsed_pages.push(number);
                            }
                        }
                    }
                }
                if current.is_some_and(|number| state.collapsed_pages.contains(&number)) {
                    continue;
                }
            } else {
                let heading = group_of(cx, thread, state.sort);
                if heading.is_some() && heading != group {
                    text(ui, cx.tokens, cx.style.strong_text, cx.tokens.color.text.muted.to_egui(), heading.clone().unwrap_or_default());
                }
                group = heading;
            }
            if card(ui, cx, thread, state, edits) {
                focused = Some(drawn.len());
            }
            drawn.push(thread);
        }
        focused.map(|position| (position, drawn))
    });
    ui.ctx().accesskit_node_builder(list.response.id, |node| node.set_role(Role::List));
    let Some((position, drawn)) = list.inner else { return };
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
        && let Some(next) = position.checked_add_signed(step).and_then(|p| drawn.get(p))
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

fn select(thread: ThreadId, state: &mut CommentsPanelState, edits: &mut Vec<CommentEdit>) {
    if select_thread(state, thread) {
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
        let bar = Rect::from_min_max(rect.min, pos2(rect.min.x + style.card_accent_width, rect.max.y));
        ui.painter().rect_filled(bar, 0.0, style.card_accent);
    }
    let name = thread.messages.first().map_or_else(String::new, |m| fmt(&cx.labels.card_name, &[("author", &m.author), ("text", &m.text)]));
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_role(Role::ListItem);
        node.set_label(name);
        node.set_selected(selected);
    });
    if response.clicked() {
        select(thread.id, state, edits);
    }
    if response.has_focus() {
        ui.memory_mut(|memory| memory.set_focus_lock_filter(response.id, EventFilter { vertical_arrows: true, ..EventFilter::default() }));
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
        icons::paint(ui, rect, kind_icon(thread.kind), IconSize::Sm, tint);
        let first = thread.messages.first();
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let author = first.map_or(labels.unknown_author.as_str(), |m| m.author.as_str());
            text(ui, tokens, style.strong_text, tokens.color.text.normal.to_egui(), author);
            if let Some(m) = first {
                let then = m.modified.unwrap_or(m.created);
                text(ui, tokens, style.caption_text, tokens.color.text.muted.to_egui(), age_text(ui.ctx(), then));
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
                status_chip(ui, tokens, icon, color, status_label(labels, thread.status));
            }
        });
    });
}

/// Message `index`: its text, or the inline editor while it is being edited.
fn message_body(ui: &mut Ui, cx: &Cx, thread: &CommentThread, index: usize, state: &mut CommentsPanelState, edits: &mut Vec<CommentEdit>) {
    let Some(message) = thread.messages.get(index) else { return };
    let editing = state.editing.as_ref().is_some_and(|(t, i, _)| *t == thread.id && *i == index);
    if !editing {
        text(ui, cx.tokens, ReviewStyle::of(cx.tokens).body_text, cx.tokens.color.text.normal.to_egui(), &message.text);
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

fn reply(ui: &mut Ui, cx: &Cx, thread: &CommentThread, index: usize, editable: bool, state: &mut CommentsPanelState, edits: &mut Vec<CommentEdit>) {
    let (tokens, labels) = (cx.tokens, cx.labels);
    let Some(message) = thread.messages.get(index) else { return };
    ui.horizontal_top(|ui| {
        avatar(ui, tokens, &message.author);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
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
