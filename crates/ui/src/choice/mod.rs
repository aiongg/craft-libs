//! `Checkbox`, `Switch`, `RadioGroup`, `Segmented` and `Rating`.

pub mod check;
pub mod radio;
mod rating;
mod segmented;
mod switch;
mod words;

mod swatch;

pub use check::{Checkbox, CheckboxStyle};
pub use radio::{GroupStyle, Radio, RadioGroup, RadioKind, RadioStyle};
pub use rating::{Rating, RatingStyle};
pub use segmented::{Segmented, SegmentedStyle};
pub use switch::{Switch, SwitchStyle};
pub use words::ChoiceWords;

pub use swatch::{Swatch, SwatchFace, SwatchMetrics, SwatchOutput, SwatchStyle};

/// The outline of a check box, radio button or switch track: `text.faint`, moved only as far as it needs to be
/// identifiable on the sunken fill and on the panel and raised surfaces; `text.muted` under the pointer.
fn outline(cx: &crate::style::StyleCx<'_>) -> craft_theme::Stateful<crate::style::StyleColor> {
    use craft_theme::{Contrast, Stateful, States};
    let colors = &cx.tokens.color;
    let high = cx.contrast() == Contrast::High;
    let rest =
        cx.color(craft_theme::identifiable_outline(colors.text.faint, colors.surface.sunken, &[colors.surface.panel, colors.surface.raised], high));
    let (muted, disabled) = (cx.color(colors.text.muted), cx.color(colors.text.disabled));
    Stateful::new(rest).off(States::new(rest, muted, muted, disabled))
}
