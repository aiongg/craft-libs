//! `LinkList`: the Links panel's rows, filter, sort, info section and actions.

use std::collections::HashMap;
use std::sync::Arc;

use craft_docpath::links::{Link, LinkStatus};
use craft_platform::open::natural_cmp;
use craft_theme::TypeRole;
use craft_ui::behavior::{RowInfo, Selection};
use craft_ui::canvas::textures::{Priority, TextureKey, TextureState, ThumbPaint, ThumbRequest, ThumbSource, Thumbnails};
use craft_ui::composites::{EmptyState, PanelFooter, SectionHeader};
use craft_ui::icons::Icon;
use craft_ui::list::{List, ListEvent, ListRow, ThumbSize};
use craft_ui::localized::{SharedLabels as _, Words};
use craft_ui::menu::{ContextMenu, MenuEntry, MenuItem, OpenAt};
use craft_ui::theme::{Tokens, TokensOf as _, role_format};
use egui::{AsIdSalt, Context, Id, IdSalt, Label, Modifiers, Response, ScrollArea, Ui, vec2};

use super::{RelinkStyle, StatusBadge};

/// Which links the list shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum LinkFilter {
    /// Every link.
    #[default]
    All,
    /// `LinkStatus::is_missing`.
    Missing,
    /// Modified.
    Modified,
    /// Embedded.
    Embedded,
}

/// The list's order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum LinkSort {
    /// By name (`natural_cmp`).
    #[default]
    Name,
    /// By extension, then name.
    Kind,
    /// Missing first, then Modified, then the rest; then name.
    Status,
}

/// The list's saved choices (VectorCraft's panel options); the host stores it.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct LinkListState {
    /// The filter.
    pub filter: LinkFilter,
    /// The order.
    pub sort: LinkSort,
    /// Row thumbnails: None, Small, Medium, Large.
    pub thumbnails: ThumbSize,
    /// The Link Info section is open.
    pub info_open: bool,
}

/// What the list reported, in order. Each is a `Set` the host runs as its own command, one undo
/// step each.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LinkListEvent<K> {
    /// The selection changed; already applied.
    Selected,
    /// Relink…: a file pick for the selection (Photoshop, InDesign).
    Relink(Vec<K>),
    /// Relink to Folder…: a folder pick for the selection.
    RelinkToFolder(Vec<K>),
    /// Find missing links: the host opens the `RelinkDialog` on the missing set.
    FindMissing,
    /// Update Link on the modified links of the selection.
    Update(Vec<K>),
    /// Edit Original (the host's `openable_original` check stays its own).
    EditOriginal(K),
    /// Break Link (embed).
    BreakLink(Vec<K>),
    /// Reveal in Finder.
    Reveal(K),
    /// Go To Link (double click).
    GoTo(K),
    /// The state changed (filter, sort, thumbnails, info section); the host saves it.
    State,
}

/// What [`LinkList::show`] returns.
#[derive(Debug)]
#[must_use = "the host runs the list's events"]
pub struct LinkListOutput<K> {
    /// The list.
    pub response: Response,
    /// What happened, in order.
    pub events: Vec<LinkListEvent<K>>,
}

craft_ui::words! {
    /// The Links panel's words, English by default.
    #[non_exhaustive]
    pub struct LinkListLabels {
        list: "browse.links" = "Links";
        show_all: "browse.linksShowAll" = "Show All";
        show_missing: "browse.linksShowMissing" = "Show Missing";
        show_modified: "browse.linksShowModified" = "Show Modified";
        show_embedded: "browse.linksShowEmbedded" = "Show Embedded";
        sort_name: "browse.linksSortName" = "Sort by Name";
        sort_kind: "browse.linksSortKind" = "Sort by Kind";
        sort_status: "browse.linksSortStatus" = "Sort by Status";
        relink: "browse.linksRelink" = "Relink…";
        relink_folder: "browse.linksRelinkFolder" = "Relink to Folder…";
        find_missing: "browse.linksFindMissing" = "Find Missing Links…";
        update: "browse.linksUpdate" = "Update Link";
        edit_original: "browse.linksEditOriginal" = "Edit Original";
        break_link: "browse.linksBreak" = "Break Link";
        reveal: "browse.linksReveal" = "Reveal in Finder";
        go_to: "browse.linksGoTo" = "Go to Link";
        info: "browse.linksInfo" = "Link Info";
        /// `{n}` the page.
        page: "browse.linksPage" = "Page {n}";
        missing: "browse.linksMissing" = "Missing";
        modified: "browse.linksModified" = "Modified";
        unreadable: "browse.linksUnreadable" = "Unreadable";
        /// `{reason}` from `Refused`'s words.
        refused: "browse.linksRefused" = "Not followed: {reason}";
        refused_network: "browse.linksRefusedNetwork" = "the path is on another computer";
        refused_device: "browse.linksRefusedDevice" = "the path names a device, not a file";
        refused_url: "browse.linksRefusedUrl" = "the link is not a file path";
        refused_invalid: "browse.linksRefusedInvalid" = "the path is not valid";
        embedded: "browse.linksEmbedded" = "Embedded";
        thumbnails: "browse.linksThumbnails" = "Thumbnails";
        menu: "browse.linksMenu" = "Links Menu";
        empty: "browse.linksEmpty" = "No links";
    }
}

