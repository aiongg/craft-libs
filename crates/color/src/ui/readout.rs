//! `ColorReadout`: a colour as a hex field and editable R, G and B fields (the readout of a Color panel).

#[cfg(feature = "screen-testing")]
mod stories;

use craft_behavior::{EditEvent, Metrics, Mixed};
use craft_theme::Rgba;
use craft_ui::contract::{
    Bind, BodyCx, Common, Component, Container, ContainerProps, Cx, Outcome, Pattern, Props as _, Role, Show as _, ShowContainer as _, Sizing,
};
use craft_ui::layout::BodyLayout;
use craft_ui::layout_components::Row;
use craft_ui::runtime::env::{Env, Label};
use craft_ui::style::{ComponentStyle, StyleCx};
use craft_ui::text::{NumberField, TextField};
use egui::{Response, Ui};

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

/// The style of the readout: the gap between its fields.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct ColorReadoutStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// What a [`ColorReadout`] reports.
#[derive(Debug)]
#[non_exhaustive]
#[must_use = "the host applies the edit"]
pub struct ColorReadoutOutput {
    /// The readout's response.
    pub response: Response,
    /// The pass's edits of the colour folded into one; `None` when nothing was edited.
    pub event: Option<EditEvent<Rgba>>,
}

impl Outcome for ColorReadoutOutput {
    type Value = Rgba;

    fn response(&self) -> &Response {
        &self.response
    }

    #[cfg(feature = "screen-testing")]
    fn story_facts(&self) -> craft_ui::testing::StoryOutput {
        let mut facts = craft_ui::testing::StoryOutput::default();
        facts.id = Some(self.response.id);
        facts.edits = self.event.iter().filter_map(craft_ui::testing::EditKind::of).collect();
        facts
    }
}

/// A colour as a hex field and three number fields (R, G, B, each 0 to 255), in one row. An edit in any field
/// reports the whole colour, with the other channels and the alpha as given. Each field's gesture is the readout's.
///
/// AccessKit: a group named by the label, the hex field "Hex color" and a spin button per channel named by its
/// colour.
#[must_use = "show the readout with `show`"]
#[derive(Pattern)]
pub struct ColorReadout<'a> {
    label: Label,
    value: Bind<'a, Rgba>,
}

impl<'a> ColorReadout<'a> {
    /// A readout of `value` named `label`.
    pub fn new(label: impl Into<Label>, value: Bind<'a, Rgba>) -> Self {
        ColorReadout { label: label.into(), value }
    }

    /// Shows the readout.
    pub fn show(self, ui: &mut Ui) -> ColorReadoutOutput {
        let ColorReadout { label, mut value } = self;
        let words = Env::of(ui).words::<ColorReadoutWords>();
        let shown = ReadoutFrame::new().label(label).show(ui, |ui, body| {
            let held: Mixed<Rgba> = value.reborrow().read(&mut *body).unwrap_or(Mixed::Mixed);
            let mut events: Vec<EditEvent<Rgba>> = Vec::new();
            Row::new().show(ui, |ui, _| {
                events.extend(TextField::hex(words.hex.as_str().to_owned(), Bind::given(held)).show(ui).event);
                let names = [&words.red, &words.green, &words.blue];
                for (i, name) in names.into_iter().enumerate() {
                    let field = NumberField::new(
                        name.as_str().to_owned(),
                        Bind::given(match held {
                            Mixed::Same(color) => Mixed::Same(Some(f64::from(color.0[i]))),
                            Mixed::Mixed => Mixed::Mixed,
                        }),
                    )
                    .range(0.0..=255.0)
                    .step(1.0)
                    .snap_typed(true)
                    .commit_decimals(0);
                    let base = held.same().copied().unwrap_or(Rgba::rgb(0, 0, 0));
                    let channel = move |number: Option<f64>| {
                        let mut mine = base;
                        if let Some(number) = number {
                            mine.0[i] = number.round().clamp(0.0, 255.0) as u8;
                        }
                        mine
                    };
                    events.extend(field.show(ui).event.map(|event| event.map(channel)));
                }
            });
            for event in &events {
                let _ = value.reborrow().write(&mut *body, event);
            }
            events.into_iter().fold(None, EditEvent::fold)
        });
        ColorReadoutOutput { response: shown.output, event: shown.inner.flatten() }
    }
}

/// The container behind [`ColorReadout`]: the group the fields sit in.
#[derive(craft_ui::contract::Component, craft_ui::contract::Props)]
#[component(container)]
struct ReadoutFrame {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
}

impl ReadoutFrame {
    fn new() -> Self {
        ReadoutFrame { common: Common::new(), props: ContainerProps::new().sizing(Sizing::Content).layout(BodyLayout::stack()) }
    }
}

impl Component for ReadoutFrame {
    const ROLE: Role = Role::GROUP;
    const PARTS: bool = false;
    const STOPS: bool = false;
    const OVERLAYS: bool = false;
    type Style = ColorReadoutStyle;
    type Output = Response;

    fn style(cx: &StyleCx<'_>, _: ReadoutFrameStyleKey) -> ColorReadoutStyle {
        ColorReadoutStyle { metrics: Metrics::new().gap(cx.control().gap) }
    }

    fn sense(&self) -> craft_behavior::Sense {
        craft_behavior::Sense::empty()
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

impl Container for ReadoutFrame {}
