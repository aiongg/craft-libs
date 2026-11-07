//! The sensing core (1.9, 8.2, 8.3): the arena's registrations, the press lifecycle over last pass's shapes, and
//! the part results and member responses built from them.
//!
//! A component's root senses through one egui widget; its parts and its members have none. The root resolves
//! hover and presses itself against the shapes registered last pass, so the winner never depends on show order.
//! Every root runs the lifecycle, arena root or not: a leaf is an arena of its own id with no members.

use crate::runtime::egui::areas::{self, Layer, LayerMap};
use crate::runtime::egui::pass;
use crate::runtime::egui::response::{self, MemberView};
use crate::runtime::env::ArenaId;
use crate::runtime::frame::PerViewport;
use crate::runtime::memory::sub;
use crate::runtime::pointer;
use craft_behavior::{
    CursorIcon, HitLevel, HitPriority, HitShape, PartKey, PartRef, PassInput, PointerSample, PressMachine, PressPhase, PressState, ResolveFor, Sense,
    Shape, resolve_mapped,
};
use craft_keymap::Mods;
use craft_theme::Flags;
use egui::{Context, Id, Pos2, Rect, Response, Ui};
use std::collections::HashMap;

/// A part's results this pass.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct PartResult {
    /// The part's bounds.
    pub rect: Rect,
    /// Its press state.
    pub press: PressState,
    /// The pointer is over it and nothing above takes the hover.
    pub hovered: bool,
    /// The pointer's position while `hovered`, in the owner's layer space.
    pub hover_pos: Option<Pos2>,
    /// The value flags the component set on the part.
    pub flags: Flags,
}

impl PartResult {
    /// What a read returns for a part with no press history: face mode, or a part first registered this pass.
    pub const IDLE: PartResult = PartResult { rect: Rect::NOTHING, press: PressState::IDLE, hovered: false, hover_pos: None, flags: Flags::empty() };
}

/// A text part's result this pass.
#[derive(Clone)]
#[non_exhaustive]
pub struct TextOutcome<V> {
    /// The part's rect.
    pub rect: Rect,
    /// The draft being typed, if any.
    pub draft: Option<String>,
    /// The value the draft parsed to.
    pub value: Option<V>,
    /// The laid-out text, so an editor draws gutters and line numbers beside it.
    pub layout: Option<super::TextLayout>,
    /// Whether the caret stroke shows this pass (the caret blink).
    pub caret_visible: bool,
    /// The draft is not a value of the codec.
    pub invalid: bool,
    /// An IME composition is under way.
    pub composing: bool,
    /// The completion candidates the list shows; empty while it is closed.
    pub completions: Vec<super::Completion>,
}

impl<V> TextOutcome<V> {
    /// An outcome over `rect` with no draft or value.
    pub const fn new(rect: Rect) -> Self {
        TextOutcome { rect, draft: None, value: None, layout: None, caret_visible: true, invalid: false, composing: false, completions: Vec::new() }
    }
}

/// A part as the sensing core registers it.
// F2-MERGE: F3 (`PartBuilder` adds tooltip and `pointer_only`)
#[derive(Clone, Debug)]
pub(crate) struct PartDecl {
    pub key: PartKey,
    pub index: Option<u64>,
    pub shape: HitShape,
    pub sense: Sense,
    pub priority: HitPriority,
    pub cursor: Option<CursorIcon>,
    /// Drawn by `paint_over`: stacks above the body members.
    pub over: bool,
    pub flags: Flags,
}

impl PartDecl {
    /// A part of `key` over `shape` sensing `sense`.
    pub(crate) fn new(key: PartKey, shape: impl Into<HitShape>, sense: Sense) -> Self {
        PartDecl { key, index: None, shape: shape.into(), sense, priority: HitPriority::BODY, cursor: None, over: false, flags: Flags::default() }
    }

    fn part_ref(&self) -> PartRef {
        PartRef { key: self.key, index: self.index }
    }
}

/// One member's registration for this pass.
#[derive(Clone, Debug)]
pub(crate) struct MemberDecl {
    pub id: Id,
    pub rect: Rect,
    pub sense: Sense,
    pub parts: Vec<PartDecl>,
    /// The shapes lie in the body layer's space; `join` sets it.
    pub local: bool,
}

impl MemberDecl {
    /// A member `id` over `rect` sensing `sense`, with `parts`.
    pub(crate) fn new(id: Id, rect: Rect, sense: Sense, parts: Vec<PartDecl>) -> Self {
        MemberDecl { id, rect, sense, parts, local: false }
    }
}