/// A host slot drawn inside the widget.
type Slot<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// The thumbnails a widget draws with: the cache and the app's source, keyed by the widget's key.
type ThumbPair<'a, K> = (&'a Thumbnails<K>, &'a dyn ThumbSource<K>);

/// The Links panel (N5.05): rows of the document's links with thumbnail, name, page and status.
///
/// - **Rows.** `ListRow`s, striped: thumbnail per `LinkListState::thumbnails` (`ThumbSize`,
///   through `Thumbnails` keyed by the link key: `state` asked before the row is built, `paint`
///   in its thumbnail closure), name, page ([`LinkListLabels::page`] in `Caption` `text.faint`),
///   the [`StatusBadge`](super::StatusBadge) as a trailing item; filtered by `filter` and sorted
///   by `sort`.
/// - **Selection.** A click replaces; Cmd toggles; Shift selects the range from the last click
///   made without Shift (with Cmd adds); a Shift-click whose anchor row is filtered out acts as a
///   plain click (VectorCraft's `click`); double click: `GoTo`.
/// - **Actions** (the row under the list, `.actions(true)`; also the panel menu): Relink… (needs
///   a selection), Relink to Folder… (a selection), Update Link (any selected is Modified), Edit
///   Original (any selected is linked and not missing), Break Link (any linked), Reveal (one
///   linked, not missing), Go to Link (one). Find Missing Links… (any link missing).
/// - **Filter and sort** in the panel menu, radio items; the thumbnail size too; all in
///   `LinkListState` (`State` reported).
/// - **Info section.** A `SectionHeader` "Link Info" (`info_open`) with the rows the host gives in
///   `.info(..)` (path, size, modified, dimensions, page).
/// - **AccessKit.** A `ListBox`; each row named "name, Page n, status in words", `selected`.
///
/// The action row has the buttons Relink…, Update Link, Edit Original, Go to Link and Break Link
/// and a menu button; the panel menu behind it (also the list's context menu) has every action
/// besides the filter, sort and thumbnail choices. Without `.selection(..)` or `.state(..)` the
/// list keeps its own in egui memory under its id.
pub struct LinkList<'a, K: TextureKey> {
    id_salt: IdSalt,
    links: &'a [Link<K>],
    statuses: &'a HashMap<K, LinkStatus>,
    selection: Option<&'a mut Selection<K>>,
    state: Option<&'a mut LinkListState>,
    thumbnails: Option<ThumbPair<'a, K>>,
    actions: bool,
    info: Option<Slot<'a>>,
    labels: Option<LinkListLabels>,
}

impl<'a, K: TextureKey + std::fmt::Debug> LinkList<'a, K> {
    /// The list of `links` with their known `statuses` (from `LinkMonitor::statuses`; a link
    /// without one shows no badge).
    pub fn new(id_salt: impl AsIdSalt, links: &'a [Link<K>], statuses: &'a HashMap<K, LinkStatus>) -> Self {
        // vectorcraft: crates/ui-egui/src/panels/links.rs:300 (`show`).
        Self {
            id_salt: IdSalt::new(id_salt),
            links,
            statuses,
            selection: None,
            state: None,
            thumbnails: None,
            actions: true,
            info: None,
            labels: None,
        }
    }

