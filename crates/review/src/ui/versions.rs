//! The version history panel and the preview banner (N5.06).

use std::collections::{HashMap, HashSet};

use craft_theme::ButtonVariant;
use craft_ui::behavior::{ClaimedKey, RowInfo, Selection, SelectionMode};
use craft_ui::controls::{Button, IconButton, Link, TextField};
use craft_ui::edit::EditEvent;
use craft_ui::focus::key_owner::{KeyOwner, OwnerGate, owner};
use craft_ui::icons::Icon;
use craft_ui::list::{List, ListEvent, ListRow, ListRowStyle, RowCommand, RowEvent};
use craft_ui::localized::{Labels as _, Words, fmt};
use craft_ui::menu::{MenuEntry, MenuItem, PressMenu};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, UiTokens, corner_radius};
use egui::{Align, AsIdSalt, Context, Frame, Id, IdSalt, Key, Layout, Margin, Modifiers, Painter, Rect, Response, Stroke, Ui, UiBuilder, pos2};

use super::ReviewStyle;
use super::comments::{age_text, text};
use crate::versions::{Branch, BranchId, BranchState, HistoryRow, Version, VersionId, history_rows};

/// The panel's count words, for the facade's `ui::COUNTS`.
pub mod counts {
    use craft_ui::localized::words::Counted;

    pub(super) const AUTOSAVES: Counted = Counted::new("versions.autosaves", "{n} autosave version", "{n} autosave versions");

    /// Every count word of the Version history panel.
    pub const ALL: &[Counted] = &[AUTOSAVES];
}

craft_ui::words! {
    /// The panel's words, English by default (FigCraft's, Figma's).
    #[non_exhaustive]
    pub struct VersionsLabels {
        title: "versions.title" = "Version history";
        current: "versions.current" = "Current version";
        save: "versions.save" = "Save version";
        title_field: "versions.titleField" = "Title";
        description: "versions.description" = "Description";
        /// The folded autosave run; the English plural of [`counts::AUTOSAVES`].
        autosaves: "versions.autosaves" = "{n} autosave versions";
        /// The date and the author under a version's title.
        detail: "versions.detail" = "{date} \u{b7} {author}";
        no_versions: "versions.noVersions" = "No saved versions yet. Save a version to mark a point in your work you can come back to.";
        restore: "versions.restore" = "Restore this version";
        rename: "versions.rename" = "Rename";
        duplicate: "versions.duplicate" = "Duplicate";
        delete: "versions.delete" = "Delete";
        delete_confirm: "versions.deleteConfirm" = "Delete \u{201c}{title}\u{201d}? This can't be undone.";
        branches: "versions.branches" = "Branches";
        branch_name: "versions.branchName" = "Branch name";
        no_branches: "versions.noBranches" = "No branches. A branch is a copy of the file to try changes in, then review and merge.";
        branch_active: "versions.branchActive" = "Active";
        branch_merged: "versions.branchMerged" = "Merged";
        branch_archived: "versions.branchArchived" = "Archived";
        /// The state and the date under a branch's name.
        branch_detail: "versions.branchDetail" = "{state} \u{b7} {date}";
        /// The name of a branch's "\u{2026}" button; `{branch}` is the branch's name.
        branch_more: "versions.branchMore" = "More ({branch})";
        new_branch: "versions.newBranch" = "Create branch";
        open: "versions.open" = "Open";
        review_merge: "versions.reviewMerge" = "Review and merge";
        archive: "versions.archive" = "Archive";
        previewing: "versions.previewing" = "You're viewing {title}";
        go_back: "versions.goBack" = "Back to current version";
        cancel: "versions.cancel" = "Cancel";
    }
}

/// What the panel asks of the app.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum VersionAction {
    Save {
        title: String,
        description: String,
    },
    /// Show a version on the canvas, read-only.
    Preview(VersionId),
    /// Back to the current version.
    Current,
    Restore(VersionId),
    Rename {
        id: VersionId,
        title: String,
    },
    Duplicate(VersionId),
    Delete(VersionId),
    CreateBranch {
        name: String,
    },
    OpenBranch(BranchId),
    MergeBranch(BranchId),
    ArchiveBranch(BranchId),
}

