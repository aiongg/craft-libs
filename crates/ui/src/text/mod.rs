//! Text and number fields: `TextField<C>` and `NumberField` on the text part (18.3).

mod adorn;
mod alt_text;
mod apply;
mod chips;
mod completions;
mod linked;
mod number;
mod selectable_text;
mod text_field;
mod words;

pub use adorn::{Adornment, Placed, place, reserve};
pub use alt_text::{AltText, AltTextField, AltTextFieldStyle, AltTextWords};
pub use apply::{ApplyRow, ApplyRowStyle};
pub use chips::{ChipCodec, ChipField, ChipFieldStyle, ChipWords};
pub use linked::{LinkMode, LinkedFields, LinkedFieldsStyle, LinkedLayout, LinkedOutput, LinkedWords, follow_equal, follow_ratio, link_toggle};
pub use number::{NumberField, NumberOutput, NumberStyle, Preset};
pub use selectable_text::{SelectableStyle, SelectableText};
pub use text_field::TextField;

use crate::primitives::TextKey;
use crate::style::{ComponentStyle, IconLook, Ink, StyleCx, SurfaceLook};
use craft_behavior::{Insets, Metrics, PartKey};
use craft_theme::Stateful;
use egui::Vec2;

/// The style of a text field: padding and minimum size from the control metrics, and the sizes of the adornments
/// a field draws beside its text (`adorn::place`).
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct FieldStyle {
    #[style(surface)]
    surface: SurfaceLook,
    /// Padding, the gap between adornments and the text, and the minimum size: `size.length.field` wide, a control
    /// high.
    #[style(metrics)]
    metrics: Metrics,
    /// A glyph adornment: its side and colour.
    #[style(icon)]
    glyph: IconLook,
    /// The side of a button adornment: a control height.
    button: f32,
}

impl FieldStyle {
    fn at(cx: &StyleCx<'_>) -> FieldStyle {
        let control = cx.control();
        let metrics = Metrics::new()
            .padding(Insets::new(control.pad_x, 0.0, control.pad_x, 0.0))
            .gap(control.gap)
            .min_size(Vec2::new(cx.tokens.size.length.field, control.height));
        FieldStyle {
            surface: cx.field(),
            metrics,
            glyph: IconLook::new(control.icon, Stateful::new(cx.ink(Ink::OnPanel))),
            button: control.height,
        }
    }
}

/// The part key of the one text part.
const TEXT: PartKey = PartKey::new("text");

fn text_part<V>() -> TextKey<V> {
    TextKey::new(TEXT)
}
