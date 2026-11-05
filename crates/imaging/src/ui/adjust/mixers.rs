//! The channel-mixing adjustments: Black & White, Channel Mixer and Selective Color.

use craft_theme::{Srgba, TypeRole};
use egui::{AsIdSalt, Context, Id, IdSalt, Label, RichText, Ui};

use craft_ui::controls::{Button, Checkbox, RadioSet, Select, Swatch};
use craft_ui::dialog::ColorRequest;
use craft_ui::direction;
use craft_ui::edit::{EditEvent, EditPolicy, EditResponse};
use craft_ui::localized::{Labels as _, LocaleNumbers, UnitKind, Units};
use craft_ui::primitives::{Content, Gap, Stack, Text};
use craft_ui::theme::{Tokens, TokensOf as _};

use super::context::named_group;
use super::sliders::{PartSet, edit_parts, push_part};
use super::{AdjustContext, AdjustField, AdjustLabels, AdjustSliders, AdjustStyle, AdjustTrack};

/// Where Channel Mixer keeps its Output Channel and Selective Color its Colors choice, under the
/// editor's id.
const OUTPUT: &str = "output";
const RANGE: &str = "range";

/// The colour request's key.
const TINT_KEY: &str = "blackWhite.tint";

/// An opaque colour from 8-bit sRGB channels.
fn rgb8(r: u8, g: u8, b: u8) -> Srgba {
    Srgba::rgb(f32::from(r) / 255.0, f32::from(g) / 255.0, f32::from(b) / 255.0)
}

/// The editor's response: marked changed when the event carries a value, as the slider tables do.
fn finish<T>(mut response: egui::Response, event: Option<EditEvent<T>>, submitted: bool, escaped: bool) -> EditResponse<T> {
    if event.as_ref().is_some_and(|event| event.value().is_some()) {
        response.mark_changed();
    }
    EditResponse::new(response, event).submitted(submitted).escaped(escaped)
}

// ---------------------------------------------------------------------------------------------
// Black & White

/// A Black & White value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlackWhite {
    /// Reds, Yellows, Greens, Cyans, Blues, Magentas: −200..=300 %, whole numbers.
    pub weights: [f64; 6],
    /// Tint: colours the grey result with `tint_color`.
    pub tint: bool,
    /// The tint colour; (0.882, 0.827, 0.702) by default.
    pub tint_color: Srgba,
}

impl BlackWhite {
    /// Photoshop's default weights: 40, 60, 40, 60, 20, 80 (PhotoCraft's engine `BW_DEFAULTS`).
    pub const DEFAULT_WEIGHTS: [f64; 6] = [40.0, 60.0, 40.0, 60.0, 20.0, 80.0];
}

impl Default for BlackWhite {
    fn default() -> Self {
        Self { weights: Self::DEFAULT_WEIGHTS, tint: false, tint_color: Srgba::rgb(0.882, 0.827, 0.702) }
    }
}

/// What [`BlackWhiteEditor::show`] reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct BlackWhiteOutput {
    /// The edit of the value, as every editor reports it.
    pub edit: EditResponse<BlackWhite>,
    /// The user clicked the tint swatch: open the colour picker with this request (key
    /// `"blackWhite.tint"`, title "Tint Color") and write the answer into `tint_color`.
    pub pick_color: Option<ColorRequest>,
}

/// Black & White: six colour rows, Tint with its swatch, and Default.
///
/// - Rows Reds … Magentas, −200..300 %, each over a track from black to its colour while dynamic
///   tracks are on: (230, 50, 50), (235, 215, 40), (50, 200, 70), (40, 200, 220), (50, 90, 235),
///   (220, 50, 210). A double-click returns a row to its default weight.
/// - Tint (a `Set`) enables the swatch, which asks for a colour through
///   [`BlackWhiteOutput::pick_color`].
/// - Default restores the six [`BlackWhite::DEFAULT_WEIGHTS`] in one `Set`; tint unchanged.
#[must_use = "show the editor with `show`"]
pub struct BlackWhiteEditor<'a> {
    id_salt: IdSalt,
    value: &'a BlackWhite,
    numbers: &'a LocaleNumbers,
    context: AdjustContext,
    policy: EditPolicy,
    labels: Option<AdjustLabels>,
    enabled: bool,
}

