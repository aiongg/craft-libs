//! The step functions of the pipeline (1.6): named functions over plain data, in pipeline order. `run` calls
//! them for every shape.

use super::cx::{CxCore, Decls, MeasureCx};
use super::{Common, Component, Cx, IdSource, Pointer as _, Resolved, Styled as _, ValueOf};
use crate::primitives::{FocusKind, FocusRing, PaintShape, StateLayer, StateLayerKind, Surface};
use crate::runtime::commands::{self, ResolvedCommand};
use crate::runtime::egui::{memory as store, pass, repaint, response};
use crate::runtime::env::{Env, Label};
use crate::runtime::focus::{self, Group, Stop};
use crate::runtime::frame::{PerViewport, Stamped};
use crate::runtime::memory::{MemoryStage, sub};
use crate::runtime::{inspect, keys, layout_memory, wheel};
use crate::style::{ComponentStyle, Flags, Ink, StateSet, StyleCx, accepted_variant, resolve_style, surface_style};
use craft_behavior::{KeyRole, MotionMode, PartRef, Proposal, Roving, Sense};
use craft_theme::{ControlSize, State, StateMix, Variant};
use egui::{Context, Id, Rect, Response, Ui, Vec2};
use std::any::TypeId;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// The 17 steps of the pipeline, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Step {
    Env,
    Style,
    Id,
    Value,
    Measure,
    Place,
    Parts,
    Sense,
    Update,
    Overlay,
    Apply,
    State,
    Paint,
    Content,
    Settle,
    Report,
    Finish,
}

impl Step {
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Step::Env => "env",
            Step::Style => "style",
            Step::Id => "id",
            Step::Value => "value",
            Step::Measure => "measure",
            Step::Place => "place",
            Step::Parts => "parts",
            Step::Sense => "sense",
            Step::Update => "update",
            Step::Overlay => "overlay",
            Step::Apply => "apply",
            Step::State => "state",
            Step::Paint => "paint",
            Step::Content => "content",
            Step::Settle => "settle",
            Step::Report => "report",
            Step::Finish => "finish",
        }
    }
}

/// The shape a pipeline run is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ShapeKind {
    Leaf,
    Body,
    Item,
}

/// Which shapes run a step.
#[derive(Clone, Copy)]
struct Runs {
    leaf: bool,
    body: bool,
    item: bool,
}

const ALL: Runs = Runs { leaf: true, body: true, item: true };

/// The one step table: which shapes run each step (1.6). It gates only a step a shape omits, which today is the
/// content step of a leaf. The order the steps run in is the order of the `Trace::mark` calls in `run`, not the order
/// of the rows.
const TABLE: [(Step, Runs); 17] = [
    (Step::Env, ALL),
    (Step::Style, ALL),
    (Step::Id, ALL),
    (Step::Value, ALL),
    (Step::Measure, ALL),
    (Step::Place, ALL),
    (Step::Parts, ALL),
    (Step::Sense, ALL),
    (Step::Update, ALL),
    (Step::Overlay, ALL),
    (Step::Apply, ALL),
    (Step::State, ALL),
    (Step::Paint, ALL),
    (Step::Content, Runs { leaf: false, body: true, item: true }),
    (Step::Settle, ALL),
    (Step::Report, ALL),
    (Step::Finish, ALL),
];

/// Whether `shape` runs `step`.
pub(crate) fn runs(step: Step, shape: ShapeKind) -> bool {
    TABLE.iter().find(|(s, _)| *s == step).is_some_and(|(_, runs)| match shape {
        ShapeKind::Leaf => runs.leaf,
        ShapeKind::Body => runs.body,
        ShapeKind::Item => runs.item,
    })
}

/// The step record of one show. `recording` is read once per run, so a release frame with the inspector off
/// takes no lock per step; a sizing pass records nothing.
#[derive(Clone, Copy)]
pub(crate) struct Trace<'a> {
    ctx: &'a Context,
    id: Id,
    shape: ShapeKind,
    recording: bool,
}

impl<'a> Trace<'a> {
    pub(crate) fn new(ctx: &'a Context, id: Id, shape: ShapeKind, recording: bool) -> Self {
        Trace { ctx, id, shape, recording }
    }

    pub(crate) fn recording(&self) -> bool {
        self.recording
    }

