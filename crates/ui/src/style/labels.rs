//! User label colours: the default palette.

use craft_theme::{LabelColor, Rgba};

/// The colours of one label colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LabelColors {
    /// The label's swatch, as in a colour menu.
    pub swatch: Rgba,
    /// Behind a toggle cell of a row with this label.
    pub fill: Rgba,
    /// The toggle cell's icon on `fill`.
    pub icon: Rgba,
}

const ICON: Rgba = Rgba::rgb(0xE2, 0xE2, 0xE2);

/// The default colours of `color`.
pub const fn label_colors(color: LabelColor) -> LabelColors {
    let (fill, swatch) = match color {
        LabelColor::Red => (Rgba::rgb(0x9F, 0x2F, 0x30), Rgba::rgb(0xFC, 0x5D, 0x5B)),
        LabelColor::Orange => (Rgba::rgb(0x93, 0x4F, 0x0C), Rgba::rgb(0xF7, 0x97, 0x44)),
        LabelColor::Yellow => (Rgba::rgb(0x99, 0x78, 0x0C), Rgba::rgb(0xDC, 0xD6, 0x4B)),
        LabelColor::Green => (Rgba::rgb(0x4E, 0x71, 0x2E), Rgba::rgb(0x85, 0xDC, 0x6A)),
        LabelColor::Seafoam => (Rgba::rgb(0x0B, 0x54, 0x4F), Rgba::rgb(0x1C, 0x84, 0x88)),
        LabelColor::Blue => (Rgba::rgb(0x41, 0x5B, 0x87), Rgba::rgb(0x78, 0xAD, 0xF6)),
        LabelColor::Indigo => (Rgba::rgb(0x36, 0x34, 0x8E), Rgba::rgb(0x54, 0x4B, 0xE7)),
        LabelColor::Magenta => (Rgba::rgb(0x98, 0x1B, 0x51), Rgba::rgb(0xCC, 0x1A, 0x7E)),
        LabelColor::Fuchsia => (Rgba::rgb(0x71, 0x0F, 0x74), Rgba::rgb(0xB0, 0x14, 0xC0)),
        LabelColor::Violet => (Rgba::rgb(0x5D, 0x3F, 0x8E), Rgba::rgb(0x91, 0x76, 0xD5)),
        LabelColor::Gray => (Rgba::rgb(0x57, 0x57, 0x57), Rgba::rgb(0x9C, 0x9C, 0x9C)),
    };
    LabelColors { swatch, fill, icon: ICON }
}