impl<'a> BlackWhiteEditor<'a> {
    /// Draws the editor and returns this frame's edit of the `BlackWhite`.
    pub fn new(id_salt: impl AsIdSalt, value: &'a BlackWhite, numbers: &'a LocaleNumbers) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            value,
            numbers,
            context: AdjustContext::dialog(true),
            policy: EditPolicy::default(),
            labels: None,
            enabled: true,
        }
    }

    /// Sets where the editor is shown (dialog or Properties panel); a dialog by default.
    pub fn context(mut self, context: AdjustContext) -> Self {
        self.context = context;
        self
    }

    /// Sets how the editor reports gestures (live, committed, cancelled).
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Sets the words; the installed language by default.
    pub fn labels(mut self, labels: AdjustLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Turns editing off with `false`. The editor is also off while `ui` is disabled.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Draws the editor and returns this frame's edit and any colour-picker request.
    pub fn show(self, ui: &mut Ui) -> BlackWhiteOutput {
        // pc: crates/ui-egui/src/dialogs/adjust_editors/basic.rs:158-192 `black_white` (colours :160,
        // tint colour :178, Default :184-189).
        // new: the tint colour through `ColorRequest`.
        let labels = self.labels.clone().unwrap_or_else(|| AdjustLabels::of(ui));
        let enabled = self.enabled && ui.is_enabled();
        let value = *self.value;
        let names = [
            ("reds", &labels.reds, rgb8(230, 50, 50)),
            ("yellows", &labels.yellows, rgb8(235, 215, 40)),
            ("greens", &labels.greens, rgb8(50, 200, 70)),
            ("cyans", &labels.cyans, rgb8(40, 200, 220)),
            ("blues", &labels.blues, rgb8(50, 90, 235)),
            ("magentas", &labels.magentas, rgb8(220, 50, 210)),
        ];
        let black = rgb8(0, 0, 0);
        let units = Units::of(ui);
        let fields: Vec<AdjustField> = names
            .iter()
            .enumerate()
            .map(|(i, (key, label, color))| {
                AdjustField::new(key, label.as_str(), -200.0..=300.0, BlackWhite::DEFAULT_WEIGHTS[i])
                    .unit(units.unit(UnitKind::Percent))
                    .track(AdjustTrack::Colors(vec![black, *color]))
            })
            .collect();
        let mut pick_color = None;
        let id = ui.make_persistent_id(self.id_salt);
        let (group, event) = edit_parts(ui, id, &value, self.policy, enabled, |ui, edits, inner| {
            let value = *edits.value();
            named_group(ui, id, labels.black_white.as_str(), enabled, |ui| {
                Stack::new(Gap::Stack)
                    .show(ui, |ui| {
                        let rows = AdjustSliders::new((self.id_salt, "rows"), &fields, &value.weights, self.numbers)
                            .context(self.context)
                            .policy(inner)
                            .enabled(enabled)
                            .parts(ui, edits, |now: &BlackWhite, i, x| {
                                let mut next = *now;
                                next.weights[i] = x;
                                next
                            });
                        let mut tint = value.tint;
                        direction::horizontal(ui, |ui| {
                            let check = Checkbox::bool(labels.tint.as_str(), &mut tint).id_salt((self.id_salt, "tint")).enabled(enabled).show(ui);
                            push_part(edits, &check, false, |tint, now: &BlackWhite| BlackWhite { tint, ..*now });
                            let swatch = Swatch::new((self.id_salt, "swatch"), labels.tint_color.as_str(), value.tint_color.to_rgba())
                                .enabled(enabled && value.tint);
                            if ui.add(swatch).clicked() {
                                pick_color = Some(ColorRequest::new(TINT_KEY, value.tint_color).title(labels.tint_color.as_str()));
                            }
                            if ui.add(Button::new(labels.default_button.as_str()).enabled(enabled).id_salt((self.id_salt, "default"))).clicked() {
                                edits.input(PartSet::new(|now: &BlackWhite| BlackWhite { weights: BlackWhite::DEFAULT_WEIGHTS, ..*now }));
                            }
                        });
                        (rows.submitted, rows.escaped)
                    })
                    .inner
            })
        });
        let (submitted, escaped) = group.inner;
        let edit = finish(group.response, event, submitted, escaped);
        BlackWhiteOutput { edit, pick_color }
    }
}

