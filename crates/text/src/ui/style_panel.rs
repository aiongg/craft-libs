//! [`TextStylesPanel`] (F14.03): the Character Styles and Paragraph Styles panels. A list of the
//! document's styles with the one at the type selection highlighted ("+" after its name when the
//! text overrides it), and Photoshop's footer: Clear Override and Redefine at the left, Style
//! Options, Duplicate, New and Delete at the right. The styles, their attributes and the
//! commands are the app's; the panel reports what the user asked for.

use craft_ui::behavior::{RowInfo, Selection};
use craft_ui::composites::{PanelFooter, StatusLine, Tone};
use craft_ui::controls::Kbd;
use craft_ui::docking::panel_menu::PanelCommand;
use craft_ui::icons::Icon;
use craft_ui::list::{List, ListEvent, ListRow};
use craft_ui::localized::{Labels as _, Words};
use craft_ui::menu::{MenuEntry, MenuItem};
use egui::{AsIdSalt, Context, Id, IdSalt, Key, KeyboardShortcut, Modifiers, PointerButton, Response, Ui, UiBuilder};

mod picker;
#[cfg(test)]
mod picker_tests;

pub use picker::{NamedStyleRow, StyleChip, StyleChipResponse, StylePickerLabels, StylePickerView, StylePreview};

craft_ui::words! {
    /// The panels' words, English by default (PhotoCraft's strings).
    #[non_exhaustive]
    pub struct TextStylesLabels {
        character_styles: "textStyles.characterStyles" = "Character Styles";
        paragraph_styles: "textStyles.paragraphStyles" = "Paragraph Styles";
        no_character_style: "textStyles.noCharacterStyle" = "No character style";
        row_tip: "textStyles.rowTip" = "Click to apply, {key}-click to clear overrides, double-click for Style Options";
        select_text: "textStyles.selectText" = "Select text to apply a character style";
        clear_override: "textStyles.clearOverride" = "Clear Override";
        redefine: "textStyles.redefine" = "Redefine style by current selection";
        options: "textStyles.options" = "Style Options…";
        duplicate: "textStyles.duplicate" = "Duplicate style";
        new_style: "textStyles.new" = "Create new style (from the selected type)";
        delete: "textStyles.delete" = "Delete style";
        no_document: "textStyles.noDocument" = "No document";
        select_unused: "textStyles.selectUnused" = "Select All Unused";
        overridden: "textStyles.overridden" = "overridden";
    }
}

/// Which panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextStyleKind {
    Character,
    Paragraph,
}

/// A style in the list. Id 0 is the base style: "None" for character styles (shown dimmed, never
/// deleted, redefined or edited) and "Basic Paragraph" for paragraph styles (editable, never
/// deleted), as PhotoCraft's engine lists them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextStyleRow {
    pub id: u64,
    pub name: String,
}

/// What the panel keeps for the app: the row the footer and the menu act on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TextStylesState {
    /// The row selected in the panel; until the user picks one, the footer acts on the style at
    /// the type target.
    pub selected: Option<u64>,
}

/// What the panel menu needs to know of the last pass, kept in the context under the panel's id
/// salt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MenuFacts {
    kind: TextStyleKind,
    target: bool,
    overridden: bool,
    current: Option<u64>,
}

const CMD_CLEAR_OVERRIDE: &str = "textStyles.clearOverride";
const CMD_REDEFINE: &str = "textStyles.redefine";
const CMD_NEW: &str = "textStyles.new";
const CMD_DUPLICATE: &str = "textStyles.duplicate";
const CMD_DELETE: &str = "textStyles.delete";
const CMD_OPTIONS: &str = "textStyles.options";

impl MenuFacts {
    fn of(ctx: &Context, id_salt: IdSalt) -> Option<MenuFacts> {
        ctx.data(|d| d.get_temp(Id::NULL.with(id_salt)))
    }

    /// The row the footer and the menu act on: the panel's selection, else the current style.
    fn pick(&self, state: &TextStylesState) -> Option<u64> {
        state.selected.or(self.current)
    }