/// Figma's Version history panel in the left sidebar (FigCraft): the branches above (create,
/// open, review and merge, archive), then Current version, then the history newest first
/// ([`history_rows`](crate::versions::history_rows)) on a timeline line with a dot per row (the
/// current one filled), each named version with its title, date and author (relative under a
/// week with [`relative_age`](craft_ui::localized::relative_age), then the date through
/// craft-i18n's `I18n::dates()`), runs of two or more autosaves collapsed (a lone autosave is a row);
/// per-row "…" (the list's own [`row_menu`](craft_ui::list::List::row_menu)) with Restore, Rename,
/// Duplicate and Delete (a confirm strip above the list). A branch shows its state and date, with
/// a "…" menu of Open, Review and merge, and Archive while active. The history is the kernel's
/// [`List`](craft_ui::list::List) of [`ListRow`](craft_ui::list::ListRow)s (title and the date and
/// author as the detail line; the dot column painted in the row's leading slot at
/// `review.version_dot_x`, text from `review.version_text_x`): one Tab stop, Up / Down move,
/// Enter previews, F2 or Rename renames in place (`List::start_rename`), autosave groups are
/// tree rows that open and close. Save version opens an inline form (Title, Description; Enter
/// saves, Esc cancels). The branch and footer buttons are Tab stops.
pub struct VersionHistoryPanel<'a> {
    id_salt: IdSalt,
    versions: &'a [Version],
    branches: &'a [Branch],
    previewing: Option<VersionId>,
    labels: Option<VersionsLabels>,
}

impl<'a> VersionHistoryPanel<'a> {
    pub fn new(id_salt: impl AsIdSalt, versions: &'a [Version], branches: &'a [Branch]) -> Self {
        Self { id_salt: IdSalt::new(id_salt), versions, branches, previewing: None, labels: None }
    }

    pub fn previewing(mut self, version: Option<VersionId>) -> Self {
        self.previewing = version;
        self
    }

    pub fn labels(mut self, labels: VersionsLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // figcraft: crates/ui-egui/src/panels/versions.rs:111 `panel`, `:317` `form`, `:374`
    // `timeline`, `:393` `current_row`, `:408` `version_row`, `:471` `branch_row`, `:557`
    // `autosave_group`, `:582` `confirm_row`.
    pub fn show(self, ui: &mut Ui) -> VersionHistoryPanelOutput {
        let Self { id_salt, versions, branches, previewing, labels } = self;
        let labels = labels.unwrap_or_else(|| VersionsLabels::of(ui));
        let tokens = Tokens::of(ui);
        let style = ReviewStyle::of(&tokens);
        let id = ui.make_persistent_id(id_salt);
        let mut state: PanelState = ui.data(|data| data.get_temp(id)).unwrap_or_default();
        let mut actions = Vec::new();
        let cx = Cx { tokens: &tokens, style: &style, labels: &labels, id, versions, previewing };
        let response = ui
            .vertical(|ui| {
                ui.spacing_mut().item_spacing.y = tokens.space.gap.stack;
                header(ui, &cx, &mut state, &mut actions);
                branch_section(ui, &cx, branches, &mut state, &mut actions);
                confirm_strip(ui, &cx, &mut state, &mut actions);
                history(ui, &cx, &mut state, &mut actions);
            })
            .response;
        ui.data_mut(|data| data.insert_temp(id, state));
        VersionHistoryPanelOutput { response, actions }
    }
}

/// What the Version history panel did this pass.
#[must_use]
pub struct VersionHistoryPanelOutput {
    pub response: Response,
    /// In the order they happened this pass.
    pub actions: Vec<VersionAction>,
}

/// The banner over the canvas while a version is previewed: "You're viewing {title}" with its
/// date, Restore this version and Back to current version. Two Tab stops. Esc goes back while the
/// app or the canvas owns the keys ([`focus::key_owner`](craft_ui::focus::key_owner)), so an Esc
/// meant for a text field, a menu or a canvas tool is theirs.
pub struct VersionPreviewBanner<'a> {
    id_salt: IdSalt,
    version: &'a Version,
    labels: Option<VersionsLabels>,
}

impl<'a> VersionPreviewBanner<'a> {
    pub fn new(id_salt: impl AsIdSalt, version: &'a Version) -> Self {
        Self { id_salt: IdSalt::new(id_salt), version, labels: None }
    }

