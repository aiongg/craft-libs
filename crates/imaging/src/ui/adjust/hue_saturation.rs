//! [`HueSaturationEditor`]: Master and six colour ranges, Colorize, the two spectrum bars and the
//! four range thumbs that wrap at 360° (task AD-4).

use craft_theme::{Hsb, Srgba, TypeRole};
use egui::accesskit::{Action, ActionData, Role};
use egui::{
    AsIdSalt, Color32, Context, CornerRadius, EventFilter, Id, IdSalt, Key, Label, Mesh, Modifiers, Rect, RichText, Sense, Stroke, StrokeKind, Ui,
    pos2, vec2,
};

use craft_ui::composites::SliderRow;
use craft_ui::controls::{Checkbox, Select};
use craft_ui::direction;
use craft_ui::edit::{EditPolicy, EditResponse, Gesture};
use craft_ui::localized::{Labels as _, LocaleNumbers, Unit, UnitKind, Units};
use craft_ui::primitives::{Content, FocusRing, Gap, Interactive, Stack, Text};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _};

use super::context::named_group;
use super::sliders::{PartEdits, edit_parts, push_part};
use super::{AdjustContext, AdjustLabels, AdjustStyle};

/// The shown range in egui memory under the editor's id: 0 Master, 1 Reds … 6 Magentas.
pub(super) const RANGE: &str = "range";
const EDIT: &str = "edit";

/// Hues sampled on each spectrum bar: every 10°, 0 to 360.
const SAMPLES: usize = 37;
/// Colorize keeps Saturation at or above this (Photoshop's 25 %).
const COLORIZE_SATURATION: f64 = 25.0;

/// One of Hue/Saturation's six colour ranges (Reds, Yellows, Greens, Cyans, Blues, Magentas).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HueRangeValue {
    /// Hue shift −180..=180°, whole degrees.
    pub hue: f64,
    /// −100..=100 %.
    pub saturation: f64,
    /// −100..=100 %.
    pub lightness: f64,
    /// Where the range starts fading in, is fully on, starts fading out and ends, in degrees
    /// (Photoshop's four range sliders); may run past 0 or 360.
    pub bounds: [f64; 4],
}

impl HueRangeValue {
    /// Range `index` (0 Reds … 5 Magentas) unchanged, with Photoshop's extent: centre `60 ×
    /// index`, fully on for 30° around it, fading over 30° each side (`[c − 45, c − 15, c + 15,
    /// c + 45]`).
    pub fn neutral(index: usize) -> Self {
        // pc: crates/doc/src/adjust.rs:142-147 `HueRange::neutral`.
        let c = 60.0 * index as f64;
        Self { hue: 0.0, saturation: 0.0, lightness: 0.0, bounds: [c - 45.0, c - 15.0, c + 15.0, c + 45.0] }
    }

    /// The canonical form of four bounds: increasing, within one turn (each at most 359° past the
    /// first), centre (the mean of the inner two) moved into −30..330. Any non-finite bound gives
    /// the Reds' default `[−45, −15, 15, 45]`.
    pub fn canonical_bounds(bounds: [f64; 4]) -> [f64; 4] {
        // pc: crates/doc/src/adjust.rs:153-166 `HueRange::canonical_bounds`.
        if bounds.iter().any(|b| !b.is_finite()) {
            return [-45.0, -15.0, 15.0, 45.0];
        }
        let mut out = [bounds[0]; 4];
        for k in 1..4 {
            out[k] = (out[k - 1] + (bounds[k] - out[k - 1]).rem_euclid(360.0)).min(out[0] + 359.0);
        }
        let centre = (out[1] + out[2]) / 2.0;
        let shift = (centre + 30.0).rem_euclid(360.0) - 30.0 - centre;
        out.map(|b| b + shift)
    }

