//! [`BezierOverlay`]: paths with their anchors and direction handles on a canvas, edited as
//! PhotoCraft's Direct Selection and Convert Point tools edit them.

mod model;
pub(crate) mod story;

use egui::{AsIdSalt, Color32, Context, CursorIcon, Id, IdSalt, Key, Modifiers, Painter, Pos2, Response, Shape, Stroke, Ui, vec2};
use kurbo::{Affine, BezPath, CubicBez, ParamCurve as _, ParamCurveNearest as _, PathEl, Point, Rect, Vec2};

pub use model::{EditPath, EditSubpath, HandleSide, Knot, KnotRef, PathEdit, PathSelection};

use super::claim::PrimaryPress;
use super::{CanvasView, Claim, Marquee, MarqueeDrag, MarqueeLook, Part, PartFocus, RulerUnits, hit};
use crate::behavior::nudge;
use crate::edit::order::Turn;
use crate::edit::{EditEvent, EditPolicy, Gesture};
use crate::primitives::{Handle, HandleShape, HandleStyle};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};

/// Above this many anchors in the paths shown, the overlay draws outlines only and puts no anchor
/// in the AccessKit tree (VectorCraft's `OVERLAY_MAX_ANCHORS`).
pub const MAX_ANCHORS: usize = 100_000;

/// What the pointer does to the paths.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BezierTool {
    /// Direct Selection: select and move anchors, turn handles, bend segments, marquee.
    #[default]
    Direct,
    /// Convert Point (also the Pen with ⌥): a click on an anchor makes a corner, a drag from it
    /// pulls out smooth handles; a handle drag moves that handle alone.
    Convert,
    /// Anchors drawn, not edited (PhotoCraft's Path Selection and Pen tools).
    Show,
}

/// Whose direction handles the overlay shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum HandlesShown {
    /// Both of each selected anchor and the two of a selected segment (PhotoCraft).
    #[default]
    Selected,
    /// Every anchor's (DesignCraft).
    All,
}

/// A part of a path under the pointer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BezierHit {
    /// A shown direction handle's end.
    Handle {
        path: u64,
        knot: KnotRef,
        side: HandleSide,
    },
    Anchor {
        path: u64,
        knot: KnotRef,
    },
    /// The segment leaving `segment`, at its parameter `t`.
    Segment {
        path: u64,
        segment: KnotRef,
        t: f64,
    },
}

impl BezierHit {
    /// The path the hit is on.
    pub fn path(&self) -> u64 {
        match *self {
            BezierHit::Handle { path, .. } | BezierHit::Anchor { path, .. } | BezierHit::Segment { path, .. } => path,
        }
    }
}

/// The AccessKit names of a path overlay's parts, the host's translated text.
pub struct BezierNames<'a> {
    /// The group of parts ("Path").
    pub group: String,
    /// A direction handle into an anchor ("In handle").
    pub in_handle: String,
    /// A direction handle out of an anchor ("Out handle").
    pub out_handle: String,
    /// The marquee that selects anchors ("Selection").
    pub marquee: String,
    /// The name of anchor `i` (from 1) of a path's `n`: "Anchor 3 of 12".
    pub anchor: Box<dyn Fn(usize, usize) -> String + 'a>,
}

impl<'a> BezierNames<'a> {
    pub fn new(
        group: impl Into<String>,
        in_handle: impl Into<String>,
        out_handle: impl Into<String>,
        marquee: impl Into<String>,
        anchor: impl Fn(usize, usize) -> String + 'a,
    ) -> Self {
        Self { group: group.into(), in_handle: in_handle.into(), out_handle: out_handle.into(), marquee: marquee.into(), anchor: Box::new(anchor) }
    }
}

/// A path overlay's look, from [`BezierOverlay::style`]. A host preference (VectorCraft's anchor
/// size and handle style) changes a copy and passes it to [`BezierOverlay::look`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct BezierStyle {
    /// Outline width: `bezier.line`.
    pub line: f32,
    /// A selected segment's width: `bezier.selected_segment`.
    pub selected_segment: f32,
    /// A path without a colour of its own: `overlay.line`.
    pub color: Color32,
    /// An anchor: a `bezier.anchor` square of `overlay.handle_fill` outlined in
    /// `overlay.handle_stroke`, or in the path's colour.
    pub anchor: HandleStyle,
    /// A selected anchor: filled with `overlay.handle_stroke`, or the path's colour.
    pub selected_anchor: HandleStyle,
    /// A direction handle's end: a solid `bezier.control` dot of `overlay.handle_stroke`, or the
    /// path's colour; its line from the anchor is the same colour.
    pub control: HandleStyle,
    /// The line from an anchor to a handle's end: `bezier.control_line`.
    pub control_line: f32,
    /// Distance within which the pointer takes a handle, an anchor or a segment: `bezier.hit`.
    pub hit: f32,
    /// Farthest a press may move and still be a click: `bezier.click_slop`.
    pub click_slop: f32,
    /// Largest distance between a curve and the lines it is drawn with: `bezier.flatten`.
    pub flatten: f32,
    /// Side of an anchor drawn by [`BezierOverlay::highlight`]: `bezier.close_anchor`.
    pub highlight_anchor: f32,
    /// Under a highlighted segment: `overlay.halo`, `handle.halo` wider on each side.
    pub highlight_halo: f32,
    /// The colour of [`highlight_halo`](Self::highlight_halo): `overlay.halo`.
    pub halo: Color32,
}

impl BezierStyle {
    /// The anchor, selected anchor and handle end looks of a path: in its own colour, or the
    /// overlay's handle colours, which keep the handle contrast rule.
    fn looks(&self, color: Option<Color32>) -> (HandleStyle, HandleStyle, HandleStyle) {
        let Some(color) = color else { return (self.anchor, self.selected_anchor, self.control) };
        let anchor = HandleStyle { stroke: Stroke::new(self.anchor.stroke.width, color), ..self.anchor };
        let selected = HandleStyle { fill: color, stroke: Stroke::new(self.selected_anchor.stroke.width, color), ..self.selected_anchor };
        let control = HandleStyle { fill: color, ..self.control };
        (anchor, selected, control)
    }
}

