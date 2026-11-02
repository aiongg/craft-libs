//! [`CurvesPanel`]: the Curves adjustment around the kernel's [`CurveEditor`]: the Channel select,
//! the mode icons (points, pencil), Smooth, the point pickers, the other channels overlaid on the
//! composite and the hint line; and the ⌘/Ctrl-click-on-the-image helper
//! ([`CurvesPanel::add_point_from_sample`]) (task AD-2).
//!
//! [`CurveEditor`]: craft_ui::composites::CurveEditor

use craft_theme::{ButtonVariant, ControlSize, Srgba, TypeRole};
use egui::accesskit::Role;
use egui::{AsIdSalt, Context, Id, IdSalt, Key, KeyboardShortcut, Modifiers, Ui, UiBuilder};

use craft_ui::composites::{ChannelColor, Curve, CurveChannel, CurveEditor, CurveLabels, CurveRules, HistogramData, Sampler};
use craft_ui::controls::{Button, IconButton, Kbd, Select, TooltipExt as _};
use craft_ui::direction;
use craft_ui::edit::{EditEvent, EditPolicy, EditResponse};
use craft_ui::icons::Icon;
use craft_ui::localized::{Labels as _, LocaleNumbers, Words};
use craft_ui::primitives::{Content, Gap, Stack, Text};

use super::{AdjustContext, AdjustLabels, ToneChannel, ToneChannels, TonePicker, TonePickerRow};

/// The key of the shown channel in egui memory, under the panel's id.
const CHANNEL: &str = "channel";
/// The key of the inner editor's id in egui memory, under the panel's id.
const EDITOR: &str = "editor";
/// The levels of a Curves value (8-bit tones) and the most points Photoshop allows.
const RULES: CurveRules = CurveRules { levels: 255.0, max_points: 16 };
/// Smooth's moving-average radius (PhotoCraft's `smooth_curve_map`).
const SMOOTH_RADIUS: usize = 3;

/// The map `outputs` smoothed by a moving average over `2 * SMOOTH_RADIUS + 1` levels, the window
/// shrunk at the ends so the first and last levels keep their values.
fn smoothed(outputs: &[f32]) -> Vec<f32> {
    let n = outputs.len();
    (0..n)
        .map(|k| {
            let r = SMOOTH_RADIUS.min(k).min(n - k - 1);
            match outputs.get(k - r..=k + r) {
                Some(window) if r > 0 => window.iter().sum::<f32>() / window.len() as f32,
                _ => outputs[k],
            }
        })
        .collect()
}

/// The curves the editor overlays on the shown channel: every channel's on a composite
/// (Photoshop), none on a single channel or an isolated one.
pub(super) fn overlays_for<'a>(channels: &ToneChannels, shown: usize, curves: &'a [Curve]) -> &'a [Curve] {
    if channels.is_composite(shown) && !channels.is_isolated() { curves } else { &[] }
}

/// The modifier words of `modifiers` in the platform's notation (`⌘`, `Ctrl`, `Shift`).
fn modifier_words(ui: &Ui, modifiers: Modifiers) -> String {
    let mut parts = Kbd::parts_in(&KeyboardShortcut::new(modifiers, Key::A), ui.ctx().os(), &Words::of(ui));
    parts.pop();
    parts.concat()
}

/// How the shown curve is edited: Photoshop's two buttons above the graph.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CurvesMode {
    /// Edit points to modify the curve (`Icon::Spline`); every channel is a [`Curve::Points`].
    #[default]
    Points,
    /// Draw to modify the curve (`Icon::Pencil`); every channel is a [`Curve::Map`].
    Pencil,
}

/// A Curves value: one curve per channel of the panel's [`ToneChannels`] (same index), all in
/// one [`CurvesMode`].
#[derive(Clone, Debug, PartialEq)]
pub struct ToneCurves {
    pub curves: Vec<Curve>,
    pub mode: CurvesMode,
}

impl ToneCurves {
    /// Identity point curves for `channels`, 0..=255.
    pub fn identity(channels: &ToneChannels) -> Self {
        Self { curves: vec![Curve::identity(255.0); channels.len()], mode: CurvesMode::Points }
    }
}