    pub fn labels(mut self, labels: VersionsLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Drawn at the top of `canvas`.
    // figcraft: crates/ui-egui/src/panels/versions.rs:523 `preview_banner`.
    pub fn show(self, ui: &mut Ui, canvas: Rect) -> VersionPreviewBannerOutput {
        let Self { id_salt, version, labels } = self;
        let labels = labels.unwrap_or_else(|| VersionsLabels::of(ui));
        let tokens = Tokens::of(ui);
        let style = ReviewStyle::of(&tokens);
        let id = ui.make_persistent_id(id_salt);
        let mut action = None;
        let name = version_name(ui.ctx(), version);
        let mut inner = ui.new_child(UiBuilder::new().max_rect(canvas).layout(Layout::top_down(Align::Center)).id_salt(id));
        let response = Frame::new()
            .fill(tokens.color.surface.chrome.to_egui())
            .corner_radius(corner_radius(tokens.shape.radius.control))
            .inner_margin(Margin::same(tokens.space.pad.control as i8))
            .show(&mut inner, |ui| {
                ui.horizontal(|ui| {
                    text(ui, &tokens, style.strong_text, tokens.color.text.normal.to_egui(), fmt(&labels.previewing, &[("title", &name)]));
                    text(ui, &tokens, style.body_text, tokens.color.text.muted.to_egui(), age_text(ui.ctx(), version.created));
                    if ui.add(Button::new(&labels.restore).variant(ButtonVariant::Primary).id_salt(id.with("restore"))).clicked() {
                        action = Some(VersionAction::Restore(version.id));
                    }
                    if ui.add(Button::new(&labels.go_back).id_salt(id.with("back"))).clicked() {
                        action = Some(VersionAction::Current);
                    }
                });
            })
            .response;
        if action.is_none()
            && owner(ui.ctx(), &OwnerGate::default()) == KeyOwner::App
            && ui.input_mut(|input| ClaimedKey::exact(Modifiers::NONE, Key::Escape).consume(input))
        {
            action = Some(VersionAction::Current);
        }
        VersionPreviewBannerOutput { response, action }
    }
}

/// What the preview banner did this pass.
#[must_use]
pub struct VersionPreviewBannerOutput {
    pub response: Response,
    pub action: Option<VersionAction>,
}

/// The title of `version`, or its date for an autosave.
fn version_name(ctx: &Context, version: &Version) -> String {
    if version.title.trim().is_empty() { age_text(ctx, version.created) } else { version.title.clone() }
}

/// What the panel keeps between passes.
#[derive(Clone, Default)]
struct PanelState {
    selection: Selection<RowKey>,
    /// Autosave groups the user opened, by their newest version.
    open: HashSet<VersionId>,
    /// The Save version form: title and description.
    form: Option<(String, String)>,
    /// The Create branch form: the name.
    branch: Option<String>,
    /// The version whose Delete awaits its confirmation.
    confirm: Option<VersionId>,
}

/// A row of the history list.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum RowKey {
    Current,
    Version(VersionId),
    /// A run of autosaves, by its newest version.
    Group(VersionId),
}

/// What every part of the panel reads.
struct Cx<'a> {
    tokens: &'a UiTokens,
    style: &'a ReviewStyle,
    labels: &'a VersionsLabels,
    id: Id,
    versions: &'a [Version],
    previewing: Option<VersionId>,
}

/// The branch "\u{2026}" menu's entries.
#[derive(Clone, Copy)]
enum BranchMenu {
    Open,
    Merge,
    Archive,
}

/// The history row menu's entries other than Rename, numbered for [`RowCommand::Other`].
#[derive(Clone, Copy)]
enum RowMenu {
    Restore,
    Duplicate,
    Delete,
}

impl RowMenu {
    const ALL: [RowMenu; 3] = [RowMenu::Restore, RowMenu::Duplicate, RowMenu::Delete];

    fn command(self) -> RowCommand {
        RowCommand::Other(self as usize)
    }

    fn of(command: RowCommand) -> Option<RowMenu> {
        match command {
            RowCommand::Other(n) => Self::ALL.get(n).copied(),
            _ => None,
        }
    }
}

