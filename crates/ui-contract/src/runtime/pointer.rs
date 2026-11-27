//! The pointer snapshot, press targets, the shape record and press cancels (8.1, 8.3).
//!
//! The begin hook converts the pass's pointer input once. The sensing core in `primitives` runs the press
//! lifecycle over it and writes the shapes it registers into the record the focus service reads.

use crate::runtime::drag;
use crate::runtime::egui::areas;

use craft_behavior::{Buttons, MultiTap, MultiTouch, PartRef, PointerButton, PointerEvent, PointerKind, PointerSample, PressStart, Shape, Winner};
use craft_keymap::Mods;
use egui::{Id, IdMap, IdSet, Pos2, Rect, Vec2};
use smallvec::SmallVec;

/// The pointer input of one pass of one viewport, egui-converted. Input enters a frame's first pass only (egui hands
/// the frame's events to it, and injected steps and held-back presses follow the frame): a rerun pass of the
/// frame carries no events, so its `delta`, `press`, `release` and `scroll` are empty while the held facts (`pos`,
/// `buttons`) persist. A consumer applies what its pass hands it and never turns a held fact into an event.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct PointerFrame {
    /// The latest pointer position, `None` while the pointer is not in the window.
    pub pos: Option<Pos2>,
    /// The pointer movement of the pass.
    pub delta: Vec2,
    /// The buttons held.
    pub buttons: Buttons,
    /// The primary press of the pass.
    pub press: Option<PressStart>,
    /// Modifiers carried by `press`, including the app's latched modifiers. A pass has at most one press.
    pub press_mods: Option<Mods>,
    /// The button released in the pass.
    pub release: Option<PointerButton>,
    /// The modifiers held, with the ones the app latches for the user (a Modifier Keys panel) added: the app holds
    /// them down for every press and drag. Key chords read the keyboard's own modifiers instead.
    pub mods: Mods,
    /// The modifiers of the pass's last button release, with the latched ones added; the held `mods` in a pass with no
    /// release and in a rerun pass. The one source of a pick's and a drop's modifiers.
    pub pick_mods: Mods,
    /// Where the pass's last button release went up; the latest `pos` in a pass with no release and in a rerun pass. The
    /// one source of a drop's position: a pointer that moves on or leaves the window after the release in the same
    /// frame does not move the drop.
    pub pick_pos: Option<Pos2>,
    /// The tap a per-viewport recognizer reports when the last finger lifts, in the frame's first pass only.
    pub multi_tap: Option<craft_behavior::MultiTap>,
    /// The pass's time in seconds.
    pub time: f64,
    /// The device.
    pub kind: PointerKind,
    /// Pressure and tilt of the current contact.
    pub sample: Option<PointerSample>,
    /// The two-finger gesture.
    pub multi_touch: Option<MultiTouch>,
    /// Wheel and trackpad scroll, in points.
    pub scroll: Vec2,
    /// Pinch and Ctrl or Cmd with the wheel, folded into one factor.
    pub zoom: f32,
    /// The pass's summed `Event::Rotate` radians.
    pub rotation: f32,
    /// The movement that turns a press into a drag.
    pub slop: f32,
    /// The least hit size across for this pointer kind.
    pub target: f32,
}

/// egui's click and drag widgets for the latest press.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[doc(hidden)]
pub struct PressTargets {
    pub click: Option<Id>,
    pub drag: Option<Id>,
}

/// What the begin hook read of the pass, for the sensing core and the services. Built from the pass's own events,
/// so a rerun pass of a frame (see [`PointerFrame`]) reads none.
#[derive(Clone, Debug, Default)]
#[doc(hidden)]
pub struct PassPointer {
    pub frame: Option<PointerFrame>,
    /// The primary events, presses carrying their click counts.
    pub events: Vec<PointerEvent>,
    pub hover: Option<Pos2>,
    pub primary_down: bool,
    pub press_targets: PressTargets,
    pub focus_lost: bool,
    /// Whether the real pointer moved or entered, or a held modifier or key changed, in the pass.
    pub stirred: bool,
    /// A finger landed in the pass while another was down: a press open now is a gesture's.
    pub gesture: bool,
}

/// What one root registered on one layer: its shapes and its egui widgets.
#[derive(Clone, Default)]
struct RootRecord {
    shapes: Vec<Shape<Id>>,
    /// Where each shape of `shapes` stands, at the same index.
    placed: Vec<Placed>,
    /// The root's egui widgets: the click and drag targets that stand for it.
    widgets: Vec<Id>,
    /// The root's widgets sense a click.
    click: bool,
}

/// Where a recorded shape stands: its place in the pass's paint order (the registration that recorded it; later is on
/// top) and the clip it shows through, in its layer's space.
#[derive(Clone, Copy)]
struct Placed {
    order: u32,
    clip: Rect,
}

