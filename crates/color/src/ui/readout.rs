//! `ColorReadout`: a colour as a hex field and editable R, G and B fields (the readout of a Color panel).

#[cfg(feature = "screen-testing")]
mod stories;

use craft_behavior::{Flow, Metrics, Proposal, Sense};
use craft_theme::Rgba;
use craft_ui::contract::{
    Bind, BindMut, Common, Component, Cx, Edit as _, EditResponse, MeasureCx, PaintUi, Props as _, Role, WordSetCx as _,
};
use craft_ui::runtime::env::Label;
use craft_ui::style::{ComponentStyle, StyleCx};
use craft_ui::text::{NumberField, TextField};
use egui::{Rect, Vec2, pos2};

/// The channels' names and letters.
const CHANNELS: [&str; 3] = ["Red", "Green", "Blue"];

craft_ui::words! {
    /// The readout's words, English by default.
    #[non_exhaustive]
    pub struct ColorReadoutWords {
        /// The hex field's accessible name.
        hex: "colorReadout.hex" = "Hex color";
        /// The red field's accessible name.
        red: "colorReadout.red" = "Red";
        /// The green field's accessible name.
        green: "colorReadout.green" = "Green";
        /// The blue field's accessible name.
        blue: "colorReadout.blue" = "Blue";
    }
}

/// The style of the readout: the gap between fields and the width of each.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct ColorReadoutStyle {
    #[style(metrics)]
    metrics: Metrics,
    /// The width of the hex field.
    hex: f32,
    /// The width of a channel field.
    channel: f32,
}

/// A colour as a hex field and three number fields (R, G, B, each 0 to 255), in one row. An edit in any field
/// reports the whole colour, with the other channels and the alpha as given. Each field's gesture is the readout's.
///
/// AccessKit: a group named by the label, the hex field "Hex color" and a spin button per channel named by its
/// colour.
#[derive(crate::contract::Component, crate::contract::Props)]
pub struct ColorReadout<'a> {
    #[common]
    common: Common,
    value: Bind<'a, Rgba>,
}

impl<'a> ColorReadout<'a> {
    /// A readout of `value` named `label`.
    pub fn new(label: impl Into<Label>, value: Bind<'a, Rgba>) -> Self {
        ColorReadout { common: Common::new(), value }.label(label)
    }
}

impl Component for ColorReadout<'_> {
    const ROLE: Role = Role::GROUP;
    const PARTS: bool = false;
    const STOPS: bool = false;
    const OVERLAYS: bool = false;
    type Style = ColorReadoutStyle;
    type Output = EditResponse<Rgba>;

    fn style(cx: &StyleCx<'_>, _: ColorReadoutStyleKey) -> ColorReadoutStyle {
        ColorReadoutStyle { metrics: Metrics::new().gap(cx.control().gap), hex: cx.tokens.size.length.field, channel: cx.tokens.size.length.short }
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let height = cx.child("hex", &mut TextField::hex("", Bind::given(Rgba::BLACK)), Proposal::Ideal).y;
        Vec2::new(cx.style.hex + 3.0 * (cx.style.channel + cx.gap()), height)
    }

    fn bind(&mut self) -> Option<BindMut<'_, Rgba>> {
        Some(self.value.reborrow())
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let shown = cx.value().copied().unwrap_or(Rgba::BLACK);
        let words = cx.words::<ColorReadoutWords>();
        let gap = cx.style.metrics.gap;
        let flow = Flow::new(cx.rect, cx.env.direction().is_rtl());
        let place = |x: f32, width: f32| flow.to_physical(Rect::from_min_size(pos2(x, 0.0), Vec2::new(width, cx.rect.height())).translate(cx.rect.min.to_vec2()));
        let hex = ui.child("hex", place(0.0, cx.style.hex), TextField::hex(words.hex.as_str().to_owned(), Bind::given(shown)));
        cx.adopt(&hex, &Rgba::BLACK, |value, mine| *mine = *value);
        let names = [&words.red, &words.green, &words.blue];
        for (i, name) in names.into_iter().enumerate() {
            let x = cx.style.hex + gap + i as f32 * (cx.style.channel + gap);
            let field = NumberField::new(name.as_str().to_owned(), Bind::given(Some(f64::from(shown.0[i]))))
                .range(0.0..=255.0)
                .step(1.0)
                .snap_typed(true)
                .commit_decimals(0);
            let edit = ui.child(("channel", CHANNELS[i]), place(x, cx.style.channel), field);
            cx.adopt(&edit, &Rgba::BLACK, |value, mine| {
                if let Some(value) = value {
                    mine.0[i] = value.round().clamp(0.0, 255.0) as u8;
                }
            });
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<Rgba> {
        cx.edit_response()
    }
}