    /// `bounds` with thumb `index` (0 range start … 3 range end) dragged to `degrees` (0..=360 on
    /// the bar), PhotoCraft's rule: canonicalise first; the start moves by the shortest turn to
    /// `degrees`, at most to the second thumb and at least 180° before it; any other thumb moves
    /// to `degrees` measured from the start, between its neighbours (the end at most 359° past the
    /// start). An index past 3 changes nothing but the canonical form.
    pub fn move_bound(bounds: [f64; 4], index: usize, degrees: f64) -> [f64; 4] {
        // pc: crates/ui-egui/src/dialogs/adjust_editors/hue_saturation.rs:25-39 `move_hue_range_handle`.
        let mut b = Self::canonical_bounds(bounds);
        if index >= b.len() {
            return b;
        }
        if index == 0 {
            let delta = (degrees - b[0] + 180.0).rem_euclid(360.0) - 180.0;
            b[0] = (b[0] + delta).min(b[1]).max(b[1] - 180.0);
        } else {
            let rel = (degrees - b[0]).rem_euclid(360.0);
            let hi = if index == 3 { b[0] + 359.0 } else { b[index + 1] };
            b[index] = (b[0] + rel).clamp(b[index - 1], hi);
        }
        b
    }
}

/// A Hue/Saturation value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HueSaturation {
    /// Master hue: −180..=180°, or 0..=360° with Colorize.
    pub hue: f64,
    /// Master saturation: −100..=100 %, or 0..=100 % with Colorize.
    pub saturation: f64,
    /// Master lightness: −100..=100 %.
    pub lightness: f64,
    /// Colorize: tints the whole image with the master hue.
    pub colorize: bool,
    /// Reds, Yellows, Greens, Cyans, Blues, Magentas.
    pub ranges: [HueRangeValue; 6],
}

impl Default for HueSaturation {
    fn default() -> Self {
        Self { hue: 0.0, saturation: 0.0, lightness: 0.0, colorize: false, ranges: std::array::from_fn(HueRangeValue::neutral) }
    }
}

/// The Saturation slider's track: grey 128 to the hue at full saturation and brightness.
pub(crate) fn saturation_track(hue_degrees: f64) -> [Srgba; 2] {
    // pc: crates/ui-egui/src/dialogs/adjust_editors/hue_saturation.rs:14-17 `saturation_track`.
    [Srgba::rgb(128.0 / 255.0, 128.0 / 255.0, 128.0 / 255.0), hue_color(hue_degrees)]
}

/// The Lightness slider's track: black, grey 128, white, whatever the hue (Photoshop).
pub(crate) fn lightness_track() -> [Srgba; 3] {
    // pc: crates/ui-egui/src/dialogs/adjust_editors/hue_saturation.rs:19-23 `lightness_track`.
    [Srgba::BLACK, Srgba::rgb(128.0 / 255.0, 128.0 / 255.0, 128.0 / 255.0), Srgba::WHITE]
}

/// Hue/Saturation.
///
/// - **Range select** (Master, Reds … Magentas): view state under the editor's id, Master at
///   first. Disabled and forced to Master while Colorize is on.
/// - **Master**: Hue (−180..180°; 0..360° with Colorize) over the hue spectrum track, Saturation
///   (−100..100 %; 0..100 % with Colorize) over a track from grey to the current hue, Lightness
///   (−100..100 %) over a fixed black, grey, white track. Tracks only while dynamic tracks are on.
/// - **A range**: its Hue, Saturation and Lightness rows (−180..180°, −100..100 %), then the
///   range thumbs (below).
/// - **Colorize** (a `Set`): on, Hue wraps into 0..360 (`rem_euclid`) and Saturation becomes at
///   least 25; off, a Hue above 180 wraps back by −360.
/// - **Spectrum bars**: below the rows, as wide as the sliders, `adjust.spectrum_bar` (10) high:
///   the input hues above (37 samples, every 10°), the host's
///   [`adjusted_spectrum`](Self::adjusted_spectrum) below (the same 37 hues after the
///   adjustment; the input bar again without it). Not mirrored in RTL (a scale of the image).
/// - **Range thumbs** (a range shown): between the bars, the fall-off band (`text.faint`) and the
///   fully-affected band (`text.muted`), `adjust.range_band` (6) high, wrapping across the ends;
///   four 5 × 12 thumbs (inner two `text.normal`, outer two `surface.sunken`, edge `text.muted`).
///   A press within 10 pt of a thumb grabs the nearest; the drag moves it with
///   [`HueRangeValue::move_bound`], whole degrees, `Begin`, `Update`, `Commit`. Under the bars a
///   readout "a° / b°  ·  c° \ d°" (`type.mono`, `text.faint`), each `rem_euclid(360)`.
/// - **Keyboard**: each thumb is a Tab stop; ← and → move it 1°, Shift 10°, through
///   [`HueRangeValue::move_bound`] (one `Set` each).
/// - **AccessKit**: the bars are `Image`s "Hue spectrum, before" and "Hue spectrum, after"; each
///   thumb a `Slider` ("Range start", "Fully affected start", "Fully affected end", "Range end")
///   valued in degrees.
#[must_use = "show the editor with `show`"]
pub struct HueSaturationEditor<'a> {
    id_salt: IdSalt,
    value: &'a HueSaturation,
    numbers: &'a LocaleNumbers,
    adjusted: Option<&'a dyn Fn(f64) -> Srgba>,
    context: AdjustContext,
    policy: EditPolicy,
    labels: Option<AdjustLabels>,
    enabled: bool,
}

