//! Command palettes: the modal search box over the app's commands. Sizes only (no colours): the box
//! is a popup surface, its scrim is `surface.scrim`, its active row `menu.active_fill` and
//! `menu.active_text`, its detail text `menu.shortcut` and its disabled rows `text.disabled`.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Command palette tokens, in points (`max_rows` and `recent` are counts).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct CommandPaletteTokens {
    /// Width of the box: `command_palette.width`. Not scaled: it is a text surface.
    pub width: f32,
    /// From the window's top to the box's top: `command_palette.top`.
    pub top: f32,
    /// Inner margin of the box: `command_palette.pad`.
    pub pad: f32,
    /// Height of a result row: `command_palette.row_height`.
    pub row_height: f32,
    /// Most result rows shown before the list scrolls: `command_palette.max_rows`.
    pub max_rows: usize,
    /// Commands remembered for the empty query: `command_palette.recent`.
    pub recent: usize,
}

/// No overridable field: the sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CommandPaletteField {}

impl Family for CommandPaletteTokens {
    type Field = CommandPaletteField;
    const FIELDS: &'static [CommandPaletteField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { CommandPaletteTokens;
        "width" 0.0..=1024.0 => width;
        "top" 0.0..=1024.0 => top;
        "pad" 0.0..=64.0 => pad;
        "row_height" 0.0..=1024.0 => row_height;
    };

    /// PhotoCraft's palette (560 wide, 110 from the top, 32 rows, 12 rows shown); the row height
    /// follows the Size.
    fn defaults(s: &Semantic<'_>) -> CommandPaletteTokens {
        CommandPaletteTokens {
            width: values::COMMAND_PALETTE_WIDTH,
            top: values::COMMAND_PALETTE_TOP,
            pad: values::COMMAND_PALETTE_PAD,
            row_height: s.set.command_palette_row_height,
            max_rows: values::COMMAND_PALETTE_MAX_ROWS,
            recent: values::COMMAND_PALETTE_RECENT,
        }
    }

    fn path(f: CommandPaletteField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<CommandPaletteField> {
        None
    }

    fn kind(f: CommandPaletteField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: CommandPaletteField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: CommandPaletteField, _: OverrideValue) {
        match f {}
    }

    // The colours are the menu's, whose pairs are declared with the menu family.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
