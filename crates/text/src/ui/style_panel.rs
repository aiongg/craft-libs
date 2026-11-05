//! [`TextStylesPanel`] (F14.03): the Character Styles and Paragraph Styles panels. A list of the
//! document's styles with the one at the type selection highlighted ("+" after its name when the
//! text overrides it), and Photoshop's footer: Clear Override and Redefine at the left, Style
//! Options, Duplicate, New and Delete at the right. The styles, their attributes and the
//! commands are the app's; the panel reports what the user asked for.

use craft_ui::behavior::{Axis, ClaimedKey, KeyClaim, ListDrag, Reorder, RowInfo, Selection, SelectionPick};
use craft_ui::composites::{PanelFooter, StatusLine, Tone};
use craft_ui::controls::Kbd;
use craft_ui::docking::panel_menu::PanelCommand;
use craft_ui::icons::Icon;
use craft_ui::list::{List, ListEvent, ListRow};
use craft_ui::localized::{Labels as _, Words};
use craft_ui::menu::{MenuEntry, MenuItem, facts_id};

use crate::styles::dragged_styles;
use egui::{AsIdSalt, Context, IdSalt, Key, KeyboardShortcut, Modifiers, PointerButton, Response, Sides, Ui};
use std::cell::RefCell;

mod picker;
#[cfg(test)]
mod picker_tests;

pub use picker::{NamedStyleRow, StyleChip, StyleChipResponse, StylePickerLabels, StylePickerStyle, StylePickerView, StylePreview};

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
pub(crate) struct MenuFacts {
    kind: TextStyleKind,
    target: bool,
    overridden: bool,
    current: Option<u64>,
    /// How many rows the panel's multi-row highlight holds: Duplicate and Delete act on all of
    /// them from the second on.
    pub(crate) highlighted: usize,
}

const CMD_CLEAR_OVERRIDE: &str = "textStyles.clearOverride";
const CMD_REDEFINE: &str = "textStyles.redefine";
const CMD_NEW: &str = "textStyles.new";
const CMD_DUPLICATE: &str = "textStyles.duplicate";
const CMD_DELETE: &str = "textStyles.delete";
const CMD_OPTIONS: &str = "textStyles.options";

impl MenuFacts {
    fn of(ctx: &Context, id_salt: IdSalt) -> Option<MenuFacts> {
        ctx.data(|d| d.get_temp(facts_id(id_salt)))
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
            CMD_DUPLICATE if self.highlighted > 1 => Some(TextStyleAction::DuplicateHighlighted),
            CMD_DELETE if self.highlighted > 1 => Some(TextStyleAction::DeleteHighlighted),
            CMD_DUPLICATE => editable.map(TextStyleAction::Duplicate),
            CMD_DELETE => editable.filter(|id| *id != 0).map(TextStyleAction::Delete),
            CMD_OPTIONS => editable.map(TextStyleAction::Options),
            _ => None,
        }
    }
}