    /// Records that `step` runs, and returns whether the shape runs it.
    pub(crate) fn mark(&self, step: Step) -> bool {
        let run = runs(step, self.shape);
        if run && self.recording {
            inspect::step(self.ctx, self.id, step.name());
        }
        run
    }
}

/// What step 1 resolves.
pub(crate) struct EnvStep {
    pub enabled: bool,
    pub read_only: bool,
    pub size: ControlSize,
    /// The variant the component accepts for the one requested.
    pub variant: Variant,
    /// The variant as requested, which the style cache logs a fallback from.
    pub requested: Variant,
    pub label: Label,
    pub command: Option<Arc<ResolvedCommand>>,
}

/// Step 1, env: enabled = instance AND scopes AND the bound command's state; read-only = instance OR scopes;
/// size; variant; label.
pub(crate) fn env<C: Component>(ctx: &Context, env: &Env, common: &Common) -> EnvStep {
    let default = <C::Style as ComponentStyle>::VARIANTS.first().copied().unwrap_or(Variant::SECONDARY);
    let requested = common.variant.unwrap_or(default);
    let variant = accepted_variant::<C::StyleKey, C::Style>(requested).variant;
    let command = common.command.as_ref().map(|command| Arc::new(commands::resolve_bound(ctx, &env.commands, command)));
    let label = match (&common.label, &command) {
        (Label::Empty, Some(command)) => Label::Shared(command.label.clone()),
        (label, _) => label.clone(),
    };
    EnvStep {
        enabled: common.enabled.unwrap_or(true) && env.enabled && command.as_ref().is_none_or(|command| command.enabled),
        read_only: common.read_only == Some(true) || env.read_only,
        size: common.size.unwrap_or(env.size),
        variant,
        requested,
        label,
        command,
    }
}

/// `Resolved` over the frame's locals.
pub(crate) fn resolved<'p>(id: Id, step: &'p EnvStep, common: &Common) -> Resolved<'p> {
    Resolved {
        id,
        label: step.label.as_str(),
        label_value: &step.label,
        enabled: step.enabled,
        read_only: step.read_only,
        size: step.size,
        variant: step.variant,
        selected: common.selected,
        keeps_name: common.keeps_name,
        doc_key: common.doc_key,
    }
}

/// The `StyleCx` of the env the component shows in.
pub(crate) fn style_cx<'a>(ctx: &'a Context, env: &'a Env, step: &EnvStep) -> StyleCx<'a> {
    StyleCx::new(ctx, &env.tokens, &env.values, step.size, step.requested, env.direction, env.scale, env.os, env.active, env.key)
}

/// Step 2, style: `&C::Style` from the cache, keyed `(TypeId of StyleKey, key, EnvKey, variant, size)`.
pub(crate) fn style<C: Component>(ctx: &Context, scx: &StyleCx<'_>, key: C::StyleKey) -> Arc<C::Style> {
    resolve_style(scx, key, ctx, C::style)
}

/// The `Auto` ordinals and the ids registered in one pass of one viewport.
#[derive(Default)]
struct IdPass {
    next: HashMap<(Id, TypeId), u32>,
    seen: HashSet<Id>,
}

/// Step 3, id.
///
/// `register` is false for `measure_in`, which peeks, and in a sizing pass. `advance` is false only for
/// `measure_in`: a sizing pass advances the ordinals, so its ids equal those of the pass that follows it.
pub(crate) fn id<C: Component>(ui: &Ui, source: &IdSource, register: bool, advance: bool) -> Id {
    let ctx = ui.ctx();
    let viewport = pass::current_viewport(ctx);
    let pass_nr = pass::pass_nr(ctx, viewport);
    let scope = ui.scope_id();
    let ty = TypeId::of::<C::StyleKey>();
    let id = match source {
        IdSource::Id(id) => *id,
        IdSource::Salt(salt) => scope.with(*salt),
        _ => {
            let n = PerViewport::<Stamped<IdPass>>::with_pass(
                ctx,
                viewport,
                pass_nr,
                |pass, _| *pass = IdPass::default(),
                |pass| {
                    let slot = pass.next.entry((scope, ty)).or_insert(0);
                    let n = *slot;
                    if advance {
                        *slot += 1;
                    }
                    n
                },
            );
            scope.with((ty, n))
        }
    };
    if register {
        crate::runtime::memory::register_owner(ctx, id);
        let repeated =
            PerViewport::<Stamped<IdPass>>::with_pass(ctx, viewport, pass_nr, |pass, _| *pass = IdPass::default(), |pass| !pass.seen.insert(id));
        if repeated {
            log::warn!("{} was shown twice with the id {id:?} in one pass", std::any::type_name::<C>());
            inspect::finding(ctx, inspect::Finding { rule: "duplicate-id", component: std::any::type_name::<C>(), subject: format!("{id:?}") });
        }
    }
    id
}

