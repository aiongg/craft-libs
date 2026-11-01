//! The version history panel and the preview banner (N5.06).

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use craft_theme::{ButtonVariant, TypeRole};
use craft_ui::behavior::{RowInfo, Selection, SelectionMode};
use craft_ui::controls::{Button, ControlSize, IconButton, TextField};
use craft_ui::edit::EditEvent;
use craft_ui::icons::Icon;
use craft_ui::list::{List, ListEvent, ListRow, RowEvent};
use craft_ui::localized::{Labels as _, fmt};
use craft_ui::menu::{MenuEntry, MenuItem, PressMenu};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, UiTokens, corner_radius};
use egui::{Align, AsIdSalt, Context, Frame, Id, IdSalt, Key, Layout, Margin, Modifiers, Painter, Rect, Response, Stroke, Ui, UiBuilder, pos2};

use super::ReviewStyle;
use super::comments::{age_text, text};
use crate::versions::{Branch, BranchId, BranchState, HistoryRow, Version, VersionId, history_rows};

craft_ui::words! {
    /// The panel's words, English by default (FigCraft's, Figma's).
    #[non_exhaustive]
    pub struct VersionsLabels {
        title: "versions.title" = "Version history";
        current: "versions.current" = "Current version";
        save: "versions.save" = "Save version";
        title_field: "versions.titleField" = "Title";
        description: "versions.description" = "Description";
        autosaves: "versions.autosaves" = "{n} autosave versions";
        restore: "versions.restore" = "Restore this version";
        rename: "versions.rename" = "Rename";
        duplicate: "versions.duplicate" = "Duplicate";
        delete: "versions.delete" = "Delete";
        delete_confirm: "versions.deleteConfirm" = "Delete this version?";
        branches: "versions.branches" = "Branches";
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
/// craft-i18n's `I18n::dates()`), autosave runs collapsed; per-row "…" with Restore, Rename,
/// Duplicate and Delete (an inline confirm). The history is the kernel's
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
        let mut action = None;
        let cx = Cx { tokens: &tokens, style: &style, labels: &labels, id, versions, previewing };
        let response = ui
            .vertical(|ui| {
                ui.spacing_mut().item_spacing.y = tokens.space.gap.stack;
                header(ui, &cx, &mut state, &mut action);
                branch_section(ui, &cx, branches, &mut state, &mut action);
                confirm_strip(ui, &cx, &mut state, &mut action);
                history(ui, &cx, &mut state, &mut action);
            })
            .response;
        ui.data_mut(|data| data.insert_temp(id, state));
        VersionHistoryPanelOutput { response, action }
    }
}

/// What the Version history panel did this pass.
#[must_use]
pub struct VersionHistoryPanelOutput {
    pub response: Response,
    pub action: Option<VersionAction>,
}

/// The banner over the canvas while a version is previewed: "You're viewing {title}", Restore
/// this version and Back to current version. Two Tab stops; Esc goes back.
pub struct VersionPreviewBanner<'a> {
    id_salt: IdSalt,
    version: &'a Version,
    labels: Option<VersionsLabels>,
}

impl<'a> VersionPreviewBanner<'a> {
    pub fn new(id_salt: impl AsIdSalt, version: &'a Version) -> Self {
        Self { id_salt: IdSalt::new(id_salt), version, labels: None }
    }

    /// Drawn at the top of `canvas`.
    // figcraft: crates/ui-egui/src/panels/versions.rs:523 `preview_banner`.
    pub fn show(self, ui: &mut Ui, canvas: Rect) -> Option<VersionAction> {
        let Self { id_salt, version, labels } = self;
        let labels = labels.unwrap_or_else(|| VersionsLabels::of(ui));
        let tokens = Tokens::of(ui);
        let id = ui.make_persistent_id(id_salt);
        let mut action = None;
        let name = version_name(ui.ctx(), version);
        let strip = Rect::from_min_max(canvas.min, pos2(canvas.max.x, canvas.max.y));
        let mut inner = ui.new_child(UiBuilder::new().max_rect(strip).layout(Layout::top_down(Align::Center)).id_salt(id));
        Frame::new()
            .fill(tokens.color.surface.chrome.to_egui())
            .corner_radius(corner_radius(tokens.shape.radius.control))
            .inner_margin(Margin::same(tokens.space.pad.control as i8))
            .show(&mut inner, |ui| {
                ui.horizontal(|ui| {
                    text(ui, &tokens, TypeRole::Body, tokens.color.text.normal.to_egui(), fmt(&labels.previewing, &[("title", &name)]));
                    if ui.add(Button::new(&labels.restore).variant(ButtonVariant::Primary).id_salt(id.with("restore"))).clicked() {
                        action = Some(VersionAction::Restore(version.id));
                    }
                    if ui.add(Button::new(&labels.go_back).id_salt(id.with("back"))).clicked() {
                        action = Some(VersionAction::Current);
                    }
                });
            });
        if action.is_none() && ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape)) {
            action = Some(VersionAction::Current);
        }
        action
    }
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
    /// A version to rename in place on the next pass.
    rename: Option<VersionId>,
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

