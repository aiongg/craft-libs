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
    CursorIcon, HitLevel, HitPriority, HitShape, PartKey, PartRef, PassInput, PressMachine, PressPhase, PressState, ResolveFor, Sense, Shape,
    resolve_mapped,
};
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
    /// The value flags the component set on the part.
    pub flags: Flags,
}

impl PartResult {
    /// What a read returns for a part with no press history: face mode, or a part first registered this pass.
    pub const IDLE: PartResult = PartResult { rect: Rect::NOTHING, press: PressState::IDLE, hovered: false, flags: Flags::empty() };
}

/// A text part's result this pass.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct TextOutcome<V> {
    /// The part's rect.
    pub rect: Rect,
    /// The draft being typed, if any.
    pub draft: Option<String>,
    /// The value the draft parsed to.
    pub value: Option<V>,
}

impl<V> TextOutcome<V> {
    /// An outcome over `rect` with no draft or value.
    pub const fn new(rect: Rect) -> Self {
        TextOutcome { rect, draft: None, value: None }
    }
}

/// A part as the sensing core registers it.
// F2-MERGE: F3 (`PartBuilder` adds tooltip and `pointer_only`)
#[derive(Clone, Debug)]
#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the pipeline of F3 is the caller"))]
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
    #[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the pipeline of F3 is the caller"))]
    pub(crate) fn new(key: PartKey, shape: impl Into<HitShape>, sense: Sense) -> Self {
        PartDecl { key, index: None, shape: shape.into(), sense, priority: HitPriority::BODY, cursor: None, over: false, flags: Flags::default() }
    }

    #[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the pipeline of F3 is the caller"))]
    fn part_ref(&self) -> PartRef {
        PartRef { key: self.key, index: self.index }
    }
}

/// One member's registration for this pass.
#[derive(Clone, Debug)]
#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the pipeline of F3 is the caller"))]
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
    #[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the pipeline of F3 is the caller"))]
    pub(crate) fn new(id: Id, rect: Rect, sense: Sense, parts: Vec<PartDecl>) -> Self {
        MemberDecl { id, rect, sense, parts, local: false }
    }
}

/// What a root registers at step 8: its rect and its own sense.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the pipeline of F3 is the caller"))]
struct RootInfo {
    id: Id,
    rect: Rect,
    own_sense: Sense,
}

/// A shape's owner and, for a part, its key.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the pipeline of F3 is the caller"))]
pub(crate) struct Hit {
    pub owner: Id,
    pub part: Option<PartRef>,
    pub sense: Sense,
    pub local: bool,
}

impl Hit {
    #[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the pipeline of F3 is the caller"))]
    fn same(&self, other: &Hit) -> bool {
        self.owner == other.owner && self.part == other.part
    }
}

/// What the arena resolved this pass; the root computes it once, members and the root read it.
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the pipeline of F3 is the caller"))]
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
#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the pipeline of F3 is the caller"))]
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
#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the pipeline of F3 is the caller"))]
struct Arenas {
    by_root: HashMap<Id, ArenaState>,
    swept: u64,
}

#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the pipeline of F3 is the caller"))]
fn with_arena<R>(ctx: &Context, arena: ArenaId, pass_nr: u64, f: impl FnOnce(&mut ArenaState) -> R) -> R {
    PerViewport::<Arenas>::with(ctx, pass::current_viewport(ctx), |arenas| {
        if arenas.swept != pass_nr {
            arenas.swept = pass_nr;
            arenas.by_root.retain(|_, state| state.pass + 2 >= pass_nr);
        }
        f(arenas.by_root.entry(arena.0).or_default())
    })
}

#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the pipeline of F3 is the caller"))]
fn rotate(state: &mut ArenaState, pass_nr: u64) {
    if state.pass != pass_nr {
        state.previous = std::mem::take(&mut state.members);
        state.previous_own = std::mem::take(&mut state.own);
        state.previous_union_sense = std::mem::take(&mut state.union_sense);
        state.pass = pass_nr;
    }
}

#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the pipeline of F3 is the caller"))]
const OVER: u32 = u32::MAX;