/// The shown copy of a component's value, in shown memory.
pub(crate) struct ShownCopy<V>(pub Option<Arc<V>>);

impl<V> Clone for ShownCopy<V> {
    fn clone(&self) -> Self {
        ShownCopy(self.0.clone())
    }
}

impl<V> Default for ShownCopy<V> {
    fn default() -> Self {
        ShownCopy(None)
    }
}

/// Step 4, value: the shown copy of the binding's value. A binding equal to the held copy is not cloned.
// F3-LATER: values (F4: the live gesture, `Kept` and saved bindings, `Mixed`, the `HostChanged` cancel)
pub(crate) fn value<C: Component>(comp: &mut C, stage: &mut MemoryStage, id: Id) -> Option<Arc<ValueOf<C>>> {
    let held = stage.shown::<ShownCopy<ValueOf<C>>>(id).0.clone();
    let shown = match (comp.bind(), held) {
        (Some(bind), Some(held)) if *held == *bind.get() => Some(held),
        (Some(bind), _) => Some(Arc::new(bind.get().clone())),
        (None, _) => None,
    };
    stage.shown::<ShownCopy<ValueOf<C>>>(id).0 = shown.clone();
    shown
}

/// The shown value for measuring and faces: reads the binding, applies nothing, opens no stage.
pub(crate) fn peek_value<C: Component>(comp: &mut C, ctx: &Context, id: Id) -> Option<Arc<ValueOf<C>>> {
    let held = MemoryStage::peek::<ShownCopy<ValueOf<C>>>(ctx, id).0;
    match (comp.bind(), held) {
        (Some(bind), Some(held)) if *held == *bind.get() => Some(held),
        (Some(bind), _) => Some(Arc::new(bind.get().clone())),
        (None, held) => held,
    }
}

/// What step 5 reads besides the component.
pub(crate) struct MeasureIn<'a> {
    pub ui: &'a Ui,
    pub env: &'a Env,
    pub proposal: Proposal,
    pub room: Option<f32>,
}

/// Step 5, measure: content size plus padding, then `max(min_size)`. `extent` is what the shape adds to the
/// content: a body's remembered size or a collection's content size.
pub(crate) fn measure<C: Component>(
    comp: &mut C,
    input: &MeasureIn<'_>,
    style: &C::Style,
    resolved: &Resolved<'_>,
    value: Option<&ValueOf<C>>,
    extent: Vec2,
) -> Vec2 {
    let metrics = style.metrics();
    let cx = measure_cx::<C>(input, style, resolved, value);
    let content = comp.measure(&cx) + extent;
    let padding = Vec2::new(metrics.padding.start + metrics.padding.end, metrics.padding.top + metrics.padding.bottom);
    (content + padding).max(metrics.min_size)
}

/// The read view `measure` and `parts` get.
pub(crate) fn measure_cx<'a, C: Component>(
    input: &MeasureIn<'a>,
    style: &'a C::Style,
    resolved: &'a Resolved<'a>,
    value: Option<&'a ValueOf<C>>,
) -> MeasureCx<'a, C> {
    MeasureCx {
        env: input.env.view(),
        style,
        resolved,
        proposal: input.proposal,
        room: input.room,
        value,
        ui: input.ui,
        tokens: &input.env.tokens,
        ink: input.env.ink,
        id: resolved.id,
        enabled_size: (resolved.enabled, resolved.size),
    }
}

/// The size guard: finite and at most `MAX_EXTENT` on both sides. A leaf or collection also needs both sides
/// above zero; a zero size from a leaf means nothing to show.
pub(crate) fn guard(size: Vec2, shape: ShapeKind) -> bool {
    let sane =
        size.x.is_finite() && size.y.is_finite() && size.x >= 0.0 && size.y >= 0.0 && size.x <= super::MAX_EXTENT && size.y <= super::MAX_EXTENT;
    sane && (shape == ShapeKind::Body || size.x > 0.0 && size.y > 0.0)
}