impl<'a> HueSaturationEditor<'a> {
    /// Draws the editor and returns this frame's edit of the `HueSaturation`.
    pub fn new(id_salt: impl AsIdSalt, value: &'a HueSaturation, numbers: &'a LocaleNumbers) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            value,
            numbers,
            adjusted: None,
            context: AdjustContext::dialog(true),
            policy: EditPolicy::default(),
            labels: None,
            enabled: true,
        }
    }

    /// What a fully saturated, full-brightness hue (degrees 0..=360) becomes under the shown
    /// value: the host runs its engine (PhotoCraft's `adjusted_spectrum` applies the adjustment to
    /// a row of hues). Called 37 times a pass.
    pub fn adjusted_spectrum(mut self, adjusted: &'a dyn Fn(f64) -> Srgba) -> Self {
        self.adjusted = Some(adjusted);
        self
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

    /// The range the editor with `id` shows: `None` for Master, else 0 Reds … 5 Magentas.
    pub fn shown_range(ctx: &Context, id: Id) -> Option<usize> {
        // pc: crates/ui-egui/src/dialogs/adjust_editors/hue_saturation.rs:67-68 (`hs-range` in memory).
        ctx.data(|d| d.get_temp::<usize>(id.with(RANGE))).and_then(|shown| shown.checked_sub(1)).filter(|&range| range < 6)
    }

    /// Draws the editor and returns this frame's edit of the `HueSaturation`.
    pub fn show(self, ui: &mut Ui) -> EditResponse<HueSaturation> {
        // pc: crates/ui-egui/src/dialogs/adjust_editors/hue_saturation.rs:65-210 `hue_saturation`
        // (range select :70-77, Master rows :80-103, range rows :104-125, Colorize :127-142,
        // bars :143-153, band and thumbs :154-180, thumb drag :181-205, readout :206-207).
        // new: thumbs by keyboard and AccessKit; the look from `AdjustStyle::of`.
        let labels = self.labels.clone().unwrap_or_else(|| AdjustLabels::of(ui));
        let style = AdjustStyle::of(&Tokens::of(ui));
        let id = ui.make_persistent_id(self.id_salt);
        let enabled = self.enabled && ui.is_enabled();
        let stored = ui.data(|d| d.get_temp::<usize>(id.with(RANGE))).unwrap_or(0).min(6);
        let names = [&labels.reds, &labels.yellows, &labels.greens, &labels.cyans, &labels.blues, &labels.magentas];

        let ((group, colorize, picked), event) = edit_parts(ui, id, self.value, self.policy, enabled, |ui, edits, inner| {
            let value = *edits.value();
            // Colorize works on the master only.
            let shown = if value.colorize { 0 } else { stored };
            let range = shown.checked_sub(1);
            let mut picked = shown;
            let group = named_group(ui, id, labels.hue_saturation.as_str(), enabled, |ui| {
                Stack::new(Gap::Stack)
                    .show(ui, |ui| {
                        direction::horizontal(ui, |ui| {
                            ui.add(Text::new(labels.range.as_str()).role(TypeRole::Label).content(Content::Muted));
                            let mut select =
                                Select::new((self.id_salt, "range"), labels.range.as_str(), &mut picked).option(0, labels.master.as_str());
                            for (i, name) in names.iter().enumerate() {
                                select = select.option(i + 1, name.as_str());
                            }
                            let _ = select.enabled(enabled && !value.colorize).show(ui);
                        });

                        let (mut submitted, mut escaped) = (false, false);
                        let stops = hue_stops();
                        let hue_track = self.context.track(&stops);
                        let mut rows: Vec<(Row, RowSpec<'_>)> = Vec::new();
                        let salt = match range {
                            None => {
                                let (hue_range, saturation_low) = if value.colorize { (0.0..=360.0, 0.0) } else { (-180.0..=180.0, -100.0) };
                                let hue = value.hue.clamp(*hue_range.start(), *hue_range.end());
                                let saturation_default = if value.colorize { COLORIZE_SATURATION } else { 0.0 };
                                rows.push((Row::Hue, RowSpec::new(&labels.hue, hue, hue_range, Unit::new("{n}°"), 0.0).track(hue_track)));
                                rows.push((
                                    Row::Saturation,
                                    RowSpec::new(
                                        &labels.saturation,
                                        value.saturation.clamp(saturation_low, 100.0),
                                        saturation_low..=100.0,
                                        Unit::new("{n} %"),
                                        saturation_default,
                                    )
                                    .colors(saturation_track(hue)),
                                ));
                                rows.push((
                                    Row::Lightness,
                                    RowSpec::new(&labels.lightness, value.lightness, -100.0..=100.0, Unit::new("{n} %"), 0.0)
                                        .colors(lightness_track()),
                                ));
                                ("master", 0)
                            }
                            Some(r) => {
                                let current = value.ranges[r];
                                rows.push((
                                    Row::Hue,
                                    RowSpec::new(&labels.hue, current.hue, -180.0..=180.0, Unit::new("{n}°"), 0.0).track(hue_track),
                                ));
                                rows.push((
                                    Row::Saturation,
                                    RowSpec::new(&labels.saturation, current.saturation, -100.0..=100.0, Unit::new("{n} %"), 0.0),
                                ));
                                rows.push((
                                    Row::Lightness,
                                    RowSpec::new(&labels.lightness, current.lightness, -100.0..=100.0, Unit::new("{n} %"), 0.0),
                                ));
                                ("range", r * 3)
                            }
                        };
                        for (row, spec) in rows {
                            let out = self.row(ui, edits, inner, (salt.0, salt.1 + row as usize), spec, enabled, range, row);
                            (submitted, escaped) = (submitted | out.0, escaped | out.1);
                        }

                        let mut colorize = value.colorize;
                        let check =
                            Checkbox::bool(labels.colorize.as_str(), &mut colorize).id_salt((self.id_salt, "colorize")).enabled(enabled).show(ui);
                        push_part(edits, &check, false, |on, now: &HueSaturation| now.colorized(on));

                        if let Some(bars) = self.bars(ui, id, &value, inner, &style, &labels, range, enabled) {
                            let r = range.unwrap_or_default();
                            push_part(edits, &bars, false, move |dragged: HueSaturation, now: &HueSaturation| {
                                let mut next = *now;
                                next.ranges[r].bounds = dragged.ranges[r].bounds;
                                next
                            });
                        }
                        (submitted, escaped)
                    })
                    .inner
            });
            (group, value.colorize, picked)
        });
        // An edit belongs to the range it was drawn for; a switch shows from the next pass.
        ui.data_mut(|d| d.insert_temp(id.with(RANGE), if colorize { 0 } else { picked }));
        let (submitted, escaped) = group.inner;
        EditResponse::new(group.response, event).submitted(submitted).escaped(escaped)
    }

    /// One slider row, its field and slider pushed as parts of `edits`; whether the field was
    /// submitted or passed on an Esc press.
    #[allow(clippy::too_many_arguments)]
    fn row(
        &self,
        ui: &mut Ui,
        edits: &mut PartEdits<HueSaturation>,
        policy: EditPolicy,
        salt: (&'static str, usize),
        spec: RowSpec<'_>,
        enabled: bool,
        range: Option<usize>,
        which: Row,
    ) -> (bool, bool) {
        let mut row = SliderRow::new((self.id_salt, salt), spec.label, spec.value, spec.range, self.numbers)
            .unit(spec.unit)
            .decimals(0)
            .step(1.0)
            .default_value(spec.default)
            .wheel(self.context.wheel())
            .policy(policy)
            .enabled(enabled);
        if let Some(stops) = spec.track {
            row = row.track_colors(stops.iter().copied());
        } else if let Some(stops) = spec.colors.as_deref().and_then(|stops| self.context.track(stops)) {
            row = row.track_colors(stops.iter().copied());
        }
        let response = row.show(ui);
        let map = move |v: f64, now: &HueSaturation| now.with_row(range, which, v);
        push_part(edits, &response.field, true, map);
        push_part(edits, &response.slider, false, map);
        (response.submitted(), response.field.escaped || response.slider.escaped)
    }

    /// The two spectrum bars with the range thumbs between them and the degree readout; the
    /// thumbs' edit.
    #[allow(clippy::too_many_arguments)]
    fn bars(
        &self,
        ui: &mut Ui,
        id: Id,
        value: &HueSaturation,
        policy: EditPolicy,
        style: &AdjustStyle,
        labels: &AdjustLabels,
        range: Option<usize>,
        enabled: bool,
    ) -> Option<EditResponse<HueSaturation>> {
        let width = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(vec2(width, style.spectrum_bar * 2.0 + style.spectrum_gap), Sense::hover());
        let top = Rect::from_min_size(rect.min, vec2(width, style.spectrum_bar));
        let bottom = Rect::from_min_size(pos2(rect.left(), rect.bottom() - style.spectrum_bar), vec2(width, style.spectrum_bar));
        let sense = if range.is_some() && enabled { Sense::click_and_drag() } else { Sense::hover() };
        let bars = ui.interact(rect, id.with("bars"), sense);
        for (bar, salt, label) in [(top, "before", &labels.spectrum_before), (bottom, "after", &labels.spectrum_after)] {
            ui.interact(bar, id.with(salt), Sense::hover()).widget_info(|| egui::WidgetInfo::labeled(Role::Image, enabled, label.as_str()));
        }
        // Scales of the image: not mirrored in RTL.
        let input: Vec<Color32> = (0..SAMPLES).map(|i| hue_color(i as f64 * 10.0).to_rgba().to_egui()).collect();
        let adjusted: Vec<Color32> = match self.adjusted {
            Some(adjusted) => (0..SAMPLES).map(|i| adjusted(i as f64 * 10.0).to_rgba().to_egui()).collect(),
            None => input.clone(),
        };
        paint_spectrum(ui, top, &input);
        paint_spectrum(ui, bottom, &adjusted);

        let r = range?;
        let thumb_ids: [Id; 4] = std::array::from_fn(|k| id.with(("thumb", k)));
        // ← and → belong to the focused thumb, not to egui's focus movement.
        let keys = EventFilter { horizontal_arrows: true, vertical_arrows: false, tab: false, escape: false };
        let mut gesture = Gesture::load(ui, id.with(EDIT), value, policy).focus_owners(&thumb_ids).event_filter(keys).enabled(enabled);
        let xof = |degrees: f64| rect.left() + degrees.rem_euclid(360.0) as f32 / 360.0 * width;

        // Pointer: a press near a thumb grabs it, and the thumb follows the pointer until release.
        let grab_id = id.with("grab");
        let mut grabbed: Option<usize> = ui.data(|d| d.get_temp::<Option<usize>>(grab_id)).flatten();
        if grabbed.is_some() && !gesture.is_active() {
            grabbed = None;
        }
        let down = enabled && bars.is_pointer_button_down_on();
        if down
            && ui.input(|i| i.pointer.primary_pressed())
            && let Some(pos) = ui.input(|i| i.pointer.press_origin())
        {
            let xs = HueRangeValue::canonical_bounds(gesture.shown().ranges[r].bounds).map(xof);
            grabbed = nearest(&xs, pos.x).filter(|&k| (xs[k] - pos.x).abs() < style.hue_grab);
            if let Some(k) = grabbed {
                ui.memory_mut(|m| m.request_focus(thumb_ids[k]));
                gesture.begin_with_value();
                if !gesture.is_active() {
                    grabbed = None;
                }
            }
        }
        if let Some(k) = grabbed {
            if down && let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                let degrees = f64::from(((pos.x - rect.left()) / width * 360.0).clamp(0.0, 360.0));
                let next = gesture.shown().with_bound(r, k, degrees);
                gesture.update(next, ui.input(|i| i.modifiers));
            }
            if !down {
                gesture.commit();
                grabbed = None;
            }
        }
        ui.data_mut(|d| d.insert_temp(grab_id, grabbed));

        // Keyboard and screen readers: one `Set` per step.
        if grabbed.is_none() {
            for (k, thumb) in thumb_ids.into_iter().enumerate() {
                let Some(step) = requested_move(ui, thumb, enabled) else { continue };
                let now = *gesture.shown();
                let at = HueRangeValue::canonical_bounds(now.ranges[r].bounds)[k];
                let target = match step {
                    Move::By(delta) => at + delta,
                    Move::To(degrees) => degrees,
                };
                let next = now.with_bound(r, k, target);
                if next != now {
                    gesture.set(next);
                }
                break;
            }
        }

        let bounds = HueRangeValue::canonical_bounds(gesture.shown().ranges[r].bounds);
        let painter = ui.painter();
        let mid = rect.center().y;
        let half = style.range_band / 2.0;
        let band = |from: f64, to: f64, color: Color32| {
            let (a, z) = (xof(from), xof(to));
            if a <= z {
                painter.rect_filled(Rect::from_min_max(pos2(a, mid - half), pos2(z, mid + half)), 0.0, color);
            } else {
                painter.rect_filled(Rect::from_min_max(pos2(a, mid - half), pos2(rect.right(), mid + half)), 0.0, color);
                painter.rect_filled(Rect::from_min_max(pos2(rect.left(), mid - half), pos2(z, mid + half)), 0.0, color);
            }
        };
        band(bounds[0], bounds[3], style.band_outer);
        band(bounds[1], bounds[2], style.band_inner);

        let thumb_names = [&labels.range_start, &labels.full_start, &labels.full_end, &labels.range_end];
        for k in 0..4 {
            let thumb_rect = Rect::from_center_size(pos2(xof(bounds[k]), mid), style.range_thumb);
            let response = Interactive::with_id(thumb_ids[k], Role::Slider, thumb_names[k].as_str())
                .sense(Sense::FOCUSABLE)
                .enabled(enabled)
                .state_layer(false)
                .focus_ring(false)
                .interact_at(ui, thumb_rect)
                .numeric(bounds[k].rem_euclid(360.0), 0.0..=360.0, 1.0)
                .finish(ui);
            let fill = if k == 1 || k == 2 { style.thumb_inner } else { style.thumb_outer };
            ui.painter().rect_filled(thumb_rect, style.range_thumb_radius, fill);
            ui.painter().rect_stroke(thumb_rect, style.range_thumb_radius, Stroke::new(1.0, style.thumb_edge), StrokeKind::Inside);
            FocusRing::paint_radius(ui, thumb_rect, CornerRadius::ZERO, &response);
        }

        let [a, b, c, d] = bounds.map(|degrees| degrees.rem_euclid(360.0));
        let units = Units::of(ui);
        let [a, b, c, d] = [a, b, c, d].map(|degrees| units.text(UnitKind::Degrees, degrees, 0..=0));
        let readout = craft_i18n::fmt(&labels.range_readout, &[("a", &a), ("b", &b), ("c", &c), ("d", &d)]);
        ui.add(Label::new(RichText::new(readout).font(style.degrees_font.clone()).color(style.degrees_color)).selectable(false));
        Some(EditResponse::new(bars, gesture.finish()))
    }
}

/// Which of the three rows of a view.
#[derive(Clone, Copy)]
enum Row {
    Hue,
    Saturation,
    Lightness,
}

/// What a slider row shows.
struct RowSpec<'a> {
    label: &'a str,
    value: f64,
    range: std::ops::RangeInclusive<f64>,
    unit: Unit,
    default: f64,
    track: Option<&'a [Srgba]>,
    /// Colour stops of the row's own, drawn while dynamic tracks are on.
    colors: Option<Vec<Srgba>>,
}

