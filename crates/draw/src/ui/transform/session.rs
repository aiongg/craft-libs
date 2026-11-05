//! [`TransformSession`]: the draft of a Free Transform while its box is open (PhotoCraft's
//! `TransformSession` and its `Steps`), headless and testable.
//!
//! The session is a plain value the app owns. Widgets ([`TransformOverlay`](super::TransformOverlay),
//! [`TransformBar`](super::TransformBar)) never change it: they report [`TransformEdit`]s, and
//! the app hands each to [`TransformSession::apply`] in the order they came. They also report
//! `apply` and `discard` (Enter, the bar's ✓ and ⊘): the app answers them after the edits with
//! [`commit`](TransformSession::commit) and [`discard`](TransformSession::discard). The session
//! keeps its own Undo and Redo ([`LocalHistory`]); the app routes Edit › Undo and Redo to
//! [`undo`](TransformSession::undo) and [`redo`](TransformSession::redo) while it is open, and
//! cancels a running drag with [`CancelReason::UndoDuringGesture`](craft_ui::edit::CancelReason)
//! first (the box reports that cancel as an edit). It never touches the document: a commit gives
//! the app a [`TransformResult`] to run as its own command.
//!
//! The warp of the Warp phase is the app's type `W` (decisions §2.1: the drawing library cannot
//! name the imaging library's mesh). The session only stores, clones and compares it, and asks
//! [`TransformWarp::is_identity`] whether there is anything to apply.
//!
//! ```
//! use craft_draw::ui::transform::{TransformEdit, TransformRequest, TransformSession};
//! use kurbo::Rect;
//!
//! // A session over a layer's bounds, without a warp type.
//! let session: TransformSession = TransformSession::new(Rect::new(0.0, 0.0, 200.0, 100.0));
//! assert!(!session.dragging());
//! # let _ = |s: &mut TransformSession, _now: f64| match s.commit() {
//! #     TransformRequest::Apply(_result) => {}
//! #     _ => {}
//! # };
//! ```

use kurbo::{Point, Rect};

use craft_ui::canvas::quad::{Anchor9, BoxAllow, BoxRules, BoxState, Homography, ScaleRule, TransformMode};
use craft_ui::dialog::LocalHistory;
use craft_ui::dialog::Step;
use craft_ui::edit::EditEvent;
use craft_ui::localized::units::LengthUnit;
use craft_ui::menu::{MenuEntry, MenuItem};
use craft_ui::words;

use super::TransformRow;

/// The app's warp as the session holds it in the Warp phase. The session clones and compares it
/// and asks whether it changes anything; the warp model itself stays the app's (PhotoCraft's
/// `Warp` in `photocraft-geom`).
pub trait TransformWarp: Clone + PartialEq {
    /// Whether committing this warp would change nothing: the untouched default grid of the box.
    /// An app whose type layers commit an identity warp (a Warp Text set back to None) answers
    /// false for those.
    fn is_identity(&self) -> bool;
}

/// No warp: a session that never enters the Warp phase (VectorCraft's Free Transform, Content-Aware
/// Scale).
impl TransformWarp for () {
    fn is_identity(&self) -> bool {
        true
    }
}

/// Which part of Free Transform is open: the box, or the warp mesh over it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TransformPhase {
    /// The box with its handles ([`TransformBox`](super::TransformBox)).
    #[default]
    Box,
    /// The app's warp over the box (imaging's `WarpGrid`); the box itself does not draw.
    Warp,
}