/// Step 6, place: in flow one allocation with no widget, at a rect none.
pub(crate) fn place(ui: &mut Ui, at: super::run::ShowAt, size: Vec2) -> Rect {
    match at {
        // F5-LATER: the `Grid` cursor places a child in flow under a grid body (16.3).
        super::run::ShowAt::Flow => crate::runtime::egui::alloc::space(ui, size),
        super::run::ShowAt::Rect(rect) => rect,
    }
}

/// What step 7 gathers.
pub(crate) struct PartsStep {
    pub decls: Decls,
    pub rovings: Vec<Roving>,
}

/// The parts of last pass's complete set in which this pass's results are read; first-frame parts read idle.
/// Step 7's tail: the focus stops. The root's interest when the component or any part is focusable, its
/// `(id, part)` stops, and its roving groups.
pub(crate) struct StopsIn<'a> {
    pub ctx: &'a Context,
    pub id: Id,
    /// The egui widget that holds key focus: the component, or an arena member's root.
    pub holder: Id,
    pub focusable: bool,
    pub keys: focus::KeyUse,
    pub role: KeyRole,
    pub rtl: bool,
    pub parts: &'a PartsStep,
}

pub(crate) fn register_stops(input: &StopsIn<'_>) {
    let ctx = input.ctx;
    debug_assert!(input.parts.rovings.len() <= 1, "a component declares one roving group; a second would be dropped");
    let group = (!input.parts.rovings.is_empty()).then(|| sub(input.id, "roving"));
    if let (Some(group), Some(roving)) = (group, input.parts.rovings.first()) {
        focus::register_group(ctx, Group::new(group, *roving).rtl(input.rtl));
    }
    let own = || Stop::new(input.id, input.keys).root(input.holder).role(input.role);
    if input.focusable {
        focus::register_stop(ctx, own());
    }
    for (decl, extras) in &input.parts.decls {
        if extras.focusable {
            let stop = own().part(PartRef { key: decl.key, index: decl.index });
            focus::register_stop(ctx, if let Some(group) = group { stop.in_group(group) } else { stop });
        }
    }
}

/// Step 7's finding (22.4): a part that senses `CLICK` or `DRAG` is focusable or declares its keyboard path.
pub(crate) fn check_parts(ctx: &Context, component: &'static str, parts: &PartsStep) {
    for (decl, extras) in &parts.decls {
        if decl.sense.intersects(Sense::CLICK | Sense::DRAG) && !extras.focusable && extras.pointer_only.is_none() {
            inspect::finding(
                ctx,
                inspect::Finding {
                    rule: "pointer-part-without-keyboard-path",
                    component,
                    subject: format!("{:?}", PartRef { key: decl.key, index: decl.index }),
                },
            );
        }
    }
}

/// Registers a wheel taker for next pass's split (8.6). A component with `WHEEL` takes vertical scroll and zoom, and
/// one with `WHEEL_X` the horizontal share, with no focus needed; a value control takes the vertical wheel only
/// while focused or armed.
pub(crate) fn register_wheel(ctx: &Context, id: Id, rect: Rect, sense: Sense, value_control: bool) {
    let taker = craft_behavior::WheelTaker::new(id, rect);
    if sense.intersects(Sense::WHEEL | Sense::WHEEL_X) {
        let wheel_y = sense.contains(Sense::WHEEL);
        wheel::register(ctx, taker.horizontal(true).vertical(wheel_y).zoom(wheel_y));
    } else if value_control {
        wheel::register(ctx, taker.vertical(true).gated(true));
    }
}

/// The part states: each part's press and hover from the sensing core, with its own flags.
pub(crate) fn part_state(result: &crate::primitives::PartResult, enabled: bool, focus_visible: Option<bool>) -> StateSet {
    let mut flags = result.flags;
    if let Some(visible) = focus_visible {
        flags |= Flags::FOCUSED;
        if visible {
            flags |= Flags::FOCUS_VISIBLE;
        }
    }
    let pressed = result.press.phase.is_press();
    let state = if !enabled {
        State::Disabled
    } else if pressed {
        State::Pressed
    } else if result.hovered {
        State::Hover
    } else {
        State::Rest
    };
    let on = flags.intersects(Flags::ON);
    StateSet::new(state, flags, StateMix::new(f32::from(u8::from(result.hovered)), f32::from(u8::from(pressed)), f32::from(u8::from(on))))
}