// ---------------------------------------------------------------------------------------------
// Channel Mixer

/// A Channel Mixer value: per output channel the Red, Green, Blue source weights and the
/// Constant, each −200..=200 %, whole numbers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChannelMixer {
    /// Red output channel: source weights for Red, Green, Blue and the Constant.
    pub red: [f64; 4],
    /// Green output channel, in the same order.
    pub green: [f64; 4],
    /// Blue output channel, in the same order.
    pub blue: [f64; 4],
    /// Monochrome's Gray row; `Some` while Monochrome is on.
    pub gray: Option<[f64; 4]>,
}

impl ChannelMixer {
    /// The Gray row Monochrome starts with: [40, 40, 20, 0] (Photoshop).
    pub const MONOCHROME_START: [f64; 4] = [40.0, 40.0, 20.0, 0.0];
}

impl Default for ChannelMixer {
    fn default() -> Self {
        Self { red: [100.0, 0.0, 0.0, 0.0], green: [0.0, 100.0, 0.0, 0.0], blue: [0.0, 0.0, 100.0, 0.0], gray: None }
    }
}

impl ChannelMixer {
    /// The row the editor shows for output `out` (0 Red, 1 Green, 2 Blue): the Gray row with
    /// Monochrome.
    fn shown_row(&self, out: usize) -> [f64; 4] {
        match (self.gray, out) {
            (Some(gray), _) => gray,
            (None, 0) => self.red,
            (None, 1) => self.green,
            (None, _) => self.blue,
        }
    }

    /// The value with the row shown for `out` replaced, whole numbers.
    fn with_row(mut self, out: usize, row: [f64; 4]) -> Self {
        let row = row.map(f64::round);
        match (self.gray, out) {
            (Some(_), _) => self.gray = Some(row),
            (None, 0) => self.red = row,
            (None, 1) => self.green = row,
            (None, _) => self.blue = row,
        }
        self
    }

    /// The weights of the three sources in `row`, as the Total adds them.
    fn total(row: [f64; 4]) -> f64 {
        row[0] + row[1] + row[2]
    }
}

/// The Total readout's text: the sum with its sign and a percent sign (PhotoCraft `{total:+.0}%`),
/// in the user's digits and word order.
pub(super) fn total_text(units: &Units, labels: &AdjustLabels, total: f64) -> String {
    let whole = total.round();
    let text = units.text(UnitKind::Percent, whole, 0..=0);
    if whole < 0.0 { text } else { craft_i18n::fmt(&labels.plus, &[("n", &text)]) }
}

/// Whether the Total is over 100 %, where the readout turns to the warning colour.
pub(super) fn total_warns(total: f64) -> bool {
    total > 100.0
}

/// Channel Mixer: the Output Channel select, three source rows, the Total readout, Constant and
/// Monochrome.
///
/// - Output Channel (Red, Green, Blue; only Gray with Monochrome) is view state under the
///   editor's id, Red at first.
/// - Rows Red, Green, Blue −200..200 % over a track from black to (225, 50, 50), (50, 190, 70),
///   (60, 100, 235) while dynamic tracks are on; Constant −200..200 % with the plain track.
/// - Total: the sum of the three source weights, "+N%" in `type.mono`, `status.warn.fg` above
///   100 % (AccessKit value adds "over 100 percent"). Not editable.
/// - Monochrome on (a `Set`) seeds `gray` with [`ChannelMixer::MONOCHROME_START`] and shows Gray;
///   off removes it.
#[must_use = "show the editor with `show`"]
pub struct ChannelMixerEditor<'a> {
    id_salt: IdSalt,
    value: &'a ChannelMixer,
    numbers: &'a LocaleNumbers,
    context: AdjustContext,
    policy: EditPolicy,
    labels: Option<AdjustLabels>,
    enabled: bool,
}