#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the pipeline of F3 is the caller"))]
fn own_shapes(own: &[PartDecl], root: Id) -> Vec<Shape<Id>> {
    own.iter()
        .map(|part| Shape::new(part.shape.clone(), part.sense, root, Some(part.part_ref()), part.priority, if part.over { OVER } else { 0 }))
        .collect()
}

#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the pipeline of F3 is the caller"))]
fn member_shapes(member: &MemberDecl, position: u32) -> Vec<Shape<Id>> {
    let local = member.local;
    let root =
        (!member.sense.is_empty()).then(|| Shape::new(member.rect.into(), member.sense, member.id, None, HitPriority::BODY, position).local(local));
    let parts = member
        .parts
        .iter()
        .map(|part| Shape::new(part.shape.clone(), part.sense, member.id, Some(part.part_ref()), part.priority, position).local(local));
    root.into_iter().chain(parts).collect()
}

/// The shapes of last pass in resolution order: own parts, then members in show order.
#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the pipeline of F3 is the caller"))]
fn candidates(state: &ArenaState) -> Vec<Shape<Id>> {
    let root = state.root.map_or(Id::NULL, |root| root.id);
    let mut shapes = own_shapes(&state.previous_own, root);
    for (at, member) in state.previous.iter().enumerate() {
        shapes.extend(member_shapes(member, 1 + at as u32));
    }
    shapes
}

/// Joins `decl` to `arena`: a member registers its root shape and parts at its show-order position.
/// Returns whether its shapes lie in the arena's body layer.
#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the pipeline of F3 is the caller"))]
pub(crate) fn join(ctx: &Context, arena: ArenaId, layer: Layer, mut decl: MemberDecl) -> bool {
    let pass_nr = pass::pass_nr(ctx, pass::current_viewport(ctx));
    let (local, shapes) = with_arena(ctx, arena, pass_nr, |state| {
        rotate(state, pass_nr);
        decl.local = state.layer == Some(layer);
        let shapes = member_shapes(&decl, 1 + state.members.len() as u32);
        state.union_sense |= decl.sense;
        for part in &decl.parts {
            state.union_sense |= part.sense;
        }
        let local = decl.local;
        state.members.push(decl);
        (local, shapes)
    });
    pointer::record_shapes(ctx, layer, shapes);
    local
}

/// Records the transformed body's egui layer.
#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the pipeline of F3 is the caller"))]
pub(crate) fn set_layer(ctx: &Context, arena: ArenaId, layer: Layer) {
    let pass_nr = pass::pass_nr(ctx, pass::current_viewport(ctx));
    with_arena(ctx, arena, pass_nr, |state| state.layer = Some(layer));
}

/// What a root registers and reads at step 8.
#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the pipeline of F3 is the caller"))]
pub(crate) struct RootSpec<'a> {
    pub id: Id,
    pub rect: Rect,
    pub own_sense: Sense,
    pub own: &'a [PartDecl],
    pub enabled: bool,
}

/// What a root reads back: the response, the lifecycle outcome and the owner's own results.
#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the pipeline of F3 is the caller"))]
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

#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the pipeline of F3 is the caller"))]
pub(crate) type PartResults = HashMap<(PartKey, Option<u64>), PartResult>;