    /// The action of a footer button or menu command, `None` when its condition doesn't hold.
    fn action(&self, state: &TextStylesState, command: &str) -> Option<TextStyleAction> {
        let paragraph = self.kind == TextStyleKind::Paragraph;
        let editable = self.pick(state).filter(|id| *id != 0 || paragraph);
        match command {
            CMD_CLEAR_OVERRIDE => (self.target && self.overridden).then_some(TextStyleAction::ClearOverride),
            CMD_REDEFINE => (self.target && self.current.is_some_and(|id| id != 0 || paragraph)).then_some(TextStyleAction::Redefine),
            CMD_NEW => Some(TextStyleAction::New { from_selection: self.target }),
            CMD_DUPLICATE => editable.map(TextStyleAction::Duplicate),
            CMD_DELETE => editable.filter(|id| *id != 0).map(TextStyleAction::Delete),
            CMD_OPTIONS => editable.map(TextStyleAction::Options),
            _ => None,
        }
    }
}

/// What the user asked for this pass. The app runs the command on its document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextStyleAction {
    /// A click on a row with a type target: apply the style; `clear_overrides` with Alt held.
    /// For character styles only with text selected; at a bare caret the panel sends
    /// [`TextStylesPanelResponse::status`] instead.
    Apply { id: u64, clear_overrides: bool },
    /// Clear Override, with a target whose style is overridden.
    ClearOverride,
    /// Redefine: the current style takes the target's attributes. Not for character style 0.
    Redefine,
    /// New: a style from the selected type when there is a target (`from_selection`), else a
    /// default one. The app opens its Style Options for the new id.
    New { from_selection: bool },
    /// Duplicate the selected row's style.
    Duplicate(u64),
    /// Delete the selected row's style (never id 0).
    Delete(u64),
    /// Style Options for a style: a double-click on a row or the footer button (not for
    /// character style 0). The app shows its options editor under the list, as PhotoCraft.
    Options(u64),
    /// Duplicate (the footer button, the panel menu, or rows dropped on New) with several rows
    /// highlighted ([`TextStylesPanel::highlighted`]): every highlighted style, in list order.
    DuplicateHighlighted,
    /// Delete with several rows highlighted: every highlighted style but id 0.
    DeleteHighlighted,
    /// The panel menu's Select All Unused: the app highlights the styles no text uses.
    SelectUnused,
}

/// The panel's result.
#[derive(Clone, Debug)]
pub struct TextStylesPanelResponse {
    pub action: Option<TextStyleAction>,
    /// A message for the status bar ("Select text to apply a character style").
    pub status: Option<StatusLine>,
    pub response: Response,
}

/// One styles panel, built per pass:
///
/// ```ignore
/// let r = TextStylesPanel::new("paragraph-styles", TextStyleKind::Paragraph, &rows, &mut state)
///     .current(at_target.paragraph, at_target.paragraph_override)
///     .target(has_type_target)
///     .show(ui);
/// if let Some(action) = r.action { run_style_command(action) }
/// ```
///
/// - Dense rows (`list.row_height_dense`) in a scrolling list up to 220 pt tall. The style at the type target has the
///   selected-row fill; the row selected in the panel (for the footer buttons) the hover fill; "+"
///   follows the current name while the text overrides it.
/// - Click: [`TextStyleAction::Apply`] (Alt-click clears overrides); the clicked row becomes the
///   selected row. Double-click: [`TextStyleAction::Options`]. The row's tooltip says so, with
///   the platform's name for Alt.
/// - Keys: the list is one Tab stop; ↑/↓ move the selected row, Enter applies it, Delete deletes
///   it (not id 0), as list rows elsewhere in the suite.
/// - Footer buttons are icon buttons with tooltips; each does nothing when its condition (above)
///   doesn't hold.
/// - The panel menu ([`panel_menu`](Self::panel_menu), [`run_command`](Self::run_command)) has the
///   same commands as the footer, under the same conditions.
pub struct TextStylesPanel<'a> {
    id_salt: IdSalt,
    kind: TextStyleKind,
    rows: &'a [TextStyleRow],
    state: &'a mut TextStylesState,
    current: Option<u64>,
    overridden: bool,
    target: bool,
    highlighted: Option<&'a mut Selection<u64>>,
    text_selected: bool,
    labels: Option<TextStylesLabels>,
}

impl<'a> TextStylesPanel<'a> {
    /// The panel over `rows`; `state` holds the row the footer acts on, kept by the app.
    pub fn new(id_salt: impl AsIdSalt, kind: TextStyleKind, rows: &'a [TextStyleRow], state: &'a mut TextStylesState) -> Self {
        TextStylesPanel {
            id_salt: IdSalt::new(id_salt),
            kind,
            rows,
            state,
            current: None,
            overridden: false,
            target: false,
            text_selected: true,
            highlighted: None,
            labels: None,
        }
    }

    /// The style at the type target (the selection's start, or the edited layer) and whether the
    /// text overrides it.
    pub fn current(mut self, id: Option<u64>, overridden: bool) -> Self {
        self.current = id;
        self.overridden = overridden;
        self
    }