/// Something a widget or a menu reports to the open session. The app applies each with
/// [`TransformSession::apply`], in order, in the pass it came in.
#[derive(Clone, Debug, PartialEq)]
pub enum TransformEdit<W> {
    /// The overlay's drags and arrow keys on the box: a gesture (Begin, Update, Commit, Cancel)
    /// or a Set (a nudge). Cancel restores the box the gesture began on.
    Box(EditEvent<BoxState>),
    /// The bar's X, Y, W, H, angle and skew fields: the box and the V skew it reads with
    /// (`TransformFields`). Each report is built from the box at the run's start.
    Fields(EditEvent<TransformRow>),
    /// The right-click menu's or Edit › Transform's mode. Scale picks [`TransformMode::Free`].
    Mode(TransformMode),
    /// Rotate 180°, Rotate 90° Clockwise or Counter Clockwise, Flip Horizontal or Vertical: turns
    /// the open box about the reference point without committing it (Photoshop; PhotoCraft
    /// `intercept`).
    Preset(TransformPreset),
    /// The bar's interpolation select. Session state, not an undo step (PhotoCraft's steps hold
    /// the box and the warp only).
    Interpolation(Interpolation),
    /// The bar's Show Reference Point check. View state, not an undo step.
    ReferencePoint(bool),
    /// The reference point picker beside the fields: the pivot moves to that point of the box.
    Pivot(Anchor9),
    /// The bar's Free Transform / Warp switch, or Warp in the menu, from the Box phase. The
    /// session answers [`TransformRequest::NeedWarp`]; the app builds its warp and reports
    /// [`Warp`](Self::Warp).
    WarpRequested,
    /// The app's new or edited warp. From the Box phase it enters the Warp phase; the box and the
    /// warp are one undo step.
    Warp(W),
    /// The switch from the Warp phase: drops the warp; the box stays as it was (PhotoCraft
    /// `leave_warp`).
    LeaveWarp,
}

/// What the app does after an edit, a commit or a discard.
#[derive(Clone, Debug, PartialEq)]
pub enum TransformRequest<W> {
    /// [`TransformSession::commit`] with something to apply: the app runs its transform command with the result and
    /// closes the session.
    Apply(TransformResult<W>),
    /// [`TransformSession::commit`] of an untouched session: the app closes it without a document step. What the
    /// session began with (a ⌥⌘T copy, a placed file) stays (PhotoCraft `commit.rs:71`).
    CloseUntouched,
    /// [`TransformSession::discard`]: the app closes the session and takes back what it made for it.
    Discard,
    /// [`TransformEdit::WarpRequested`]: the app builds its warp from the box (`rect` mapped onto
    /// `state`, a smart object's own warp first) and answers [`TransformEdit::Warp`].
    NeedWarp { rect: Rect, state: BoxState },
}

/// What a commit applies: the source frame, the box it was moved to, the interpolation, and the
/// warp when the Warp phase was open.
#[derive(Clone, Debug, PartialEq)]
pub struct TransformResult<W> {
    /// The source frame the session began on (the layer's bounds).
    pub rect: Rect,
    /// The box: `rect`'s corners moved, and the reference point. In the Warp phase the
    /// [`warp`](Self::warp) already holds the box (PhotoCraft builds it from the box when the
    /// phase opens), so the app applies the warp and ignores this box.
    pub state: BoxState,
    pub interpolation: Interpolation,
    /// The app's warp, when the session ended in the Warp phase.
    pub warp: Option<W>,
}

/// Edit › Transform's presets while a box is open (PhotoCraft `box_preset`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TransformPreset {
    Rotate180,
    Rotate90Clockwise,
    Rotate90CounterClockwise,
    FlipHorizontal,
    FlipVertical,
}

impl TransformPreset {
    /// Every preset, in the menu's order.
    pub const ALL: [TransformPreset; 5] = [
        TransformPreset::Rotate180,
        TransformPreset::Rotate90Clockwise,
        TransformPreset::Rotate90CounterClockwise,
        TransformPreset::FlipHorizontal,
        TransformPreset::FlipVertical,
    ];