fn header(ui: &mut Ui, cx: &Cx, state: &mut PanelState, actions: &mut Vec<VersionAction>) {
    let (tokens, labels) = (cx.tokens, cx.labels);
    ui.horizontal(|ui| {
        text(ui, tokens, ReviewStyle::of(tokens).title_text, tokens.color.text.normal.to_egui(), &labels.title);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if state.form.is_none() && ui.add(Button::new(&labels.save).id_salt(cx.id.with("save"))).clicked() {
                state.form = Some((String::new(), String::new()));
            }
        });
    });
    let Some((title, description)) = state.form.as_mut() else { return };
    let (mut save, mut cancel) = (false, false);
    let field = TextField::new(cx.id.with("form-title"), &labels.title_field, title.as_str()).placeholder(&labels.title_field).show(ui);
    if let Some(value) = field.event.as_ref().and_then(EditEvent::value) {
        title.clone_from(value);
    }
    save |= field.submitted;
    cancel |= field.escaped;
    let field = TextField::new(cx.id.with("form-description"), &labels.description, description.as_str()).placeholder(&labels.description).show(ui);
    if let Some(value) = field.event.as_ref().and_then(EditEvent::value) {
        description.clone_from(value);
    }
    save |= field.submitted;
    cancel |= field.escaped;
    ui.horizontal(|ui| {
        save |= ui.add(Button::new(&labels.save).variant(ButtonVariant::Primary).id_salt(cx.id.with("form-save"))).clicked();
        cancel |= ui.add(Button::new(&labels.cancel).id_salt(cx.id.with("form-cancel"))).clicked();
    });
    if save {
        let (title, description) = state.form.take().unwrap_or_default();
        actions.push(VersionAction::Save { title, description });
    } else if cancel {
        state.form = None;
    }
}

fn branch_section(ui: &mut Ui, cx: &Cx, branches: &[Branch], state: &mut PanelState, actions: &mut Vec<VersionAction>) {
    let (tokens, labels) = (cx.tokens, cx.labels);
    text(ui, tokens, ReviewStyle::of(tokens).caption_text, tokens.color.text.muted.to_egui(), &labels.branches);
    if branches.is_empty() {
        text(ui, tokens, cx.style.caption_text, tokens.color.text.muted.to_egui(), &labels.no_branches);
    }
    for branch in branches {
        ui.horizontal(|ui| {
            let salt = cx.id.with(("branch", branch.id));
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                if ui.add(Link::new(&branch.name).id_salt(salt.with("open"))).clicked() {
                    actions.push(VersionAction::OpenBranch(branch.id));
                }
                let state_word = match branch.state {
                    BranchState::Active => &labels.branch_active,
                    BranchState::Merged => &labels.branch_merged,
                    BranchState::Archived => &labels.branch_archived,
                };
                let date = age_text(ui.ctx(), branch.created);
                let detail = fmt(&labels.branch_detail, &[("state", state_word), ("date", &date)]);
                text(ui, tokens, cx.style.caption_text, tokens.color.text.muted.to_egui(), detail);
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let mut entries = vec![
                    MenuItem::new(labels.open.clone(), BranchMenu::Open).into(),
                    MenuItem::new(labels.review_merge.clone(), BranchMenu::Merge).into(),
                ];
                if branch.state == BranchState::Active {
                    entries.push(MenuItem::new(labels.archive.clone(), BranchMenu::Archive).into());
                }
                let name = fmt(&labels.branch_more, &[("branch", &branch.name)]);
                let more = ui.add(IconButton::new(Icon::More, name).id_salt(salt.with("more")));
                match PressMenu::new(&entries).show(ui, &more).chosen {
                    Some(BranchMenu::Open) => actions.push(VersionAction::OpenBranch(branch.id)),
                    Some(BranchMenu::Merge) => actions.push(VersionAction::MergeBranch(branch.id)),
                    Some(BranchMenu::Archive) => actions.push(VersionAction::ArchiveBranch(branch.id)),
                    None => {}
                }
            });
        });
    }
    let Some(name) = state.branch.as_mut() else {
        if ui.add(Button::new(&labels.new_branch).id_salt(cx.id.with("new-branch"))).clicked() {
            state.branch = Some(String::new());
        }
        return;
    };
    let (mut create, mut cancel) = (false, false);
    let field = TextField::new(cx.id.with("branch-name"), &labels.branch_name, name.as_str()).placeholder(&labels.branch_name).show(ui);
    if let Some(value) = field.event.as_ref().and_then(EditEvent::value) {
        name.clone_from(value);
    }
    create |= field.submitted;
    cancel |= field.escaped;
    let named = !name.trim().is_empty();
    ui.horizontal(|ui| {
        create |=
            ui.add_enabled(named, Button::new(&labels.new_branch).variant(ButtonVariant::Primary).id_salt(cx.id.with("branch-create"))).clicked();
        cancel |= ui.add(Button::new(&labels.cancel).id_salt(cx.id.with("branch-cancel"))).clicked();
    });
    if create && named {
        let name = state.branch.take().unwrap_or_default();
        actions.push(VersionAction::CreateBranch { name: name.trim().to_owned() });
    } else if cancel {
        state.branch = None;
    }
}