/// What a pass of [`BezierOverlay::show`] reports.
#[derive(Clone, Debug, PartialEq)]
#[must_use = "apply the path edits and the selection to the host's paths"]
pub struct BezierResponse {
    /// The overlay's id, also its Tab stop's.
    pub id: Id,
    /// An edit: a drag reports Begin past the click slop, Update and Commit (Cancel on Esc), each
    /// value the whole move from the press; a key a Set.
    pub event: Option<EditEvent<PathEdit>>,
    /// The selection the overlay asks for.
    pub select: Option<PathSelection>,
    /// Insert: add an anchor to the segment leaving a knot of a path, at `t` (PhotoCraft's Add
    /// Anchor Point).
    pub insert: Option<(u64, KnotRef, f64)>,
    /// Delete or Backspace: remove these anchors of a path (Delete Anchor Point).
    pub delete: Option<(u64, Vec<KnotRef>)>,
    /// A secondary click on a part of a path, for the host's path menu.
    pub context: Option<BezierHit>,
    /// The overlay or its marquee owns the current press: the host's tool leaves it alone.
    pub claimed: bool,
}

/// The host's snapping of a dragged point ([`BezierOverlay::snap`]).
type Snap<'a> = Box<dyn Fn(&BezierHit, Point, Modifiers) -> Point + 'a>;
/// The host's pointer per hit kind ([`BezierOverlay::cursor`]).
type Cursor<'a> = Box<dyn Fn(&BezierHit) -> CursorIcon + 'a>;

/// What a press drags, kept between passes.
#[derive(Clone, Debug, PartialEq)]
enum Target {
    /// Anchors with their handles; `from` is the grabbed point, `hit` what was grabbed.
    Anchors {
        path: u64,
        anchors: Vec<KnotRef>,
        from: Point,
        hit: BezierHit,
    },
    Handle {
        path: u64,
        knot: KnotRef,
        side: HandleSide,
        from: Point,
    },
    /// A curved segment, grabbed at `t` where it passes `from`.
    Bend {
        path: u64,
        segment: KnotRef,
        t: f64,
        from: Point,
    },
    Convert {
        path: u64,
        knot: KnotRef,
    },
    /// A press the overlay took that moves nothing (⇧ on a selected anchor deselects it).
    Nothing,
}

/// A press the overlay owns.
#[derive(Clone, Debug)]
struct Drag {
    press: PrimaryPress,
    target: Target,
    /// Past the click slop: the gesture began.
    began: bool,
    /// Esc ended it: the rest of the press does nothing.
    cancelled: bool,
    /// The press waits for another control's typing run to end ([`Claim::turn`]), with the
    /// modifiers at the press; its target and selection are made when its turn comes.
    waits: Option<Modifiers>,
}

/// A keyboard part: an anchor or one of its shown handles.
#[derive(Clone, Copy, Debug, PartialEq)]
struct PartRef {
    /// The index in the paths.
    path: usize,
    knot: KnotRef,
    side: Option<HandleSide>,
    at: Pos2,
}

/// A shown handle: its path's index, its knot, side and screen end.
#[derive(Clone, Copy, Debug)]
struct Shown {
    path: usize,
    knot: KnotRef,
    side: HandleSide,
    at: Pos2,
}

/// What a pass works with.
struct Pass<'p> {
    ctx: Context,
    id: Id,
    canvas: &'p Response,
    view: &'p CanvasView,
    style: BezierStyle,
    /// The handles shown, in path order.
    shown: Vec<Shown>,
    /// Over [`MAX_ANCHORS`]: outlines only.
    limited: bool,
}

/// Paths with anchors and direction handles over a canvas, edited by the pointer and the keyboard
/// as PhotoCraft's Direct Selection and Convert Point tools edit them. The overlay draws, senses
/// and reports; the host applies each [`PathEdit`] to its own path type and passes the edited
/// paths back each frame. The overlay never edits a path.
///
/// ```
/// use craft_ui::canvas::{BezierNames, BezierOverlay, CanvasView, EditPath, EditSubpath, Knot, PathSelection, RulerUnits, kurbo::Point};
/// use craft_ui::localized::LocaleNumbers;
/// # egui::__run_test_ui(|ui| {
/// let (rect, canvas) = ui.allocate_exact_size(egui::vec2(400.0, 300.0), egui::Sense::click_and_drag());
/// let view = CanvasView::new(rect, Point::new(200.0, 150.0), 1.0);
/// let knots = vec![Knot::corner(Point::new(40.0, 40.0)), Knot::smooth(Point::new(200.0, 120.0), Point::new(260.0, 120.0))];
/// let paths = vec![EditPath { key: 1, subpaths: vec![EditSubpath { knots, closed: false }], color: None }];
/// let selection = PathSelection::default();
/// let px = RulerUnits::pixels(LocaleNumbers::from_tag("en"));
/// let names = BezierNames::new("Path", "In handle", "Out handle", "Selection", |i, n| format!("Anchor {i} of {n}"));
/// let r = BezierOverlay::new("direct", &paths, &selection, &px, &px, names).show(ui, &canvas, &view);
/// if !r.claimed {
///     // the tool's input
/// }
/// # });
/// ```
///
/// - Look: outlines `bezier.line` in the path's colour, flattened on the screen; a selected
///   segment `bezier.selected_segment`; anchors hollow squares, selected ones filled; direction
///   handles a line and a dot, for the anchors [`handles`](Self::handles) names, never retracted
///   ones. Above [`MAX_ANCHORS`] anchors, outlines only.
/// - Pointer (in screen points, within `bezier.hit`): shown handles first, then anchors, then
///   segments; the nearest of each kind, of equal distances the path drawn last. The host shows
///   the overlay before its guides and tool input and skips the tool while
///   [`claimed`](BezierResponse::claimed). [`BezierTool`] says what a press does. Past
///   `bezier.click_slop` a press drags; Esc cancels the drag and the rest of the press does
///   nothing.
/// - Keyboard: one Tab stop ([`PartFocus`]). Page Down and Page Up pick the next and previous
///   part (anchors in path order, each followed by its shown handles, in then out), Home and End
///   the first and last; picking an anchor selects it alone. ⇧ Page Down and ⇧ Page Up add the
///   next and previous anchor; ⌘A selects every anchor of the path. The arrows move the selected
///   anchors or the picked handle (⌥ alone); Delete and Backspace report
///   [`delete`](BezierResponse::delete), Insert [`insert`](BezierResponse::insert) halfway along
///   the segment leaving the picked anchor; Esc deselects.
/// - Selection is the host's: it passes it and applies [`select`](BezierResponse::select).
pub struct BezierOverlay<'a> {
    id_salt: IdSalt,
    paths: &'a [EditPath],
    selection: &'a PathSelection,
    units: (&'a RulerUnits, &'a RulerUnits),
    names: BezierNames<'a>,
    tool: BezierTool,
    handles: HandlesShown,
    look: Option<BezierStyle>,
    zero: Point,
    steps: (f64, f64),
    snap: Option<Snap<'a>>,
    cursor: Option<Cursor<'a>>,
    policy: EditPolicy,
    modifiers: Option<Modifiers>,
    highlight: Option<BezierHit>,
}