    /// Whether there is type to apply styles to (a type layer selected or being edited).
    pub fn target(mut self, on: bool) -> Self {
        self.target = on;
        self
    }

    /// VectorCraft's and InDesign's multi-row highlight, kept by the app (craft-ui's list
    /// [`Selection`], as the page thumbnails and the page list keep theirs): ⇧-click highlights
    /// every row from the last plain click through this one, ⌘/Ctrl-click adds or removes one,
    /// both leaving the text as it is. A row dragged onto New or Delete carries every highlighted
    /// style when it is one of them ([`crate::styles::dragged_styles`]), else itself; New and
    /// Delete outline while a row is held over them. A highlighted style no longer listed is
    /// dropped from the highlight. Without this, the panel highlights one row (PhotoCraft).
    // vectorcraft: crates/ui-egui/src/panels/text_styles.rs:64 `highlighted`, :131 `row_dropped`.
    pub fn highlighted(mut self, highlighted: &'a mut Selection<u64>) -> Self {
        self.highlighted = Some(highlighted);
        self
    }

    /// Whether the type target has text selected (default true). At a bare caret a click on a
    /// character style sets [`TextStylesPanelResponse::status`] instead of applying.
    pub fn text_selected(mut self, on: bool) -> Self {
        self.text_selected = on;
        self
    }

    pub fn labels(mut self, labels: TextStylesLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // pc: crates/ui-egui/src/panels/type_panels_ui.rs:215 `styles_panel` (rows, "+" marker,
    // click / Alt-click / double-click, footer buttons and their conditions).
    pub fn show(self, ui: &mut Ui) -> TextStylesPanelResponse {
        // The multi-row highlight (`highlighted`) is wired by the step 6b fill.
        let TextStylesPanel { id_salt, kind, rows, state, current, overridden, target, text_selected, highlighted: _highlighted, labels } = self;
        let selected = &mut state.selected;
        let facts = MenuFacts { kind, target, overridden, current };
        ui.data_mut(|d| d.insert_temp(Id::NULL.with(id_salt), facts));
        let labels = labels.unwrap_or_else(|| TextStylesLabels::of_ctx(ui.ctx()));
        let paragraph = kind == TextStyleKind::Paragraph;
        let alt = alt_name(ui.ctx());
        let tip = labels.row_tip.replace("{key}", &alt);
        let mut action: Option<TextStyleAction> = None;
        let mut status: Option<StatusLine> = None;
        let hint = || StatusLine { text: labels.select_text.clone(), tone: Tone::Warn };

        // The footer acts on the row selected in the panel, which is the current style until the
        // user picks another.
        let shown = selected.or(current);
        let mut selection = shown.map_or_else(Selection::new, Selection::only);
        let infos: Vec<RowInfo<u64>> = rows.iter().map(|row| RowInfo::leaf(row.id)).collect();
        let tokens = craft_ui::theme::current(ui.ctx());
        let list_label = if paragraph { &labels.paragraph_styles } else { &labels.character_styles };
        let out = List::new((id_salt, kind), list_label.as_str(), infos.as_slice())
            .selection(&mut selection)
            .row_height(tokens.component.list.row_height_dense)
            .max_height(tokens.component.text_styles.list_height)
            .show(ui, |at, &id| {
                let row = &rows[at];
                let is_current = target && current == Some(id);
                let marked = is_current && overridden;
                let none = id == 0 && !paragraph;
                let name = if marked { format!("{}+", row.name) } else { row.name.clone() };
                let spoken = if marked { format!("{}, {}", row.name, labels.overridden) } else { row.name.clone() };
                ListRow::new(id, name)
                    .accessible_name(spoken)
                    .description(if none { labels.no_character_style.clone() } else { tip.clone() })
                    .dimmed(none)
                    .highlight(is_current && shown != Some(id))
            });
        let (alt_held, double) = ui.input(|i| (i.modifiers.alt, i.pointer.button_double_clicked(PointerButton::Primary)));
        let mut picked = *selected;
        for event in out.events {
            match event {
                ListEvent::Selection(change) => {
                    if let Some(id) = change.row {
                        picked = Some(id);
                    }
                }
                ListEvent::Clicked(id) => {
                    picked = Some(id);
                    if target && !paragraph && !text_selected {
                        status = Some(hint());
                    } else if target {
                        action = Some(TextStyleAction::Apply { id, clear_overrides: alt_held });
                    }
                }
                ListEvent::Activate(id) if double => {
                    if id != 0 || paragraph {
                        action = Some(TextStyleAction::Options(id));
                    }
                }
                ListEvent::Activate(id) => {
                    if target && !paragraph && !text_selected {
                        status = Some(hint());
                    } else if target {
                        action = Some(TextStyleAction::Apply { id, clear_overrides: false });
                    }
                }
                _ => {}
            }
        }
        if out.response.has_focus()
            && let Some(id) = picked.or(shown).filter(|id| *id != 0)
            && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Delete))
        {
            action = Some(TextStyleAction::Delete(id));
        }
        if picked != *selected {
            *selected = picked;
        }