    /// The selection.
    #[must_use]
    pub fn selection(mut self, selection: &'a mut Selection<K>) -> Self {
        // new: builder.
        self.selection = Some(selection);
        self
    }

    /// The saved filter, sort, thumbnails and info state.
    #[must_use]
    pub fn state(mut self, state: &'a mut LinkListState) -> Self {
        // vectorcraft: crates/ui-egui/src/panels/links.rs:56 (`options`).
        self.state = Some(state);
        self
    }

    /// Row thumbnails keyed by the link key.
    #[must_use]
    pub fn thumbnails(mut self, thumbnails: &'a Thumbnails<K>, source: &'a dyn ThumbSource<K>) -> Self {
        // new: builder.
        self.thumbnails = Some((thumbnails, source));
        self
    }

    /// Shows the action row under the list (default on).
    #[must_use]
    pub fn actions(mut self, on: bool) -> Self {
        // vectorcraft: crates/ui-egui/src/panels/links.rs:300 (the bottom bar).
        self.actions = on;
        self
    }

    /// The Link Info section's body for the lead link.
    #[must_use]
    pub fn info(mut self, info: impl FnOnce(&mut Ui) + 'a) -> Self {
        // vectorcraft: crates/ui-egui/src/panels/links.rs:300 (Show Link Info).
        self.info = Some(Box::new(info));
        self
    }

    /// The words.
    #[must_use]
    pub fn labels(mut self, labels: LinkListLabels) -> Self {
        // new: builder.
        self.labels = Some(labels);
        self
    }

    /// Draws the panel body and reports what happened.
    pub fn show(self, ui: &mut Ui) -> LinkListOutput<K> {
        // vectorcraft: crates/ui-egui/src/panels/links.rs:300 (`show`), :280 (`click`).
        let Self { id_salt, links, statuses, selection, state, thumbnails, actions, info, labels } = self;
        let tokens = Tokens::of(ui);
        let labels = labels.map_or_else(|| LinkListLabels::shared(ui), Arc::new);
        let style = RelinkStyle::of(&tokens);
        let row_style = ListRow::<K>::style(&tokens);
        let id = ui.make_persistent_id(id_salt);
        let (state_slot, selection_slot, menu_slot) = (id.with("state"), id.with("selection"), id.with("menu-open"));
        let ctx = ui.ctx().clone();

        let saved = state.as_deref().cloned().or_else(|| ctx.data(|d| d.get_temp::<LinkListState>(state_slot))).unwrap_or_default();
        let before = selection.as_deref().cloned().or_else(|| ctx.data(|d| d.get_temp::<Selection<K>>(selection_slot))).unwrap_or_default();
        let mut working = saved.clone();

        let shown = visible_links(links, statuses, working.filter, working.sort);
        let keys: Vec<K> = shown.iter().map(|link| link.key.clone()).collect();
        let order: Vec<RowInfo<K>> = keys.iter().cloned().map(RowInfo::leaf).collect();

        let ppp = ui.pixels_per_point();
        let footer = if actions { PanelFooter::reserved_height(&PanelFooter::style(&tokens, ppp)) } else { 0.0 };
        let section = SectionHeader::style(&tokens, ppp);
        let header = if info.is_some() { section.row_height + section.gap * 2.0 } else { 0.0 };
        let free = (ui.available_height() - footer - header).max(0.0);
        let (list_height, info_height) = if info.is_some() && working.info_open { (free / 2.0, free / 2.0) } else { (free, free) };

        let side = row_style.thumb_side(working.thumbnails);
        let cx = RowContext { ctx: ctx.clone(), thumbnails, side, labels: &labels, style, statuses };
        let mut sel = before.clone();
        let mut events = Vec::new();
        let mut commands = Vec::new();

        let inner = ui.scope(|ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            let mut list_response = None;
            if shown.is_empty() {
                EmptyState::new(labels.empty.clone()).show(ui);
            } else {
                let mut list_sel = before.clone();
                let out = List::new((id, "list"), labels.list.clone(), order.as_slice())
                    .selection(&mut list_sel)
                    .thumbnails(working.thumbnails)
                    .max_height(list_height)
                    .show(ui, |at, _| link_row(&cx, shown[at], at));
                let modifiers = ui.input(|input| input.modifiers);
                let clicked: Vec<&K> =
                    out.events.iter().filter_map(|event| if let ListEvent::Clicked(key) = event { Some(key) } else { None }).collect();
                let mut menu_request = None;
                for event in &out.events {
                    match event {
                        ListEvent::Clicked(key) => sel = pick(&sel, &keys, key, modifiers),
                        ListEvent::Selection(change) if !change.row.as_ref().is_some_and(|row| clicked.contains(&row)) => {
                            sel = change.selection.clone();
                        }
                        ListEvent::Activate(key) => events.push(LinkListEvent::GoTo(key.clone())),
                        ListEvent::ContextMenu(request) => {
                            if let Some(row) = request.row.as_ref().filter(|_| !request.selected) {
                                sel = Selection::only(row.clone());
                            }
                            menu_request = Some(request.pointer.map_or(OpenAt::Rect(request.anchor), OpenAt::Pointer));
                        }
                        _ => {}
                    }
                }
                list_response = Some((out.response, menu_request));
            }
            let facts = Facts::of(&shown, &sel, links, statuses);
            let entries = menu_entries(ui, &labels, &facts, &working);
            if let Some((response, request)) = &list_response {
                let menu = ContextMenu::new(&entries).own_gestures(false).open_at(*request).show(ui, response);
                commands.extend(menu.chosen);
            }
            if let Some(show_info) = info {
                let mut open = working.info_open;
                SectionHeader::new((id, "info"), labels.info.clone()).open(&mut open).show(ui, |ui| {
                    ScrollArea::vertical().max_height(info_height).auto_shrink([false, true]).show(ui, show_info);
                });
                working.info_open = open;
            }
            if actions {
                let menu_open = ctx.data(|d| d.get_temp::<bool>(menu_slot)).unwrap_or(false);
                PanelFooter::show(ui, |ui| footer_buttons(ui, &labels, &facts, &entries, menu_open, menu_slot, &mut commands));
            }
            list_response.map(|(response, _)| response)
        });

