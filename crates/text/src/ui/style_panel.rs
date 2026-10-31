//! [`TextStylesPanel`] (F14.03): the Character Styles and Paragraph Styles panels. A list of the
//! document's styles with the one at the type selection highlighted ("+" after its name when the
//! text overrides it), and Photoshop's footer: Clear Override and Redefine at the left, Style
//! Options, Duplicate, New and Delete at the right. The styles, their attributes and the
//! commands are the app's; the panel reports what the user asked for.

use craft_ui::behavior::{RowInfo, Selection};
use craft_ui::composites::PanelFooter;
use craft_ui::controls::Kbd;
use craft_ui::icons::Icon;
use craft_ui::list::{List, ListEvent, ListRow};
use craft_ui::localized::{Labels as _, Words};
use egui::{Key, KeyboardShortcut, Modifiers, PointerButton, Response, Ui, UiBuilder};

/// Row height and list height limit (PhotoCraft's 22 pt and 220 pt).
const ROW_HEIGHT: f32 = 22.0;
const LIST_HEIGHT: f32 = 220.0;

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

/// What the user asked for this pass. The app runs the command on its document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextStyleAction {
    /// A click on a row with a type target: apply the style; `clear_overrides` with Alt held.
    /// For character styles only with text selected; at a bare caret the panel sends
    /// [`TextStylesResponse::status`] instead.
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
}

/// The panel's result.
#[derive(Clone, Debug)]
pub struct TextStylesResponse {
    pub action: Option<TextStyleAction>,
    /// A message for the status bar ("Select text to apply a character style").
    pub status: Option<String>,
    pub response: Response,
}

/// One styles panel, built per pass:
///
/// ```ignore
/// let r = TextStylesPanel::new(TextStyleKind::Paragraph, &rows, &mut selected)
///     .current(at_target.paragraph, at_target.paragraph_override)
///     .target(has_type_target)
///     .show(ui);
/// if let Some(action) = r.action { run_style_command(action) }
/// ```
///
/// - Rows 22 pt high in a scrolling list up to 220 pt tall. The style at the type target has the
///   selected-row fill; the row selected in the panel (for the footer buttons) the hover fill; "+"
///   follows the current name while the text overrides it.
/// - Click: [`TextStyleAction::Apply`] (Alt-click clears overrides); the clicked row becomes the
///   selected row. Double-click: [`TextStyleAction::Options`]. The row's tooltip says so, with
///   the platform's name for Alt.
/// - Keys: the list is one Tab stop; ↑/↓ move the selected row, Enter applies it, Delete deletes
///   it (not id 0), as list rows elsewhere in the suite.
/// - Footer buttons are icon buttons with tooltips; each does nothing when its condition (above)
///   doesn't hold.
pub struct TextStylesPanel<'a> {
    kind: TextStyleKind,
    rows: &'a [TextStyleRow],
    selected: &'a mut Option<u64>,
    current: Option<u64>,
    overridden: bool,
    target: bool,
    text_selected: bool,
    labels: Option<TextStylesLabels>,
}

impl<'a> TextStylesPanel<'a> {
    /// The panel over `rows`; `selected` is the row the footer acts on, kept by the app.
    pub fn new(kind: TextStyleKind, rows: &'a [TextStyleRow], selected: &'a mut Option<u64>) -> Self {
        TextStylesPanel { kind, rows, selected, current: None, overridden: false, target: false, text_selected: true, labels: None }
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

    /// Whether the type target has text selected (default true). At a bare caret a click on a
    /// character style sets [`TextStylesResponse::status`] instead of applying.
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
    pub fn show(self, ui: &mut Ui) -> TextStylesResponse {
        let TextStylesPanel { kind, rows, selected, current, overridden, target, text_selected, labels } = self;
        let labels = labels.unwrap_or_else(|| TextStylesLabels::of_ctx(ui.ctx()));
        let paragraph = kind == TextStyleKind::Paragraph;
        let alt = alt_name(ui.ctx());
        let tip = labels.row_tip.replace("{key}", &alt);
        let mut action: Option<TextStyleAction> = None;
        let mut status: Option<String> = None;

        // The footer acts on the row selected in the panel, which is the current style until the
        // user picks another.
        let shown = selected.or(current);
        let mut selection = shown.map_or_else(Selection::new, Selection::only);
        let infos: Vec<RowInfo<u64>> = rows.iter().map(|row| RowInfo::leaf(row.id)).collect();
        let list_label = if paragraph { &labels.paragraph_styles } else { &labels.character_styles };
        let out = List::new(("text-styles", kind), list_label.as_str(), infos.as_slice())
            .selection(&mut selection)
            .row_height(ROW_HEIGHT)
            .max_height(LIST_HEIGHT)
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
                        status = Some(labels.select_text.clone());
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
                        status = Some(labels.select_text.clone());
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

        let pick = picked.or(shown);
        let editable = pick.filter(|id| *id != 0 || paragraph);
        PanelFooter::show(ui, |ui| {
            let left = ui.max_rect();
            ui.scope_builder(UiBuilder::new().max_rect(left).layout(craft_ui::direction::row(ui)), |ui| {
                if PanelFooter::button(ui, Icon::Ban, &labels.clear_override).clicked() && target && overridden {
                    action = Some(TextStyleAction::ClearOverride);
                }
                if PanelFooter::button(ui, Icon::Check, &labels.redefine).clicked() && target && current.is_some_and(|id| id != 0 || paragraph) {
                    action = Some(TextStyleAction::Redefine);
                }
            });
            if PanelFooter::button(ui, Icon::Trash, &labels.delete).clicked()
                && let Some(id) = editable.filter(|id| *id != 0)
            {
                action = Some(TextStyleAction::Delete(id));
            }
            if PanelFooter::button(ui, Icon::Plus, &labels.new_style).clicked() {
                action = Some(TextStyleAction::New { from_selection: target });
            }
            if PanelFooter::button(ui, Icon::Copy, &labels.duplicate).clicked()
                && let Some(id) = editable
            {
                action = Some(TextStyleAction::Duplicate(id));
            }
            if PanelFooter::button(ui, Icon::Settings, &labels.options).clicked()
                && let Some(id) = editable
            {
                action = Some(TextStyleAction::Options(id));
            }
        });
        TextStylesResponse { action, status, response: out.response }
    }
}

/// The platform's name for Alt ("Alt", or "⌥" on a Mac).
fn alt_name(ctx: &egui::Context) -> String {
    let chord = KeyboardShortcut::new(Modifiers::ALT, Key::A);
    Kbd::parts_in(&chord, ctx.os(), &Words::of_ctx(ctx)).into_iter().next().unwrap_or_else(|| "Alt".to_owned())
}