    /// What the preset does to the box, as `(kx, ky, angle)`: a scale along the box's own axes,
    /// then a turn in radians, clockwise on screen, both about the reference point. Rotate 180° is
    /// `(1, 1, π)`, Rotate 90° Clockwise `(1, 1, π/2)`, Counter Clockwise `(1, 1, −π/2)`, Flip
    /// Horizontal `(−1, 1, 0)`, Flip Vertical `(1, −1, 0)`.
    pub fn scale_and_turn(self) -> (f64, f64, f64) {
        // pc: crates/ui-egui/src/tools/transform_tool/commit.rs:329 (`box_preset`).
        use std::f64::consts::{FRAC_PI_2, PI};
        match self {
            TransformPreset::Rotate180 => (1.0, 1.0, PI),
            TransformPreset::Rotate90Clockwise => (1.0, 1.0, FRAC_PI_2),
            TransformPreset::Rotate90CounterClockwise => (1.0, 1.0, -FRAC_PI_2),
            TransformPreset::FlipHorizontal => (-1.0, 1.0, 0.0),
            TransformPreset::FlipVertical => (1.0, -1.0, 0.0),
        }
    }
}

/// How the transform resamples the moving pixels: PhotoCraft's three choices (LC11). The app maps
/// them to its engine; [`id`](Self::id) gives PhotoCraft's ids.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Interpolation {
    #[default]
    Bicubic,
    Bilinear,
    NearestNeighbor,
}

impl Interpolation {
    /// Every choice, in the select's order.
    pub const ALL: [Interpolation; 3] = [Interpolation::Bicubic, Interpolation::Bilinear, Interpolation::NearestNeighbor];

    /// PhotoCraft's id: "bicubic", "bilinear", "nearest".
    pub fn id(self) -> &'static str {
        // pc: crates/ui-egui/src/tools/transform_tool/options_bar.rs:241 (the select's options).
        match self {
            Interpolation::Bicubic => "bicubic",
            Interpolation::Bilinear => "bilinear",
            Interpolation::NearestNeighbor => "nearest",
        }
    }

    /// The choice with PhotoCraft's `id`; `None` for any other text.
    pub fn from_id(id: &str) -> Option<Interpolation> {
        // pc: crates/ui-egui/src/tools/transform_tool/options_bar.rs:241.
        Interpolation::ALL.into_iter().find(|choice| choice.id() == id)
    }
}

/// A pick in the box's right-click menu ([`TransformSession::menu`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TransformChoice {
    /// Free Transform, Scale (as Free), Rotate, Skew, Distort, Perspective: apply as
    /// [`TransformEdit::Mode`].
    Mode(TransformMode),
    /// Warp: apply as [`TransformEdit::WarpRequested`].
    Warp,
    /// Apply as [`TransformEdit::Preset`].
    Preset(TransformPreset),
}

/// Free Transform's persisted section: the app keeps it with its preferences. Unknown fields are
/// ignored, missing ones take their defaults.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TransformSettings {
    /// PhotoCraft's Use Legacy Free Transform: corners stretch freely and ⇧ keeps the proportions
    /// ([`ScaleRule::Free`]). Off by default: corners keep the proportions and ⇧ frees them
    /// (Photoshop CC 2019 and later, LC1).
    pub legacy: bool,
    /// The W and H fields' link (the chain button), kept between sessions.
    pub aspect_linked: bool,
    /// Show Reference Point: off by default (Photoshop CC 2019 and later, LC2). Hidden, the point
    /// is not drawn, hit or picked; its stored position still pivots ⌥ scaling and rotating.
    pub show_reference_point: bool,
    /// The interpolation a new session starts with.
    pub interpolation: Interpolation,
    /// The W and H fields' unit; [`LengthUnit::Percent`] by default.
    pub size_unit: Option<LengthUnit>,
}

impl TransformSettings {
    /// The scaling rule [`legacy`](Self::legacy) picks.
    pub fn scale_rule(&self) -> ScaleRule {
        if self.legacy { ScaleRule::Free } else { ScaleRule::Proportional }
    }
}