/// "Delete this version?" with Delete and Cancel, above the list.
fn confirm_strip(ui: &mut Ui, cx: &Cx, state: &mut PanelState, actions: &mut Vec<VersionAction>) {
    let Some(pending) = state.confirm else { return };
    let (tokens, labels) = (cx.tokens, cx.labels);
    let name = cx.versions.iter().find(|v| v.id == pending).map_or_else(String::new, |v| version_name(ui.ctx(), v));
    text(ui, tokens, ReviewStyle::of(tokens).body_text, tokens.color.text.normal.to_egui(), fmt(&labels.delete_confirm, &[("title", &name)]));
    ui.horizontal(|ui| {
        if ui.add(Button::new(&labels.delete).variant(ButtonVariant::Primary).id_salt(cx.id.with("confirm-delete"))).clicked() {
            state.confirm = None;
            actions.push(VersionAction::Delete(pending));
        }
        if ui.add(Button::new(&labels.cancel).id_salt(cx.id.with("confirm-cancel"))).clicked() {
            state.confirm = None;
        }
    });
}

/// The timeline's stroke (FigCraft's `border_strong`, `versions.rs:376`).
fn timeline_stroke(painter: &Painter, tokens: &UiTokens) -> Stroke {
    Stroke::new(craft_theme::Stroke::W1.width(painter.ctx().pixels_per_point()), tokens.color.line.border.to_egui())
}

/// A dot on the timeline line, centred in `rect`: filled for the version shown, else hollow
/// (FigCraft `versions.rs:386`, `:388`).
fn paint_dot(painter: &Painter, rect: Rect, tokens: &UiTokens, style: &ReviewStyle, filled: bool) {
    let line = timeline_stroke(painter, tokens);
    let color = tokens.color.accent.solid.to_egui();
    if filled {
        painter.circle_filled(rect.center(), style.version_dot_radius, color);
    } else {
        painter.circle(rect.center(), style.version_dot_hollow_radius, tokens.color.surface.panel.to_egui(), Stroke::new(line.width, color));
    }
}

/// The radius of the dot of a row, which the timeline line stops at.
fn dot_radius(style: &ReviewStyle, filled: bool) -> f32 {
    if filled { style.version_dot_radius } else { style.version_dot_hollow_radius }
}

/// Whether the dot of `key` is the filled one: the version the canvas shows.
fn dot_filled(cx: &Cx, key: &RowKey) -> bool {
    match key {
        RowKey::Current => cx.previewing.is_none(),
        RowKey::Version(id) => cx.previewing == Some(*id),
        RowKey::Group(_) => false,
    }
}