/// What a root registers at step 8: its rect and its own sense.
#[derive(Clone, Copy, Debug)]
struct RootInfo {
    id: Id,
    rect: Rect,
    own_sense: Sense,
}

/// A shape's owner and, for a part, its key.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Hit {
    pub owner: Id,
    pub part: Option<PartRef>,
    pub sense: Sense,
    pub local: bool,
}

impl Hit {
    fn same(&self, other: &Hit) -> bool {
        self.owner == other.owner && self.part == other.part
    }
}

/// What the arena resolved this pass; the root computes it once, members and the root read it.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Outcome {
    pass: u64,
    pub hover: Option<Hit>,
    /// The press winner while a press is open, or the one released or cancelled this pass.
    pub target: Option<Hit>,
    pub phase: Option<PressPhase>,
    pub dragging: bool,
    /// The press winner released as a click this pass.
    pub activated: Option<Hit>,
    xf: Option<LayerMap>,
}

#[derive(Default)]
struct ArenaState {
    /// The pass the registration lists below belong to.
    pass: u64,
    members: Vec<MemberDecl>,
    previous: Vec<MemberDecl>,
    own: Vec<PartDecl>,
    previous_own: Vec<PartDecl>,
    /// The senses and bounds of the members that joined this pass and last pass.
    union_sense: Sense,
    previous_union_sense: Sense,
    layer: Option<Layer>,
    root: Option<RootInfo>,
    /// The egui widgets of the root: the root's, the transform forwarder and the edge widgets.
    widgets: Vec<Id>,
    machine: PressMachine<Hit>,
    outcome: Option<Outcome>,
}

#[derive(Default)]
struct Arenas {
    by_root: HashMap<Id, ArenaState>,
    swept: u64,
    /// The end sweep is registered; the runtime layer below cannot register it at install.
    sweep_registered: bool,
}

fn with_arena<R>(ctx: &Context, arena: ArenaId, pass_nr: u64, f: impl FnOnce(&mut ArenaState) -> R) -> R {
    PerViewport::<Arenas>::with(ctx, pass::current_viewport(ctx), |arenas| {
        if !arenas.sweep_registered {
            arenas.sweep_registered = true;
            crate::runtime::frame::register_end_sweep(ctx, std::any::TypeId::of::<Arenas>(), sweep_undrawn);
            pointer::register_press_probe(ctx, any_press_open);
        }
        if arenas.swept != pass_nr {
            arenas.swept = pass_nr;
            arenas.by_root.retain(|_, state| state.pass + 2 >= pass_nr);
        }
        f(arenas.by_root.entry(arena.0).or_default())
    })
}

fn rotate(state: &mut ArenaState, pass_nr: u64) {
    if state.pass != pass_nr {
        state.previous = std::mem::take(&mut state.members);
        state.previous_own = std::mem::take(&mut state.own);
        state.previous_union_sense = std::mem::take(&mut state.union_sense);
        state.pass = pass_nr;
    }
}

const OVER: u32 = u32::MAX;

fn own_shapes(own: &[PartDecl], root: Id, target: f32) -> Vec<Shape<Id>> {
    own.iter()
        .map(|part| {
            Shape::new(part.shape.clone(), part.sense, root, Some(part.part_ref()), part.priority, if part.over { OVER } else { 0 })
                .min_across(target)
        })
        .collect()
}

fn member_shapes(member: &MemberDecl, position: u32, target: f32) -> Vec<Shape<Id>> {
    let local = member.local;
    let root =
        (!member.sense.is_empty()).then(|| Shape::new(member.rect.into(), member.sense, member.id, None, HitPriority::BODY, position).local(local));
    let parts = member.parts.iter().map(|part| {
        Shape::new(part.shape.clone(), part.sense, member.id, Some(part.part_ref()), part.priority, position).local(local).min_across(target)
    });
    root.into_iter().chain(parts).collect()
}

/// The shapes of last pass in resolution order: own parts, then members in show order.
fn candidates(state: &ArenaState, target: f32) -> Vec<Shape<Id>> {
    let root = state.root.map_or(Id::NULL, |root| root.id);
    let mut shapes = own_shapes(&state.previous_own, root, target);
    for (at, member) in state.previous.iter().enumerate() {
        shapes.extend(member_shapes(member, 1 + at as u32, target));
    }
    shapes
}