#[derive(Default)]
pub(crate) struct Shapes {
    this: LayerRecord,
    last: LayerRecord,
    this_bodies: BodyRecord,
    last_bodies: BodyRecord,
    /// The egui widgets components made for their own text (a field's text, a selection part) in the running pass and
    /// the last, each with its component: such a widget takes its press itself.
    this_texts: IdMap<Id>,
    last_texts: IdMap<Id>,
    /// Every id components registered or made in the running pass: the roots, their egui widgets and the widgets made
    /// for their text and editor parts (`made_by_component`).
    this_made: IdSet,
    /// The layers' own egui widgets (an overlay `Area`'s, which senses a click where nothing on the layer does) in the
    /// running pass and the last: such a widget lies under every widget and shape on its layer and stands for none.
    this_grounds: IdSet,
    last_grounds: IdSet,
    /// The screen rects the running pass's components follow the pointer in ([`follows_pointer`]): a move there runs a
    /// pass whatever lies under it.
    this_follows: Vec<Rect>,
    /// The paint order the running pass's next registration takes.
    next: u32,
    /// What lies under the pass's hover point, found over `last` at the begin hook with the pass's egui targets:
    /// every root's hover reads it, and a press at the same point and targets reuses it.
    hover: Option<(Pos2, PressTargets, Under)>,
    /// The egui widgets under the pass's hover point in the last finished pass, with the click and drag targets there
    /// (`Context::widgets_at`), found once at the begin hook for every view's `sensed`.
    hover_widgets: Option<(Vec<Id>, Option<Id>, Option<Id>)>,
    /// A press was open when the pass began: only its target hovers.
    captured: bool,
    /// The pass's tap went to a component.
    tap_taken: bool,
    /// Parts holding a press at the start of this pass, before enclosing arenas process its release.
    pressed_parts: Vec<(Id, PartRef)>,
    /// What lies under another point a press asked for in the pass, with its targets: every root asks for the same
    /// press.
    winner: Option<(Pos2, PressTargets, Under)>,
    /// The resolutions the running pass made (`find_under`), for the count trials.
    #[cfg(feature = "testing")]
    resolutions: u32,
    /// The `made_by_component` lookups the running pass made, for the count trials.
    #[cfg(feature = "suite")]
    made_lookups: u32,
}

/// The roots a press at a point goes to, each with the shape of it that wins it: its click's and its drag's (8.3, H7).
/// Among the roots under the point ([`Under`]), each resolves its own shapes by the arena rule, and the roots rank by
/// their winning shapes' place in the pass's paint order, top first. The top one takes every kind of press its shape
/// senses. Under a top shape that only drags (a window's resize grip), the click goes to the first root below whose
/// shape clicks; under one that only clicks (a button), the drag goes to the first root below whose shape drags, when
/// that root takes no click anywhere. Under a top shape that clicks and passes its drag (`Sense::PASSES_DRAG`), the
/// drag goes to the next shape under it that drags: its own root's, else the first root's below.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct PressWinners {
    pub click: Option<(Winner<Id>, Id)>,
    pub drag: Option<(Winner<Id>, Id)>,
}

impl PressWinners {
    /// The root that starts the press for its kinds of press, with the kinds it takes.
    pub(crate) fn senses_of(&self, root: Id) -> craft_behavior::Sense {
        let mut sense = craft_behavior::Sense::empty();
        if self.click.is_some_and(|(_, owner)| owner == root) {
            sense |= craft_behavior::Sense::CLICK;
        }
        if self.drag.is_some_and(|(_, owner)| owner == root) {
            sense |= craft_behavior::Sense::DRAG;
        }
        sense
    }

    /// The shape of `root` a press goes to: its click's, else its drag's.
    pub(crate) fn hit_of(&self, root: Id) -> Option<Winner<Id>> {
        [self.click, self.drag].into_iter().flatten().find(|(_, owner)| *owner == root).map(|(winner, _)| winner)
    }
}

/// What lies under one point of a viewport, from last pass's shape record: the one answer a press and the hover read
/// (8.3). On the topmost layer under the point, the roots recorded there in the last pass stack in paint order; a
/// transformed body's layer shows its root's own shapes beneath everything on it. A shape counts only where its clip
/// shows it; at a point no clip on the layer holds (one automation names past the window), a shape counts when some of
/// it shows inside its clip. The members of an arena whose host withholds them (`withhold_members`) do not count.
/// Where egui's hit test gives the point to a widget no component made (a raw egui widget on top), no press or hover
/// goes to a root; where it gives it to a widget a component made for its own text, the press is that widget's and the
/// hover goes to that component's topmost shape under the point. A layer's own widget, beneath everything on the layer
/// ([`record_layer_widget`]), is no widget there. The roots that contain the point are those with a shape under it
/// either way.
///
/// Hover shows where a press goes: the press winners' shapes hover, and so does the topmost shape under the point when
/// it takes no press (a label with a tooltip over a clickable row). A shape that takes a press and loses it does not.
#[derive(Clone, Debug, Default)]
pub(crate) struct Under {
    /// The roots a press at the point starts on.
    pub press: PressWinners,
    /// The shape that hovers beside the press winners, with its root: the topmost shape under the point when it takes
    /// no press, or the topmost one of the component whose text widget takes the press.
    pub hover_top: Option<(Winner<Id>, Id)>,
    /// Every root with a shape under the point, on top or not: the roots that contain the pointer.
    pub over: SmallVec<[Id; 4]>,
    /// Every component (root or member) that owns a shape under the point, on top or not.
    pub owners: SmallVec<[Id; 4]>,
    /// The roots' own parts with a shape under the point, on top or not, each with its root: a collection's row that
    /// contains the pointer under the row's own shapes.
    pub parts: SmallVec<[(Id, PartRef); 4]>,
}

impl Under {
    /// The shape of `root` that hovers beside the press winners ([`Under::hover_top`]).
    pub(crate) fn hover_top_of(&self, root: Id) -> Option<Winner<Id>> {
        self.hover_top.filter(|(_, owner)| *owner == root).map(|(winner, _)| winner)
    }

    /// Whether a shape of `root` lies under the point.
    pub(crate) fn contains(&self, root: Id) -> bool {
        self.over.contains(&root)
    }
}
