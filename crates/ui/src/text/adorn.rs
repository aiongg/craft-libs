//! The adornment layout of a field (18.1): glyphs and buttons before and after the text, placed by one function so a
//! field's parts and its paint read the same rects.

use super::FieldStyle;
use crate::buttons::IconButton;
use crate::contract::{PaintUi, Props as _};
use crate::primitives::IconSpec;
use crate::runtime::env::Label;
use crate::style::StyleColor;
use craft_behavior::{Flow, PartKey};
use egui::{Rect, vec2};
use smallvec::SmallVec;

/// Something a field draws beside its text.
pub struct Adornment {
    icon: IconSpec<'static>,
    button: Option<(PartKey, Label)>,
}

impl Adornment {
    /// A decorative icon of the style's glyph side.
    pub fn glyph(icon: impl Into<IconSpec<'static>>) -> Adornment {
        Adornment { icon: icon.into(), button: None }
    }

    /// An icon button, the part `part` named `name`, of the style's button side.
    pub fn button(part: PartKey, icon: impl Into<IconSpec<'static>>, name: Label) -> Adornment {
        Adornment { icon: icon.into(), button: Some((part, name)) }
    }

    /// The part key of a button; `None` for a glyph.
    pub fn part(&self) -> Option<PartKey> {
        self.button.as_ref().map(|(part, _)| *part)
    }

    /// The side of the square the adornment takes in the line.
    fn side(&self, style: &FieldStyle) -> f32 {
        if self.button.is_some() { style.button } else { style.glyph.side }
    }

    /// The icon, painted in `color` when there is one.
    fn spec(&self, color: Option<StyleColor>) -> IconSpec<'static> {
        IconSpec {
            source: self.icon.source.clone(),
            ink: self.icon.ink,
            color: color.or(self.icon.color),
            mirror: self.icon.mirror,
            set: self.icon.set,
        }
    }

    /// Paints the adornment in `rect`, one of `place`'s rects: a glyph in `glyph_color`, a button as an icon button face
    /// over its part.
    pub fn paint(&self, ui: &mut PaintUi<'_>, rect: Rect, glyph_color: StyleColor) {
        match &self.button {
            None => ui.draw().icon(rect, &self.spec(Some(glyph_color))),
            Some((part, name)) => ui.face(*part, IconButton::new(self.spec(None)).label(name.clone()).focusable(false)),
        }
    }
}

/// Where the text and each adornment sit.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Placed {
    /// What is left for the text.
    pub text: Rect,
    /// The rects of the leading adornments, in the order given.
    pub leading: SmallVec<[Rect; 2]>,
    /// The rects of the trailing adornments, in the order given.
    pub trailing: SmallVec<[Rect; 2]>,
}

/// The width the adornments take from a line, gaps included.
pub fn reserve(style: &FieldStyle, leading: &[Adornment], trailing: &[Adornment]) -> f32 {
    leading.iter().chain(trailing).map(|adornment| adornment.side(style) + style.metrics.gap).sum()
}

/// Places `leading` from the reading start and `trailing` from the reading end of `rect` (the field's content
/// rect), each `style.metrics.gap` from its neighbour and centred on the line, and leaves the rest to the text.
/// `trailing` is in reading order: its last adornment is at the end.
pub fn place(rect: Rect, style: &FieldStyle, leading: &[Adornment], trailing: &[Adornment], rtl: bool) -> Placed {
    let gap = style.metrics.gap;
    let centred = |cell: Rect, side: f32| Rect::from_center_size(cell.center(), vec2(cell.width(), side));
    let mut flow = Flow::new(rect, rtl);
    let mut lead = SmallVec::new();
    for adornment in leading {
        let (cell, rest) = flow.take_start(adornment.side(style));
        lead.push(centred(cell, adornment.side(style)));
        flow = rest.take_start(gap).1;
    }
    let mut trail: SmallVec<[Rect; 2]> = SmallVec::new();
    for adornment in trailing.iter().rev() {
        let (cell, rest) = flow.take_end(adornment.side(style));
        trail.push(centred(cell, adornment.side(style)));
        flow = rest.take_end(gap).1;
    }
    trail.reverse();
    Placed { text: flow.rect(), leading: lead, trailing: trail }
}