impl<'a> ChannelMixerEditor<'a> {
    /// Draws the editor and returns this frame's edit of the `ChannelMixer`.
    pub fn new(id_salt: impl AsIdSalt, value: &'a ChannelMixer, numbers: &'a LocaleNumbers) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            value,
            numbers,
            context: AdjustContext::dialog(true),
            policy: EditPolicy::default(),
            labels: None,
            enabled: true,
        }
    }

    /// Sets where the editor is shown (dialog or Properties panel); a dialog by default.
    pub fn context(mut self, context: AdjustContext) -> Self {
        self.context = context;
        self
    }

    /// Sets how the editor reports gestures (live, committed, cancelled).
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Sets the words; the installed language by default.
    pub fn labels(mut self, labels: AdjustLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Turns editing off with `false`. The editor is also off while `ui` is disabled.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Draws the editor and returns this frame's edit of the `ChannelMixer`.
    pub fn show(self, ui: &mut Ui) -> EditResponse<ChannelMixer> {
        // pc: crates/ui-egui/src/dialogs/adjust_editors/basic.rs:243-290 `channel_mixer` (output
        // select :245-255, rows :262-266, Total :267-272, Monochrome :280-288).
        let labels = self.labels.clone().unwrap_or_else(|| AdjustLabels::of(ui));
        let style = AdjustStyle::of(&Tokens::of(ui));
        let units = Units::of(ui);
        let enabled = self.enabled && ui.is_enabled();
        let id = ui.make_persistent_id(self.id_salt);
        let (group, event) = edit_parts(ui, id, self.value, self.policy, enabled, |ui, edits, inner| {
            let value = *edits.value();
            let mono = value.gray.is_some();
            // Monochrome has the one output, Gray.
            let outputs = if mono { 1 } else { 3 };
            let out = ui.data(|d| d.get_temp::<usize>(id.with(OUTPUT))).unwrap_or(0);
            let out = if out < outputs { out } else { 0 };
            let row = value.shown_row(out);
            // A row starts from the identity of its output (Gray from Photoshop's mix).
            let identity: [f64; 4] = if mono { ChannelMixer::MONOCHROME_START } else { std::array::from_fn(|i| if i == out { 100.0 } else { 0.0 }) };
            let sources =
                [("red", &labels.red, rgb8(225, 50, 50)), ("green", &labels.green, rgb8(50, 190, 70)), ("blue", &labels.blue, rgb8(60, 100, 235))];
            let black = rgb8(0, 0, 0);
            let fields: Vec<AdjustField> = sources
                .iter()
                .enumerate()
                .map(|(i, (key, label, color))| {
                    AdjustField::new(key, label.as_str(), -200.0..=200.0, identity[i])
                        .unit(units.unit(UnitKind::Percent))
                        .track(AdjustTrack::Colors(vec![black, *color]))
                })
                .collect();
            let constant = [AdjustField::new("constant", labels.constant.as_str(), -200.0..=200.0, 0.0).unit(units.unit(UnitKind::Percent))];
            // Each output has its own slider state.
            let shown = if mono { 3 } else { out };
            let mut picked = out;
            let set = move |now: &ChannelMixer, i: usize, x: f64| {
                let mut row = now.shown_row(out);
                row[i] = x;
                now.with_row(out, row)
            };
            let group = named_group(ui, id, labels.channel_mixer.as_str(), enabled, |ui| {
                Stack::new(Gap::Stack)
                    .show(ui, |ui| {
                        direction::horizontal(ui, |ui| {
                            ui.add(Text::new(labels.output_channel.as_str()).role(TypeRole::Label).content(Content::Muted));
                            let mut select = Select::new((self.id_salt, "output"), labels.output_channel.as_str(), &mut picked);
                            if mono {
                                select = select.option(0, labels.gray.as_str());
                            } else {
                                for (i, name) in [&labels.red, &labels.green, &labels.blue].into_iter().enumerate() {
                                    select = select.option(i, name.as_str());
                                }
                            }
                            let _ = select.enabled(enabled).show(ui);
                        });
                        let table = AdjustSliders::new((self.id_salt, "rows", shown), &fields, &row[..3], self.numbers)
                            .context(self.context)
                            .policy(inner)
                            .enabled(enabled)
                            .parts(ui, edits, set);
                        let total = ChannelMixer::total(row);
                        let warn = total_warns(total);
                        direction::horizontal(ui, |ui| {
                            ui.add(Text::new(labels.total.as_str()).role(TypeRole::Label).content(Content::Muted));
                            let color = if warn { style.total_warn } else { style.total_color };
                            let shown = total_text(&units, &labels, total);
                            let readout =
                                ui.add(Label::new(RichText::new(shown.as_str()).font(style.total_font.clone()).color(color)).selectable(false));
                            let named = if warn { craft_i18n::fmt(&labels.total_over, &[("value", &shown)]) } else { shown.clone() };
                            ui.ctx().accesskit_node_builder(readout.id, |node| {
                                node.set_label(labels.total.as_str());
                                node.set_value(named);
                            });
                        });
                        let offset = AdjustSliders::new((self.id_salt, "constant", shown), &constant, &row[3..], self.numbers)
                            .context(self.context)
                            .policy(inner)
                            .enabled(enabled)
                            .parts(ui, edits, move |now: &ChannelMixer, _, x| set(now, 3, x));
                        let mut monochrome = mono;
                        let check = Checkbox::bool(labels.monochrome.as_str(), &mut monochrome)
                            .id_salt((self.id_salt, "monochrome"))
                            .enabled(enabled)
                            .show(ui);
                        push_part(edits, &check, false, |on, now: &ChannelMixer| ChannelMixer {
                            gray: on.then_some(ChannelMixer::MONOCHROME_START),
                            ..*now
                        });
                        (table.submitted | offset.submitted, table.escaped | offset.escaped)
                    })
                    .inner
            });
            ui.data_mut(|d| d.insert_temp(id.with(OUTPUT), picked));
            group
        });
        let (submitted, escaped) = group.inner;
        finish(group.response, event, submitted, escaped)
    }
}