words! {
    /// The words of the box's right-click menu ([`TransformSession::menu`]), Photoshop's and
    /// PhotoCraft's order (`canvas_tool_menu.rs:75`). `Default` is English.
    pub struct TransformMenuLabels {
        free_transform: "transformMenu.freeTransform" = "Free Transform";
        scale: "transformMenu.scale" = "Scale";
        rotate: "transformMenu.rotate" = "Rotate";
        skew: "transformMenu.skew" = "Skew";
        distort: "transformMenu.distort" = "Distort";
        perspective: "transformMenu.perspective" = "Perspective";
        warp: "transformMenu.warp" = "Warp";
        rotate_180: "transformMenu.rotate180" = "Rotate 180°";
        rotate_90_clockwise: "transformMenu.rotate90Clockwise" = "Rotate 90° Clockwise";
        rotate_90_counter_clockwise: "transformMenu.rotate90CounterClockwise" = "Rotate 90° Counter Clockwise";
        flip_horizontal: "transformMenu.flipHorizontal" = "Flip Horizontal";
        flip_vertical: "transformMenu.flipVertical" = "Flip Vertical";
    }
}

/// What one undo step of the session holds: everything Undo restores together (PhotoCraft's
/// `Step`: the box, and the warp). View-like state (the mode, the interpolation, the reference
/// point's visibility, the picked part, the zoom) stays outside it, so Undo never moves the view
/// and changing one of them is no undo step of its own.
#[derive(Clone, Debug, PartialEq)]
struct TransformDraft<W> {
    state: BoxState,
    skew_v: f64,
    warp: Option<W>,
}

/// The draft of one Free Transform: the source frame, the box and its V skew, the app's warp in
/// the Warp phase, the mode and interpolation, and the session's undo steps.
///
/// - Changes only through [`apply`](Self::apply). [`settle`](Self::settle), once per pass, records
///   an undo step whenever the draft has come to rest changed: once per drag, typing run or run
///   of arrow nudges within the [nudge pause](Self::nudge_pause).
/// - [`undo`](Self::undo) and [`redo`](Self::redo) are the app's Edit commands while the session is
///   open; both are refused while a drag runs, so the app cancels the drag first (`UndoDuringGesture`).
///   The session binds no key.
/// - No serde (LC7): the app builds its own record from the getters.
#[derive(Clone, Debug)]
pub struct TransformSession<W = ()> {
    /// The source frame (the layer's bounds); fixed for the session.
    rect: Rect,
    /// What the content allows (type refuses distort and perspective).
    allow: BoxAllow,
    draft: TransformDraft<W>,
    /// The current mode (view-like state: not an undo step).
    mode: TransformMode,
    /// The interpolation (view-like state: not an undo step).
    interpolation: Interpolation,
    history: LocalHistory<TransformDraft<W>>,
    /// The draft when the running gesture began: a Cancel restores it.
    before: Option<TransformDraft<W>>,
    /// Show Reference Point (view state).
    reference_point: bool,
    /// The app's Nudge Bundle Pause in seconds; `None` never folds nudges.
    nudge_pause: Option<f64>,
    /// When the latest arrow nudge was applied (the `now` of [`apply`](Self::apply)).
    last_nudge: Option<f64>,
}

impl<W: TransformWarp> TransformSession<W> {
    /// A session on the source frame `rect`, its box on the frame's corners and its reference
    /// point at the centre, in [`TransformMode::Free`] with bicubic interpolation.
    pub fn new(rect: Rect) -> Self {
        Self::with_state(rect, BoxState::from_rect(rect))
    }

    /// A session on `rect` whose box starts at `state`: Content-Aware Move's placed box, a box
    /// reopened after a failed commit.
    pub fn with_state(rect: Rect, state: BoxState) -> Self {
        let draft = TransformDraft { state, skew_v: 0.0, warp: None };
        Self {
            rect,
            allow: BoxAllow::default(),
            history: LocalHistory::new(&draft),
            draft,
            mode: TransformMode::Free,
            interpolation: Interpolation::default(),
            before: None,
            reference_point: false,
            nudge_pause: None,
            last_nudge: None,
        }
    }