/// The timeline line between the dots, drawn over the rows after the list: the glyph slot is
/// painted clipped to its own rect, so the line between two rows cannot be part of it. It runs
/// from the first row's dot to the last row's.
fn paint_timeline(ui: &Ui, cx: &Cx, rows: &[(RowKey, Rect)], clip: Rect, (first, last): (&RowKey, &RowKey)) {
    let painter = ui.painter().with_clip_rect(clip);
    let stroke = timeline_stroke(&painter, cx.tokens);
    for (key, rect) in rows {
        let x = rect.min.x + cx.style.version_dot_x;
        let radius = dot_radius(cx.style, dot_filled(cx, key));
        let y = rect.center().y;
        if key != first {
            painter.line_segment([pos2(x, rect.min.y), pos2(x, y - radius)], stroke);
        }
        if key != last {
            painter.line_segment([pos2(x, y + radius), pos2(x, rect.max.y)], stroke);
        }
        if matches!(key, RowKey::Group(_)) {
            // The group's slot is empty: its dot belongs on the timeline, not beside the disclosure.
            paint_dot(&painter, Rect::from_center_size(pos2(x, y), egui::Vec2::splat(2.0 * radius)), cx.tokens, cx.style, false);
        }
    }
}

/// The display order of the history: Current version, then the rows newest first, an open
/// autosave group followed by its versions.
fn display_order(cx: &Cx, history: &[HistoryRow], state: &PanelState) -> Vec<RowInfo<RowKey>> {
    let mut order = vec![RowInfo::leaf(RowKey::Current)];
    for row in history {
        match row {
            HistoryRow::Version(at) => order.push(RowInfo::leaf(RowKey::Version(cx.versions[*at].id))),
            HistoryRow::Autosaves(members) => {
                let head = cx.versions[members[0]].id;
                let open = group_open(cx, members, state);
                order.push(RowInfo::group(RowKey::Group(head), open));
                if open {
                    order.extend(members.iter().map(|&at| RowInfo::leaf(RowKey::Version(cx.versions[at].id)).at_depth(1)));
                }
            }
        }
    }
    order
}

/// A group is open when the user opened it or one of its versions is shown.
fn group_open(cx: &Cx, members: &[usize], state: &PanelState) -> bool {
    state.open.contains(&cx.versions[members[0]].id) || members.iter().any(|&at| Some(cx.versions[at].id) == cx.previewing)
}

/// Where the history rows put the dot and the text.
pub(super) struct Columns {
    /// The glyph slot's width.
    pub(super) slot: f32,
    /// The indent of a row without a disclosure column.
    indent: f32,
}

impl Columns {
    /// The slot is as wide as puts the label at `review.version_text_x` with the dot centred at
    /// `review.version_dot_x` (FigCraft's `DOT_X` and `TEXT_X`, `versions.rs:109`).
    pub(super) fn new(style: &ReviewStyle, row: &ListRowStyle) -> Self {
        let slot = (2.0 * (style.version_text_x - style.version_dot_x - row.gap)).max(0.0);
        let indent = (style.version_dot_x - row.pad_x - slot / 2.0).max(0.0);
        Columns { slot, indent }
    }

    /// A group's disclosure column comes before the slot (the kernel has no way to put the
    /// glyph first), so a group takes no indent and its dot is drawn with the timeline.
    pub(super) fn indent(&self, group: bool) -> f32 {
        if group { 0.0 } else { self.indent }
    }
}