impl<'a> BezierOverlay<'a> {
    /// The host's `paths` (drawn in order, the last on top) and `selection`, measured in the ruler
    /// units `x` and `y` (for the AccessKit values and the marquee's) and named `names` in the
    /// AccessKit tree.
    pub fn new(
        id_salt: impl AsIdSalt,
        paths: &'a [EditPath],
        selection: &'a PathSelection,
        x: &'a RulerUnits,
        y: &'a RulerUnits,
        names: BezierNames<'a>,
    ) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            paths,
            selection,
            units: (x, y),
            names,
            tool: BezierTool::default(),
            handles: HandlesShown::default(),
            look: None,
            zero: Point::ZERO,
            steps: (nudge::STEP, nudge::BIG_STEP),
            snap: None,
            cursor: None,
            policy: EditPolicy::default(),
            modifiers: None,
            highlight: None,
        }
    }

    /// What the pointer does; [`BezierTool::Direct`] by default.
    pub fn tool(mut self, tool: BezierTool) -> Self {
        self.tool = tool;
        self
    }

    /// Whose direction handles show; [`HandlesShown::Selected`] by default.
    pub fn handles(mut self, handles: HandlesShown) -> Self {
        self.handles = handles;
        self
    }

    /// A look of the host's in place of [`style`](Self::style)'s.
    pub fn look(mut self, style: BezierStyle) -> Self {
        self.look = Some(style);
        self
    }

    /// The document point the rulers count from, for the AccessKit values.
    pub fn zero(mut self, zero: Point) -> Self {
        self.zero = zero;
        self
    }

    /// The arrow keys' move, in document units, plain and with Shift (1 and 10).
    pub fn steps(mut self, step: f64, big_step: f64) -> Self {
        self.steps = (step, big_step);
        self
    }

    /// The host's snapping of a dragged point: what is dragged, the point and the modifiers to the
    /// point to use.
    pub fn snap(mut self, snap: impl Fn(&BezierHit, Point, Modifiers) -> Point + 'a) -> Self {
        self.snap = Some(Box::new(snap));
        self
    }

    /// The host's pointer over and while dragging each kind of hit; the host's tool pointer
    /// stays without it.
    pub fn cursor(mut self, cursor: impl Fn(&BezierHit) -> CursorIcon + 'a) -> Self {
        self.cursor = Some(Box::new(cursor));
        self
    }

    /// The modifiers every test of the overlay reads (the press, the drag, the host's snap) in
    /// place of the input state: a touch host's sticky modifiers, or Ctrl turning snapping off.
    pub fn modifiers(mut self, modifiers: Modifiers) -> Self {
        self.modifiers = Some(modifiers);
        self
    }

    /// Draws `hit` in the hover look of the Add, Delete and Convert Anchor tools: a ring on a
    /// segment, an enlarged anchor on a point. The host picks the hit with its own tool logic;
    /// [`cursor`](Self::cursor) stays for the pointer.
    pub fn highlight(mut self, hit: Option<BezierHit>) -> Self {
        self.highlight = hit;
        self
    }

    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn style(tokens: &UiTokens) -> BezierStyle {
        let b = &tokens.component.bezier;
        let anchor = HandleStyle { size: b.anchor, hit: 2.0 * b.hit, ..Handle::style(tokens, HandleShape::Square) };
        let selected_anchor = HandleStyle { size: b.anchor, hit: 2.0 * b.hit, ..Handle::selected_style(tokens, HandleShape::Square) };
        let control = HandleStyle { size: b.control, hit: 2.0 * b.hit, ..Handle::style(tokens, HandleShape::Rotate) };
        BezierStyle {
            line: b.line,
            selected_segment: b.selected_segment,
            color: tokens.color.overlay.line.to_egui(),
            anchor,
            selected_anchor,
            control,
            control_line: b.control_line,
            hit: b.hit,
            click_slop: b.click_slop,
            flatten: b.flatten,
            highlight_anchor: b.close_anchor,
            highlight_halo: tokens.component.handle.halo,
            halo: tokens.color.overlay.halo.to_egui(),
        }
    }

    /// Draws the paths over `view` and senses through the host's `canvas` response.
    pub fn show(self, ui: &mut Ui, canvas: &Response, view: &CanvasView) -> BezierResponse {
        let ctx = ui.ctx().clone();
        let id = ui.make_persistent_id(self.id_salt);
        let style = self.look.unwrap_or_else(|| BezierOverlay::style(&Tokens::of(ui)));
        let limited = self.paths.iter().map(EditPath::anchor_count).sum::<usize>() > MAX_ANCHORS;
        let shown = if limited { Vec::new() } else { self.shown_handles(view) };
        let pass = Pass { ctx: ctx.clone(), id, canvas, view, style, shown, limited };
        let editing = self.tool != BezierTool::Show && ui.is_enabled();
        let mut out = BezierResponse { id, event: None, select: None, insert: None, delete: None, context: None, claimed: false };
        let picked_key = id.with("craft_ui::canvas::BezierOverlay::picked");
        let parts = if limited { Vec::new() } else { self.parts(&pass) };
        let mut picked: Option<u64> = ctx.data(|d| d.get_temp(picked_key)).filter(|k| parts.iter().any(|(p, _)| p.key == *k));

        // A press whose end the overlay missed is dropped, its gesture with it.
        let mut drag: Option<Drag> = ctx
            .data(|d| d.get_temp(drag_key(id)))
            .filter(|d: &Drag| d.press.holds(&ctx) || (d.waits.is_some() && Claim::is_owner(&ctx, canvas.id, id)));
        let waited = drag.as_ref().is_some_and(|d| d.waits.is_some());
        if let Some(d) = &drag
            && let Some(mods) = d.waits
        {
            drag = match Claim::turn(&ctx, canvas.id, id) {
                Turn::Waits => drag,
                Turn::Runs => Some(self.start(&pass, &parts, &mut picked, &mut out, d.press, mods)),
                Turn::Now => None,
            };
        }
        if drag.is_none() && !Claim::keeps_waiting(&ctx, canvas.id, id, waited) && editing {
            drag = self.press(&pass, &parts, &mut picked, &mut out);
        }
        let reported = Gesture::<PathEdit>::reported(ui, id).map(|(_, value)| value);
        let mut gesture = Gesture::load(ui, id, &reported.unwrap_or_else(idle_value), self.policy).focus_owners(&[]).enabled(ui.is_enabled());
        if let Some(d) = &mut drag {
            if d.began && !gesture.is_active() {
                // Esc, `cancel_all` or a missed pass ended it; the press stays claimed.
                d.cancelled = true;
            }
            if !d.began && !d.cancelled && d.waits.is_none() && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
                d.cancelled = true;
            }
        }
        if let Some(d) = drag.take() {
            drag = if d.waits.is_some() { Some(d) } else { self.track(&pass, d, &mut gesture) };
        }
        let marquee_id = canvas.id.with_salt(IdSalt::new(marquee_salt(id)));
        if editing && self.tool == BezierTool::Direct && drag.is_none() {
            self.marquee(ui, &pass, &parts, &mut picked, &mut out);
        }
        if editing && drag.is_none() {
            self.pointer(&pass, &mut out);
        }

        let fallback = picked.or_else(|| self.first_selected(&parts));
        let focus_parts: Vec<Part> = parts.iter().map(|(p, _)| p.clone()).collect();
        let focus = PartFocus::new(id, &self.names.group).parts(&focus_parts).picked(fallback).focusable(editing).show(ui, canvas);
        match focus.pick {
            Some(Some(key)) => {
                picked = Some(key);
                if let Some((_, part)) = parts.iter().find(|(p, _)| p.key == key)
                    && part.side.is_none()
                {
                    let path = &self.paths[part.path];
                    out.select = Some(PathSelection { path: Some(path.key), anchors: vec![part.knot], segment: None });
                }
            }
            Some(None) => picked = None,
            None => picked = fallback,
        }
        if focus.focused && editing && drag.is_none() {
            self.keys(ui, &parts, &mut picked, &mut gesture, &mut out);
        }

        out.event = gesture.finish();
        out.claimed = Claim::owner(&ctx, canvas.id).is_some_and(|owner| owner == id || owner == marquee_id);
        ctx.data_mut(|d| {
            match &drag {
                Some(drag) => {
                    d.insert_temp(drag_key(id), drag.clone());
                }
                None => d.remove::<Drag>(drag_key(id)),
            }
            match picked {
                Some(key) => {
                    d.insert_temp(picked_key, key);
                }
                None => d.remove::<u64>(picked_key),
            }
        });
        self.paint(ui, &pass);
        if focus.keyboard
            && let Some((_, part)) = picked.and_then(|k| parts.iter().find(|(p, _)| p.key == k))
        {
            let size = if part.side.is_some() { style.control.size } else { style.anchor.size };
            PartFocus::ring(&ui.painter_at(view.rect), egui::Rect::from_center_size(part.at, vec2(size, size)));
        }
        out
    }

    /// A press on the canvas: the part under it, taken for the tool's gesture.
    fn press(&self, pass: &Pass<'_>, parts: &[(Part, PartRef)], picked: &mut Option<u64>, out: &mut BezierResponse) -> Option<Drag> {
        let ctx = &pass.ctx;
        if !ctx.input(|i| i.pointer.primary_pressed()) {
            return None;
        }
        let press = PrimaryPress::current(ctx)?;
        if !(pass.view.rect.contains(press.origin) && pass.canvas.contains_pointer()) {
            return None;
        }
        let hit = self.hit_at(pass, press.origin)?;
        if !self.takes(&hit) || !Claim::take(ctx, pass.canvas.id, pass.id) {
            return None;
        }
        let mods = self.modifiers.unwrap_or_else(|| ctx.input(|i| i.modifiers));
        if Claim::turn(ctx, pass.canvas.id, pass.id) == Turn::Waits {
            return Some(Drag { press, target: Target::Nothing, began: false, cancelled: false, waits: Some(mods) });
        }
        Some(self.start(pass, parts, picked, out, press, mods))
    }

    /// The press `press` made with `mods`, against the paths now: the target it drags and the
    /// selection it makes.
    fn start(
        &self,
        pass: &Pass<'_>,
        parts: &[(Part, PartRef)],
        picked: &mut Option<u64>,
        out: &mut BezierResponse,
        press: PrimaryPress,
        mods: Modifiers,
    ) -> Drag {
        let nothing = Drag { press, target: Target::Nothing, began: false, cancelled: true, waits: None };
        let Some(hit) = self.hit_at(pass, press.origin).filter(|hit| self.takes(hit)) else {
            return nothing;
        };
        let part_key = |path: u64, knot: KnotRef, side: Option<HandleSide>| {
            parts.iter().find(|(_, r)| self.paths[r.path].key == path && r.knot == knot && r.side == side).map(|(p, _)| p.key)
        };
        let target = match hit {
            BezierHit::Handle { path, knot, side } => {
                *picked = part_key(path, knot, Some(side));
                let from = self.knot(path, knot).map_or(Point::ZERO, |k| k.handle(side));
                Target::Handle { path, knot, side, from }
            }
            BezierHit::Anchor { path, knot } => {
                *picked = part_key(path, knot, None);
                let from = self.knot(path, knot).map_or(Point::ZERO, |k| k.anchor);
                if self.tool == BezierTool::Convert {
                    Target::Convert { path, knot }
                } else {
                    let selected = self.selection.has_anchor(path, knot);
                    let anchors = if mods.alt {
                        self.subpath_anchors(path, knot.subpath)
                    } else if mods.shift && selected {
                        let rest: Vec<KnotRef> = self.selection.anchors.iter().copied().filter(|&k| k != knot).collect();
                        out.select = Some(PathSelection { path: Some(path), anchors: rest, segment: None });
                        return Drag { press, target: Target::Nothing, began: false, cancelled: false, waits: None };
                    } else if mods.shift && self.selection.path == Some(path) {
                        self.selection.anchors.iter().copied().chain([knot]).collect()
                    } else if selected {
                        self.selection.anchors.clone()
                    } else {
                        vec![knot]
                    };
                    if !selected || mods.alt || mods.shift {
                        out.select = Some(PathSelection { path: Some(path), anchors: anchors.clone(), segment: None });
                    }
                    Target::Anchors { path, anchors, from, hit }
                }
            }
            BezierHit::Segment { path, segment, t } => {
                *picked = None;
                let sub = self.subpath(path, segment.subpath);
                let from = sub.and_then(|s| s.segment(segment.knot)).map_or(Point::ZERO, |c| c.eval(t));
                if mods.alt {
                    let anchors = self.subpath_anchors(path, segment.subpath);
                    out.select = Some(PathSelection { path: Some(path), anchors: anchors.clone(), segment: None });
                    Target::Anchors { path, anchors, from, hit }
                } else {
                    out.select = Some(PathSelection { path: Some(path), anchors: Vec::new(), segment: Some(segment) });
                    match sub.and_then(|s| s.is_straight(segment.knot).then(|| s.segment_ends(segment.knot)).flatten()) {
                        Some((a, b)) => {
                            Target::Anchors { path, anchors: vec![KnotRef::new(segment.subpath, a), KnotRef::new(segment.subpath, b)], from, hit }
                        }
                        None => Target::Bend { path, segment, t, from },
                    }
                }
            }
        };
        Drag { press, target, began: false, cancelled: false, waits: None }
    }

    /// One pass of a press the overlay owns: the press that goes on, if it does.
    fn track(&self, pass: &Pass<'_>, mut d: Drag, gesture: &mut Gesture<PathEdit>) -> Option<Drag> {
        let (pointer, down, input_mods) = pass.ctx.input(|i| (i.pointer.latest_pos(), i.pointer.primary_down(), i.modifiers));
        let mods = self.modifiers.unwrap_or(input_mods);
        if d.cancelled {
            return down.then_some(d);
        }
        if let Some(pointer) = pointer {
            if !d.began && d.target != Target::Nothing && pointer.distance(d.press.origin) > pass.style.click_slop {
                gesture.begin();
                d.began = true;
            }
            if d.began
                && let Some(edit) = self.edit(pass.view, &d, pointer, mods)
            {
                gesture.update(edit, mods);
            }
        }
        if d.began
            && let Some(cursor) = &self.cursor
            && let Some(hit) = target_hit(&d.target)
        {
            pass.ctx.set_cursor_icon(cursor(&hit));
        }
        if down {
            return Some(d);
        }
        if d.began {
            gesture.commit();
        } else if let Target::Convert { path, knot } = d.target {
            gesture.set(PathEdit::Convert { path, knot, out: None });
        }
        None
    }

    /// The edit of a press dragged to `pointer`: the whole move from the press.
    fn edit(&self, view: &CanvasView, d: &Drag, pointer: Pos2, mods: Modifiers) -> Option<PathEdit> {
        let delta = view.to_doc(pointer) - view.to_doc(d.press.origin);
        let snapped = |hit: &BezierHit, p: Point| self.snap.as_ref().map_or(p, |snap| snap(hit, p, mods));
        Some(match &d.target {
            Target::Anchors { path, anchors, from, hit } => {
                let delta = if mods.shift { constrain(delta) } else { delta };
                PathEdit::MoveAnchors { path: *path, anchors: anchors.clone(), by: snapped(hit, *from + delta) - *from }
            }
            &Target::Handle { path, knot, side, from } => {
                let to = snapped(&BezierHit::Handle { path, knot, side }, from + delta);
                PathEdit::MoveHandle { path, knot, side, to, alone: self.tool == BezierTool::Convert || mods.alt }
            }
            &Target::Bend { path, segment, t, from } => {
                let by = snapped(&BezierHit::Segment { path, segment, t }, from + delta) - from;
                PathEdit::BendSegment { path, segment, t, by }
            }
            &Target::Convert { path, knot } => {
                let out = snapped(&BezierHit::Anchor { path, knot }, view.to_doc(pointer));
                PathEdit::Convert { path, knot, out: Some(out) }
            }
            Target::Nothing => return None,
        })
    }

    /// A press on empty canvas runs a marquee: on release it selects the anchors inside it on the
    /// frontmost path that has any (⇧ adds); a click deselects.
    fn marquee(&self, ui: &mut Ui, pass: &Pass<'_>, parts: &[(Part, PartRef)], picked: &mut Option<u64>, out: &mut BezierResponse) {
        let m = MarqueeDrag::new(marquee_salt(pass.id)).show(ui, pass.canvas, pass.view);
        if m.clicked && *self.selection != PathSelection::default() {
            out.select = Some(PathSelection::default());
        }
        if let Some(EditEvent::Commit { value, .. } | EditEvent::Set { value, .. }) = &m.event {
            let add = m.press_modifiers.shift;
            let rect = value.rect();
            let inside = self.paths.iter().rev().find_map(|path| {
                let knots: Vec<KnotRef> = path.knot_refs().filter(|&k| path.knot(k).is_some_and(|k| contains(rect, k.anchor))).collect();
                (!knots.is_empty()).then_some((path.key, knots))
            });
            match inside {
                Some((path, knots)) => {
                    let mut anchors = if add && self.selection.path == Some(path) { self.selection.anchors.clone() } else { Vec::new() };
                    for k in knots {
                        if !anchors.contains(&k) {
                            anchors.push(k);
                        }
                    }
                    *picked = parts
                        .iter()
                        .find(|(_, r)| self.paths[r.path].key == path && r.side.is_none() && Some(&r.knot) == anchors.first())
                        .map(|(p, _)| p.key);
                    out.select = Some(PathSelection { path: Some(path), anchors, segment: None });
                    // The keys then act on what the marquee took.
                    if !parts.is_empty() {
                        ui.memory_mut(|mem| mem.request_focus(pass.id));
                    }
                }
                None if !add => out.select = Some(PathSelection::default()),
                None => {}
            }
        }
        if let Some(rect) = m.rect {
            let (x, _) = self.units;
            Marquee::rect(rect, &self.names.marquee).look(MarqueeLook::Ants).bounds_value(&x.numbers, &x.unit).id_salt(marquee_salt(pass.id)).show(
                ui,
                pass.canvas,
                pass.view,
            );
        }
    }

    /// The hover pointer and secondary clicks.
    fn pointer(&self, pass: &Pass<'_>, out: &mut BezierResponse) {
        let canvas = pass.canvas;
        let other = Claim::owner(&pass.ctx, canvas.id).is_some_and(|owner| owner != pass.id);
        if !other
            && let Some(cursor) = &self.cursor
            && let Some(pointer) = canvas.hover_pos()
            && let Some(hit) = self.hit_at(pass, pointer)
            && self.takes(&hit)
        {
            pass.ctx.set_cursor_icon(cursor(&hit));
        }
        if canvas.secondary_clicked() {
            out.context = canvas.interact_pointer_pos().and_then(|p| self.hit_at(pass, p));
        }
    }

    /// Whether the tool takes a press on `hit`.
    fn takes(&self, hit: &BezierHit) -> bool {
        match self.tool {
            BezierTool::Direct => true,
            BezierTool::Convert => !matches!(hit, BezierHit::Segment { .. }),
            BezierTool::Show => false,
        }
    }

    /// The keys while the overlay has focus and no press is under way.
    fn keys(&self, ui: &Ui, parts: &[(Part, PartRef)], picked: &mut Option<u64>, gesture: &mut Gesture<PathEdit>, out: &mut BezierResponse) {
        let part = picked.and_then(|k| parts.iter().find(|(p, _)| p.key == k)).map(|(_, r)| *r);
        let focused_path = self.focused_path(part);
        let (next, previous) = ui.input_mut(|i| (i.consume_key(Modifiers::SHIFT, Key::PageDown), i.consume_key(Modifiers::SHIFT, Key::PageUp)));
        if (next || previous) && next != previous {
            let anchors: Vec<&(Part, PartRef)> = parts.iter().filter(|(_, r)| r.side.is_none()).collect();
            let at = part.and_then(|r| anchors.iter().position(|(_, a)| a.path == r.path && a.knot == r.knot));
            let to = match (at, next) {
                (Some(i), true) => Some((i + 1).min(anchors.len().saturating_sub(1))),
                (Some(i), false) => Some(i.saturating_sub(1)),
                (None, true) => (!anchors.is_empty()).then_some(0),
                (None, false) => anchors.len().checked_sub(1),
            };
            if let Some(&&(ref p, r)) = to.and_then(|i| anchors.get(i)) {
                let path = self.paths[r.path].key;
                let mut list = if self.selection.path == Some(path) { self.selection.anchors.clone() } else { Vec::new() };
                if !list.contains(&r.knot) {
                    list.push(r.knot);
                }
                *picked = Some(p.key);
                out.select = Some(PathSelection { path: Some(path), anchors: list, segment: None });
            }
        }
        if ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::A))
            && let Some(path) = focused_path
        {
            out.select = Some(PathSelection { path: Some(path.key), anchors: path.knot_refs().collect(), segment: None });
        }
        if let Some((by, alone)) = nudge::read_with_alt(ui, self.steps.0, self.steps.1) {
            match part {
                Some(PartRef { path, knot, side: Some(side), .. }) => {
                    let path = &self.paths[path];
                    if let Some(k) = path.knot(knot) {
                        gesture.set(PathEdit::MoveHandle { path: path.key, knot, side, to: k.handle(side) + by, alone });
                    }
                }
                _ => {
                    if let Some(path) = self.selection.path
                        && !self.selection.anchors.is_empty()
                    {
                        gesture.set(PathEdit::MoveAnchors { path, anchors: self.selection.anchors.clone(), by });
                    }
                }
            }
        }
        if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Delete) || i.consume_key(Modifiers::NONE, Key::Backspace))
            && let Some(path) = self.selection.path
            && !self.selection.anchors.is_empty()
        {
            out.delete = Some((path, self.selection.anchors.clone()));
        }
        if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Insert))
            && let Some(r) = part
        {
            let path = &self.paths[r.path];
            if path.subpaths.get(r.knot.subpath).is_some_and(|s| s.segment_ends(r.knot.knot).is_some()) {
                out.insert = Some((path.key, r.knot, 0.5));
            }
        }
        if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) && *self.selection != PathSelection::default() {
            out.select = Some(PathSelection::default());
        }
    }

    /// The path ⌘A selects: the selection's, else the picked part's, else the only one.
    fn focused_path(&self, part: Option<PartRef>) -> Option<&EditPath> {
        let by_key = |key: u64| self.paths.iter().find(|p| p.key == key);
        self.selection
            .path
            .and_then(by_key)
            .or_else(|| part.map(|r| &self.paths[r.path]))
            .or_else(|| if self.paths.len() == 1 { self.paths.first() } else { None })
    }

    /// The part of the first selected anchor, which the keys act on while nothing is picked.
    fn first_selected(&self, parts: &[(Part, PartRef)]) -> Option<u64> {
        let path = self.selection.path?;
        let first = self.selection.anchors.first()?;
        parts.iter().find(|(_, r)| r.side.is_none() && &r.knot == first && self.paths[r.path].key == path).map(|(p, _)| p.key)
    }

    /// The keyboard parts and their AccessKit nodes: anchors in path order, each followed by its
    /// shown handles.
    fn parts(&self, pass: &Pass<'_>) -> Vec<(Part, PartRef)> {
        let (view, hit) = (pass.view, pass.style.hit);
        let (x, y) = self.units;
        let value = |p: Point| format!("{}, {}", x.format_value(p.x, self.zero.x), y.format_value(p.y, self.zero.y));
        let reach = |at: Pos2| egui::Rect::from_center_size(at, vec2(2.0 * hit, 2.0 * hit));
        let mut parts = Vec::new();
        for (index, path) in self.paths.iter().enumerate() {
            let count = path.anchor_count();
            for (n, knot_ref) in path.knot_refs().enumerate() {
                let Some(knot) = path.knot(knot_ref) else { continue };
                let at = view.to_screen(knot.anchor);
                let selected = self.selection.has_anchor(path.key, knot_ref);
                let key = part_key(path.key, knot_ref, None);
                let name = (self.names.anchor)(n + 1, count);
                parts.push((
                    Part { key, name, value: Some(value(knot.anchor)), rect: reach(at), selected },
                    PartRef { path: index, knot: knot_ref, side: None, at },
                ));
                for s in pass.shown.iter().filter(|s| s.path == index && s.knot == knot_ref) {
                    let name = if s.side == HandleSide::In { &self.names.in_handle } else { &self.names.out_handle };
                    parts.push((
                        Part {
                            key: part_key(path.key, knot_ref, Some(s.side)),
                            name: name.clone(),
                            value: Some(value(knot.handle(s.side))),
                            rect: reach(s.at),
                            selected: false,
                        },
                        PartRef { path: index, knot: knot_ref, side: Some(s.side), at: s.at },
                    ));
                }
            }
        }
        parts
    }

    /// The handles to show, in path order, in then out, leaving out retracted ones.
    fn shown_handles(&self, view: &CanvasView) -> Vec<Shown> {
        let mut shown = Vec::new();
        let sel = self.selection;
        for (index, path) in self.paths.iter().enumerate() {
            let segment_ends = sel.segment.filter(|_| sel.path == Some(path.key)).and_then(|s| {
                let (a, b) = path.subpaths.get(s.subpath)?.segment_ends(s.knot)?;
                Some((KnotRef::new(s.subpath, a), KnotRef::new(s.subpath, b)))
            });
            for knot_ref in path.knot_refs() {
                let Some(knot) = path.knot(knot_ref) else { continue };
                for side in [HandleSide::In, HandleSide::Out] {
                    let wanted = match self.handles {
                        HandlesShown::All => true,
                        HandlesShown::Selected => {
                            sel.has_anchor(path.key, knot_ref)
                                || segment_ends
                                    .is_some_and(|(a, b)| (side == HandleSide::Out && a == knot_ref) || (side == HandleSide::In && b == knot_ref))
                        }
                    };
                    if wanted && !knot.retracted(side) {
                        shown.push(Shown { path: index, knot: knot_ref, side, at: view.to_screen(knot.handle(side)) });
                    }
                }
            }
        }
        shown
    }

    /// What is under `pointer`: the shown handles first, then anchors, then segments; the nearest
    /// within `bezier.hit` of each kind, of equal distances the path drawn last.
    fn hit_at(&self, pass: &Pass<'_>, pointer: Pos2) -> Option<BezierHit> {
        let reach = pass.style.hit;
        let nearest = |best: &mut Option<(BezierHit, f32)>, hit: BezierHit, d: f32| {
            if d <= reach && best.is_none_or(|(_, bd)| d <= bd) {
                *best = Some((hit, d));
            }
        };
        let mut best = None;
        for s in &pass.shown {
            let path = self.paths[s.path].key;
            nearest(&mut best, BezierHit::Handle { path, knot: s.knot, side: s.side }, hit::point(pointer, s.at));
        }
        if best.is_none() && !pass.limited {
            for path in self.paths {
                for k in path.knot_refs() {
                    if let Some(knot) = path.knot(k) {
                        nearest(&mut best, BezierHit::Anchor { path: path.key, knot: k }, hit::point(pointer, pass.view.to_screen(knot.anchor)));
                    }
                }
            }
        }
        if best.is_none() {
            let to_screen = pass.view.affine();
            let p = Point::new(f64::from(pointer.x), f64::from(pointer.y));
            for path in self.paths {
                for (s, sub) in path.subpaths.iter().enumerate() {
                    for i in 0..sub.segment_count() {
                        let Some(c) = sub.segment(i) else { continue };
                        let nearest_t = map_cubic(to_screen, c).nearest(p, 1e-3);
                        let d = nearest_t.distance_sq.sqrt() as f32;
                        nearest(&mut best, BezierHit::Segment { path: path.key, segment: KnotRef::new(s, i), t: nearest_t.t }, d);
                    }
                }
            }
        }
        best.map(|(hit, _)| hit)
    }

    /// Draws the outlines, the selected segment, the shown handles and the anchors.
    fn paint(&self, ui: &Ui, pass: &Pass<'_>) {
        let (style, view) = (&pass.style, pass.view);
        let painter = ui.painter_at(view.rect);
        let ppp = ui.pixels_per_point();
        let to_screen = view.affine();
        let tolerance = f64::from(style.flatten);
        for (index, path) in self.paths.iter().enumerate() {
            let color = path.color.unwrap_or(style.color);
            for sub in &path.subpaths {
                outline(&painter, to_screen * sub.bez_path(), tolerance, Stroke::new(style.line, color));
            }
            if let Some(s) = self.selection.segment.filter(|_| self.selection.path == Some(path.key))
                && let Some(c) = path.subpaths.get(s.subpath).and_then(|sub| sub.segment(s.knot))
            {
                let mut seg = BezPath::new();
                seg.move_to(c.p0);
                seg.curve_to(c.p1, c.p2, c.p3);
                outline(&painter, to_screen * seg, tolerance, Stroke::new(style.selected_segment, color));
            }
            if pass.limited {
                continue;
            }
            let (anchor, selected, control) = style.looks(path.color);
            let shown: Vec<&Shown> = pass.shown.iter().filter(|s| s.path == index).collect();
            for s in &shown {
                if let Some(k) = path.knot(s.knot) {
                    painter.line_segment([view.to_screen(k.anchor), s.at], Stroke::new(style.control_line, control.fill));
                }
            }
            for k in path.knot_refs() {
                let Some(knot) = path.knot(k) else { continue };
                let look = if self.selection.has_anchor(path.key, k) { &selected } else { &anchor };
                painter.extend(Handle::shapes_with(look, HandleShape::Square, view.to_screen(knot.anchor), ppp));
            }
            for s in &shown {
                painter.extend(Handle::shapes_with(&control, HandleShape::Rotate, s.at, ppp));
            }
        }
        if let Some(hit) = self.highlight {
            self.paint_highlight(&painter, pass, hit, ppp);
        }
    }

    /// The hover look of the Add, Delete and Convert Anchor tools on `hit`.
    fn paint_highlight(&self, painter: &Painter, pass: &Pass<'_>, hit: BezierHit, ppp: f32) {
        let (style, view) = (&pass.style, pass.view);
        let Some(path) = self.paths.iter().find(|p| p.key == hit.path()) else { return };
        let color = path.color.unwrap_or(style.color);
        let tolerance = f64::from(style.flatten);
        let (anchor, _, control) = style.looks(path.color);
        let (point, shape, base) = match hit {
            BezierHit::Segment { segment, .. } => {
                if let Some(c) = path.subpaths.get(segment.subpath).and_then(|sub| sub.segment(segment.knot)) {
                    let mut seg = BezPath::new();
                    seg.move_to(c.p0);
                    seg.curve_to(c.p1, c.p2, c.p3);
                    let seg = view.affine() * seg;
                    outline(painter, seg.clone(), tolerance, Stroke::new(style.selected_segment + 2.0 * style.highlight_halo, style.halo));
                    outline(painter, seg, tolerance, Stroke::new(style.selected_segment, color));
                }
                return;
            }
            BezierHit::Anchor { knot, .. } => (path.knot(knot).map(|k| k.anchor), HandleShape::Square, anchor),
            BezierHit::Handle { knot, side, .. } => (path.knot(knot).map(|k| k.handle(side)), HandleShape::Rotate, control),
        };
        if let Some(at) = point {
            let look = HandleStyle { size: style.highlight_anchor, ..base };
            painter.extend(Handle::shapes_with(&look, shape, view.to_screen(at), ppp));
        }
    }

    fn knot(&self, path: u64, knot: KnotRef) -> Option<&Knot> {
        self.paths.iter().find(|p| p.key == path)?.knot(knot)
    }

    fn subpath(&self, path: u64, subpath: usize) -> Option<&EditSubpath> {
        self.paths.iter().find(|p| p.key == path)?.subpaths.get(subpath)
    }

    /// Every anchor of a subpath (⌥ selects them).
    fn subpath_anchors(&self, path: u64, subpath: usize) -> Vec<KnotRef> {
        self.subpath(path, subpath).map_or_else(Vec::new, |s| (0..s.knots.len()).map(|k| KnotRef::new(subpath, k)).collect())
    }
}

