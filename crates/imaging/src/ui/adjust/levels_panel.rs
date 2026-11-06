//! [`LevelsPanel`]: the Levels adjustment around the [`LevelsEditor`]: the Channel select, Auto,
//! the point pickers and the editor over the shown channel's histogram.

use craft_theme::TypeRole;
use egui::accesskit::Role;
use egui::{AsIdSalt, Context, Id, IdSalt, Modifiers, Ui, UiBuilder};

use craft_ui::composites::HistogramData;
use craft_ui::controls::{Button, Select};
use craft_ui::direction;
use craft_ui::edit::{EditEvent, EditPolicy, EditResponse};
use craft_ui::localized::{Labels as _, LocaleNumbers};
use craft_ui::primitives::{Content, Gap, Stack, Text};

use super::{AdjustContext, AdjustLabels, Levels, LevelsEditor, LevelsLabels, ToneChannels, TonePicker, TonePickerRow};

/// The key of the shown channel in egui memory, under the panel's id.
const CHANNEL: &str = "channel";

impl Levels {
    /// Photoshop's Auto: the black and white inputs clip 0.1 % of the pixels at each end of
    /// `counts` (256 bins over the levels), gamma back to 1, the outputs kept.
    ///
    /// PhotoCraft's rule, copied exactly: `clip = total / 1000` (integer); from the bottom, add
    /// bins while `lo < 253` and the running sum plus the next bin stays `<= clip`; from the top
    /// likewise while `hi > lo + 2`. The loops alone decide: an all-empty histogram gives 253
    /// and 255.
    pub fn auto(&self, counts: &[u32; 256]) -> Levels {
        // pc: crates/ui-egui/src/dialogs/adjust_editors/levels.rs:42-57 `auto_levels`, :78-85 (gamma
        // to 1).
        let total: u64 = counts.iter().map(|&n| u64::from(n)).sum();
        let clip = total / 1000;
        let (mut sum, mut lo) = (0u64, 0usize);
        while lo < 253 && sum + u64::from(counts[lo]) <= clip {
            sum += u64::from(counts[lo]);
            lo += 1;
        }
        let (mut sum, mut hi) = (0u64, 255usize);
        while hi > lo + 2 && sum + u64::from(counts[hi]) <= clip {
            sum += u64::from(counts[hi]);
            hi -= 1;
        }
        Levels { in_black: lo as f64, gamma: 1.0, in_white: hi as f64, ..*self }
    }
}

/// What [`LevelsPanel::show`] reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct LevelsPanelOutput {
    /// The shown channel's edit (the editor's gestures, and Auto as one `Set`). The response is
    /// the panel's group.
    pub edit: EditResponse<Levels>,
    /// The channel the edit is for, and the one shown: an index into the panel's
    /// [`ToneChannels`] (Red is 1 in RGB). The panel keeps it between passes.
    pub channel: usize,
    /// The point picker the user armed (`Some(Some(p))`) or disarmed (`Some(None)`) this pass.
    pub picker: Option<Option<TonePicker>>,
}

/// The Levels adjustment: Image › Adjustments › Levels and a Levels layer's Properties panel
/// (the same call with [`AdjustContext::properties`]).
///
/// Top to bottom (PhotoCraft's and Photoshop's order):
/// 1. The Channel select ([`ToneChannels`], labels from [`AdjustLabels`]) and Auto at the row's
///    end. Picking a channel shows that channel's levels and histogram; the choice is the panel's
///    view state (egui memory under its id), reported as [`LevelsPanelOutput::channel`].
/// 2. The [`LevelsEditor`](super::LevelsEditor) for the shown channel, over its histogram drawn in the channel's colour
///    ([`ToneChannel::color`](super::ToneChannel::color)); `stale` histograms draw dimmed.
/// 3. With [`pickers`](Self::pickers) (on by default), the [`TonePickerRow`](super::TonePickerRow)
///    below: Set Black, Neutral Gray, White Point.
///
/// - **Auto** sets the shown channel to [`Levels::auto`] of its histogram as one `Set`; without a
///   histogram it is disabled.
/// - **Edits**: the editor's events pass through unchanged, for the shown channel. `Cancel`
///   leaves the value to the host (it restores what it kept at `Begin`).
/// - **Keyboard**: Tab visits the Channel select, Auto, the editor, then the pickers.
/// - **AccessKit**: a `Group` named "Levels" ([`AdjustLabels::levels`]) holding the children.
#[must_use = "show the panel with `show`"]
pub struct LevelsPanel<'a> {
    id_salt: IdSalt,
    levels: &'a [Levels],
    channels: ToneChannels,
    numbers: &'a LocaleNumbers,
    histograms: &'a [[u32; 256]],
    stale: bool,
    picker: Option<TonePicker>,
    pickers: bool,
    picker_enabled: [bool; 3],
    unavailable: bool,
    context: AdjustContext,
    policy: EditPolicy,
    labels: Option<AdjustLabels>,
    levels_labels: Option<LevelsLabels>,
    enabled: bool,
}