    /// The mode the session opens in (Edit › Transform › Rotate opens in Rotate).
    pub fn mode(mut self, mode: TransformMode) -> Self {
        self.mode = mode;
        self
    }

    /// What the content allows; everything by default.
    pub fn allow(mut self, allow: BoxAllow) -> Self {
        self.allow = allow;
        self
    }

    /// The persisted section the session opens with: its interpolation and whether the reference
    /// point shows.
    pub fn settings(mut self, settings: &TransformSettings) -> Self {
        self.interpolation = settings.interpolation;
        self.reference_point = settings.show_reference_point;
        self
    }

    /// The app's Nudge Bundle Pause, in seconds: arrow nudges each within this of the one before
    /// are one undo step (PhotoCraft `commit.rs:270`, `input/nudge_bundle.rs`). `None` (the
    /// default) makes every nudge its own step. The library reads no preference (LC6).
    pub fn nudge_pause(mut self, seconds: Option<f64>) -> Self {
        self.nudge_pause = seconds;
        self
    }

    /// The source frame.
    pub fn rect(&self) -> Rect {
        self.rect
    }

    /// The box.
    pub fn state(&self) -> &BoxState {
        &self.draft.state
    }

    /// The box's corners, clockwise from the source frame's top left.
    pub fn quad(&self) -> [Point; 4] {
        self.draft.state.quad
    }

    /// The reference point, in document units.
    pub fn pivot(&self) -> Point {
        self.draft.state.pivot
    }

    /// The V skew the fields read the box with, in degrees.
    pub fn skew_v(&self) -> f64 {
        self.draft.skew_v
    }

    /// The current mode.
    pub fn current_mode(&self) -> TransformMode {
        self.mode
    }

    /// What the content allows.
    pub fn allows(&self) -> BoxAllow {
        self.allow
    }

    pub fn interpolation(&self) -> Interpolation {
        self.interpolation
    }

    /// Whether the reference point shows (Show Reference Point).
    pub fn reference_point(&self) -> bool {
        self.reference_point
    }

    /// [`TransformPhase::Warp`] while the session holds a warp.
    pub fn phase(&self) -> TransformPhase {
        if self.draft.warp.is_some() { TransformPhase::Warp } else { TransformPhase::Box }
    }

    /// The app's warp in the Warp phase.
    pub fn warp(&self) -> Option<&W> {
        self.draft.warp.as_ref()
    }

    /// Whether a gesture (a box drag, a field scrub or typing run) is running: between its Begin
    /// and its Commit or Cancel.
    pub fn dragging(&self) -> bool {
        self.before.is_some()
    }

    /// The map from the source frame to the box: the source frame's corners to the unit square,
    /// then [`Homography::unit_to_quad`] of the box. Carried content (marching ants, S7-select's
    /// `SelectionOverlay::mapped`) follows the box through it. `None` for a degenerate box (three
    /// corners on a line, or the frame empty).
    pub fn homography(&self) -> Option<Homography> {
        // pc: crates/ui-egui/src/tools/transform_tool/draw.rs:113 (`Homography::rect_to_quad` of
        // the unit square, then the box).
        let (w, h) = (self.rect.width(), self.rect.height());
        if !(w.abs() > 1e-9 && h.abs() > 1e-9) {
            return None;
        }
        let unit = Homography::unit_to_quad(self.draft.state.quad)?;
        // The box's map after the frame's affine map onto the unit square.
        let (x0, y0) = (self.rect.x0, self.rect.y0);
        let row = |i: usize| {
            let r = unit.row(i);
            [r[0] / w, r[1] / h, r[2] - r[0] * x0 / w - r[1] * y0 / h]
        };
        let rows = [row(0), row(1), row(2)];
        rows.iter().flatten().all(|v| v.is_finite()).then(|| Homography::from_rows(rows))
    }