/// Step 9, update, the part the library writes: the keyboard activation.
pub(crate) fn update_inputs(core: &mut CxCore<'_>) {
    if core.intents.take(craft_behavior::Intent::Activate) {
        core.activation = Some(super::Activation { by_key: true });
    }
}

/// Writes back what the component left untaken, and its state-dependent claims for the next frames.
pub(crate) fn after_update(core: &mut CxCore<'_>) {
    keys::settle(&core.ctx, Some(core.id), &core.intents);
    for chord in std::mem::take(&mut core.claims) {
        keys::claim(&core.ctx, core.id, chord);
    }
}

/// The `(hovered, focus_visible)` a component showed last pass, which `PartsCx::hovered` and `focus_visible` read.
#[derive(Clone, Copy, Default)]
pub(crate) struct LastState(pub bool, pub bool);

/// The eased hover, press and on values of a component.
#[derive(Clone, Copy, Default)]
struct Eased {
    mix: [f32; 3],
}

// F3-LATER: motion service (F4: `Animate`, `MotionSpec`, one clock and one repaint scheduler). Frame-based easing.
fn ease(core: &mut CxCore<'_>, targets: [bool; 3]) -> StateMix {
    let mode = core.env.motion().mode;
    let id = core.id;
    let face = core.face_mode;
    let mut moving = false;
    let mix = {
        let eased = core.memory.shown::<Eased>(sub(id, "eased"));
        for (value, target) in eased.mix.iter_mut().zip(targets) {
            let goal = f32::from(u8::from(target));
            *value = if face || mode == MotionMode::Off { goal } else { *value + (goal - *value) * 0.25 };
            if (*value - goal).abs() < 0.01 {
                *value = goal;
            } else {
                moving = true;
            }
        }
        eased.mix
    };
    if moving {
        repaint::request(&core.ctx);
    }
    StateMix::new(mix[0], mix[1], mix[2])
}

/// Step 12, state: flags first, then `state`; animates hover, press and on; the ink of the surface, else `inherited`.
pub(crate) fn state<C: Component>(comp: &C, cx: &mut Cx<'_, C>, open: bool, inherited: Ink) {
    let core = &mut cx.core;
    let mut flags = comp.flags();
    if open {
        flags |= Flags::OPEN;
    }
    if core.resolved.read_only {
        flags |= Flags::READ_ONLY;
    }
    if core.resolved.selected == Some(true) {
        flags |= Flags::SELECTED;
    }
    let (hovered, pressed) = (core.response.hovered(), core.press.phase.is_press());
    let focused = focus::is_focused(&core.ctx, core.id);
    let visible = focus::focus_visible(&core.ctx, core.id);
    if focused {
        flags |= Flags::FOCUSED;
    }
    if visible {
        flags |= Flags::FOCUS_VISIBLE;
    }
    let id = core.id;
    *core.memory.shown::<LastState>(sub(id, "last_state")) = LastState(hovered, visible);
    let state = if !core.resolved.enabled {
        State::Disabled
    } else if pressed || flags.contains(Flags::OPEN) {
        State::Pressed
    } else if hovered {
        State::Hover
    } else {
        State::Rest
    };
    let t = ease(core, [hovered, pressed, flags.intersects(Flags::ON)]);
    core.state = StateSet::new(state, flags, t);
    core.ink = surface_ink(cx.style.surface(), &core.state, inherited);
    core.stated = true;
}

/// The ink the surface reads in: the surface kind of the state, else `inherited`.
pub(crate) fn surface_ink(look: Option<&crate::style::SurfaceLook>, state: &StateSet, inherited: Ink) -> Ink {
    look.and_then(|look| look.kind_in(state)).map_or(inherited, |kind| kind.ink())
}

/// Step 13, paint, the pipeline's part before `C::paint`: the root surface and the state fill.
pub(crate) fn paint_surface<C: Component>(ui: &Ui, env: &Env, cx: &Cx<'_, C>) {
    let Some(look) = cx.style.surface() else { return };
    let fill = cx.core.resolve(&look.fill);
    let ppp = ui.ctx().pixels_per_point();
    let shapes = match look.kind_in(&cx.core.state) {
        Some(kind) => Surface::shapes(&surface_style(&env.tokens, kind), Some(fill.egui()), cx.core.rect, look.corners, env.direction, ppp),
        None => {
            let radius = look.corners.apply(look.radius, env.direction);
            let mut shapes = StateLayer::shapes(StateLayerKind::Fill, fill, cx.core.rect, radius, 0.0);
            if let Some(border) = &look.border {
                let color = cx.core.resolve(border);
                shapes.extend(StateLayer::shapes(StateLayerKind::Outline, color, cx.core.rect, radius, look.border_width));
            }
            shapes
        }
    };
    ui.painter().extend(shapes);
}