/// Records the transformed body's egui layer.
pub(crate) fn set_layer(ctx: &Context, arena: ArenaId, layer: Layer) {
    let pass_nr = pass::pass_nr(ctx, pass::current_viewport(ctx));
    with_arena(ctx, arena, pass_nr, |state| state.layer = Some(layer));
}

/// What a root registers and reads at step 8.
pub(crate) struct RootSpec<'a> {
    pub id: Id,
    pub rect: Rect,
    pub own_sense: Sense,
    pub own: &'a [PartDecl],
    pub enabled: bool,
}

/// What a root reads back: the response, the lifecycle outcome and the owner's own results.
pub(crate) struct RootResult {
    pub response: Response,
    pub parts: PartResults,
    pub press: PressState,
    /// The root's own shapes won a click.
    pub activation: bool,
    /// The member a click activated.
    pub activated_member: Option<Id>,
    /// The sense the root's egui widget carries.
    pub sense: Sense,
}

pub(crate) type PartResults = HashMap<(PartKey, Option<u64>), PartResult>;

/// Step 8 for a root: its egui widgets, its registration, the lifecycle and what it reads back.
pub(crate) fn sense_root(ui: &Ui, spec: RootSpec<'_>) -> RootResult {
    let ctx = ui.ctx();
    let arena = ArenaId::new(spec.id);
    let pass_nr = pass::pass_nr(ctx, pass::current_viewport(ctx));
    let (clip, layer) = (ui.clip_rect(), ui.layer_id());
    let last_union = with_arena(ctx, arena, pass_nr, |state| {
        rotate(state, pass_nr);
        state.previous_union_sense
    });
    // A disabled root only hovers, whatever its parts and members sense; it keeps the focus stop its own sense or a part's grants.
    let sense = if spec.enabled {
        spec.own.iter().fold(spec.own_sense | last_union, |sense, part| sense | part.sense)
    } else {
        Sense::HOVER | spec.own.iter().fold(spec.own_sense, |sense, part| sense | part.sense).intersection(Sense::FOCUS)
    };
    let response = response::widget(ui, spec.id, spec.rect.intersect(clip), sense);
    // Each edge or handle part that reaches outside the root rect gets its own widget, resolved as the root.
    let mut widgets = vec![spec.id, sub(spec.id, "transform")];
    for part in spec.own.iter().filter(|part| part.priority.level >= HitLevel::Edge) {
        let bounds = part.shape.bounds();
        if !spec.rect.contains_rect(bounds) {
            let id = sub(spec.id, ("edge", part.key, part.index));
            response::widget(ui, id, bounds.intersect(clip), part.sense);
            crate::runtime::egui::access::hide(ctx, id);
            widgets.push(id);
        }
    }
    let mut shapes = own_shapes(spec.own, spec.id, pointer::frame(ctx).target);
    if !spec.own_sense.is_empty() {
        shapes.push(Shape::new(spec.rect.into(), spec.own_sense, spec.id, None, HitPriority::new(HitLevel::Background, 0), 0));
    }
    with_arena(ctx, arena, pass_nr, |state| {
        state.own = spec.own.to_vec();
        state.root = Some(RootInfo { id: spec.id, rect: spec.rect, own_sense: spec.own_sense });
        state.widgets = widgets.clone();
    });
    pointer::record_shapes(ctx, layer, spec.id, shapes);
    pointer::record_widgets(ctx, layer, spec.id, &widgets, sense.contains(Sense::CLICK));
    let mut response = response;
    let outcome = outcome(ctx, arena, layer, spec.enabled, sense);
    response::keep_press_flags(&mut response, outcome.target.is_some_and(|target| target.owner == spec.id));
    let (activation, activated_member) = match outcome.activated {
        Some(hit) if hit.owner == spec.id => (true, None),
        Some(hit) => (false, Some(hit.owner)),
        None => (false, None),
    };
    RootResult {
        response,
        parts: part_results(ctx, arena, spec.id, spec.own),
        press: shape_press(ctx, arena, spec.id),
        activation,
        activated_member,
        sense,
    }
}

/// A member's step 8: it joins the arena and builds its `Response` and results from the arena's outcome.
pub(crate) struct MemberSpec<'a> {
    pub id: Id,
    pub rect: Rect,
    pub sense: Sense,
    pub parts: &'a [PartDecl],
    pub enabled: bool,
}

pub(crate) struct MemberResult {
    pub response: Response,
    pub parts: PartResults,
    pub press: PressState,
    /// The press released as a click on the member's own shape or one of its parts.
    pub activated: bool,
}

