//! [`SelectionFrame`]: the box round selected objects, with eight grips and a movable inside.

use egui::{AsIdSalt, Color32, Context, Id, IdSalt, Modifiers, Response, Ui};

use crate::behavior::nudge;
use crate::canvas::quad::{BoxPart, BoxRules, BoxState};
use crate::canvas::{CanvasView, RulerUnits};
use crate::edit::{EditEvent, EditPolicy};

crate::words! {
    /// The AccessKit names of a [`SelectionFrame`] and its parts, English by default (the corner
    /// and edge names match Free Transform's).
    #[non_exhaustive]
    pub struct SelectionFrameLabels {
        group: "selectionFrame.group" = "Selection frame";
        whole: "selectionFrame.whole" = "Whole frame";
        top_left: "selectionFrame.topLeftCorner" = "Top left corner";
        top_right: "selectionFrame.topRightCorner" = "Top right corner";
        bottom_right: "selectionFrame.bottomRightCorner" = "Bottom right corner";
        bottom_left: "selectionFrame.bottomLeftCorner" = "Bottom left corner";
        top: "selectionFrame.topEdge" = "Top edge";
        right: "selectionFrame.rightEdge" = "Right edge";
        bottom: "selectionFrame.bottomEdge" = "Bottom edge";
        left: "selectionFrame.leftEdge" = "Left edge";
    }
}

/// What [`SelectionFrame::show`] reports.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct SelectionFrameResponse {
    /// The frame's id, for [`SelectionFrame::focus`].
    pub id: Id,
    /// A drag (`Begin` on the first move past the click slop, `Update`, `Commit` on release; Esc
    /// cancels it) or one arrow press's `Set`. The value is the whole box.
    pub event: Option<EditEvent<BoxState>>,
    /// The part the keys act on ([`BoxPart::Inside`]: the whole frame), picked by the pointer or
    /// the Page keys; `None` before either.
    pub picked: Option<BoxPart>,
    /// The part a press at the pointer would take; `None` where the press is the tool's.
    pub hovered: Option<BoxPart>,
    /// Whether the frame owns the current press: the host skips its tool input.
    pub claimed: bool,
}

/// The selection frame of the object-editing apps: a 1 pt box round the selected objects in the
/// layer colour, eight square grips that resize it, and an inside that moves it.
///
/// ```no_run
/// use craft_ui::canvas::CanvasView;
/// use craft_ui::canvas::quad::BoxState;
/// use craft_ui::canvas::selection::SelectionFrame;
/// # fn demo(ui: &mut egui::Ui, canvas: &egui::Response, view: &CanvasView, bounds: &BoxState, colour: egui::Color32) {
/// let out = SelectionFrame::new("frame", bounds).colour(colour).key_object(true).centre_mark(true).show(ui, canvas, view);
/// if let Some(event) = out.event {
///     // The host's gesture host previews Update, applies Commit and Set as one undo step each.
///     # let _ = event;
/// }
/// # }
/// ```
///
/// - Look: the box line `selection_frame.line` (1 pt) in [`colour`](Self::colour), else
///   `overlay.line`; with [`key_object`](Self::key_object) `selection_frame.key_line` (3 pt);
///   with [`centre_mark`](Self::centre_mark) a filled square `selection_frame.centre` (3.5 pt) at
///   the centre; grips are [`Handle`](crate::primitives::Handle)s
///   ([`HandleShape::Square`](crate::primitives::HandleShape::Square), `handle.size` 7) filled
///   `overlay.handle_fill` and outlined in the frame's colour, each grown by
///   [`grow`](Self::grow) steps of `selection_frame.grow_step`.
/// - Pointer, through the host's canvas response and [`Claim`](crate::canvas::Claim) first: a
///   press within `handle.hit` of a grip takes that corner or edge, a press inside takes the
///   inside, a press outside is not the frame's. A drag begins on the first move and follows
///   [`quad::drag`](crate::canvas::quad::drag) with the [rules](Self::rules) (resize only by
///   default: corners stretch freely and ⇧ keeps the proportions, ⌥ scales about the centre, an
///   edge stretches its one axis, the inside moves and ⇧ locks it to the axes). Parts the rules
///   refuse show no grip and take no press.
/// - Keys (one Tab stop through [`PartFocus`](crate::canvas::PartFocus)), whenever no text field
///   has focus and no popup is open: Page Down and Page Up pick the next and previous part (the
///   whole frame, then the corners and edges clockwise from the top left), Home and End the first
///   and last; the arrows move the picked part as a plain drag of it would, by
///   [`steps`](Self::steps) (⇧ the big step), one `Set` per press; Esc cancels a drag only.
/// - AccessKit: a group named [`SelectionFrameLabels::group`] valued with the bounds (in the
///   [`units`](Self::units)), parts named by the labels; the picked part is the active
///   descendant, with the focus ring round it when the keyboard moved last.
/// - A screen-reader step on a part during a drag of another waits until the drag ends (the edit
///   [`Gesture`](crate::edit::Gesture) host).
/// - Nothing mirrors in a right-to-left interface: the box is the document's.
pub struct SelectionFrame<'a> {
    id_salt: IdSalt,
    state: &'a BoxState,
    labels: Option<SelectionFrameLabels>,
    rules: Option<BoxRules>,
    colour: Option<Color32>,
    key_object: bool,
    centre_mark: bool,
    grips: bool,
    grow: u8,
    units: Option<(&'a RulerUnits, &'a RulerUnits)>,
    value: Option<String>,
    steps: (f64, f64),
    modifiers: Option<Modifiers>,
    policy: EditPolicy,
}