    /// The box rules for the current mode and what the content allows, with the settings' scaling
    /// rule and aspect link and the library's rotate step (15° with ⇧, LC3). The overlay uses them
    /// unless the app passes its own.
    pub fn box_rules(&self, settings: &TransformSettings) -> BoxRules {
        // new: one place maps the session and the settings to `BoxRules` (the PhotoCraft port maps
        // its preferences in its own `transform_tool/pointer.rs`).
        BoxRules { mode: self.mode, scale: settings.scale_rule(), aspect_linked: settings.aspect_linked, allow: self.allow, ..BoxRules::default() }
    }

    /// Whether committing now would change nothing. In the Box phase: the box on the frame's
    /// corners. In the Warp phase the warp holds the box, so the warp alone decides: an
    /// [identity](TransformWarp::is_identity) warp changes nothing wherever the box was moved
    /// before. Mode and interpolation alone change nothing.
    pub fn is_untouched(&self) -> bool {
        // pc: crates/ui-egui/src/tools/transform_tool/commit.rs:65 (warp identity), :70 (box on
        // the frame's corners).
        match &self.draft.warp {
            Some(warp) => warp.is_identity(),
            None => self.draft.state.quad == BoxState::from_rect(self.rect).quad,
        }
    }

    /// What a commit applies; `None` when [untouched](Self::is_untouched).
    pub fn result(&self) -> Option<TransformResult<W>> {
        // pc: crates/ui-egui/src/tools/transform_tool/commit.rs:63.
        if self.is_untouched() {
            return None;
        }
        Some(TransformResult { rect: self.rect, state: self.draft.state, interpolation: self.interpolation, warp: self.draft.warp.clone() })
    }

    /// Applies one edit; `now` is the pass's input time in seconds (the nudge pause reads it).
    ///
    /// - `Box` and `Fields`: Begin takes the draft as `before`, Update and Commit set the box (and
    ///   the V skew from a field row), Cancel puts `before` back. A Set is an arrow nudge or a
    ///   typed field: it sets the box and, for a nudge, notes `now` as the latest nudge.
    /// - `Mode`, `Interpolation`: set them; `ReferencePoint` sets the view state; `Pivot` moves the
    ///   reference point to [`BoxState::point_at`].
    /// - `Preset`: [`TransformPreset::scale_and_turn`] about the reference point
    ///   ([`BoxState::scaled_about_pivot`], [`BoxState::turned_about_pivot`]); refused in the Warp
    ///   phase, as PhotoCraft refuses it ("Commit or cancel the transform first").
    /// - `WarpRequested` in the Box phase answers [`TransformRequest::NeedWarp`] with the current
    ///   box; `Warp(w)` stores the warp (entering the Warp phase); `LeaveWarp` drops it.
    ///
    /// Ending the session is [`commit`](Self::commit) or [`discard`](Self::discard), which the app
    /// calls after the pass's edits when a widget reported `apply` or `discard`.
    pub fn apply(&mut self, edit: TransformEdit<W>, now: f64) -> Option<TransformRequest<W>> {
        // pc: crates/ui-egui/src/tools/transform_tool/commit.rs:352 (`intercept`: presets, warp
        // refusal), :13 (`commit`); begin.rs:736 (`leave_warp`).
        let nudge = matches!(&edit, TransformEdit::Box(EditEvent::Set { .. }));
        if !nudge {
            // Whatever comes after a run of nudges ends the run: the nudges are their own step.
            self.end_nudges();
        }
        match edit {
            TransformEdit::Box(event) => self.gesture(event, now, nudge, |draft, state| draft.state = *state),
            TransformEdit::Fields(event) => self.gesture(event, now, false, |draft, row| {
                draft.state = row.state;
                draft.skew_v = row.skew_v;
            }),
            TransformEdit::Mode(mode) => self.mode = mode,
            TransformEdit::Preset(preset) => {
                if self.draft.warp.is_none() {
                    let (kx, ky, angle) = preset.scale_and_turn();
                    self.draft.state = self.draft.state.scaled_about_pivot(kx, ky).turned_about_pivot(angle);
                }
            }
            TransformEdit::Interpolation(interpolation) => self.interpolation = interpolation,
            TransformEdit::ReferencePoint(show) => self.reference_point = show,
            TransformEdit::Pivot(anchor) => self.draft.state.pivot = self.draft.state.point_at(anchor),
            TransformEdit::WarpRequested => {
                if self.draft.warp.is_none() {
                    return Some(TransformRequest::NeedWarp { rect: self.rect, state: self.draft.state });
                }
            }
            TransformEdit::Warp(warp) => self.draft.warp = Some(warp),
            TransformEdit::LeaveWarp => self.draft.warp = None,
        }
        None
    }

