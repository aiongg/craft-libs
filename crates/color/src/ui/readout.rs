//! `ColorReadout`: a colour as a hex field and editable R, G and B fields (the readout of a Color panel).

#[cfg(feature = "screen-testing")]
mod stories;

use craft_behavior::{EditEvent, Mixed};
use craft_theme::Rgba;
use craft_ui::contract::{Bind, Outcome, Pattern, Props as _, Show as _, ShowContainer as _};
use craft_ui::layout_components::{Row, Stack};
use craft_ui::runtime::env::{Env, Label};
use craft_ui::text::{NumberField, TextField};
use egui::{Response, Ui};

/// The channels the fields edit, in field order.
const CHANNELS: [ReadoutChannel; 3] = [ReadoutChannel::Red, ReadoutChannel::Green, ReadoutChannel::Blue];

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

/// A channel of the readout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReadoutChannel {
    /// Red.
    Red,
    /// Green.
    Green,
    /// Blue.
    Blue,
}

impl ReadoutChannel {
    /// The channel's place in an [`Rgba`].
    pub const fn index(self) -> usize {
        self as usize
    }
}

/// An edit of one channel of a mixed colour: the host sets that channel of every item and keeps their other channels
/// and their alpha. The value is `None` while the field holds text that is not a number.
#[derive(Debug)]
#[non_exhaustive]
pub struct ChannelEdit {
    /// The channel the field edits.
    pub channel: ReadoutChannel,
    /// The field's edit, as the field reports it.
    pub event: EditEvent<Option<u8>>,
}

/// What a [`ColorReadout`] reports.
#[derive(Debug)]
#[non_exhaustive]
#[must_use = "the host applies the edit"]
pub struct ColorReadoutOutput {
    /// The readout's response.
    pub response: Response,
    /// The pass's edits of the whole colour folded into one; `None` when nothing was edited. A channel field edits the
    /// whole colour only while the value is the same for every item; the hex field always does.
    pub event: Option<EditEvent<Rgba>>,
    /// The pass's edit of one channel while the value is mixed; `None` when no channel field was edited or the
    /// value is the same for every item.
    pub channel: Option<ChannelEdit>,
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
        facts.edits = self
            .event
            .iter()
            .filter_map(craft_ui::testing::EditKind::of)
            .chain(self.channel.iter().filter_map(|edit| craft_ui::testing::EditKind::of(&edit.event)))
            .collect();
        facts
    }
}

/// A colour as a hex field and three number fields (R, G, B, each 0 to 255), in one row. An edit in any field
/// reports the whole colour, with the other channels and the alpha as given. While the value is mixed, the hex field
/// still sets the whole colour and a channel field reports that channel alone ([`ColorReadoutOutput::channel`]), for
/// the host to set on each item. Each field's gesture is the readout's.
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
        let shown = Stack::new().label(label).show(ui, |ui, body| {
            let held: Mixed<Rgba> = value.reborrow().read(&mut *body).unwrap_or(Mixed::Mixed);
            let mut events: Vec<EditEvent<Rgba>> = Vec::new();
            let mut channel_edit = None;
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
                    let byte = |number: Option<f64>| number.map(|number| number.round().clamp(0.0, 255.0) as u8);
                    let out = field.show(ui);
                    let Some(event) = out.event.as_ref() else { continue };
                    match held {
                        Mixed::Same(base) => events.push(event.clone().map(move |number| {
                            let mut mine = base;
                            if let Some(byte) = byte(number) {
                                mine.0[i] = byte;
                            }
                            mine
                        })),
                        Mixed::Mixed => channel_edit = Some(ChannelEdit { channel: CHANNELS[i], event: event.clone().map(byte) }),
                    }
                }
            });
            for event in &events {
                let _ = value.reborrow().write(&mut *body, event);
            }
            (events.into_iter().fold(None, EditEvent::fold), channel_edit)
        });
        let (event, channel) = shown.inner.unwrap_or((None, None));
        ColorReadoutOutput { response: shown.output, event, channel }
    }
}