impl<'a> RowSpec<'a> {
    fn new(label: &'a str, value: f64, range: std::ops::RangeInclusive<f64>, unit: Unit, default: f64) -> Self {
        Self { label, value, range, unit, default, track: None, colors: None }
    }

    fn track(mut self, track: Option<&'a [Srgba]>) -> Self {
        self.track = track;
        self
    }

    fn colors(mut self, stops: impl Into<Vec<Srgba>>) -> Self {
        self.colors = Some(stops.into());
        self
    }
}

impl HueSaturation {
    /// `self` with a row set to `value`, whole degrees or percent.
    fn with_row(mut self, range: Option<usize>, row: Row, value: f64) -> Self {
        let value = value.round();
        match (range, row) {
            (None, Row::Hue) => self.hue = value,
            (None, Row::Saturation) => self.saturation = value,
            (None, Row::Lightness) => self.lightness = value,
            (Some(r), Row::Hue) => self.ranges[r].hue = value,
            (Some(r), Row::Saturation) => self.ranges[r].saturation = value,
            (Some(r), Row::Lightness) => self.ranges[r].lightness = value,
        }
        self
    }

    /// `self` with Colorize switched: Hue wraps into 0..360 and Saturation reaches at least 25; off,
    /// a Hue above 180 wraps back by 360.
    fn colorized(mut self, on: bool) -> Self {
        self.colorize = on;
        if on {
            self.hue = self.hue.rem_euclid(360.0);
            self.saturation = self.saturation.max(COLORIZE_SATURATION);
        } else if self.hue > 180.0 {
            self.hue -= 360.0;
        }
        self
    }