    /// Enter, a click outside, a double click inside or the bar's ✓: ends any running gesture and
    /// answers [`TransformRequest::Apply`] with [`result`](Self::result), or
    /// [`TransformRequest::CloseUntouched`] when the session is [untouched](Self::is_untouched).
    /// The app closes the session.
    pub fn commit(&mut self) -> TransformRequest<W> {
        // pc: crates/ui-egui/src/tools/transform_tool/commit.rs:13 (`commit`).
        self.before = None;
        self.result().map_or(TransformRequest::CloseUntouched, TransformRequest::Apply)
    }

    /// Esc with no drag running or the bar's ⊘: puts back the draft a running gesture began on and
    /// answers [`TransformRequest::Discard`]. The app closes the session.
    pub fn discard(&mut self) -> TransformRequest<W> {
        if let Some(before) = self.before.take() {
            self.draft = before;
        }
        TransformRequest::Discard
    }

    /// Records an undo step when the draft has come to rest changed. Call once per pass after the
    /// edits, with `busy` true while the app has a pointer button down, a text field focused or a
    /// warp drag running (PhotoCraft `commit.rs:258`). The session adds its own gesture and the
    /// nudge pause: within [`nudge_pause`](Self::nudge_pause) seconds of the latest nudge it is
    /// still busy, so a run of nudges is one step.
    pub fn settle(&mut self, now: f64, busy: bool) {
        // pc: crates/ui-egui/src/tools/transform_tool/commit.rs:258 (`track_steps`), :270
        // (`nudge`).
        let nudging = match (self.last_nudge, self.nudge_pause) {
            (Some(at), Some(pause)) => now - at < pause,
            _ => false,
        };
        if !nudging {
            self.last_nudge = None;
        }
        self.history.settle(&self.draft, busy || self.dragging() || nudging);
    }

    /// Edit › Undo while the session is open (the app routes it here; the session reads no undo
    /// key): steps back one undo step: the box, the V skew and the warp together. The mode, the
    /// interpolation and the reference point stay. False while a gesture runs (the app cancels it
    /// with `UndoDuringGesture` before it routes Undo) or with nothing to undo.
    pub fn undo(&mut self) -> bool {
        // pc: crates/ui-egui/src/tools/transform_tool/commit.rs:376 (`step`).
        self.step(Step::Undo)
    }