pub(crate) fn sense_member(ui: &Ui, arena: ArenaId, spec: MemberSpec<'_>) -> MemberResult {
    let ctx = ui.ctx();
    let layer = ui.layer_id();
    let pass_nr = pass::pass_nr(ctx, pass::current_viewport(ctx));
    // One read of the pointer and one of the arena serve the join, the outcome and the part results.
    let (pointer_pos, target, mods, sample) = pointer::with_frame(ctx, |frame, hover| {
        (hover, frame.map_or(0.0, |frame| frame.target), frame.map_or(Mods::NONE, |frame| frame.mods), frame.and_then(|frame| frame.sample))
    });
    let mut decl = MemberDecl::new(spec.id, spec.rect, spec.sense, spec.parts.to_vec());
    let (local, shapes, outcome, known) = with_arena(ctx, arena, pass_nr, |state| {
        rotate(state, pass_nr);
        decl.local = state.layer == Some(layer);
        let shapes = member_shapes(&decl, 1 + state.members.len() as u32, target);
        state.union_sense |= decl.sense;
        for part in &decl.parts {
            state.union_sense |= part.sense;
        }
        let local = decl.local;
        state.members.push(decl);
        let outcome = state.outcome.filter(|outcome| outcome.pass == pass_nr).unwrap_or_default();
        let known = known_in(state, spec.id);
        (local, shapes, outcome, known)
    });
    pointer::record_shapes(ctx, layer, arena.0, shapes);
    let view = member_view(&outcome, spec.id, local, pointer_pos);
    MemberResult {
        response: response::member(ui, spec.id, spec.rect, spec.enabled, view),
        parts: part_results_in(&outcome, &known, (mods, sample, pointer_pos), spec.id, spec.parts),
        press: shape_press_in(&outcome, mods, sample, spec.id),
        activated: view.clicked,
    }
}

fn member_view(outcome: &Outcome, id: Id, local: bool, pointer_pos: Option<Pos2>) -> MemberView {
    let phase = outcome.target.filter(|target| target.owner == id).and(outcome.phase);
    MemberView {
        hovered: outcome.hover.is_some_and(|hit| hit.owner == id),
        pressed: phase.is_some(),
        drag_started: matches!(phase, Some(PressPhase::DragStart { .. })),
        dragged: phase.is_some_and(|phase| phase.is_drag()),
        drag_stopped: matches!(phase, Some(PressPhase::Release { click: false, .. })),
        clicked: outcome.activated.is_some_and(|hit| hit.owner == id),
        pointer: pointer_pos.map(|at| if local { outcome.xf.map_or(at, |map| map.to_layer(at)) } else { at }),
    }
}

/// The resolve rule over last pass's shapes at `point`. A press inside the root rect that hits no shape goes to
/// the root's own background when the root senses a press.
fn resolve_in(state: &ArenaState, xf: Option<LayerMap>, point: Pos2, kind: ResolveFor, target: f32) -> Option<Hit> {
    let shapes = candidates(state, target);
    let won = resolve_mapped(&shapes, point, kind, |p| xf.map_or(p, |map| map.to_layer(p)));
    won.map(|winner| Hit { owner: winner.owner, part: winner.part, sense: winner.sense, local: winner.local })
}

