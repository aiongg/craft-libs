//! Keyframe lanes, the keyframe navigator and the animation toggle (N1.04).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::behavior::{Selection, SelectionChange};
use craft_ui::edit::EditEvent;
use egui::{AsIdSalt, Color32, IdSalt, Painter, Pos2, Rect, Response, Ui};

use crate::rates::FrameRate;
use crate::time::{Tick, TimeRange, TimeView};

craft_ui::words! {
    /// The keyframe controls' tooltips and names, English by default (FilmCraft's).
    #[non_exhaustive]
    pub struct KeyframeLabels {
        previous: "keyframes.previous" = "Go to previous keyframe";
        next: "keyframes.next" = "Go to next keyframe";
        add_remove: "keyframes.addRemove" = "Add/Remove Keyframe";
        toggle_animation: "keyframes.toggleAnimation" = "Toggle animation";
        lane: "keyframes.lane" = "Keyframes";
    }
}

/// A keyframe's id, the app's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyframeId(pub u64);

/// How the segment leaving a keyframe interpolates (Premiere's temporal set; After Effects'
/// Linear, Bezier and Hold are a subset). Also picks the glyph.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum KeyframeInterpolation {
    #[default]
    Linear,
    Hold,
    Bezier,
    AutoBezier,
    ContinuousBezier,
    EaseIn,
    EaseOut,
}

/// A keyframe on a lane.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KeyframeMark {
    pub id: KeyframeId,
    pub time: Tick,
    pub interpolation: KeyframeInterpolation,
}

/// Draws a keyframe glyph centred at `center` (Premiere's glyphs): a `style.keyframe_hold`
/// square for Hold, the diamond filling the `style.keyframe_glyph` box for Linear, a circle of
/// radius `style.keyframe_bezier` for the Bezier kinds ([`TimelineStyle`](super::TimelineStyle)).
// filmcraft: crates/ui-egui/src/panels/effect_controls_selection.rs:256 `paint`.
pub fn paint_keyframe(painter: &Painter, center: Pos2, interpolation: KeyframeInterpolation, color: Color32, style: &super::TimelineStyle) {
    todo!()
}

/// The keyframes of one property row on its lane.
///
/// - The selection is the app's [`Selection`] of keys in time order; clicks and keys change it
///   with the kernel's [`ListSelection`](craft_ui::behavior::ListSelection) rules over that
///   order, with Photoshop's and Premiere's picks: a click replaces, Shift+click and
///   Cmd/Ctrl+click toggle the key; a drag on empty lane draws a marquee (Shift adds to the
///   selection). Changes arrive as [`SelectionChange`]s.
/// - Dragging a selected key moves every selected key by the same delta, snapped to frames of
///   `rate` and kept inside `bounds` (the clip); with Shift it snaps to the playhead and the
///   other keys within `style.keyframe_snap`. Reported as an `EditEvent<Tick>` (the delta)
///   gesture; Esc cancels.
/// - A double-click and a secondary click on a key are reported (After Effects opens the
///   keyframe's dialog and menu, `timeline.rs:4083`, `:4101`).
/// - Keyboard: one Tab stop when it has keys; Left and Right move the selection to the previous
///   and next key, Shift+Left/Right nudge the selected keys one frame (a `Set` on
///   [`KeyframeLaneResponse::moved`]), Delete reports [`KeyframeLaneResponse::delete`] (claimed
///   through [`KeyClaim`](craft_ui::behavior::KeyClaim) so the app's Delete shortcut leaves it).
///   AccessKit: a `ListBox` named by [`KeyframeLabels::lane`], one `ListBoxOption` per key named
///   by its time as timecode of the lane's rate, `selected` set.
pub struct KeyframeLane<'a> {
    id_salt: IdSalt,
    keys: &'a [KeyframeMark],
    selection: &'a Selection<KeyframeId>,
    view: &'a TimeView,
    rate: FrameRate,
    playhead: Tick,
    bounds: Option<TimeRange>,
    labels: Option<KeyframeLabels>,
}