/// What a row's "…" menu asks for.
#[derive(Clone, Copy)]
enum RowMenu {
    Restore,
    Rename,
    Duplicate,
    Delete,
}

fn header(ui: &mut Ui, cx: &Cx, state: &mut PanelState, action: &mut Option<VersionAction>) {
    let (tokens, labels) = (cx.tokens, cx.labels);
    ui.horizontal(|ui| {
        text(ui, tokens, TypeRole::Title, tokens.color.text.normal.to_egui(), &labels.title);
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
        *action = Some(VersionAction::Save { title, description });
    } else if cancel {
        state.form = None;
    }
}

fn branch_section(ui: &mut Ui, cx: &Cx, branches: &[Branch], state: &mut PanelState, action: &mut Option<VersionAction>) {
    let (tokens, labels) = (cx.tokens, cx.labels);
    text(ui, tokens, TypeRole::Caption, tokens.color.text.muted.to_egui(), &labels.branches);
    for branch in branches {
        ui.horizontal(|ui| {
            let color = if branch.state == BranchState::Active { tokens.color.text.normal } else { tokens.color.text.muted };
            text(ui, tokens, TypeRole::Body, color.to_egui(), &branch.name);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let salt = cx.id.with(("branch", branch.id));
                if branch.state == BranchState::Active {
                    if ui.add(Button::new(&labels.archive).id_salt(salt.with("archive"))).clicked() {
                        *action = Some(VersionAction::ArchiveBranch(branch.id));
                    }
                    if ui.add(Button::new(&labels.review_merge).id_salt(salt.with("merge"))).clicked() {
                        *action = Some(VersionAction::MergeBranch(branch.id));
                    }
                }
                if ui.add(Button::new(&labels.open).id_salt(salt.with("open"))).clicked() {
                    *action = Some(VersionAction::OpenBranch(branch.id));
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
    let field = TextField::new(cx.id.with("branch-name"), &labels.branches, name.as_str()).placeholder(&labels.branches).show(ui);
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
        *action = Some(VersionAction::CreateBranch { name: name.trim().to_owned() });
    } else if cancel {
        state.branch = None;
    }
}

/// "Delete this version?" with Delete and Cancel, above the list.
fn confirm_strip(ui: &mut Ui, cx: &Cx, state: &mut PanelState, action: &mut Option<VersionAction>) {
    let Some(pending) = state.confirm else { return };
    let (tokens, labels) = (cx.tokens, cx.labels);
    let name = cx.versions.iter().find(|v| v.id == pending).map_or_else(String::new, |v| version_name(ui.ctx(), v));
    text(ui, tokens, TypeRole::Body, tokens.color.text.normal.to_egui(), format!("{} {name}", labels.delete_confirm));
    ui.horizontal(|ui| {
        if ui.add(Button::new(&labels.delete).variant(ButtonVariant::Primary).id_salt(cx.id.with("confirm-delete"))).clicked() {
            state.confirm = None;
            *action = Some(VersionAction::Delete(pending));
        }
        if ui.add(Button::new(&labels.cancel).id_salt(cx.id.with("confirm-cancel"))).clicked() {
            state.confirm = None;
        }
    });
}

/// A dot on the timeline line: filled for the version shown.
fn paint_dot(painter: &Painter, rect: Rect, tokens: &UiTokens, filled: bool) {
    let line = Stroke::new(craft_theme::Stroke::W1.width(painter.ctx().pixels_per_point()), tokens.color.line.divider.to_egui());
    painter.line_segment([rect.center_top(), rect.center_bottom()], line);
    let color = tokens.color.accent.solid.to_egui();
    let radius = rect.width() / 4.0;
    if filled {
        painter.circle_filled(rect.center(), radius, color);
    } else {
        painter.circle(rect.center(), radius, tokens.color.surface.panel.to_egui(), Stroke::new(line.width, color));
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

fn history(ui: &mut Ui, cx: &Cx, state: &mut PanelState, action: &mut Option<VersionAction>) {
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
    let menu = RefCell::new(None::<(VersionId, RowMenu)>);
    let button = IconButton::style(tokens, ControlSize::Sm).side;
    let row_style = ListRow::<RowKey>::style(tokens);
    // The dot sits `review.version_dot_x` in from the row's edge; the row's own gap puts the text
    // at `review.version_text_x`.
    let indent = (cx.style.version_dot_x - row_style.pad_x - row_style.icon_size / 2.0).max(0.0);
    let mut selection = std::mem::take(&mut state.selection);
    let mut list = List::new(cx.id.with("history"), &labels.title, order.as_slice())
        .selection(&mut selection)
        .mode(SelectionMode::Single)
        .tree(true)
        .detail_lines(true)
        .renamable(true)
        .can_rename(|key| matches!(key, RowKey::Version(id) if cx.versions.iter().any(|v| v.id == *id && !v.autosave)));
    if let Some(id) = state.rename.take() {
        list = list.start_rename(RowKey::Version(id));
    }
    let out = list.show(ui, |_, key| {
        let current = cx.previewing.is_none();
        match key {
            RowKey::Current => {
                ListRow::new(key.clone(), &labels.current).indent(indent).glyph(move |painter, rect| paint_dot(painter, rect, cx.tokens, current))
            }
            RowKey::Group(head) => {
                let n = sizes.get(head).copied().unwrap_or_default();
                ListRow::new(key.clone(), fmt(&labels.autosaves, &[("n", &n.to_string())]))
                    .indent(indent)
                    .glyph(move |painter, rect| paint_dot(painter, rect, cx.tokens, false))
            }
            RowKey::Version(id) => {
                let id = *id;
                let Some(version) = cx.versions.iter().find(|v| v.id == id) else { return ListRow::new(key.clone(), String::new()) };
                let shown = cx.previewing == Some(id);
                let date = age_text(&ctx, version.created);
                let detail = if version.author.is_empty() { date.clone() } else { format!("{date} \u{b7} {}", version.author) };
                let label = if version.title.trim().is_empty() { date } else { version.title.clone() };
                let menu = &menu;
                ListRow::new(key.clone(), label)
                    .indent(indent)
                    .detail(detail)
                    .glyph(move |painter, rect| paint_dot(painter, rect, cx.tokens, shown))
                    .trailing_item(button, move |ui| {
                        let entries = row_menu(labels, version.autosave);
                        let more = ui.add(IconButton::new(Icon::More, "\u{2026}").id_salt(("more", id.0)));
                        if let Some(chosen) = PressMenu::new(&entries).show(ui, &more).chosen {
                            *menu.borrow_mut() = Some((id, chosen));
                        }
                    })
            }
        }
    });
    state.selection = selection;
    for event in out.events {
        match event {
            ListEvent::Clicked(RowKey::Version(id)) if cx.previewing != Some(id) => *action = Some(VersionAction::Preview(id)),
            ListEvent::Activate(RowKey::Version(id)) => *action = Some(VersionAction::Preview(id)),
            ListEvent::Clicked(RowKey::Current) | ListEvent::Activate(RowKey::Current) if cx.previewing.is_some() => {
                *action = Some(VersionAction::Current);
            }
            ListEvent::Activate(RowKey::Group(head)) => toggle(&mut state.open, head),
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
                    *action = Some(VersionAction::Rename { id, title: title.to_owned() });
                }
            }
            _ => {}
        }
    }
    if let Some((id, chosen)) = menu.into_inner() {
        match chosen {
            RowMenu::Restore => *action = Some(VersionAction::Restore(id)),
            RowMenu::Rename => state.rename = Some(id),
            RowMenu::Duplicate => *action = Some(VersionAction::Duplicate(id)),
            RowMenu::Delete => state.confirm = Some(id),
        }
    }
}

fn toggle(open: &mut HashSet<VersionId>, head: VersionId) {
    if !open.remove(&head) {
        open.insert(head);
    }
}

fn row_menu(labels: &VersionsLabels, autosave: bool) -> Vec<MenuEntry<RowMenu>> {
    let mut entries = vec![MenuItem::new(labels.restore.clone(), RowMenu::Restore).into()];
    if !autosave {
        entries.push(MenuItem::new(labels.rename.clone(), RowMenu::Rename).into());
    }
    entries.push(MenuItem::new(labels.duplicate.clone(), RowMenu::Duplicate).into());
    entries.push(MenuItem::new(labels.delete.clone(), RowMenu::Delete).into());
    entries
}