/// The gesture's host value while idle: an edit that moves nothing, so a key's Set always differs.
fn idle_value() -> PathEdit {
    PathEdit::MoveAnchors { path: 0, anchors: Vec::new(), by: Vec2::ZERO }
}

fn drag_key(id: Id) -> Id {
    id.with("craft_ui::canvas::BezierOverlay::drag")
}

fn marquee_salt(id: Id) -> (&'static str, Id) {
    ("craft_ui::canvas::BezierOverlay::marquee", id)
}

/// A keyboard part's key, stable while the path keeps its knots.
fn part_key(path: u64, knot: KnotRef, side: Option<HandleSide>) -> u64 {
    IdSalt::new(("craft_ui::canvas::BezierOverlay::part", path, knot, side)).value()
}

/// What a target drags, for the host's pointer.
fn target_hit(target: &Target) -> Option<BezierHit> {
    match *target {
        Target::Anchors { hit, .. } => Some(hit),
        Target::Handle { path, knot, side, .. } => Some(BezierHit::Handle { path, knot, side }),
        Target::Bend { path, segment, t, .. } => Some(BezierHit::Segment { path, segment, t }),
        Target::Convert { path, knot } => Some(BezierHit::Anchor { path, knot }),
        Target::Nothing => None,
    }
}