        let facts = Facts::of(&shown, &sel, links, statuses);
        for command in commands {
            facts.run(command, &mut working, &mut events);
        }
        if sel != before {
            events.insert(0, LinkListEvent::Selected);
        }
        if working != saved {
            events.push(LinkListEvent::State);
        }
        match state {
            Some(slot) => *slot = working,
            None => {
                ctx.data_mut(|d| d.insert_temp(state_slot, working));
            }
        }
        match selection {
            Some(slot) => *slot = sel,
            None => {
                ctx.data_mut(|d| d.insert_temp(selection_slot, sel));
            }
        }
        LinkListOutput { response: inner.response, events }
    }
}

/// One pass's shared inputs of the rows.
struct RowContext<'a, K: TextureKey> {
    ctx: Context,
    thumbnails: Option<ThumbPair<'a, K>>,
    /// The thumbnail side in points; 0 for none.
    side: f32,
    labels: &'a LinkListLabels,
    style: RelinkStyle,
    statuses: &'a HashMap<K, LinkStatus>,
}

/// The status of `link`: the monitor's, else `Embedded` for an embedded link, else unknown.
fn status_of<K: TextureKey>(link: &Link<K>, statuses: &HashMap<K, LinkStatus>) -> Option<LinkStatus> {
    statuses.get(&link.key).copied().or(link.embedded.then_some(LinkStatus::Embedded))
}

/// Whether the link's file is the document's own data.
fn is_embedded(status: Option<LinkStatus>) -> bool {
    status == Some(LinkStatus::Embedded)
}

/// The links `filter` lets through, in the order of `sort`.
fn visible_links<'a, K: TextureKey>(links: &'a [Link<K>], statuses: &HashMap<K, LinkStatus>, filter: LinkFilter, sort: LinkSort) -> Vec<&'a Link<K>> {
    let mut shown: Vec<&Link<K>> = links
        .iter()
        .filter(|link| {
            let status = status_of(link, statuses);
            match filter {
                LinkFilter::All => true,
                LinkFilter::Missing => status.is_some_and(LinkStatus::is_missing),
                LinkFilter::Modified => status == Some(LinkStatus::Modified),
                LinkFilter::Embedded => is_embedded(status),
            }
        })
        .collect();
    let extension = |name: &str| std::path::Path::new(name).extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let rank = |link: &Link<K>| match status_of(link, statuses) {
        Some(status) if status.is_missing() => 0,
        Some(LinkStatus::Modified) => 1,
        _ => 2,
    };
    shown.sort_by(|a, b| {
        let by_name = natural_cmp(&a.name, &b.name);
        match sort {
            LinkSort::Name => by_name,
            LinkSort::Kind => extension(&a.name).cmp(&extension(&b.name)).then(by_name),
            LinkSort::Status => rank(a).cmp(&rank(b)).then(by_name),
        }
    });
    shown
}