/// Step 8 for a root: its egui widgets, its registration, the lifecycle and what it reads back.
#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the pipeline of F3 is the caller"))]
pub(crate) fn sense_root(ui: &Ui, spec: RootSpec<'_>) -> RootResult {
    let ctx = ui.ctx();
    let arena = ArenaId::new(spec.id);
    let pass_nr = pass::pass_nr(ctx, pass::current_viewport(ctx));
    let (clip, layer) = (ui.clip_rect(), ui.layer_id());
    let last_union = with_arena(ctx, arena, pass_nr, |state| {
        rotate(state, pass_nr);
        state.previous_union_sense
    });
    let sense = spec.own.iter().fold(spec.own_sense | last_union, |sense, part| sense | part.sense);
    let response = response::widget(ui, spec.id, spec.rect.intersect(clip), sense);
    // Each edge or handle part that reaches outside the root rect gets its own widget, resolved as the root.
    let mut widgets = vec![spec.id, sub(spec.id, "transform")];
    for part in spec.own.iter().filter(|part| part.priority.level >= HitLevel::Edge) {
        let bounds = part.shape.bounds();
        if !spec.rect.contains_rect(bounds) {
            let id = sub(spec.id, ("edge", part.key, part.index));
            response::widget(ui, id, bounds.intersect(clip), part.sense);
            widgets.push(id);
        }
    }
    let mut shapes = own_shapes(spec.own, spec.id);
    if !spec.own_sense.is_empty() {
        shapes.push(Shape::new(spec.rect.into(), spec.own_sense, spec.id, None, HitPriority::new(HitLevel::Background, 0), 0));
    }
    with_arena(ctx, arena, pass_nr, |state| {
        state.own = spec.own.to_vec();
        state.root = Some(RootInfo { id: spec.id, rect: spec.rect, own_sense: spec.own_sense });
        state.widgets = widgets;
    });
    pointer::record_shapes(ctx, layer, shapes);
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
#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the pipeline of F3 is the caller"))]
pub(crate) struct MemberSpec<'a> {
    pub id: Id,
    pub rect: Rect,
    pub sense: Sense,
    pub parts: &'a [PartDecl],
    pub enabled: bool,
}

#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the pipeline of F3 is the caller"))]
pub(crate) struct MemberResult {
    pub response: Response,
    pub parts: PartResults,
    pub press: PressState,
    /// The press released as a click on the member's own shape or one of its parts.
    pub activated: bool,
}

#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the pipeline of F3 is the caller"))]
pub(crate) fn sense_member(ui: &Ui, arena: ArenaId, spec: MemberSpec<'_>) -> MemberResult {
    let ctx = ui.ctx();
    let local = join(ctx, arena, ui.layer_id(), MemberDecl::new(spec.id, spec.rect, spec.sense, spec.parts.to_vec()));
    let outcome = read_outcome(ctx, arena);
    let pointer_pos = pointer::pass_pointer(ctx).hover;
    let view = member_view(&outcome, spec.id, local, pointer_pos);
    MemberResult {
        response: response::member(ui, spec.id, spec.rect, spec.enabled, view),
        parts: part_results(ctx, arena, spec.id, spec.parts),
        press: shape_press(ctx, arena, spec.id),
        activated: outcome.activated.is_some_and(|hit| hit.owner == spec.id),
    }
}

#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the pipeline of F3 is the caller"))]
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
#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the pipeline of F3 is the caller"))]
fn resolve_in(state: &ArenaState, xf: Option<LayerMap>, point: Pos2, kind: ResolveFor) -> Option<Hit> {
    let shapes = candidates(state);
    let won = resolve_mapped(&shapes, point, kind, |p| xf.map_or(p, |map| map.to_layer(p)));
    won.map(|winner| Hit { owner: winner.owner, part: winner.part, sense: winner.sense, local: winner.local })
}