/// `v` on the nearest 45° ray (PhotoCraft's `Edit::constrain`): its projection there.
pub(super) fn constrain(v: Vec2) -> Vec2 {
    if v.hypot2() == 0.0 {
        return v;
    }
    let step = std::f64::consts::FRAC_PI_4;
    let angle = (v.atan2() / step).round() * step;
    let dir = Vec2::from_angle(angle);
    dir * v.dot(dir)
}

fn contains(r: Rect, p: Point) -> bool {
    (r.x0..=r.x1).contains(&p.x) && (r.y0..=r.y1).contains(&p.y)
}

fn map_cubic(a: Affine, c: CubicBez) -> CubicBez {
    CubicBez::new(a * c.p0, a * c.p1, a * c.p2, a * c.p3)
}

/// Strokes a screen-space path as polylines within `tolerance` of its curves.
pub(super) fn outline(painter: &Painter, path: BezPath, tolerance: f64, stroke: Stroke) {
    let mut lines: Vec<(Vec<Pos2>, bool)> = Vec::new();
    let pos = |p: Point| Pos2::new(p.x as f32, p.y as f32);
    kurbo::flatten(&path, tolerance, |el| match el {
        PathEl::MoveTo(p) => lines.push((vec![pos(p)], false)),
        PathEl::LineTo(p) => {
            if let Some((points, _)) = lines.last_mut() {
                points.push(pos(p));
            }
        }
        PathEl::ClosePath => {
            if let Some((_, closed)) = lines.last_mut() {
                *closed = true;
            }
        }
        _ => {}
    });
    for (points, closed) in lines {
        if points.len() < 2 {
            continue;
        }
        painter.add(if closed { Shape::closed_line(points, stroke) } else { Shape::line(points, stroke) });
    }
}

#[cfg(test)]
mod tests;