    /// `self` with thumb `k` of range `r` dragged to `degrees`, whole degrees.
    fn with_bound(mut self, r: usize, k: usize, degrees: f64) -> Self {
        self.ranges[r].bounds = HueRangeValue::move_bound(self.ranges[r].bounds, k, degrees).map(f64::round);
        self
    }
}

/// A fully saturated, full-brightness hue.
fn hue_color(degrees: f64) -> Srgba {
    Srgba::from_hsb(Hsb { h: degrees as f32, s: 1.0, b: 1.0 })
}

/// The hue spectrum for a colour track: red, yellow, green, cyan, blue, magenta, red.
fn hue_stops() -> Vec<Srgba> {
    (0..=12).map(|i| hue_color(f64::from(i) * 30.0)).collect()
}

/// The index of the x nearest to `x`.
fn nearest(xs: &[f32; 4], x: f32) -> Option<usize> {
    (0..4).min_by(|&a, &b| (xs[a] - x).abs().total_cmp(&(xs[b] - x).abs()))
}

/// A bar through `colors`, left to right.
fn paint_spectrum(ui: &Ui, rect: Rect, colors: &[Color32]) {
    if colors.len() < 2 || !ui.is_rect_visible(rect) {
        return;
    }
    let mut mesh = Mesh::default();
    let last = (colors.len() - 1) as f32;
    for (i, color) in colors.iter().enumerate() {
        let x = rect.left() + rect.width() * i as f32 / last;
        mesh.colored_vertex(pos2(x, rect.top()), *color);
        mesh.colored_vertex(pos2(x, rect.bottom()), *color);
    }
    for i in 0..colors.len() as u32 - 1 {
        let a = 2 * i;
        mesh.add_triangle(a, a + 1, a + 3);
        mesh.add_triangle(a, a + 3, a + 2);
    }
    ui.painter().add(mesh);
}