// ---------------------------------------------------------------------------------------------
// Selective Color

/// How Selective Color applies its percentages.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SelectiveMethod {
    /// Relative to the ink already there; the default.
    #[default]
    Relative,
    #[allow(missing_docs)]
    Absolute,
}

/// A Selective Color value: for each of Reds, Yellows, Greens, Cyans, Blues, Magentas, Whites,
/// Neutrals, Blacks the Cyan, Magenta, Yellow, Black shifts, −100..=100 %, whole numbers.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SelectiveColor {
    /// Per colour range (Reds, Yellows, Greens, Cyans, Blues, Magentas, Whites, Neutrals, Blacks): Cyan, Magenta, Yellow, Black percentages.
    pub ranges: [[f64; 4]; 9],
    /// How the percentages apply.
    pub method: SelectiveMethod,
}

/// Selective Color: the Colors select (nine ranges), four rows for the shown range, and the
/// Relative / Absolute radios.
///
/// - Colors is view state under the editor's id, Reds at first; rows edit only the shown range.
/// - Rows Cyan, Magenta, Yellow, Black −100..100 % with plain tracks; double-click to 0.
/// - Relative / Absolute (a `RadioSet`) is one `Set`.
#[must_use = "show the editor with `show`"]
pub struct SelectiveColorEditor<'a> {
    id_salt: IdSalt,
    value: &'a SelectiveColor,
    numbers: &'a LocaleNumbers,
    context: AdjustContext,
    policy: EditPolicy,
    labels: Option<AdjustLabels>,
    enabled: bool,
}