/// Runs the press lifecycle for this pass, once: later calls in the pass return the same outcome. A repeated
/// pass has no events, so phases hold and a release, click or activation does not repeat. `union_sense` is
/// what the root's egui widgets sense.
#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the pipeline of F3 is the caller"))]
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
        let input = PassInput::new(&pass_pointer.events, frame.slop, pos, pass_pointer.primary_down).delta(frame.delta).cancel(cancel);
        let mut machine = std::mem::take(&mut state.machine);
        let step = machine.step(input, |at| {
            // The press-target rule: a root starts a press only when egui's targets for it are this root's or none.
            let mine = |id: Option<Id>| id.is_none_or(|id| state.widgets.contains(&id));
            let targets = pass_pointer.press_targets;
            let allowed = (!union_sense.contains(Sense::CLICK) || mine(targets.click)) && (!union_sense.contains(Sense::DRAG) || mine(targets.drag));
            if !enabled || !allowed {
                return None;
            }
            let hit = resolve_in(state, xf, at, ResolveFor::Press).or_else(|| {
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
        let under = pass_pointer.hover.filter(|_| hover_layer_ok).and_then(|at| resolve_in(state, xf, at, ResolveFor::Hover));
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

/// The arena's outcome of the running pass, idle when the root has not run yet.
#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the pipeline of F3 is the caller"))]
pub(crate) fn read_outcome(ctx: &Context, arena: ArenaId) -> Outcome {
    let pass_nr = pass::pass_nr(ctx, pass::current_viewport(ctx));
    with_arena(ctx, arena, pass_nr, |state| state.outcome.filter(|outcome| outcome.pass == pass_nr)).unwrap_or_default()
}

/// A phase in the owner's space: a member of a transformed body reads positions in the body layer.
#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the pipeline of F3 is the caller"))]
fn in_owner_space(phase: PressPhase, target: &Hit, xf: Option<LayerMap>) -> PressPhase {
    match xf {
        Some(map) if target.local => phase.mapped(|at| map.to_layer(at), |delta| map.delta_to_layer(delta)),
        _ => phase,
    }
}

#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the pipeline of F3 is the caller"))]
fn known_parts(ctx: &Context, arena: ArenaId, owner: Id) -> Vec<(PartKey, Option<u64>)> {
    let pass_nr = pass::pass_nr(ctx, pass::current_viewport(ctx));
    with_arena(ctx, arena, pass_nr, |state| {
        let parts: Vec<&PartDecl> = if state.root.is_some_and(|root| root.id == owner) {
            state.previous_own.iter().collect()
        } else {
            state.previous.iter().find(|member| member.id == owner).map(|member| member.parts.iter().collect()).unwrap_or_default()
        };
        parts.into_iter().map(|part| (part.key, part.index)).collect()
    })
}

/// The part results of one owner: a part reads the press phase when it is the press winner, `Hover` when it is
/// the topmost shape under the pointer, else idle. A part first registered this pass is not in last pass's
/// registrations and reads idle.
#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the pipeline of F3 is the caller"))]
pub(crate) fn part_results(ctx: &Context, arena: ArenaId, owner: Id, decls: &[PartDecl]) -> PartResults {
    let outcome = read_outcome(ctx, arena);
    let known = known_parts(ctx, arena, owner);
    let frame = pointer::frame(ctx);
    decls
        .iter()
        .map(|decl| {
            let key = (decl.key, decl.index);
            let is_known = known.contains(&key);
            let mine = |hit: &Hit| hit.owner == owner && hit.part == Some(decl.part_ref());
            let won = outcome.target.filter(|target| is_known && mine(target));
            let hovered = is_known && outcome.hover.is_some_and(|hit| mine(&hit));
            let phase = match (won, outcome.phase) {
                (Some(target), Some(phase)) => in_owner_space(phase, &target, outcome.xf),
                _ if hovered => PressPhase::Hover,
                _ => PressPhase::Idle,
            };
            let captured = won.is_some() && outcome.dragging && phase.is_drag();
            let press = PressState::new(phase).mods(frame.mods).sample(frame.sample).captured(captured);
            (key, PartResult { rect: decl.shape.bounds(), press, hovered, flags: decl.flags })
        })
        .collect()
}

/// The owner's own shape's press state: a member's root shape or the root background. `Idle` unless it won.
#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the pipeline of F3 is the caller"))]
pub(crate) fn shape_press(ctx: &Context, arena: ArenaId, owner: Id) -> PressState {
    let outcome = read_outcome(ctx, arena);
    let frame = pointer::frame(ctx);
    let won = outcome.target.filter(|target| target.owner == owner && target.part.is_none());
    let phase = match (won, outcome.phase) {
        (Some(target), Some(phase)) => in_owner_space(phase, &target, outcome.xf),
        _ if outcome.hover.is_some_and(|hit| hit.owner == owner && hit.part.is_none()) => PressPhase::Hover,
        _ => PressPhase::Idle,
    };
    let captured = won.is_some() && phase.is_drag();
    PressState::new(phase).mods(frame.mods).sample(frame.sample).captured(captured)
}

/// The cursor a hovered part of `decls` declares, if any: the sensing core's request for the cursor service.
#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "the pipeline of F3 is the caller"))]
pub(crate) fn hovered_cursor(decls: &[PartDecl], results: &PartResults) -> Option<CursorIcon> {
    decls.iter().filter(|decl| results.get(&(decl.key, decl.index)).is_some_and(|result| result.hovered)).find_map(|decl| decl.cursor)
}