impl<'a> SelectionFrame<'a> {
    /// The frame of the host's box `state` (document units, corners clockwise from the top left).
    pub fn new(id_salt: impl AsIdSalt, state: &'a BoxState) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            state,
            labels: None,
            rules: None,
            colour: None,
            key_object: false,
            centre_mark: false,
            grips: true,
            grow: 0,
            units: None,
            value: None,
            steps: (nudge::STEP, nudge::BIG_STEP),
            modifiers: None,
            policy: EditPolicy::default(),
        }
    }

    /// The AccessKit names; [`SelectionFrameLabels::of`](crate::localized::Labels::of) the
    /// installed words by default.
    pub fn labels(mut self, labels: SelectionFrameLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// How drags change the box. By default `BoxRules::resize_only()` (no rotate, skew or
    /// distort; [`ScaleRule::Free`](crate::canvas::quad::ScaleRule::Free)), the rule of
    /// VectorCraft's and DesignCraft's frames.
    pub fn rules(mut self, rules: BoxRules) -> Self {
        self.rules = Some(rules);
        self
    }

    /// The colour of the box line and the grip outlines: the selected object's layer colour.
    /// `overlay.line` by default.
    pub fn colour(mut self, colour: Color32) -> Self {
        self.colour = Some(colour);
        self
    }

    /// Draws the box with `selection_frame.key_line` (3 pt): the key object Align aligns to
    /// (DesignCraft 3 pt, VectorCraft 2.5 pt).
    pub fn key_object(mut self, key: bool) -> Self {
        self.key_object = key;
        self
    }

    /// Draws the centre mark (DesignCraft's).
    pub fn centre_mark(mut self, on: bool) -> Self {
        self.centre_mark = on;
        self
    }

    /// Whether the grips show and take presses; true by default. Off, the frame is a line only
    /// (DesignCraft with a placed graphic's content selected: the content's own frame has the
    /// grips; drawn as a second `SelectionFrame` in the content colour).
    pub fn grips(mut self, on: bool) -> Self {
        self.grips = on;
        self
    }

    /// Grows each grip by `steps` × `selection_frame.grow_step`: VectorCraft's Selection & Anchor
    /// Display › Size, which the host passes as its size minus 3 (its default, 3, adds nothing).
    pub fn grow(mut self, steps: u8) -> Self {
        self.grow = steps;
        self
    }

    /// The ruler units of the document's x and y axes, for the AccessKit values of the group and
    /// the parts. Without them the values are left out.
    pub fn units(mut self, x: &'a RulerUnits, y: &'a RulerUnits) -> Self {
        self.units = Some((x, y));
        self
    }

    /// The group's value for a screen reader in place of the bounds, e.g. "3 objects".
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// The arrow keys' step and ⇧ step in document units; `nudge::STEP` and `nudge::BIG_STEP` by
    /// default (the host passes its own Keyboard Increment).
    pub fn steps(mut self, step: f64, big_step: f64) -> Self {
        self.steps = (step, big_step);
        self
    }

    /// The modifiers every test of the frame reads in place of the input state (a touch host's
    /// sticky modifiers).
    pub fn modifiers(mut self, modifiers: Modifiers) -> Self {
        self.modifiers = Some(modifiers);
        self
    }

    /// How the drag reports; realtime and cancellable by default.
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Gives the frame `id` ([`SelectionFrameResponse::id`]) the keyboard.
    pub fn focus(ctx: &Context, id: Id) {
        // new: as `TransformBox::focus` (crates/draw/src/ui/transform/transform_box.rs:452).
        todo!()
    }

    /// Draws the frame over `view` and senses through the host's `canvas` response. The host
    /// calls it after the document and before its tool, so a press the frame takes never reaches
    /// the tool.
    pub fn show(self, ui: &mut Ui, canvas: &Response, view: &CanvasView) -> SelectionFrameResponse {
        // designcraft: crates/ui-egui/src/canvas/frames.rs:148-276 `draw_selection` (1 pt box in the
        // layer colour, key object 3 pt at :180, centre mark 3.5 pt at :248, eight 6.5 pt handles
        // filled white with a hair outline in the colour at :247-258; here `Handle` at 7 pt).
        // new: the press, drag, keys, parts and paint passes of `TransformBox::show`
        // (crates/draw/src/ui/transform/transform_box.rs:457-582, `press_pass` :667, `track_pass`
        // :713, `keys` :765, `parts` :796, `paint` :897) without rotation, the reference point, the
        // readout or apply and discard; the look from `SelectionFrame::style`.
        // needs: A `BoxRules::resize_only()` (task K2) for `rules: None`.
        todo!()
    }
}