/// Step 13, paint, after `C::paint`: the focus ring of the focused component or its active part, when focus is
/// keyboard-visible. Never on `Well` surfaces.
pub(crate) fn paint_focus<C: Component>(ui: &Ui, cx: &Cx<'_, C>) {
    let Some(look) = cx.style.focus() else { return };
    let ppp = ui.ctx().pixels_per_point();
    let flags = cx.core.state.flags;
    let well = cx.style.surface().is_some_and(|surface| surface.kind == Some(crate::style::SurfaceKind::Well));
    if let Some(part) = cx.core.active.filter(|_| FocusRing::visible(flags)) {
        let rect = cx.core.part(part).rect;
        ui.painter().extend(FocusRing::shapes(look, FocusKind::Ring, &PaintShape::Rect(rect, egui::CornerRadius::ZERO), ppp));
    } else if FocusRing::visible(flags) && !well {
        let radius = cx.style.surface().map_or(egui::CornerRadius::ZERO, |surface| surface.radius);
        ui.painter().extend(FocusRing::shapes(look, FocusKind::Ring, &PaintShape::Rect(cx.core.rect, radius), ppp));
    }
}

/// A request to bring a rect into view, read by the scroll owner that step 4 of its next pass names.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Reveal {
    pub owner: Id,
    pub rect: Rect,
}

#[derive(Default)]
struct Reveals(Vec<Reveal>);

/// The reveal request addressed to `owner`, once.
#[cfg_attr(not(feature = "testing"), expect(dead_code, reason = "read by the body and item scroll (F5, F6)"))]
pub(crate) fn take_reveal(ctx: &Context, owner: Id) -> Option<Rect> {
    store::Service::<Reveals>::of(ctx).with(|reveals| {
        let at = reveals.0.iter().position(|reveal| reveal.owner == owner)?;
        Some(reveals.0.remove(at).rect)
    })
}

/// Step 15, settle: what the component asked for during the pass is written to its owner. A reveal request goes to the
/// nearest scroll owner; anything queued for the next pass requests a repaint.
pub(crate) fn settle<C: Component>(cx: &mut Cx<'_, C>) {
    // F4-LATER: values (adopted child edits, check and apply again), F5-LATER: open and gate changes.
    if let Some(rect) = cx.core.reveal.take()
        && let Some(owner) = wheel::nearest_scroll_owner(&cx.core.ctx, rect)
    {
        store::Service::<Reveals>::of(&cx.core.ctx).with(|reveals| reveals.0.push(Reveal { owner, rect }));
        repaint::request(&cx.core.ctx);
    }
}

/// Step 16, report: the cursor request.
// F4-LATER: access (the AccessKit node, sub-nodes, actions and focus); F5-LATER: tooltips (`contract::tooltip`)
pub(crate) fn report<C: Component>(cx: &Cx<'_, C>) {
    super::cx::write_cursor(&cx.core);
}

/// Step 17, finish, before `C::finish`: the click flags of the response are rewritten, so every output built from it
/// carries them.
pub(crate) fn before_finish<C: Component>(cx: &mut Cx<'_, C>, activated: bool) {
    let by_key = activated && cx.core.activation.is_some_and(|activation| activation.by_key);
    response::rewrite_clicks(&mut cx.core.response, by_key, cx.core.command.is_some());
    if activated && let Some(command) = &cx.core.command {
        commands::fire_bound(&cx.core.ctx, command, by_key);
    }
    // F4-LATER: gesture finish and `EditEvent::fold`.
}

/// The sizes a body or collection records at step 17.
pub(crate) fn record_size(ctx: &Context, id: Id, real: Vec2) {
    layout_memory::record(ctx, id, real);
}

/// The response of a component before any pass has created its root widget: inert, for the component's own id.
pub(crate) fn inert_response(ui: &Ui, id: Id) -> Response {
    response::inert(ui, id)
}