/// Runs the press lifecycle for this pass, once: later calls in the pass return the same outcome. A repeated
/// pass has no events, so phases hold and a release, click or activation does not repeat. `union_sense` is
/// what the root's egui widgets sense.
fn outcome(ctx: &Context, arena: ArenaId, layer: Layer, enabled: bool, union_sense: Sense) -> Outcome {
    let pass_nr = pass::pass_nr(ctx, pass::current_viewport(ctx));
    let done = with_arena(ctx, arena, pass_nr, |state| state.outcome.filter(|outcome| outcome.pass == pass_nr));
    if let Some(outcome) = done {
        return outcome;
    }
    let pass_pointer = pointer::pass_pointer(ctx);
    let frame = pointer::frame(ctx);
    let cancel = if enabled { pointer::cancel_of_pass(ctx) } else { Some(craft_behavior::PressCancel::Disabled) };
    let body_layer = with_arena(ctx, arena, pass_nr, |state| state.layer);
    let xf = body_layer.and_then(|layer| areas::layer_map(ctx, layer));
    // Hover needs the root's layer on top under the pointer, or egui's layer cover (a popup) would be ignored.
    let top = pass_pointer.hover.and_then(|at| areas::layer_at(ctx, at));
    let hover_layer_ok = top.is_some_and(|top| top == layer || Some(top) == body_layer);
    let mut cancelled = false;
    let outcome = with_arena(ctx, arena, pass_nr, |state| {
        let root = state.root.unwrap_or(RootInfo { id: Id::NULL, rect: Rect::NOTHING, own_sense: Sense::empty() });
        let pos = frame.pos.unwrap_or(Pos2::ZERO);
        let input = PassInput::new(
            &pass_pointer.events,
            frame.slop,
            pos,
            pass_pointer.primary_down || frame.buttons.intersects(craft_behavior::Buttons::MIDDLE | craft_behavior::Buttons::SECONDARY),
        )
        .delta(frame.delta)
        .cancel(cancel);
        let mut machine = std::mem::take(&mut state.machine);
        let step = machine.step(input, |at| {
            // The press-target rule: egui's click target decides, its drag target only for a drag-only root; the
            // root starts a press when that target is its own or there is none.
            let mine = |id: Option<Id>| id.is_none_or(|id| state.widgets.contains(&id));
            let targets = pass_pointer.press_targets;
            let allowed = if union_sense.contains(Sense::CLICK) {
                mine(targets.click)
            } else if union_sense.contains(Sense::DRAG) {
                mine(targets.drag)
            } else {
                true
            };
            if !enabled || !allowed {
                return None;
            }
            let hit = resolve_in(state, xf, at, ResolveFor::Press, frame.target).or_else(|| {
                (root.rect.contains(at) && root.own_sense.takes_press()).then_some(Hit {
                    owner: root.id,
                    part: None,
                    sense: root.own_sense,
                    local: false,
                })
            });
            hit.map(|hit| (hit, hit.sense))
        });
        cancelled = matches!(step.phase, Some(PressPhase::Cancel { .. }));
        let open = machine.target();
        let dragging = step.dragging;
        state.machine = machine;
        let under = pass_pointer.hover.filter(|_| hover_layer_ok).and_then(|at| resolve_in(state, xf, at, ResolveFor::Hover, frame.target));
        // While a press is open only its target hovers, and only while the pointer is over it or dragging.
        let hover = match open {
            Some(target) if dragging => Some(target),
            Some(target) => under.filter(|under| under.same(&target)),
            None => under,
        };
        let outcome = Outcome { pass: pass_nr, hover, target: step.target, phase: step.phase, dragging, activated: step.activated, xf };
        state.outcome = Some(outcome);
        outcome
    });
    if cancelled {
        crate::runtime::frame::note_cancelled_press(ctx);
    }
    outcome
}

/// The end hook's sweep: a press whose root, or whose member owner, was not drawn in the pass ends, so a removed
/// owner never keeps the pointer captured. egui's own drag ends at the next begin hook.
fn sweep_undrawn(ctx: &Context) {
    let viewport = pass::current_viewport(ctx);
    let pass_nr = pass::pass_nr(ctx, viewport);
    let gone = PerViewport::<Arenas>::with(ctx, viewport, |arenas| {
        let mut gone = false;
        for state in arenas.by_root.values_mut() {
            let orphaned = |state: &ArenaState| {
                state.machine.target().is_some_and(|target| {
                    state.pass != pass_nr
                        || (state.root.is_some_and(|root| root.id != target.owner) && !state.members.iter().any(|member| member.id == target.owner))
                })
            };
            if orphaned(state) {
                state.machine = PressMachine::default();
                gone = true;
            }
        }
        gone
    });
    if gone {
        crate::runtime::frame::note_cancelled_press(ctx);
    }
}

fn any_press_open(ctx: &Context) -> bool {
    PerViewport::<Arenas>::with(ctx, pass::current_viewport(ctx), |arenas| arenas.by_root.values().any(|state| state.machine.target().is_some()))
}

/// The arena's outcome of the running pass, idle when the root has not run yet.
pub(crate) fn read_outcome(ctx: &Context, arena: ArenaId) -> Outcome {
    let pass_nr = pass::pass_nr(ctx, pass::current_viewport(ctx));
    with_arena(ctx, arena, pass_nr, |state| state.outcome.filter(|outcome| outcome.pass == pass_nr)).unwrap_or_default()
}

/// A phase in the owner's space: a member of a transformed body reads positions in the body layer.
fn in_owner_space(phase: PressPhase, target: &Hit, xf: Option<LayerMap>) -> PressPhase {
    match xf {
        Some(map) if target.local => phase.mapped(|at| map.to_layer(at), |delta| map.delta_to_layer(delta)),
        _ => phase,
    }
}