/// The selection after a click on `key` among the shown `order` (VectorCraft's `click`): plain
/// replaces and sets the anchor; Cmd toggles and sets the anchor; Shift selects the rows from
/// the anchor to `key` (with Cmd adds them) and keeps the anchor. A Shift-click whose anchor is
/// not shown acts as a click without Shift.
fn pick<K: TextureKey>(selection: &Selection<K>, order: &[K], key: &K, modifiers: Modifiers) -> Selection<K> {
    let position = |key: &K| order.iter().position(|k| k == key);
    let Some(at) = position(key) else { return selection.clone() };
    if modifiers.shift
        && let Some(from) = selection.anchor.as_ref().and_then(position)
    {
        let range = &order[from.min(at)..=from.max(at)];
        let mut items = if modifiers.command { selection.items.clone() } else { Vec::new() };
        let added: Vec<K> = range.iter().filter(|k| !items.contains(k)).cloned().collect();
        items.extend(added);
        return Selection { items, lead: Some(key.clone()), anchor: selection.anchor.clone() };
    }
    if modifiers.command {
        let mut items = selection.items.clone();
        match items.iter().position(|k| k == key) {
            Some(index) => {
                items.remove(index);
            }
            None => items.push(key.clone()),
        }
        let lead = if items.contains(key) { Some(key.clone()) } else { items.last().cloned() };
        Selection { items, lead, anchor: Some(key.clone()) }
    } else {
        Selection::only(key.clone())
    }
}

/// The row of `link`: striped by position, with its thumbnail, page and status badge.
fn link_row<'r, K: TextureKey + std::fmt::Debug>(cx: &'r RowContext<'r, K>, link: &'r Link<K>, at: usize) -> ListRow<'r, K> {
    let status = status_of(link, cx.statuses);
    let words = status.map(|status| StatusBadge::words(status, cx.labels)).unwrap_or_default();
    let page = link.page.map(|n| cx.labels.page.replace("{n}", &n.to_string()));
    let mut name = vec![link.name.clone()];
    name.extend(page.clone());
    if !words.is_empty() {
        name.push(words.clone());
    }
    let mut row = ListRow::new(link.key.clone(), link.name.clone()).striped(at % 2 == 1).accessible_name(name.join(", "));
    if !words.is_empty() {
        row = row.tooltip(words);
    }
    if cx.side > 0.0 {
        row = with_thumbnail(row, cx, link);
    }
    let style = cx.style;
    row = row.trailing_item(style.badge, move |ui| {
        if let Some(status) = status {
            StatusBadge::paint(ui.painter(), ui.max_rect(), status, &style);
        }
    });
    if let Some(text) = page {
        let tokens = craft_ui::theme::current(&cx.ctx);
        let format = role_format(&tokens, TypeRole::Caption, style.page);
        let job = egui::text::LayoutJob::simple_singleline(text, format.font_id, style.page);
        let width = cx.ctx.fonts_mut(|fonts| fonts.layout_job(job.clone()).size().x).ceil();
        row = row.trailing_item(width, move |ui| {
            ui.add(Label::new(job).selectable(false));
        });
    }
    row
}

/// `row` with the link's picture from the cache, or the placeholder without a cache.
fn with_thumbnail<'r, K: TextureKey + std::fmt::Debug>(row: ListRow<'r, K>, cx: &'r RowContext<'r, K>, link: &Link<K>) -> ListRow<'r, K> {
    let side = cx.side;
    let request = ThumbRequest::for_cell(side, cx.ctx.pixels_per_point());
    let picture = cx.thumbnails.map(|(thumbnails, source)| (thumbnails, thumbnails.state(&cx.ctx, source, &link.key, &request, Priority::VISIBLE)));
    let mut row = row;
    if let Some((_, TextureState::Failed { message })) = &picture {
        row = row.tooltip(message.clone());
    }
    row.thumb_sized(vec2(side, side), move |painter, rect| {
        let drawn = picture.as_ref().is_some_and(|(thumbnails, state)| thumbnails.paint(painter, rect, state, &ThumbPaint::default()));
        if !drawn {
            let placeholder = Thumbnails::<K>::style(&craft_ui::theme::current(painter.ctx())).placeholder;
            painter.rect_filled(rect, 0.0, placeholder);
        }
    })
}