impl<'a> LevelsPanel<'a> {
    /// The panel over `levels`, one value per channel of `channels` (same index).
    pub fn new(id_salt: impl AsIdSalt, levels: &'a [Levels], channels: ToneChannels, numbers: &'a LocaleNumbers) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            levels,
            channels,
            numbers,
            histograms: &[],
            stale: false,
            picker: None,
            pickers: true,
            picker_enabled: [true; 3],
            unavailable: false,
            context: AdjustContext::dialog(true),
            policy: EditPolicy::default(),
            labels: None,
            levels_labels: None,
            enabled: true,
        }
    }

    /// The host's histograms, one per channel (same index), 256 bins each. Without them the
    /// editor draws no histogram and Auto is disabled.
    pub fn histograms(mut self, histograms: &'a [[u32; 256]]) -> Self {
        self.histograms = histograms;
        self
    }

    /// The histograms are being recounted: drawn dimmed ([`LevelsEditor::stale`](super::LevelsEditor::stale)).
    pub fn stale(mut self, stale: bool) -> Self {
        self.stale = stale;
        self
    }

    /// The armed point picker, shown selected.
    pub fn picker(mut self, armed: Option<TonePicker>) -> Self {
        self.picker = armed;
        self
    }

    /// Whether the point picker row shows; on by default (VectorCraft's Levels has none).
    pub fn pickers(mut self, on: bool) -> Self {
        self.pickers = on;
        self
    }

    /// Enables or disables one picker ([`TonePicker::supported`] for the document's mode).
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

    /// The editor's edit policy ([`LevelsEditor::policy`](super::LevelsEditor::policy)).
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// The panel's words; the installed ones by default.
    pub fn labels(mut self, labels: AdjustLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The editor's own words ([`LevelsEditor::labels`](super::LevelsEditor::labels)).
    pub fn levels_labels(mut self, labels: LevelsLabels) -> Self {
        self.levels_labels = Some(labels);
        self
    }

    /// Enables or disables the whole panel.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// The panel's id in `ui`, for [`shown_channel`](Self::shown_channel).
    pub fn id_in(ui: &Ui, id_salt: impl AsIdSalt) -> Id {
        ui.make_persistent_id(IdSalt::new(id_salt))
    }

    /// The channel the panel with `id` shows (0 before it was first drawn): the host's tone
    /// picker application needs it between passes.
    pub fn shown_channel(ctx: &Context, id: Id) -> usize {
        // pc: crates/ui-egui/src/dialogs/adjust_editors/curves.rs:50-55 (the choice in egui memory).
        ctx.data(|d| d.get_temp::<usize>(id.with(CHANNEL))).unwrap_or(0)
    }

    /// Draws the panel and reports the shown channel's edit.
    pub fn show(self, ui: &mut Ui) -> LevelsPanelOutput {
        // pc: crates/ui-egui/src/dialogs/adjust_editors/levels.rs:63-85 (Channel select, Auto at the
        // row's end, Auto from the shown channel's histogram), crates/ui-egui/src/dialogs/adjust_dialog.rs:186-216
        // (the picker row in the dialog).
        // lib: `LevelsEditor::new(.., &self.levels[channel], ..).channel(color).histogram(..)`;
        // the picker row is `TonePickerRow`.
        let labels = self.labels.clone().unwrap_or_else(|| AdjustLabels::of(ui));
        let id = ui.make_persistent_id(self.id_salt);
        let enabled = self.enabled && ui.is_enabled();
        let count = self.channels.len();
        let channel = Self::shown_channel(ui.ctx(), id).min(count.saturating_sub(1));
        let value = self.levels.get(channel).copied().unwrap_or_else(|| Levels::identity(0.0..=255.0));
        let counts = self.histograms.get(channel);
        let color = self.channels.as_slice().get(channel).map_or(craft_ui::composites::ChannelColor::Neutral, |c| c.color());
        let data = counts.map(|counts| HistogramData::tone(counts, color));
        let shown_name = self.channels.as_slice().get(channel).map(|c| c.label(&labels).to_owned()).unwrap_or_default();

        let mut builder = UiBuilder::new().scope_id(id).accessibility_role(Role::Group).accessibility_label(labels.levels.as_str());
        if !enabled {
            builder = builder.disabled();
        }
        let group = ui.scope_builder(builder, |ui| {
            Stack::new(Gap::Stack)
                .show(ui, |ui| {
                    let mut picked = channel;
                    let mut auto = false;
                    direction::horizontal(ui, |ui| {
                        ui.add(Text::new(labels.channel.as_str()).role(TypeRole::Label).content(Content::Muted));
                        let mut select = Select::new((self.id_salt, "channel"), labels.channel.as_str(), &mut picked);
                        for (i, entry) in self.channels.as_slice().iter().enumerate() {
                            select = select.option(i, entry.label(&labels));
                        }
                        let _ = select.fit(true).enabled(enabled).show(ui);
                        ui.with_layout(direction::trailing(ui), |ui| {
                            auto = ui
                                .add(Button::new(labels.auto.as_str()).enabled(enabled && counts.is_some()).id_salt((self.id_salt, "auto")))
                                .clicked();
                        });
                    });
                    let mut editor = LevelsEditor::new((self.id_salt, "levels", channel), shown_name.as_str(), &value, self.numbers)
                        .channel(color)
                        .stale(self.stale)
                        .policy(self.policy)
                        .enabled(enabled);
                    if let Some(data) = &data {
                        editor = editor.histogram(data);
                    }
                    if let Some(levels_labels) = &self.levels_labels {
                        editor = editor.labels(levels_labels.clone());
                    }
                    let edit = editor.show(ui);
                    let mut change = None;
                    if self.pickers {
                        let mut row = TonePickerRow::new((self.id_salt, "pickers"), self.picker).unavailable(self.unavailable).labels(labels.clone());
                        for picker in TonePicker::ALL {
                            // A grey document has no Neutral Gray point.
                            let usable = self.picker_enabled[picker as usize]
                                && !(picker == TonePicker::Gray && (self.channels.is_gray() || self.context.is_gray()));
                            row = row.enabled(picker, usable);
                        }
                        change = row.show(ui).change;
                    }
                    let auto = counts.filter(|_| auto).map(|counts| EditEvent::set(value.auto(counts), Modifiers::NONE));
                    (edit, auto, picked, change)
                })
                .inner
        });
        let rect = group.response.rect;
        // A `Ui`'s node gets no bounds of its own from egui.
        let bounds = egui::accesskit::Rect { x0: rect.min.x.into(), y0: rect.min.y.into(), x1: rect.max.x.into(), y1: rect.max.y.into() };
        ui.ctx().accesskit_node_builder(id, |node| node.set_bounds(bounds));
        let (edit, auto, picked, picker) = group.inner;
        let (submitted, escaped) = (edit.submitted, edit.escaped);
        let event = auto.or(edit.event);
        // An edit belongs to the channel it was drawn for; a switch shows from the next pass.
        let shown = if picked == channel || event.is_some() { channel } else { picked };
        if picked != channel {
            ui.data_mut(|d| d.insert_temp(id.with(CHANNEL), picked));
        }
        let mut edit = EditResponse::new(group.response, event);
        edit.submitted = submitted;
        edit.escaped = escaped;
        LevelsPanelOutput { edit, channel: shown, picker }
    }
}