        let footer_state = TextStylesState { selected: *selected };
        let mut run = |command: &str| {
            if let Some(found) = facts.action(&footer_state, command) {
                action = Some(found);
            }
        };
        PanelFooter::show(ui, |ui| {
            let left = ui.max_rect();
            ui.scope_builder(UiBuilder::new().max_rect(left).layout(craft_ui::direction::row(ui)), |ui| {
                if PanelFooter::button(ui, Icon::Ban, &labels.clear_override).clicked() {
                    run(CMD_CLEAR_OVERRIDE);
                }
                if PanelFooter::button(ui, Icon::Check, &labels.redefine).clicked() {
                    run(CMD_REDEFINE);
                }
            });
            if PanelFooter::button(ui, Icon::Trash, &labels.delete).clicked() {
                run(CMD_DELETE);
            }
            if PanelFooter::button(ui, Icon::Plus, &labels.new_style).clicked() {
                run(CMD_NEW);
            }
            if PanelFooter::button(ui, Icon::Copy, &labels.duplicate).clicked() {
                run(CMD_DUPLICATE);
            }
            if PanelFooter::button(ui, Icon::Settings, &labels.options).clicked() {
                run(CMD_OPTIONS);
            }
        });
        TextStylesPanelResponse { action, status, response: out.response }
    }
}

impl TextStylesPanel<'_> {
    /// The panel menu's rows, for the docking host's `panel_menu_entries` hook: Clear Override,
    /// Redefine Style, a separator, New Style, Duplicate Style, Delete Style and Style Options…,
    /// enabled under the footer buttons' conditions. `id_salt` is the one the panel was built
    /// with; before the panel's first pass every row is disabled but New Style. The commands come
    /// back as `DockEvent::PanelCommand`; the host passes them to
    /// [`run_command`](Self::run_command).
    pub fn panel_menu(ctx: &Context, id_salt: impl AsIdSalt, state: &TextStylesState) -> Vec<MenuEntry<PanelCommand>> {
        let labels = TextStylesLabels::of_ctx(ctx);
        let facts = MenuFacts::of(ctx, IdSalt::new(id_salt)).unwrap_or(MenuFacts {
            kind: TextStyleKind::Paragraph,
            target: false,
            overridden: false,
            current: None,
        });
        let item =
            |label: &str, command: &str| MenuEntry::from(MenuItem::new(label, command.to_owned()).enabled(facts.action(state, command).is_some()));
        vec![
            item(&labels.clear_override, CMD_CLEAR_OVERRIDE),
            item(&labels.redefine, CMD_REDEFINE),
            MenuEntry::Separator,
            item(&labels.new_style, CMD_NEW),
            item(&labels.duplicate, CMD_DUPLICATE),
            item(&labels.delete, CMD_DELETE),
            item(&labels.options, CMD_OPTIONS),
        ]
    }

    /// Carries out a command of [`panel_menu`](Self::panel_menu): the action for the app to run,
    /// as a footer button's. Delete clears the panel's selection when it deletes the selected
    /// row.
    pub fn run_command(ctx: &Context, id_salt: impl AsIdSalt, state: &mut TextStylesState, command: &str) -> Option<TextStyleAction> {
        let facts = MenuFacts::of(ctx, IdSalt::new(id_salt))?;
        let action = facts.action(state, command)?;
        if let TextStyleAction::Delete(id) = action
            && state.selected == Some(id)
        {
            state.selected = None;
        }
        Some(action)
    }
}

/// The platform's name for Alt ("Alt", or "⌥" on a Mac).
fn alt_name(ctx: &egui::Context) -> String {
    let chord = KeyboardShortcut::new(Modifiers::ALT, Key::A);
    Kbd::parts_in(&chord, ctx.os(), &Words::of_ctx(ctx)).into_iter().next().unwrap_or_else(|| "Alt".to_owned())
}