fn known_in(state: &ArenaState, owner: Id) -> Vec<(PartKey, Option<u64>)> {
    let parts: &[PartDecl] = if state.root.is_some_and(|root| root.id == owner) {
        &state.previous_own
    } else {
        state.previous.iter().find(|member| member.id == owner).map_or(&[], |member| member.parts.as_slice())
    };
    parts.iter().map(|part| (part.key, part.index)).collect()
}

fn known_parts(ctx: &Context, arena: ArenaId, owner: Id) -> Vec<(PartKey, Option<u64>)> {
    let pass_nr = pass::pass_nr(ctx, pass::current_viewport(ctx));
    with_arena(ctx, arena, pass_nr, |state| known_in(state, owner))
}

/// The part results of one owner: a part reads the press phase when it is the press winner, `Hover` when it is
/// the topmost shape under the pointer, else idle. A part first registered this pass is not in last pass's
/// registrations and reads idle.
pub(crate) fn part_results(ctx: &Context, arena: ArenaId, owner: Id, decls: &[PartDecl]) -> PartResults {
    let outcome = read_outcome(ctx, arena);
    let known = known_parts(ctx, arena, owner);
    let pointer =
        pointer::with_frame(ctx, |frame, hover| (frame.map_or(Mods::NONE, |frame| frame.mods), frame.and_then(|frame| frame.sample), hover));
    part_results_in(&outcome, &known, pointer, owner, decls)
}

fn part_results_in(
    outcome: &Outcome,
    known: &[(PartKey, Option<u64>)],
    (mods, sample, at): (Mods, Option<PointerSample>, Option<Pos2>),
    owner: Id,
    decls: &[PartDecl],
) -> PartResults {
    decls
        .iter()
        .map(|decl| {
            let key = (decl.key, decl.index);
            let is_known = known.contains(&key);
            let mine = |hit: &Hit| hit.owner == owner && hit.part == Some(decl.part_ref());
            let won = outcome.target.filter(|target| is_known && mine(target));
            let hover = outcome.hover.filter(|hit| is_known && mine(hit));
            let hovered = hover.is_some();
            let hover_pos = hover.zip(at).map(|(hit, at)| if hit.local { outcome.xf.map_or(at, |map| map.to_layer(at)) } else { at });
            let phase = match (won, outcome.phase) {
                (Some(target), Some(phase)) => in_owner_space(phase, &target, outcome.xf),
                _ if hovered => PressPhase::Hover,
                _ => PressPhase::Idle,
            };
            let captured = won.is_some() && outcome.dragging && phase.is_drag();
            let press = PressState::new(phase).mods(mods).sample(sample).captured(captured);
            (key, PartResult { rect: decl.shape.bounds(), press, hovered, hover_pos, flags: decl.flags })
        })
        .collect()
}

/// The owner's own shape's press state: a member's root shape or the root background. `Idle` unless it won.
pub(crate) fn shape_press(ctx: &Context, arena: ArenaId, owner: Id) -> PressState {
    let outcome = read_outcome(ctx, arena);
    let (mods, sample) = pointer::with_frame(ctx, |frame, _| (frame.map_or(Mods::NONE, |frame| frame.mods), frame.and_then(|frame| frame.sample)));
    shape_press_in(&outcome, mods, sample, owner)
}

fn shape_press_in(outcome: &Outcome, mods: Mods, sample: Option<PointerSample>, owner: Id) -> PressState {
    let won = outcome.target.filter(|target| target.owner == owner && target.part.is_none());
    let phase = match (won, outcome.phase) {
        (Some(target), Some(phase)) => in_owner_space(phase, &target, outcome.xf),
        _ if outcome.hover.is_some_and(|hit| hit.owner == owner && hit.part.is_none()) => PressPhase::Hover,
        _ => PressPhase::Idle,
    };
    let captured = won.is_some() && phase.is_drag();
    PressState::new(phase).mods(mods).sample(sample).captured(captured)
}

/// The cursor a hovered part of `decls` declares, if any: the sensing core's request for the cursor service.
pub(crate) fn hovered_cursor(decls: &[PartDecl], results: &PartResults) -> Option<CursorIcon> {
    decls.iter().filter(|decl| results.get(&(decl.key, decl.index)).is_some_and(|result| result.hovered)).find_map(|decl| decl.cursor)
}