/// What the selected rows allow, over the shown links in their order.
struct Facts<K> {
    selected: Vec<K>,
    modified: Vec<K>,
    /// Linked and not missing: Edit Original and Reveal work on these.
    openable: Vec<K>,
    linked: Vec<K>,
    lead: Option<K>,
    any_missing: bool,
}

impl<K: TextureKey> Facts<K> {
    fn of(shown: &[&Link<K>], selection: &Selection<K>, links: &[Link<K>], statuses: &HashMap<K, LinkStatus>) -> Self {
        let mut facts = Facts {
            selected: Vec::new(),
            modified: Vec::new(),
            openable: Vec::new(),
            linked: Vec::new(),
            lead: None,
            any_missing: links.iter().any(|link| status_of(link, statuses).is_some_and(LinkStatus::is_missing)),
        };
        for link in shown.iter().filter(|link| selection.items.contains(&link.key)) {
            let status = status_of(link, statuses);
            facts.selected.push(link.key.clone());
            if status == Some(LinkStatus::Modified) {
                facts.modified.push(link.key.clone());
            }
            if !is_embedded(status) {
                facts.linked.push(link.key.clone());
                if !status.is_some_and(LinkStatus::is_missing) {
                    facts.openable.push(link.key.clone());
                }
            }
        }
        facts.lead = selection.lead.clone().filter(|lead| facts.selected.contains(lead)).or_else(|| facts.selected.first().cloned());
        facts
    }

    fn enabled(&self, command: Command) -> bool {
        match command {
            Command::Relink | Command::RelinkFolder => !self.selected.is_empty(),
            Command::FindMissing => self.any_missing,
            Command::Update => !self.modified.is_empty(),
            Command::EditOriginal => !self.openable.is_empty(),
            Command::BreakLink => !self.linked.is_empty(),
            Command::Reveal => self.selected.len() == 1 && self.openable.len() == 1,
            Command::GoTo => self.selected.len() == 1,
            Command::Filter(_) | Command::Sort(_) | Command::Thumbnails(_) => true,
        }
    }

    /// Runs `command`: an action reports its event, a choice changes `state`.
    fn run(&self, command: Command, state: &mut LinkListState, events: &mut Vec<LinkListEvent<K>>) {
        if !self.enabled(command) {
            return;
        }
        match command {
            Command::Relink => events.push(LinkListEvent::Relink(self.selected.clone())),
            Command::RelinkFolder => events.push(LinkListEvent::RelinkToFolder(self.selected.clone())),
            Command::FindMissing => events.push(LinkListEvent::FindMissing),
            Command::Update => events.push(LinkListEvent::Update(self.modified.clone())),
            Command::EditOriginal => events.extend(self.openable.first().cloned().map(LinkListEvent::EditOriginal)),
            Command::BreakLink => events.push(LinkListEvent::BreakLink(self.linked.clone())),
            Command::Reveal => events.extend(self.openable.first().cloned().map(LinkListEvent::Reveal)),
            Command::GoTo => events.extend(self.lead.clone().map(LinkListEvent::GoTo)),
            Command::Filter(filter) => state.filter = filter,
            Command::Sort(sort) => state.sort = sort,
            Command::Thumbnails(size) => state.thumbnails = size,
        }
    }
}

/// A command of the panel menu and the action row.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Command {
    Relink,
    RelinkFolder,
    FindMissing,
    Update,
    EditOriginal,
    BreakLink,
    Reveal,
    GoTo,
    Filter(LinkFilter),
    Sort(LinkSort),
    Thumbnails(ThumbSize),
}

