//! [`TextStylesPanel`] (F14.03): the Character Styles and Paragraph Styles panels. A list of the
//! document's styles with the one at the type selection highlighted ("+" after its name when the
//! text overrides it), and Photoshop's footer: Clear Override and Redefine at the left, Style
//! Options, Duplicate, New and Delete at the right. The styles, their attributes and the
//! commands are the app's; the panel reports what the user asked for.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::behavior::Selection;
use egui::{Response, Ui};

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
    highlighted: Option<&'a mut Selection<u64>>,
    labels: Option<TextStylesLabels>,
}

impl<'a> TextStylesPanel<'a> {
    /// The panel over `rows`; `selected` is the row the footer acts on, kept by the app.
    pub fn new(kind: TextStyleKind, rows: &'a [TextStyleRow], selected: &'a mut Option<u64>) -> Self {
        TextStylesPanel { kind, rows, selected, current: None, overridden: false, target: false, highlighted: None, labels: None }
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

    pub fn labels(mut self, labels: TextStylesLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // pc: crates/ui-egui/src/panels/type_panels_ui.rs:215 `styles_panel` (rows, "+" marker,
    // click / Alt-click / double-click, footer buttons and their conditions).
    pub fn show(self, ui: &mut Ui) -> TextStylesResponse {
        todo!()
    }
}
