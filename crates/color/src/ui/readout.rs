//! [`ColorReadout`]: a colour as a hex field and editable R, G and B fields (PhotoCraft's Color
//! panel readout).

use craft_theme::{Rgba, TypeRole};
use egui::text::LayoutJob;
use egui::{Align, AsIdSalt, Color32, IdSalt, Layout, Ui, vec2};

use super::HexField;
use craft_ui::controls::NumberField;
use craft_ui::direction;
use craft_ui::edit::{EditEvent, EditPolicy, EditResponse};
use craft_ui::localized::LocaleNumbers;
use craft_ui::primitives::{Content, Gap, Text};
use craft_ui::theme::{Tokens, TokensOf as _, UiTokens, role_format};

/// The width of `text` in `role`.
fn text_width(ui: &Ui, tokens: &UiTokens, role: TypeRole, text: &str) -> f32 {
    ui.painter().layout_job(LayoutJob::single_section(text.to_owned(), role_format(tokens, role, Color32::PLACEHOLDER))).size().x
}

/// The readout: a muted `#` and the hex field (six digits, each as wide as its widest text and the control padding), then "R", "G" and "B" each before a number field of 0 to 255. An
/// edit in any field reports the whole colour, with the other channels and the alpha as given.
/// Each field is its own gesture, so the events of a typing run in one field are that run's.
///
/// ```
/// use craft_theme::Rgba;
/// use craft_color::ui::ColorReadout;
/// use craft_ui::localized::LocaleNumbers;
/// # egui::__run_test_ui(|ui| { let mut color = Rgba::rgb(0x33, 0x66, 0x99);
/// let numbers = LocaleNumbers::from_tag("en");
/// let edit = ColorReadout::new("readout", "Hex color", color, &numbers).show(ui);
/// if let Some(c) = edit.event.as_ref().and_then(|e| e.value()) {
///     color = *c;
/// }
/// # });
/// ```
///
/// AccessKit: the hex field's `TextInput` named `hex_label`, and a `SpinButton` per channel named
/// by [`names`](ColorReadout::names) ("Red", "Green", "Blue" by default). The `#` and the channel letters are
/// decoration. The row wraps when the width is short. The response covers the row.
#[derive(Clone, Debug)]
#[must_use = "show the readout with `show`"]
pub struct ColorReadout<'a> {
    id_salt: IdSalt,
    hex_label: String,
    value: Rgba,
    numbers: &'a LocaleNumbers,
    names: [String; 3],
    policy: EditPolicy,
    enabled: bool,
}

impl<'a> ColorReadout<'a> {
    /// A readout of `value`; `hex_label` names the hex field (translated by the host: "Hex
    /// colour"), `id_salt` keeps the fields' drafts and focus when the layout around it changes.
    pub fn new(id_salt: impl AsIdSalt, hex_label: impl Into<String>, value: Rgba, numbers: &'a LocaleNumbers) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            hex_label: hex_label.into(),
            value,
            numbers,
            names: ["Red", "Green", "Blue"].map(str::to_owned),
            policy: EditPolicy::default(),
            enabled: true,
        }
    }

    /// The channel fields' accessible names, translated by the host.
    pub fn names(mut self, names: [String; 3]) -> Self {
        self.names = names;
        self
    }

    /// How the gestures report ([`EditPolicy`]); the default reports every change at once.
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Turns input on or off; on by default.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Shows the readout and returns this frame's edit.
    pub fn show(self, ui: &mut Ui) -> EditResponse<Rgba> {
        let value = self.value;
        let mut event = None;
        let tokens = Tokens::of(ui);
        let pad = 2.0 * tokens.space.pad.control;
        let hex_width = (text_width(ui, &tokens, TypeRole::Mono, "#RRGGBB") + pad).ceil();
        let channel_width = (text_width(ui, &tokens, TypeRole::Body, "255") + pad).ceil();
        let gap = Gap::Inline.points(&tokens);
        // Wrapped, so the fields fold onto a second line only in a dock narrower than the row.
        let layout = direction::row(ui).with_main_wrap(true);
        let inner = ui.scope_builder(egui::UiBuilder::new().id_salt(self.id_salt).layout(layout), |ui| {
            ui.spacing_mut().item_spacing.x = gap;
            ui.add(Text::new("#").role(TypeRole::Mono).content(Content::Muted));
            let hex = HexField::new("hex", self.hex_label.as_str(), value)
                .prefix(false)
                .width(hex_width)
                .policy(self.policy)
                .enabled(self.enabled)
                .show(ui);
            event = event.take().or(hex.event);
            for (i, (label, name)) in ["R", "G", "B"].into_iter().zip(&self.names).enumerate() {
                ui.add(Text::new(label).role(TypeRole::Mono).content(Content::Muted));
                let channel = f64::from(value.0[i]);
                let edit = ui
                    .allocate_ui_with_layout(vec2(channel_width, ui.spacing().interact_size.y), Layout::top_down_justified(Align::Min), |ui| {
                        NumberField::new(("channel", i), name.as_str(), channel, self.numbers)
                            .range(0.0..=255.0)
                            .step(1.0)
                            .decimals(0..=0)
                            .policy(self.policy)
                            .enabled(self.enabled)
                            .show(ui)
                    })
                    .inner;
                let with = |v: f64| {
                    let mut rgba = value;
                    rgba.0[i] = v.round().clamp(0.0, 255.0) as u8;
                    rgba
                };
                event = event.take().or(edit.event.map(|e: EditEvent<f64>| e.map(with)));
            }
        });
        EditResponse::new(inner.response, event)
    }
}