/// The panel menu: the actions, then the filter, the sort and the thumbnail size as radio items.
fn menu_entries<K: TextureKey>(ui: &Ui, labels: &LinkListLabels, facts: &Facts<K>, state: &LinkListState) -> Vec<MenuEntry<Command>> {
    let action = |label: &str, command: Command| MenuItem::new(label, command).enabled(facts.enabled(command)).into();
    let choice = |label: &str, command: Command, chosen: bool| MenuItem::new(label, command).radio(chosen).into();
    let words = Words::of(ui);
    let mut entries = vec![
        action(&labels.relink, Command::Relink),
        action(&labels.relink_folder, Command::RelinkFolder),
        action(&labels.find_missing, Command::FindMissing),
        action(&labels.update, Command::Update),
        action(&labels.edit_original, Command::EditOriginal),
        action(&labels.break_link, Command::BreakLink),
        action(&labels.reveal, Command::Reveal),
        action(&labels.go_to, Command::GoTo),
        MenuEntry::Separator,
    ];
    for (label, filter) in [
        (&labels.show_all, LinkFilter::All),
        (&labels.show_missing, LinkFilter::Missing),
        (&labels.show_modified, LinkFilter::Modified),
        (&labels.show_embedded, LinkFilter::Embedded),
    ] {
        entries.push(choice(label, Command::Filter(filter), state.filter == filter));
    }
    entries.push(MenuEntry::Separator);
    for (label, sort) in [(&labels.sort_name, LinkSort::Name), (&labels.sort_kind, LinkSort::Kind), (&labels.sort_status, LinkSort::Status)] {
        entries.push(choice(label, Command::Sort(sort), state.sort == sort));
    }
    entries.push(MenuEntry::Separator);
    let sizes = ThumbSize::ALL.iter().map(|&size| choice(&size.label(&words), Command::Thumbnails(size), state.thumbnails == size)).collect();
    entries.push(MenuItem::submenu(labels.thumbnails.clone(), sizes).into());
    entries
}

/// The action row's buttons from the reading end, and the menu button.
fn footer_buttons<K: TextureKey>(
    ui: &mut Ui,
    labels: &LinkListLabels,
    facts: &Facts<K>,
    entries: &[MenuEntry<Command>],
    menu_open: bool,
    menu_slot: Id,
    commands: &mut Vec<Command>,
) {
    let buttons = [
        (Command::Relink, Icon::Link, &labels.relink),
        (Command::Update, Icon::Redo, &labels.update),
        (Command::EditOriginal, Icon::Pencil, &labels.edit_original),
        (Command::GoTo, Icon::Target, &labels.go_to),
        (Command::BreakLink, Icon::Unlink, &labels.break_link),
    ];
    for (command, icon, label) in buttons {
        let response = ui.add_enabled_ui(facts.enabled(command), |ui| PanelFooter::button(ui, icon, label)).inner;
        if response.clicked() {
            commands.push(command);
        }
    }
    let button = PanelFooter::menu_button(ui, Icon::Menu, &labels.menu, menu_open);
    let open_at = button.clicked().then_some(OpenAt::Rect(button.rect));
    let menu = ContextMenu::new(entries).own_gestures(false).open_at(open_at).show(ui, &button);
    commands.extend(menu.chosen);
    ui.ctx().data_mut(|d| d.insert_temp(menu_slot, menu.open));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selection(items: &[u8], anchor: u8) -> Selection<u8> {
        Selection { items: items.to_vec(), lead: items.last().copied(), anchor: Some(anchor) }
    }

    const ORDER: [u8; 5] = [0, 1, 2, 3, 4];

    // Shift ranges from the last click made without Shift; with Cmd it adds; a hidden anchor makes it a plain click.
    #[test]
    fn shift_ranges_from_the_anchor_and_a_hidden_anchor_is_a_plain_click() {
        let click = pick(&Selection::new(), &ORDER, &1, Modifiers::NONE);
        assert_eq!(click, Selection::only(1));
        let range = pick(&click, &ORDER, &3, Modifiers::SHIFT);
        assert_eq!((range.items.as_slice(), range.anchor), (&[1, 2, 3][..], Some(1)));
        let added = pick(&selection(&[0, 4], 4), &ORDER, &2, Modifiers { shift: true, command: true, ..Modifiers::NONE });
        assert_eq!(added.items, [0, 4, 2, 3]);
        let hidden = pick(&selection(&[9], 9), &ORDER, &2, Modifiers::SHIFT);
        assert_eq!(hidden, Selection::only(2));
    }
}