/// What the user asked for this pass. The app runs the command on its document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TextStyleAction {
    /// A click on a row with a type target: apply the style; `clear_overrides` with Alt held.
    /// For character styles only with text selected; at a bare caret the panel sends
    /// [`TextStylesPanelOutput::status`] instead.
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
pub struct TextStylesPanelOutput {
    /// What the user asked for this pass, in order; the app runs them in that order.
    pub actions: Vec<TextStyleAction>,
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
/// for action in r.actions { run_style_command(action) }
/// ```
///
/// - Dense rows (`list.row_height_dense`) in a scrolling list up to 220 pt tall. The style at the type target has the
///   selected-row fill; the row selected in the panel (for the footer buttons) the hover fill; "+"
///   follows the current name while the text overrides it.
/// - Click: [`TextStyleAction::Apply`] (Alt-click clears overrides); the clicked row becomes the
///   selected row. Double-click: [`TextStyleAction::Options`]. The row's tooltip says so, with
///   the platform's name for Alt.
/// - Keys: the list is one Tab stop; ↑/↓ move the selected row, Enter applies it, Delete or
///   Backspace deletes it (not id 0), as list rows elsewhere in the suite.
/// - The footer is one [`Sides`] row: Clear Override and Redefine at the start, Style Options,
///   Duplicate, New and Delete at the end. The start group is clipped to the room the end group
///   leaves, so at a narrow width it never sits under the end buttons.
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
    /// character style sets [`TextStylesPanelOutput::status`] instead of applying.
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
    pub fn show(self, ui: &mut Ui) -> TextStylesPanelOutput {
        let TextStylesPanel { id_salt, kind, rows, state, current, overridden, target, text_selected, mut highlighted, labels } = self;
        let selected = &mut state.selected;
        let labels = labels.unwrap_or_else(|| TextStylesLabels::of_ctx(ui.ctx()));
        let paragraph = kind == TextStyleKind::Paragraph;
        let alt = alt_name(ui.ctx());
        let tip = labels.row_tip.replace("{key}", &alt);
        let actions = RefCell::new(Vec::<TextStyleAction>::new());
        let mut status: Option<StatusLine> = None;
        let hint = || StatusLine { text: labels.select_text.clone(), tone: Tone::Warn };

        // The footer acts on the row selected in the panel, which is the current style until the
        // user picks another.
        let shown = selected.or(current);
        let mut own = shown.map_or_else(Selection::new, Selection::only);
        let list_id = List::id_in(ui, (id_salt, kind));
        if let Some(highlight) = highlighted.as_deref_mut() {
            let listed = |id: &u64| rows.iter().any(|row| row.id == *id);
            highlight.items.retain(listed);
            highlight.lead = highlight.lead.filter(listed);
            highlight.anchor = highlight.anchor.filter(listed);
        }
        let many = highlighted.is_some();
        let selection = highlighted.as_deref_mut().unwrap_or(&mut own);
        let held = selection.items.clone();
        let infos: Vec<RowInfo<u64>> = rows.iter().map(|row| RowInfo::leaf(row.id)).collect();
        let tokens = craft_ui::theme::current(ui.ctx());
        let list_label = if paragraph { &labels.paragraph_styles } else { &labels.character_styles };
        let mut list = List::new((id_salt, kind), list_label.as_str(), infos.as_slice())
            .selection(selection)
            .row_height(tokens.component.list.row_height_dense)
            .max_height(tokens.component.text_styles.list_height);
        if many {
            list = list.can_drag(|_| true).reorder(Reorder::new(Axis::Vertical).accepts(|_, _| false));
        }
        let out = list.show(ui, |at, &id| {
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
                .highlight(is_current && !held.contains(&id))
        });
        let (alt_held, double) = ui.input(|i| (i.modifiers.alt, i.pointer.button_double_clicked(PointerButton::Primary)));
        let mut picked = *selected;
        // ⇧- and ⌘/Ctrl-clicks of the highlight change it and leave the text as it is.
        let mut highlighting = false;
        for event in out.events {
            match event {
                ListEvent::Selection(change) => {
                    highlighting = many && matches!(change.pick, SelectionPick::Range | SelectionPick::Toggle);
                    if many {
                        picked = change.selection.lead.or(picked);
                    } else if let Some(id) = change.row {
                        picked = Some(id);
                    }
                }
                ListEvent::Clicked(_) if highlighting => highlighting = false,
                ListEvent::Clicked(id) => {
                    picked = Some(id);
                    if target && !paragraph && !text_selected {
                        status = Some(hint());
                    } else if target {
                        actions.borrow_mut().push(TextStyleAction::Apply { id, clear_overrides: alt_held });
                    }
                }
                ListEvent::Activate(id) if double => {
                    if id != 0 || paragraph {
                        actions.borrow_mut().push(TextStyleAction::Options(id));
                    }
                }
                ListEvent::Activate(id) => {
                    if target && !paragraph && !text_selected {
                        status = Some(hint());
                    } else if target {
                        actions.borrow_mut().push(TextStyleAction::Apply { id, clear_overrides: false });
                    }
                }
                _ => {}
            }
        }
        if picked != *selected {
            *selected = picked;
        }
        let kept: Vec<u64> = highlighted.as_deref().map(|highlight| highlight.items.clone()).unwrap_or_default();
        let facts = MenuFacts { kind, target, overridden, current, highlighted: kept.len() };
        ui.data_mut(|d| d.insert_temp(facts_id(id_salt), facts));
        let footer_state = TextStylesState { selected: *selected };
        let delete_keys = [ClaimedKey::exact(Modifiers::NONE, Key::Delete), ClaimedKey::exact(Modifiers::NONE, Key::Backspace)];
        KeyClaim::claim(ui.ctx(), list_id, &delete_keys);
        if out.response.has_focus() && ui.input_mut(|i| delete_keys.iter().fold(false, |any, key| key.consume(i) | any)) {
            if kept.len() > 1 {
                actions.borrow_mut().push(TextStyleAction::DeleteHighlighted);
            } else if let Some(id) = picked.or(shown).filter(|id| *id != 0) {
                actions.borrow_mut().push(TextStyleAction::Delete(id));
            }
        }

        let listed: Vec<u64> = rows.iter().map(|row| row.id).collect();
        let run = |command: &str| {
            if let Some(found) = facts.action(&footer_state, command) {
                actions.borrow_mut().push(found);
            }
        };
        let dropped_on_new = RefCell::new(None);
        let dropped_on_delete = RefCell::new(None);
        let start_group = |ui: &mut Ui| {
            // Only the room the end group leaves: the buttons that don't fit neither paint nor
            // take clicks over the end group's.
            ui.set_clip_rect(ui.clip_rect().intersect(ui.max_rect()));
            if PanelFooter::button(ui, Icon::Ban, &labels.clear_override).clicked() {
                run(CMD_CLEAR_OVERRIDE);
            }
            if PanelFooter::button(ui, Icon::Check, &labels.redefine).clicked() {
                run(CMD_REDEFINE);
            }
        };
        let end_group = |ui: &mut Ui| {
            let (delete, dropped) = PanelFooter::drop_button::<ListDrag<u64>>(ui, Icon::Trash, &labels.delete, true, |drag| {
                drag.list == list_id && drag.items.iter().any(|id| *id != 0)
            });
            *dropped_on_delete.borrow_mut() = dropped;
            if delete.clicked() {
                run(CMD_DELETE);
            }
            let (new, dropped) = PanelFooter::drop_button::<ListDrag<u64>>(ui, Icon::Plus, &labels.new_style, true, |drag| {
                drag.list == list_id && drag.items.iter().any(|id| *id != 0 || paragraph)
            });
            *dropped_on_new.borrow_mut() = dropped;
            if new.clicked() {
                run(CMD_NEW);
            }
            if PanelFooter::button(ui, Icon::Copy, &labels.duplicate).clicked() {
                run(CMD_DUPLICATE);
            }
            if PanelFooter::button(ui, Icon::Settings, &labels.options).clicked() {
                run(CMD_OPTIONS);
            }
        };
        let rtl = craft_ui::direction::is_rtl(ui);
        PanelFooter::show(ui, |ui| {
            // `Sides` is physical: the start group goes to the right in a right-to-left interface,
            // and the end group is the one that keeps its width there.
            let sides = Sides::new().height(ui.available_height());
            if rtl {
                sides.shrink_right().show(ui, |left| end_group(left), |right| start_group(right));
            } else {
                sides.shrink_left().show(ui, |left| start_group(left), |right| end_group(right));
            }
        });
        // A dragged row carries every highlighted style when it is one of them, else itself.
        if let Some(drag) = dropped_on_delete.into_inner() {
            actions.borrow_mut().extend(if dragged_styles(&kept, &listed, &drag.grabbed).len() > 1 {
                Some(TextStyleAction::DeleteHighlighted)
            } else {
                (drag.grabbed != 0).then_some(TextStyleAction::Delete(drag.grabbed))
            });
        }
        if let Some(drag) = dropped_on_new.into_inner() {
            actions.borrow_mut().extend(if dragged_styles(&kept, &listed, &drag.grabbed).len() > 1 {
                Some(TextStyleAction::DuplicateHighlighted)
            } else {
                (drag.grabbed != 0 || paragraph).then_some(TextStyleAction::Duplicate(drag.grabbed))
            });
        }
        TextStylesPanelOutput { actions: actions.into_inner(), status, response: out.response }
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
            highlighted: 0,
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