    /// Steps forward one undone step; false while a gesture runs or with nothing to redo.
    pub fn redo(&mut self) -> bool {
        // pc: crates/ui-egui/src/tools/transform_tool/commit.rs:376.
        self.step(Step::Redo)
    }

    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    /// The box's right-click menu: Free Transform, Scale, Rotate, Skew, Distort, Perspective, Warp,
    /// a separator, Rotate 180°, Rotate 90° Clockwise, Rotate 90° Counter Clockwise, a separator,
    /// Flip Horizontal, Flip Vertical. The current mode is checked (Scale counts as Free
    /// Transform, LC10); modes the content refuses (Distort, Perspective under
    /// `allow.distort == false`) and every preset in the Warp phase are disabled. Puppet Warp and
    /// Content-Aware Scale rows are the app's to add, greyed while a box is open.
    pub fn menu(&self, labels: &TransformMenuLabels) -> Vec<MenuEntry<TransformChoice>> {
        // pc: crates/ui-egui/src/canvas/canvas_tool_menu.rs:75 (`TRANSFORM_MENU`).
        let warping = self.phase() == TransformPhase::Warp;
        let allow = self.allow;
        let mode = |label: &str, mode: TransformMode, allowed: bool, checked: bool| {
            MenuEntry::from(MenuItem::new(label, TransformChoice::Mode(mode)).radio(checked && !warping).enabled(allowed))
        };
        let free = self.mode == TransformMode::Free;
        let preset = |label: &str, preset: TransformPreset| MenuEntry::from(MenuItem::new(label, TransformChoice::Preset(preset)).enabled(!warping));
        vec![
            mode(&labels.free_transform, TransformMode::Free, true, free),
            mode(&labels.scale, TransformMode::Free, true, false),
            mode(&labels.rotate, TransformMode::Rotate, allow.rotate, self.mode == TransformMode::Rotate),
            mode(&labels.skew, TransformMode::Skew, allow.skew, self.mode == TransformMode::Skew),
            mode(&labels.distort, TransformMode::Distort, allow.distort, self.mode == TransformMode::Distort),
            mode(&labels.perspective, TransformMode::Perspective, allow.distort, self.mode == TransformMode::Perspective),
            MenuEntry::from(MenuItem::new(&labels.warp, TransformChoice::Warp).radio(warping)),
            MenuEntry::Separator,
            preset(&labels.rotate_180, TransformPreset::Rotate180),
            preset(&labels.rotate_90_clockwise, TransformPreset::Rotate90Clockwise),
            preset(&labels.rotate_90_counter_clockwise, TransformPreset::Rotate90CounterClockwise),
            MenuEntry::Separator,
            preset(&labels.flip_horizontal, TransformPreset::FlipHorizontal),
            preset(&labels.flip_vertical, TransformPreset::FlipVertical),
        ]
    }

    /// Ends a run of nudges: records them as their own step before the next edit.
    fn end_nudges(&mut self) {
        if self.last_nudge.take().is_some() && !self.dragging() {
            self.history.settle(&self.draft, false);
        }
    }

    /// One box or field event. A Begin takes the draft as `before`; a Set outside a gesture is a
    /// typed field or, with `nudge`, an arrow nudge. Edits to the box are ignored in the Warp
    /// phase, where the mesh is the box (PhotoCraft refuses nudges there).
    fn gesture<T>(&mut self, event: EditEvent<T>, now: f64, nudge: bool, set: impl Fn(&mut TransformDraft<W>, &T)) {
        if self.draft.warp.is_some() {
            return;
        }
        match event {
            EditEvent::Begin { value, .. } => {
                self.before = Some(self.draft.clone());
                if let Some(value) = value {
                    set(&mut self.draft, &value);
                }
            }
            EditEvent::Update { value, .. } => set(&mut self.draft, &value),
            EditEvent::Commit { value, .. } => {
                set(&mut self.draft, &value);
                self.before = None;
            }
            EditEvent::Cancel { .. } => {
                if let Some(before) = self.before.take() {
                    self.draft = before;
                }
            }
            EditEvent::Set { value, .. } => {
                set(&mut self.draft, &value);
                if nudge {
                    self.last_nudge = Some(now);
                }
            }
            _ => {}
        }
    }

    /// Undo or redo one step; refused while a gesture runs.
    fn step(&mut self, step: Step) -> bool {
        if self.dragging() {
            return false;
        }
        self.last_nudge = None;
        self.history.step(&mut self.draft, step)
    }
}