impl<'a> SelectiveColorEditor<'a> {
    /// Draws the editor and returns this frame's edit of the `SelectiveColor`.
    pub fn new(id_salt: impl AsIdSalt, value: &'a SelectiveColor, numbers: &'a LocaleNumbers) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            value,
            numbers,
            context: AdjustContext::dialog(true),
            policy: EditPolicy::default(),
            labels: None,
            enabled: true,
        }
    }

    /// Sets where the editor is shown (dialog or Properties panel); a dialog by default.
    pub fn context(mut self, context: AdjustContext) -> Self {
        self.context = context;
        self
    }

    /// Sets how the editor reports gestures (live, committed, cancelled).
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Sets the words; the installed language by default.
    pub fn labels(mut self, labels: AdjustLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Turns editing off with `false`. The editor is also off while `ui` is disabled.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// The editor's id in `ui`, for [`shown_range`](Self::shown_range).
    pub fn id_in(ui: &Ui, id_salt: impl AsIdSalt) -> Id {
        ui.make_persistent_id(IdSalt::new(id_salt))
    }

    /// The range the editor with `id` shows, 0 Reds … 8 Blacks.
    pub fn shown_range(ctx: &Context, id: Id) -> usize {
        // pc: crates/ui-egui/src/panels/adjust_ui.rs:38-39 (`selc-range` in memory).
        ctx.data(|d| d.get_temp::<usize>(id.with(RANGE))).unwrap_or(0).min(8)
    }

    /// Draws the editor and returns this frame's edit of the `SelectiveColor`.
    pub fn show(self, ui: &mut Ui) -> EditResponse<SelectiveColor> {
        // pc: crates/ui-egui/src/panels/adjust_ui.rs:17 (the nine ranges), :31-81
        // `selective_color_editor` (select :40-46, rows :50-59, method :61-71).
        let labels = self.labels.clone().unwrap_or_else(|| AdjustLabels::of(ui));
        let enabled = self.enabled && ui.is_enabled();
        let id = ui.make_persistent_id(self.id_salt);
        let value = *self.value;
        let range = Self::shown_range(ui.ctx(), id);
        let names = [
            &labels.reds,
            &labels.yellows,
            &labels.greens,
            &labels.cyans,
            &labels.blues,
            &labels.magentas,
            &labels.whites,
            &labels.neutrals,
            &labels.blacks,
        ];
        let units = Units::of(ui);
        let fields: Vec<AdjustField> = [("cyan", &labels.cyan), ("magenta", &labels.magenta), ("yellow", &labels.yellow), ("black", &labels.black)]
            .into_iter()
            .map(|(key, label)| AdjustField::new(key, label.as_str(), -100.0..=100.0, 0.0).unit(units.unit(UnitKind::Percent)))
            .collect();
        let mut picked = range;
        let (group, event) = edit_parts(ui, id, &value, self.policy, enabled, |ui, edits, inner| {
            let value = *edits.value();
            named_group(ui, id, labels.selective_color.as_str(), enabled, |ui| {
                Stack::new(Gap::Stack)
                    .show(ui, |ui| {
                        direction::horizontal(ui, |ui| {
                            ui.add(Text::new(labels.colors.as_str()).role(TypeRole::Label).content(Content::Muted));
                            let mut select = Select::new((self.id_salt, "colors"), labels.colors.as_str(), &mut picked);
                            for (i, name) in names.into_iter().enumerate() {
                                select = select.option(i, name.as_str());
                            }
                            let _ = select.enabled(enabled).show(ui);
                        });
                        let table = AdjustSliders::new((self.id_salt, "rows", range), &fields, &value.ranges[range], self.numbers)
                            .context(self.context)
                            .policy(inner)
                            .enabled(enabled)
                            .parts(ui, edits, move |now: &SelectiveColor, i, x| {
                                let mut next = *now;
                                next.ranges[range][i] = x.round();
                                next
                            });
                        let mut method = value.method;
                        let mut set = RadioSet::new((self.id_salt, "method"), labels.relative_absolute.as_str(), &mut method)
                            .item(SelectiveMethod::Relative)
                            .item(SelectiveMethod::Absolute)
                            .enabled(enabled)
                            .begin(ui);
                        direction::horizontal(ui, |ui| {
                            set.radio(ui, 0, &labels.relative);
                            set.radio(ui, 1, &labels.absolute);
                        });
                        let radios = set.end(ui);
                        push_part(edits, &radios, false, |method, now: &SelectiveColor| SelectiveColor { method, ..*now });
                        (table.submitted, table.escaped)
                    })
                    .inner
            })
        });
        // An edit belongs to the range it was drawn for; a switch shows from the next pass.
        ui.data_mut(|d| d.insert_temp(id.with(RANGE), picked));
        let (submitted, escaped) = group.inner;
        finish(group.response, event, submitted, escaped)
    }
}