/// What [`CurvesPanel::show`] reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct CurvesPanelOutput {
    /// The shown channel's curve edit: the editor's gestures, and Smooth as one `Set` of the
    /// smoothed map. The response is the panel's group.
    pub edit: EditResponse<Curve>,
    /// The channel the edit is for, and the one shown (an index into the panel's
    /// [`ToneChannels`]); the panel keeps it between passes.
    pub channel: usize,
    /// The user clicked the other mode's icon. The host converts every channel with its engine
    /// (draws the point curves into maps, or fits points to the drawn maps, PhotoCraft
    /// `set_pencil`) and passes the new value back; the panel changes nothing itself.
    pub mode: Option<CurvesMode>,
    /// The point picker the user armed (`Some(Some(p))`) or disarmed (`Some(None)`) this pass.
    pub picker: Option<Option<TonePicker>>,
}

/// The Curves adjustment: Image › Adjustments › Curves and a Curves layer's Properties panel.
///
/// Top to bottom:
/// 1. The Channel select ([`ToneChannels`]); an [isolated](ToneChannels::isolated) channel is
///    the only entry, its name shown exactly as the host gave it.
/// 2. The mode row: two icon buttons (`Icon::Spline` "Edit points to modify the curve",
///    `Icon::Pencil` "Draw to modify the curve", 24 pt, the current one selected) and Smooth
///    (a secondary button, enabled in pencil mode only, tooltip "Smooth the curve values").
/// 3. The [`CurveEditor`](craft_ui::composites::CurveEditor) for the shown channel: in its
///    colour, over its histogram, `.pencil(true)` in pencil mode, and, on a composite only, the
///    other channels' curves as overlays (Photoshop; none for an isolated channel).
/// 4. The hint line (`type.caption`, `text.faint`): points mode "Click to add a point · drag off
///    or {⌘}-click or Delete to remove", pencil mode "Drag to draw the curve · {Shift}-click
///    draws a straight line", the key names from the platform's shortcut words.
/// 5. With [`pickers`](Self::pickers), the point picker row.
///
/// - **Smooth** replaces the shown map with its moving average over 7 levels (radius 3, shrunk
///   at the ends so the first and last levels keep their values: PhotoCraft's
///   `smooth_curve_map`), as one `Set`.
/// - **Mode icons** report [`CurvesPanelOutput::mode`]; the current mode's icon reports nothing.
/// - **Keyboard**: Tab visits the Channel select, the two mode icons, Smooth, the editor and the
///   pickers. The editor's own keys (Insert, ⌘/Ctrl+Enter, arrows) are unchanged.
/// - **AccessKit**: a `Group` named "Curves"; the mode icons are toggle `Button`s.
#[must_use = "show the panel with `show`"]
pub struct CurvesPanel<'a> {
    id_salt: IdSalt,
    curves: &'a ToneCurves,
    channels: ToneChannels,
    numbers: &'a LocaleNumbers,
    histograms: &'a [[u32; 256]],
    sampler: Option<&'a Sampler>,
    picker: Option<TonePicker>,
    pickers: bool,
    picker_enabled: [bool; 3],
    unavailable: bool,
    context: AdjustContext,
    policy: EditPolicy,
    labels: Option<AdjustLabels>,
    curve_labels: Option<CurveLabels>,
    enabled: bool,
}