fn history(ui: &mut Ui, cx: &Cx, state: &mut PanelState, actions: &mut Vec<VersionAction>) {
    let (tokens, labels) = (cx.tokens, cx.labels);
    let history = history_rows(cx.versions);
    let order = display_order(cx, &history, state);
    let sizes: HashMap<VersionId, usize> = history
        .iter()
        .filter_map(|row| match row {
            HistoryRow::Autosaves(members) => Some((cx.versions[members[0]].id, members.len())),
            HistoryRow::Version(_) => None,
        })
        .collect();
    let ctx = ui.ctx().clone();
    let words = Words::of(ui);
    let row_style = ListRow::<RowKey>::style(tokens);
    let columns = Columns::new(cx.style, &row_style);
    let (slot, indent) = (columns.slot, |group: bool| columns.indent(group));
    let mut selection = std::mem::take(&mut state.selection);
    let list = List::new(cx.id.with("history"), &labels.title, order.as_slice())
        .selection(&mut selection)
        .mode(SelectionMode::Single)
        .tree(true)
        .detail_lines(true)
        .renamable(true)
        .can_rename(|key| matches!(key, RowKey::Version(id) if cx.versions.iter().any(|v| v.id == *id && !v.autosave)))
        .row_menu(|key| match key {
            RowKey::Version(id) => {
                let autosave = cx.versions.iter().any(|v| v.id == *id && v.autosave);
                row_menu(labels, autosave)
            }
            RowKey::Current | RowKey::Group(_) => Vec::new(),
        });
    let out = list.show(ui, |_, key| {
        let filled = dot_filled(cx, key);
        let dot = move |painter: &Painter, rect: Rect| paint_dot(painter, rect, cx.tokens, cx.style, filled);
        match key {
            RowKey::Current => ListRow::new(key.clone(), &labels.current).indent(indent(false)).glyph_sized(slot, dot),
            RowKey::Group(head) => {
                let n = sizes.get(head).copied().unwrap_or_default();
                let text = words.counted(&ctx, counts::AUTOSAVES, &labels.autosaves, u64::try_from(n).unwrap_or(u64::MAX));
                ListRow::new(key.clone(), text).indent(indent(true)).glyph_sized(slot, |_, _| {})
            }
            RowKey::Version(id) => {
                let Some(version) = cx.versions.iter().find(|v| v.id == *id) else { return ListRow::new(key.clone(), String::new()) };
                let date = age_text(&ctx, version.created);
                let detail =
                    if version.author.is_empty() { date.clone() } else { fmt(&labels.detail, &[("date", &date), ("author", &version.author)]) };
                let label = if version.title.trim().is_empty() { date } else { version.title.clone() };
                // A version inside an open group sits in the same dot column as the others.
                ListRow::new(key.clone(), label).indent(indent(false)).paint_depth(0).detail(detail).glyph_sized(slot, dot)
            }
        }
    });
    state.selection = selection;
    if let (Some(first), Some(last)) = (order.first(), order.last()) {
        paint_timeline(ui, cx, &out.rows, out.response.rect, (&first.key, &last.key));
    }
    if order.len() == 1 && cx.versions.is_empty() {
        text(ui, tokens, cx.style.caption_text, tokens.color.text.muted.to_egui(), &labels.no_versions);
    }
    for event in out.events {
        match event {
            ListEvent::Clicked(RowKey::Version(id)) if cx.previewing != Some(id) => actions.push(VersionAction::Preview(id)),
            ListEvent::Activate(RowKey::Version(id)) => actions.push(VersionAction::Preview(id)),
            ListEvent::Clicked(RowKey::Current) | ListEvent::Activate(RowKey::Current) if cx.previewing.is_some() => {
                actions.push(VersionAction::Current);
            }
            // A click folds or unfolds the group, as in FigCraft (`versions.rs:557`).
            ListEvent::Clicked(RowKey::Group(head)) | ListEvent::Activate(RowKey::Group(head)) => toggle(&mut state.open, head),
            ListEvent::Open(change) => {
                if let RowKey::Group(head) = change.row {
                    if change.open {
                        state.open.insert(head);
                    } else {
                        state.open.remove(&head);
                    }
                }
            }
            ListEvent::Row(RowEvent::Renamed { row: RowKey::Version(id), text }) => {
                let title = text.trim();
                if !title.is_empty() && cx.versions.iter().any(|v| v.id == id && v.title != title) {
                    actions.push(VersionAction::Rename { id, title: title.to_owned() });
                }
            }
            ListEvent::RowMenu { key: RowKey::Version(id), command } => match RowMenu::of(command) {
                Some(RowMenu::Restore) => actions.push(VersionAction::Restore(id)),
                Some(RowMenu::Duplicate) => actions.push(VersionAction::Duplicate(id)),
                Some(RowMenu::Delete) => state.confirm = Some(id),
                None => {}
            },
            _ => {}
        }
    }
}

fn toggle(open: &mut HashSet<VersionId>, head: VersionId) {
    if !open.remove(&head) {
        open.insert(head);
    }
}

fn row_menu(labels: &VersionsLabels, autosave: bool) -> Vec<MenuEntry<RowCommand>> {
    let mut entries = vec![MenuItem::new(labels.restore.clone(), RowMenu::Restore.command()).into()];
    if !autosave {
        entries.push(MenuItem::new(labels.rename.clone(), RowCommand::Rename).into());
    }
    entries.push(MenuItem::new(labels.duplicate.clone(), RowMenu::Duplicate.command()).into());
    entries.push(MenuItem::new(labels.delete.clone(), RowMenu::Delete.command()).into());
    entries
}
