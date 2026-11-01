//! New Document dialog: the card grid, the page icon on a card and the Saved list. Sizes only (no colours): the colours are the surface, text and state tokens of the controls
//! inside, and no theme overrides these. The values are PhotoCraft's.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// NewDocument tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct NewDocumentTokens {
    /// Content width of the dialog: `new_document.width`.
    pub width: f32,
    /// Size of a preset card: `new_document.card`.
    pub card: [f32; 2],
    /// Columns of cards the grid shows before it scrolls: `new_document.card_columns`.
    pub card_columns: usize,
    /// Rows of cards the grid shows before it scrolls: `new_document.card_rows`.
    pub card_rows: usize,
    /// Longer side of the page icon on a card: `new_document.page_icon`.
    pub page_icon: f32,
    /// Side of the square the page icon sits in: `new_document.page_box`.
    pub page_box: f32,
    /// From a card's top to the centre of its page icon: `new_document.card_icon_center`.
    pub card_icon_center: f32,
    /// From a card's top to the centre of its name: `new_document.card_title_center`.
    pub card_title_center: f32,
    /// From a card's top to the centre of its size line: `new_document.card_line_center`.
    pub card_line_center: f32,
    /// Room beside the card grid for its scroll bar: `new_document.scroll_room`.
    pub scroll_room: f32,
    /// Height of the Saved list: `new_document.saved_height`.
    pub saved_height: f32,
    /// Height of a row of the Saved list: `new_document.saved_row`.
    pub saved_row: f32,
}

/// No overridable field: the sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NewDocumentField {}

impl Family for NewDocumentTokens {
    type Field = NewDocumentField;
    const FIELDS: &'static [NewDocumentField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { NewDocumentTokens;
        "width" 0.0..=4096.0 => width;
        "card.0" 0.0..=1024.0 => card[0];
        "card.1" 0.0..=1024.0 => card[1];
        "page_icon" 0.0..=256.0 => page_icon;
        "page_box" 0.0..=256.0 => page_box;
        "card_icon_center" 0.0..=1024.0 => card_icon_center;
        "card_title_center" 0.0..=1024.0 => card_title_center;
        "card_line_center" 0.0..=1024.0 => card_line_center;
        "scroll_room" 0.0..=64.0 => scroll_room;
        "saved_height" 0.0..=4096.0 => saved_height;
        "saved_row" 0.0..=256.0 => saved_row;
    };

    fn defaults(_: &Semantic<'_>) -> NewDocumentTokens {
        NewDocumentTokens {
            width: values::NEW_DOCUMENT_WIDTH,
            card: values::NEW_DOCUMENT_CARD,
            card_columns: values::NEW_DOCUMENT_CARD_COLUMNS,
            card_rows: values::NEW_DOCUMENT_CARD_ROWS,
            page_icon: values::NEW_DOCUMENT_PAGE_ICON,
            page_box: values::NEW_DOCUMENT_PAGE_BOX,
            card_icon_center: values::NEW_DOCUMENT_CARD_ICON_CENTER,
            card_title_center: values::NEW_DOCUMENT_CARD_TITLE_CENTER,
            card_line_center: values::NEW_DOCUMENT_CARD_LINE_CENTER,
            scroll_room: values::NEW_DOCUMENT_SCROLL_ROOM,
            saved_height: values::NEW_DOCUMENT_SAVED_HEIGHT,
            saved_row: values::NEW_DOCUMENT_SAVED_ROW,
        }
    }

    fn path(f: NewDocumentField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<NewDocumentField> {
        None
    }

    fn kind(f: NewDocumentField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: NewDocumentField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: NewDocumentField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