/// What a keyboard or a screen reader asks of a thumb.
enum Move {
    By(f64),
    To(f64),
}

/// The move asked of the thumb `id` in this pass: ← and → one degree, Shift ten (a focused thumb);
/// a screen reader's Increment, Decrement and SetValue.
fn requested_move(ui: &Ui, id: Id, enabled: bool) -> Option<Move> {
    if !enabled {
        return None;
    }
    let mut delta = 0.0;
    if ui.memory(|m| m.has_focus(id)) {
        ui.input_mut(|i| {
            for (key, sign) in [(Key::ArrowRight, 1.0), (Key::ArrowLeft, -1.0)] {
                // Shift first: `Modifiers::NONE` also matches a press with Shift held.
                let shifted = i.count_and_consume_key(Modifiers::SHIFT, key) as f64;
                delta += sign * (10.0 * shifted + i.count_and_consume_key(Modifiers::NONE, key) as f64);
            }
        });
    }
    let mut to = None;
    ui.input(|i| {
        let requests = |action| i.num_accesskit_action_requests(id, action) as f64;
        delta += requests(Action::Increment) - requests(Action::Decrement);
        to = i
            .accesskit_action_requests(id, Action::SetValue)
            .filter_map(|r| match r.data {
                Some(ActionData::NumericValue(v)) if v.is_finite() => Some(v),
                _ => None,
            })
            .last();
    });
    match (to, delta) {
        (Some(degrees), _) => Some(Move::To(degrees)),
        (None, 0.0) => None,
        (None, delta) => Some(Move::By(delta)),
    }
}