impl<'a> KeyframeLane<'a> {
    pub fn new(id_salt: impl AsIdSalt, keys: &'a [KeyframeMark], selection: &'a Selection<KeyframeId>, view: &'a TimeView, rate: FrameRate) -> Self {
        Self { id_salt: IdSalt::new(id_salt), keys, selection, view, rate, playhead: Tick::ZERO, bounds: None, labels: None }
    }

    pub fn playhead(mut self, t: Tick) -> Self {
        self.playhead = t;
        self
    }

    /// Keys stay inside this range while dragged (the clip).
    pub fn bounds(mut self, range: TimeRange) -> Self {
        self.bounds = Some(range);
        self
    }

    pub fn labels(mut self, labels: KeyframeLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // filmcraft: crates/ui-egui/src/panels/effect_controls_selection.rs:81 `update` (snapping),
    // `:110` `Interaction::begin`; effectcraft: crates/ui-egui/src/panels/timeline.rs:4059
    // `key_select`, `:4191` `key_drag_begin`, `:4246` `key_hit`.
    pub fn show(self, ui: &mut Ui, rect: Rect) -> KeyframeLaneResponse {
        todo!()
    }
}

/// What a keyframe lane did this pass.
pub struct KeyframeLaneResponse {
    pub response: Response,
    /// A click, marquee or key changed the selection.
    pub selection: Option<SelectionChange<KeyframeId>>,
    /// The selected keys' move, as a delta.
    pub moved: Option<EditEvent<Tick>>,
    pub double_clicked: Option<KeyframeId>,
    pub context_menu: Option<KeyframeId>,
    /// Delete was pressed with keys selected.
    pub delete: bool,
}

/// Premiere's keyframe navigator at a property row: ◀ (previous key), ◆ (add a key at the
/// playhead, or remove the one there), ▶ (next key). The arrows are
/// `style.keyframe_arrow_width` × `style.keyframe_arrow_height`, `style.keyframe_nav_step`
/// either side of the diamond, dimmed without a key that way; the diamond is filled when a key is
/// at the playhead, else an outline. Three Tab stops (buttons named by [`KeyframeLabels`]),
/// Space or Enter presses.
pub struct KeyframeNavigator {
    id_salt: IdSalt,
    animated: bool,
    has_previous: bool,
    has_next: bool,
    at_key: bool,
    labels: Option<KeyframeLabels>,
}

impl KeyframeNavigator {
    /// `has_previous` / `has_next`: a key before / after the playhead; `at_key`: one at it.
    pub fn new(id_salt: impl AsIdSalt, has_previous: bool, has_next: bool, at_key: bool) -> Self {
        Self { id_salt: IdSalt::new(id_salt), animated: true, has_previous, has_next, at_key, labels: None }
    }

    /// Without animation only the diamond shows (it turns animation on).
    pub fn animated(mut self, on: bool) -> Self {
        self.animated = on;
        self
    }

    pub fn labels(mut self, labels: KeyframeLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // filmcraft: crates/ui-egui/src/panels/effect_controls.rs:487 `keyframe_nav`.
    pub fn show(self, ui: &mut Ui) -> KeyframeNavigatorOutput {
        todo!()
    }
}

/// Which navigator button was pressed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[must_use]
pub struct KeyframeNavigatorOutput {
    pub previous: bool,
    pub next: bool,
    /// The diamond: add a key at the playhead, or remove the one there.
    pub toggle: bool,
}

/// The stopwatch that turns a property's animation on and off (Premiere's Toggle animation,
/// After Effects' Time-Vary Stopwatch). A toggle button: one Tab stop, Space toggles; reports
/// the new state as a `Set`.
pub struct AnimationToggle<'a> {
    id_salt: IdSalt,
    on: &'a bool,
    labels: Option<KeyframeLabels>,
}

impl<'a> AnimationToggle<'a> {
    pub fn new(id_salt: impl AsIdSalt, on: &'a bool) -> Self {
        Self { id_salt: IdSalt::new(id_salt), on, labels: None }
    }

    // effectcraft: crates/ui-egui/src/panels/timeline.rs:3922 `prop_stopwatch`.
    pub fn show(self, ui: &mut Ui) -> craft_ui::edit::EditResponse<bool> {
        todo!()
    }
}