impl<'a> CurvesPanel<'a> {
    /// The panel over `curves`, one curve per channel of `channels`.
    pub fn new(id_salt: impl AsIdSalt, curves: &'a ToneCurves, channels: ToneChannels, numbers: &'a LocaleNumbers) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            curves,
            channels,
            numbers,
            histograms: &[],
            sampler: None,
            picker: None,
            pickers: true,
            picker_enabled: [true; 3],
            unavailable: false,
            context: AdjustContext::dialog(true),
            policy: EditPolicy::default(),
            labels: None,
            curve_labels: None,
            enabled: true,
        }
    }

    /// The host's histograms, one per channel (same index), 256 bins each.
    pub fn histograms(mut self, histograms: &'a [[u32; 256]]) -> Self {
        self.histograms = histograms;
        self
    }

    /// The host's interpolation of point curves ([`CurveEditor::sampler`](craft_ui::composites::CurveEditor::sampler)).
    pub fn sampler(mut self, sampler: &'a Sampler) -> Self {
        self.sampler = Some(sampler);
        self
    }

    /// The armed point picker, shown selected.
    pub fn picker(mut self, armed: Option<TonePicker>) -> Self {
        self.picker = armed;
        self
    }

    /// Whether the point picker row shows; on by default.
    pub fn pickers(mut self, on: bool) -> Self {
        self.pickers = on;
        self
    }

    /// Enables or disables one picker.
    pub fn picker_enabled(mut self, picker: TonePicker, enabled: bool) -> Self {
        self.picker_enabled[picker as usize] = enabled;
        self
    }

    /// The document's mode has no point pickers: all disabled, with the note.
    pub fn pickers_unavailable(mut self, unavailable: bool) -> Self {
        self.unavailable = unavailable;
        self
    }

    /// Dialog or Properties panel; a dialog by default.
    pub fn context(mut self, context: AdjustContext) -> Self {
        self.context = context;
        self
    }

    /// The editor's edit policy.
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// The panel's words; the installed ones by default.
    pub fn labels(mut self, labels: AdjustLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The curve editor's own words.
    pub fn curve_labels(mut self, labels: CurveLabels) -> Self {
        self.curve_labels = Some(labels);
        self
    }

    /// Enables or disables the whole panel.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// The panel's id in `ui`, for [`shown_channel`](Self::shown_channel) and
    /// [`select_point`](Self::select_point).
    pub fn id_in(ui: &Ui, id_salt: impl AsIdSalt) -> Id {
        ui.make_persistent_id(IdSalt::new(id_salt))
    }

    /// The channel the panel with `id` shows (0 before it was first drawn).
    pub fn shown_channel(ctx: &Context, id: Id) -> usize {
        // pc: crates/ui-egui/src/dialogs/adjust_editors/curves.rs:50-55 (the choice in egui memory).
        ctx.data(|d| d.get_temp::<usize>(id.with(CHANNEL))).unwrap_or(0)
    }

    /// Selects point `index` of the shown curve in the panel with `id`, as a click on it would
    /// (after [`add_point_from_sample`](Self::add_point_from_sample)); through
    /// [`CurveEditor::select`](craft_ui::composites::CurveEditor::select) with the inner editor's id.
    pub fn select_point(ctx: &Context, id: Id, index: usize) {
        // pc: crates/ui-egui/src/dialogs/adjust_dialog.rs:153-161 (the new point selected).
        // The editor wrote its id while drawing; before the first pass there is nothing to select in.
        if let Some(editor) = ctx.data(|d| d.get_temp::<Id>(id.with(EDITOR))) {
            CurveEditor::select(ctx, editor, Some(index));
        }
    }

    /// ⌘/Ctrl-click on the image while Curves is open: adds a point to the shown channel's curve
    /// at the sampled tone, on the curve (its output there), and returns its index for
    /// [`select_point`](Self::select_point). With `every_channel` (Shift) it adds one to each of
    /// Red, Green and Blue instead, at that channel's value, and returns the shown channel's
    /// point if the shown channel is one of them.
    ///
    /// - The tone, 0..=255 rounded to a whole level: Red, Green, Blue read their channel; the
    ///   composite reads Rec. 709 luma (0.2126 R + 0.7152 G + 0.0722 B); a grey document the mean
    ///   of R, G, B.
    /// - The output is the curve's at that input (through `sampler`, the host's interpolation),
    ///   rounded; the point goes in through [`CurveRules::insert`](craft_ui::composites::CurveRules::insert),
    ///   so a point already at that input is reused and the 16-point limit holds.
    /// - Does nothing and returns `None` in pencil mode, for Lab and CMYK channels, for an
    ///   isolated mask or saved channel ([`ToneChannel::Named`]: it holds its own grey levels;
    ///   an isolated Red, Green or Blue channel takes the point) and when `sample` is off the image
    ///   (the host doesn't call it then). `every_channel` is ignored for a grey document.
    pub fn add_point_from_sample(
        curves: &mut ToneCurves,
        channels: &ToneChannels,
        shown: usize,
        sample: Srgba,
        every_channel: bool,
        sampler: &Sampler,
    ) -> Option<usize> {
        // pc: crates/ui-egui/src/dialogs/adjust_dialog.rs:95-175 `add_point_at` (pencil, mode, isolated
        // checks; the tone per channel; the output from the curve's table, rounded; insert).
        if curves.mode == CurvesMode::Pencil {
            return None;
        }
        let list = channels.as_slice();
        let shown_channel = list.get(shown)?;
        let gray = channels.is_gray();
        let level = |channel: &ToneChannel| -> Option<f32> {
            Some(match channel {
                ToneChannel::Red => sample.r,
                ToneChannel::Green => sample.g,
                ToneChannel::Blue => sample.b,
                ToneChannel::Composite(super::ToneSpace::Rgb) => sample.r * 0.2126 + sample.g * 0.7152 + sample.b * 0.0722,
                ToneChannel::Gray => (sample.r + sample.g + sample.b) / 3.0,
                _ => return None,
            })
        };
        level(shown_channel)?;
        let targets: Vec<usize> = if every_channel && !gray {
            list.iter().enumerate().filter(|(_, c)| matches!(c, ToneChannel::Red | ToneChannel::Green | ToneChannel::Blue)).map(|(i, _)| i).collect()
        } else {
            vec![shown]
        };
        let mut selected = None;
        for index in targets {
            let Some(tone) = list.get(index).and_then(level) else { continue };
            let input = (tone.clamp(0.0, 1.0) * 255.0).round();
            let Some(Curve::Points { points, corners }) = curves.curves.get_mut(index) else { continue };
            let output = sampler(points).get(input as usize).copied().unwrap_or(input).round();
            corners.resize(points.len(), false);
            let before = points.len();
            let Some(at) = RULES.insert(points, [input, output]) else { continue };
            if points.len() > before {
                corners.insert(at, false);
            }
            if index == shown {
                selected = Some(at);
            }
        }
        selected
    }

    /// Draws the panel and reports the shown channel's edit.
    pub fn show(self, ui: &mut Ui) -> CurvesPanelOutput {
        // pc: crates/ui-egui/src/dialogs/adjust_editors/curves.rs:225-249 `curves_mode_row` (icons 24,
        // Smooth enabled in pencil mode), :251-272 (Channel select, isolated channel), :343-350
        // (overlays on the composite only), :397-402 (hint, 11 pt `text_faint`); the smoothing is
        // engine `adjust_params.rs:548-560` `smooth_curve_map` (radius 3).
        // lib: `CurveEditor::new(..).channels(..).channel(..).pencil(..).overlays(..).histogram(..)`.
        let labels = self.labels.clone().unwrap_or_else(|| AdjustLabels::of(ui));
        let id = ui.make_persistent_id(self.id_salt);
        let enabled = self.enabled && ui.is_enabled();
        let count = self.channels.len();
        let channel = Self::shown_channel(ui.ctx(), id).min(count.saturating_sub(1));
        let pencil = self.curves.mode == CurvesMode::Pencil;
        let fallback = Curve::identity(255.0);
        let curve = self.curves.curves.get(channel).unwrap_or(&fallback);
        let tone = self.channels.as_slice().get(channel);
        let color = tone.map_or(ChannelColor::Neutral, ToneChannel::color);
        let shown_name = tone.map(|c| c.label(&labels).to_owned()).unwrap_or_default();
        let data = self.histograms.get(channel).map(|counts| HistogramData::tone(counts, color));
        let curve_channels: Vec<CurveChannel> = self.channels.as_slice().iter().map(|c| CurveChannel::new(c.label(&labels), c.color())).collect();
        // Photoshop overlays the other channels on the composite only.
        let overlays = overlays_for(&self.channels, channel, &self.curves.curves);
        let hint = if pencil {
            labels.pencil_hint.replace("{key}", &modifier_words(ui, Modifiers::SHIFT))
        } else {
            labels.points_hint.replace("{key}", &modifier_words(ui, Modifiers::COMMAND))
        };
        let editor_salt = (self.id_salt, "curve");

        let mut builder = UiBuilder::new().scope_id(id).accessibility_role(Role::Group).accessibility_label(labels.curves.as_str());
        if !enabled {
            builder = builder.disabled();
        }
        let group = ui.scope_builder(builder, |ui| {
            Stack::new(Gap::Stack)
                .show(ui, |ui| {
                    let mut picked = channel;
                    direction::horizontal(ui, |ui| {
                        ui.add(Text::new(labels.channel.as_str()).role(TypeRole::Label).content(Content::Muted));
                        let mut select = Select::new((self.id_salt, "channel"), labels.channel.as_str(), &mut picked);
                        for (i, entry) in self.channels.as_slice().iter().enumerate() {
                            select = select.option(i, entry.label(&labels));
                        }
                        let _ = select.enabled(enabled).show(ui);
                    });
                    let mut mode = None;
                    let mut smooth = false;
                    direction::horizontal(ui, |ui| {
                        for (target, icon, name) in
                            [(CurvesMode::Points, Icon::Spline, &labels.edit_points), (CurvesMode::Pencil, Icon::Pencil, &labels.draw_curve)]
                        {
                            let current = self.curves.mode == target;
                            let button = IconButton::new(icon, name.as_str()).size(ControlSize::Sm).selected(current).enabled(enabled).id_salt((
                                self.id_salt,
                                "mode",
                                target as u8,
                            ));
                            if ui.add(button).clicked() && !current {
                                mode = Some(target);
                            }
                        }
                        let can_smooth = enabled && pencil && matches!(curve, Curve::Map(_));
                        let button = Button::new(labels.smooth.as_str())
                            .variant(ButtonVariant::Secondary)
                            .enabled(can_smooth)
                            .id_salt((self.id_salt, "smooth"));
                        smooth = ui.add(button).tooltip(labels.smooth_tip.as_str()).clicked();
                    });
                    let mut editor = CurveEditor::new(editor_salt, shown_name.as_str(), curve, self.numbers)
                        .channels(&curve_channels)
                        .channel(channel)
                        .pencil(pencil)
                        .overlays(overlays)
                        .hint(hint.as_str())
                        .policy(self.policy)
                        .enabled(enabled);
                    if let Some(data) = &data {
                        editor = editor.histogram(data);
                    }
                    if let Some(sampler) = self.sampler {
                        editor = editor.sampler(sampler);
                    }
                    if let Some(curve_labels) = &self.curve_labels {
                        editor = editor.labels(curve_labels.clone());
                    }
                    let edit = editor.show(ui);
                    // The editor's id as drawn: it sits in this group's scope, not the caller's.
                    ui.data_mut(|d| d.insert_temp(id.with(EDITOR), edit.response.id));
                    let mut change = None;
                    if self.pickers {
                        let mut row = TonePickerRow::new((self.id_salt, "pickers"), self.picker).unavailable(self.unavailable).labels(labels.clone());
                        for picker in TonePicker::ALL {
                            let usable = self.picker_enabled[picker as usize]
                                && !(picker == TonePicker::Gray && (self.channels.is_gray() || self.context.is_gray()));
                            row = row.enabled(picker, usable);
                        }
                        change = row.show(ui).change;
                    }
                    let smoothed = match curve {
                        Curve::Map(map) if smooth => Some(EditEvent::set(Curve::Map(smoothed(map)), Modifiers::NONE)),
                        _ => None,
                    };
                    (edit, smoothed, picked, mode, change)
                })
                .inner
        });
        let rect = group.response.rect;
        // A `Ui`'s node gets no bounds of its own from egui.
        let bounds = egui::accesskit::Rect { x0: rect.min.x.into(), y0: rect.min.y.into(), x1: rect.max.x.into(), y1: rect.max.y.into() };
        ui.ctx().accesskit_node_builder(id, |node| node.set_bounds(bounds));
        let (edit, smoothed, picked, mode, picker) = group.inner;
        let (submitted, escaped) = (edit.submitted, edit.escaped);
        let event = smoothed.or(edit.event);
        // An edit belongs to the channel it was drawn for; a switch shows from the next pass.
        let shown = if picked == channel || event.is_some() { channel } else { picked };
        if picked != channel {
            ui.data_mut(|d| d.insert_temp(id.with(CHANNEL), picked));
        }
        let mut edit = EditResponse::new(group.response, event);
        edit.submitted = submitted;
        edit.escaped = escaped;
        CurvesPanelOutput { edit, channel: shown, mode, picker }
    }
}
