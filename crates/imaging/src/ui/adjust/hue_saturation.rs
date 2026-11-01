//! [`HueSaturationEditor`]: Master and six colour ranges, Colorize, the two spectrum bars and the
//! four range thumbs that wrap at 360° (task AD-4).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_theme::Srgba;
use egui::{AsIdSalt, Context, Id, IdSalt, Ui};

use craft_ui::edit::{EditPolicy, EditResponse};
use craft_ui::localized::LocaleNumbers;

use super::{AdjustContext, AdjustLabels};

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
        todo!()
    }

    /// The canonical form of four bounds: increasing, within one turn (each at most 359° past the
    /// first), centre (the mean of the inner two) moved into −30..330. Any non-finite bound gives
    /// the Reds' default `[−45, −15, 15, 45]`.
    pub fn canonical_bounds(bounds: [f64; 4]) -> [f64; 4] {
        // pc: crates/doc/src/adjust.rs:153-166 `HueRange::canonical_bounds`.
        todo!()
    }

    /// `bounds` with thumb `index` (0 range start … 3 range end) dragged to `degrees` (0..=360 on
    /// the bar), PhotoCraft's rule: canonicalise first; the start moves by the shortest turn to
    /// `degrees`, at most to the second thumb and at least 180° before it; any other thumb moves
    /// to `degrees` measured from the start, between its neighbours (the end at most 359° past the
    /// start). An index past 3 changes nothing but the canonical form.
    pub fn move_bound(bounds: [f64; 4], index: usize, degrees: f64) -> [f64; 4] {
        // pc: crates/ui-egui/src/dialogs/adjust_editors/hue_saturation.rs:25-39 `move_hue_range_handle`.
        todo!()
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
    todo!()
}

/// The Lightness slider's track: black, grey 128, white, whatever the hue (Photoshop).
pub(crate) fn lightness_track() -> [Srgba; 3] {
    // pc: crates/ui-egui/src/dialogs/adjust_editors/hue_saturation.rs:19-23 `lightness_track`.
    todo!()
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

    pub fn context(mut self, context: AdjustContext) -> Self {
        self.context = context;
        self
    }

    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn labels(mut self, labels: AdjustLabels) -> Self {
        self.labels = Some(labels);
        self
    }

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
        todo!()
    }

    pub fn show(self, ui: &mut Ui) -> EditResponse<HueSaturation> {
        // pc: crates/ui-egui/src/dialogs/adjust_editors/hue_saturation.rs:65-210 `hue_saturation`
        // (range select :70-77, Master rows :80-103, range rows :104-125, Colorize :127-142,
        // bars :143-153, band and thumbs :154-180, thumb drag :181-205, readout :206-207).
        // new: thumbs by keyboard and AccessKit; the look from `AdjustStyle::of`.
        todo!()
    }
}
