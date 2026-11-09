//! The step functions of the pipeline (1.6): named functions over plain data, in pipeline order. `run` calls
//! them for every shape.

use super::cx::gesture_key;
use super::cx::{CxCore, Decls, MeasureCx, PartExtras};
use super::values::{Checked, landed, reuse_arc};
use super::{Common, Component, Cx, IdSource, Pointer as _, Resolved, Styled as _, Validity, ValueOf};
use crate::primitives::sensing::PartDecl;
use crate::primitives::{FaceRecord, FocusKind, FocusRing, PaintShape, StateLayer, StateLayerKind, Surface};
use crate::runtime::access::{self, Access, FaceFacts, Own, Politeness, Role, SetPosition, SubDecl, SubNode, scope};
use crate::runtime::commands::{self, CommandKind, FireSource, ResolvedCommand};
use crate::runtime::egui::{memory as store, pass, repaint, response};
use crate::runtime::env::{Env, Label, Message};
use crate::runtime::focus::{self, Group, Stop};
use crate::runtime::frame::{PerViewport, Stamped};
use crate::runtime::memory::{MemoryStage, sub};
use crate::runtime::motion::{Drives as _, spec};
use crate::runtime::windows::WindowHost;
use crate::runtime::{inspect, keys, layout_memory, wheel};
use crate::style::{ComponentStyle, Flags, Ink, StateSet, StyleCx, accepted_variant, resolve_style, surface_style};
use craft_behavior::{
    CancelReason, Cursor, CursorPriority, DragWarp, EditEvent, Gesture, GesturePhase, Input, Intent, KeyRole, Mixed, NavMove, PaintedCursor, PartKey,
    PartRef, PressCancel, Proposal, Roving, Sense, Value, ValueDrag,
};
use craft_theme::{ControlSize, State, StateMix, Variant};
use egui::{Context, Id, Pos2, Rect, Response, Ui, Vec2};
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
        (label, _) => label.resolved(&env.words),
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
pub(crate) fn resolved<'p>(id: Id, step: &'p EnvStep, common: &'p Common) -> Resolved<'p> {
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
        access_key: common.access_key,
        default_stop: common.default_stop,
        ltr_axis: common.ltr_axis,
        description: common.description.as_deref(),
        tooltip: common.tooltip.as_ref(),
        doc_key: common.doc_key,
        command: step.command.as_deref(),
    }
}

/// The `StyleCx` of the env the component shows in.
pub(crate) fn style_cx<'a>(ctx: &'a Context, env: &'a Env, step: &EnvStep) -> StyleCx<'a> {
    StyleCx::new(ctx, &env.tokens, &env.values, step.size, step.requested, env.direction, env.scale, env.os, env.active, env.key, env.line_ratio)
}

/// Step 2, style: `&C::Style` from the cache, keyed `(TypeId of StyleKey, key, EnvKey, variant, size)`.
pub(crate) fn style<C: Component>(ctx: &Context, scx: &StyleCx<'_>, key: C::StyleKey) -> Arc<C::Style> {
    let style = resolve_style(scx, key, ctx, C::style);
    #[cfg(feature = "testing")]
    crate::testing::style::record::<C>(ctx, scx, key, &style);
    style
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

/// What step 4 reads: the shown value, whether the binding is mixed, and the `Cancel` of a gesture step 4 ended.
pub(crate) struct ValueStep<V> {
    pub value: Option<Arc<V>>,
    pub mixed: bool,
    pub cancelled: Option<EditEvent<V>>,
}

/// The held copy when `v` equals it, else a new copy.
fn reuse_or_new<V: Value>(held: &Option<Arc<V>>, v: V) -> Arc<V> {
    match held {
        Some(held) if **held == v => held.clone(),
        _ => Arc::new(v),
    }
}

/// The value of a `bool` component that has no binding of its own and is bound to a toggle or radio command: the
/// command's `checked` state, as a given value (10.4).
fn command_given<V: Value>(command: Option<&ResolvedCommand>) -> Option<V> {
    let command = command.filter(|command| matches!(command.kind, CommandKind::Toggle | CommandKind::Radio { .. }))?;
    let checked = command.checked?;
    (&checked as &dyn std::any::Any).downcast_ref::<V>().cloned()
}

/// The pass a component's value step last ran in.
#[derive(Clone, Default)]
struct LastDrawn(u64);

/// Step 4, value: the shown value is the running gesture's proposal, else the binding's value. The one shown copy is
/// replaced only when the binding differs from it. A gesture ends here when `cancel_all` ran, or when the app changed the
/// value under it.
pub(crate) fn value<C: Component>(comp: &mut C, stage: &mut MemoryStage, id: Id, command: Option<&ResolvedCommand>) -> ValueStep<ValueOf<C>> {
    type V<C> = ValueOf<C>;
    let held = stage.shown::<ShownCopy<V<C>>>(id).0.clone();
    let slot = gesture_key(id);
    let (serial, all_reason) = crate::runtime::edit::latest(stage.ctx());
    let mut cancelled = None;
    // A component not drawn for a pass loses its press at the end hook; its running gesture ends here, the first pass it is
    // drawn again.
    let pass_nr = crate::runtime::pointer::pass_of(stage.ctx());
    let removed = std::mem::replace(&mut stage.shown::<LastDrawn>(slot).0, pass_nr).saturating_add(1) < pass_nr;
    if stage.shown::<Gesture<V<C>>>(slot).is_running() {
        // `(given, value)` of a single-valued binding the app owns.
        let host = comp.bind().and_then(|bind| match bind.get() {
            Some(Mixed::Same(value)) if !bind.is_kept() => Some((bind.is_given(), value)),
            _ => None,
        });
        let gesture = stage.shown::<Gesture<V<C>>>(slot);
        let reason = match all_reason.filter(|_| serial > gesture.cancel_seen) {
            _ if removed => Some(CancelReason::Removed),
            Some(reason) => Some(reason),
            None => {
                // A given source: the app changed it when it differs from last pass's given value and from the last value
                // emitted, so an app that applies only `Commit` keeps its drag. Any other source: it differs from the
                // last value step 11 wrote.
                let changed = match &host {
                    Some((true, value)) => {
                        gesture.last_given.as_ref().is_some_and(|last| last != value)
                            && gesture.emitted.as_ref().is_some_and(|emitted| !comp.same_value(emitted, value))
                    }
                    Some((false, value)) => gesture.emitted.as_ref().is_some_and(|emitted| !comp.same_value(emitted, value)),
                    None => false,
                };
                changed.then_some(CancelReason::HostChanged)
            }
        };
        if let (Some((true, value)), None) = (&host, reason) {
            gesture.last_given = Some(value.clone());
        }
        if let Some(reason) = reason {
            // The document moved under the gesture, so there is nothing to restore.
            let before = gesture.before.clone().filter(|_| !reason.document_moved());
            cancelled = Some(EditEvent::cancel(gesture.id, reason, before));
            *gesture = gesture.ended(true);
        }
    }
    stage.shown::<Gesture<V<C>>>(slot).cancel_seen = serial;
    let proposal = stage.shown::<Gesture<V<C>>>(slot).proposal.clone();
    // A binding equal to the held copy is not read: no clone, no new copy.
    let bound = comp.bind().map(|bind| {
        let unchanged = held.as_deref().is_some_and(|held| bind.shown_matches(held));
        let got = if unchanged { None } else { bind.get() };
        (unchanged, got, bind.is_kept())
    });
    let mut out = ValueStep { value: None, mixed: false, cancelled };
    match (proposal, bound) {
        (Some(proposal), _) => out.value = Some(reuse_or_new(&held, proposal)),
        (None, Some((true, ..))) => out.value = held.clone(),
        (None, Some((false, Some(Mixed::Same(value)), ..))) => out.value = Some(Arc::new(value)),
        (None, Some((false, Some(Mixed::Mixed), ..))) => out.mixed = true,
        // A kept binding with a `SaveKey` reads the saved tier, one without reads session memory under the id.
        (None, Some((false, None, _))) => {
            out.value = comp.bind().and_then(|bind| bind.kept(stage, id, &held));
        }
        (None, None) => out.value = command_given::<V<C>>(command).map(|value| reuse_arc(&held, &value)),
    }
    let shown = &mut stage.shown::<ShownCopy<V<C>>>(id).0;
    let same = match (&*shown, &out.value) {
        (Some(a), Some(b)) => Arc::ptr_eq(a, b),
        (None, None) => true,
        _ => false,
    };
    if !same {
        *shown = out.value.clone();
    }
    out
}

/// The shown value for measuring and faces: reads the binding, applies nothing, opens no stage. A kept binding reads
/// shown memory.
/// Returns the value and whether the binding is mixed.
pub(crate) fn peek_value<C: Component>(comp: &mut C, ctx: &Context, id: Id, command: Option<&ResolvedCommand>) -> (Option<Arc<ValueOf<C>>>, bool) {
    let held = MemoryStage::peek::<ShownCopy<ValueOf<C>>>(ctx, id).0;
    let Some(bind) = comp.bind() else {
        let given = command_given::<ValueOf<C>>(command).map(|value| reuse_arc(&held, &value));
        return (given.or(held), false);
    };
    if held.as_deref().is_some_and(|held| bind.shown_matches(held)) {
        return (held, false);
    }
    match bind.get() {
        Some(Mixed::Same(value)) => (Some(Arc::new(value)), false),
        Some(Mixed::Mixed) => (None, true),
        None => (held, false),
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
        super::run::ShowAt::Flow => crate::layout::place_in_flow(ui, size),
        super::run::ShowAt::Rect(rect) => rect,
    }
}

/// What step 7 gathers.
pub(crate) struct PartsStep {
    pub decls: Decls,
    pub rovings: Vec<Roving>,
    pub scrolls: Vec<super::cx::ScrollDecl>,
    pub text: super::text::TextStep,
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
    /// The component takes focus (`takes_focus`): its focusable parts are stops.
    pub parts_focus: bool,
    /// The component's one stop is a text part, whose egui widget holds focus.
    pub text: bool,
    pub keys: focus::KeyUse,
    pub role: KeyRole,
    pub rtl: bool,
    pub parts: &'a PartsStep,
    /// The roving group of the body this component is shown in: its own stop joins it.
    pub group: Option<Id>,
    /// The container's `ContainerProps.roving`: its body's members form this group.
    pub body_roving: Option<Roving>,
    /// The roving group of the collection row the component is shown in: stops with no group of their own join it.
    pub row: Option<Id>,
    /// The component is the collection item of its row: its parts join the row's group instead of a group of their own.
    pub item: bool,
    /// The component's stop is its scope's default.
    pub default_stop: bool,
}

/// The id of the roving group a container's body forms.
/// Records that `id` is a member of the roving `group` in this pass.
pub(crate) fn join_group(ctx: &Context, group: Id, id: Id) {
    let slot = sub(group, ("member", id));
    store::put(ctx, slot, crate::runtime::pointer::pass_of(ctx));
    crate::runtime::memory::adopt::<u64>(ctx, id, slot);
}

/// `group` when `id` joined it in this pass or the one before, else `None`: a component that is no member keeps its
/// own Tab stop.
pub(crate) fn member_group(ctx: &Context, group: Option<Id>, id: Id) -> Option<Id> {
    let group = group?;
    let pass = crate::runtime::pointer::pass_of(ctx);
    let joined = store::peek_clone::<u64>(ctx, sub(group, ("member", id)))?;
    (joined.saturating_add(1) >= pass).then_some(group)
}

pub(crate) fn body_group(id: Id) -> Id {
    sub(id, "body_roving")
}

pub(crate) fn register_stops(input: &StopsIn<'_>) {
    let ctx = input.ctx;
    if let Some(roving) = input.body_roving {
        focus::register_group(ctx, Group::new(body_group(input.id), roving).rtl(input.rtl));
    }
    debug_assert!(input.parts.rovings.len() <= 1, "a component declares one roving group; a second would be dropped");
    let row_parts = if input.item { input.row } else { None };
    let group = (!input.parts.rovings.is_empty() && row_parts.is_none()).then(|| sub(input.id, "roving"));
    if let (Some(group), Some(roving)) = (group, input.parts.rovings.first()) {
        let arrival = input.parts.decls.iter().rev().find(|(_, extras)| extras.selected && extras.focusable);
        let group = Group::new(group, *roving).rtl(input.rtl);
        let arrival = arrival.map(|(decl, _)| (input.id, Some(PartRef { key: decl.key, index: decl.index })));
        focus::register_group(ctx, if let Some(arrival) = arrival { group.selected(arrival) } else { group });
    }
    let own = || {
        let stop = Stop::new(input.id, input.keys).root(input.holder).role(input.role);
        let stop = if input.default_stop { stop.default_stop() } else { stop };
        if input.text { stop.text() } else { stop }
    };
    if input.focusable {
        focus::register_stop(ctx, if let Some(group) = input.group.or(input.row) { own().in_group(group) } else { own() });
    }
    for (decl, extras) in &input.parts.decls {
        if extras.focusable && input.parts_focus {
            let stop = own().part(PartRef { key: decl.key, index: decl.index });
            let stop = if extras.nudges { stop.role(input.role.nudge(true)) } else { stop };
            let stop = if extras.default_stop { stop.default_stop() } else { stop };
            // A part group that joined the body's roving group has its parts registered there.
            let joined = group.and_then(|parts| focus::registration::part_group(ctx, parts));
            focus::register_stop(ctx, if let Some(group) = row_parts.or(joined).or(group).or(input.row) { stop.in_group(group) } else { stop });
        }
    }
}

/// Whether step 7 declared no parts, no roving group, no scroll part and no text or editor part.
pub(crate) fn declares_none(parts: &PartsStep) -> bool {
    parts.decls.is_empty() && parts.rovings.is_empty() && parts.scrolls.is_empty() && parts.text.decl.is_none() && parts.text.editor.is_none()
}

/// Step 7's finding (22.4): a part that senses `CLICK` or `DRAG` is focusable, declares its keyboard path, or names
/// the focusable part of the component that carries it.
pub(crate) fn check_parts(ctx: &Context, component: &'static str, parts: &PartsStep) {
    let focusable = |key: PartKey| parts.decls.iter().any(|(decl, extras)| decl.key == key && extras.focusable);
    for (decl, extras) in &parts.decls {
        let has_path = extras.focusable || extras.pointer_only.is_some() || extras.keyboard_via.is_some_and(focusable);
        if decl.sense.intersects(Sense::CLICK | Sense::DRAG) && !has_path {
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

/// Registers a wheel taker for next pass's split (8.6) from the wheel bits of `sense` (`WheelTaker::for_sense`); a
/// value control without them takes the vertical wheel only while focused or armed.
pub(crate) fn register_wheel(ctx: &Context, id: Id, rect: Rect, sense: Sense, value_control: bool) {
    if let Some(taker) = craft_behavior::WheelTaker::for_sense(id, rect, sense) {
        wheel::register(ctx, taker);
    } else if value_control {
        wheel::register(ctx, craft_behavior::WheelTaker::new(id, rect).vertical(true).gated(true));
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
    let state = if !enabled || result.inactive {
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

/// Whether the component takes activations (a click, a key, AccessKit `Click`): a disabled or read-only one drops them.
pub(crate) fn takes_activation(resolved: &Resolved<'_>) -> bool {
    resolved.enabled && !resolved.read_only
}

/// Whether the component is a focus stop: an enabled one, or a disabled one whose tooltip has a `disabled_text`, which
/// shows its reason on keyboard focus (testing 22.4). Step 9 still drops its activations and edits.
pub(crate) fn takes_focus(resolved: &Resolved<'_>) -> bool {
    resolved.enabled || resolved.tooltip.is_some_and(|tooltip| tooltip.spec().disabled_text.is_some())
}

/// A key or AccessKit activation in this pass, with the modifiers held in it: the press that routed
/// `Intent::Activate` carries them, and the pass's pointer frame reads them from the same input a pointer activation's
/// modifiers come from (Alt+Enter adds the way Alt+click does).
pub(crate) fn key_activation(ctx: &egui::Context) -> super::Activation {
    super::Activation { by_key: true, mods: crate::runtime::pointer::frame(ctx).mods }
}

/// Step 9, update, the part the library writes: the keyboard activation.
pub(crate) fn update_inputs(core: &mut CxCore<'_>) {
    if core.intents.take(craft_behavior::Intent::Activate) && takes_activation(core.resolved) {
        core.activation = Some(key_activation(&core.ctx));
    }
    // An AccessKit click on a part is a click on the component as a pointer click on the part is: the response
    // reports it, and a toggling part or a part reader takes it as it takes the pointer's.
    if core.intents.any_access_click() && takes_activation(core.resolved) {
        core.activation.get_or_insert(super::Activation { by_key: false, mods: craft_keymap::Mods::NONE });
        core.part_clicked = true;
    }
}

/// Step 9, before the component: a gesture cancelled under its press is live again once the press is released, and a
/// given caller's `Update` that fell in the pass of its `Begin` arrives now.
pub(crate) fn gesture_gate<C: Component>(comp: &C, cx: &mut Cx<'_, C>) {
    let held = cx.core.press_held() || scrub_press(comp, cx).is_some_and(|press| super::cx::press_down(&press.phase));
    let slot = gesture_key(cx.core.id);
    let gesture = cx.core.memory.shown::<Gesture<ValueOf<C>>>(slot);
    if gesture.dead && !held {
        gesture.dead = false;
    }
    if gesture.is_running()
        && let Some(value) = gesture.held.take()
    {
        let id = gesture.id;
        cx.events.push(EditEvent::update(id, value));
    }
}

/// The press that carries a scrub: the field's own, or for a scrub by the label, the label's.
fn scrub_press<C: Component>(comp: &C, cx: &Cx<'_, C>) -> Option<craft_behavior::PressState> {
    let scrub = comp.scrub()?;
    if scrub.on_field { Some(cx.core.press.clone()) } else { scope::field_label(&cx.core.ctx).map(|label| label.press) }
}

/// Step 9, the edit intents the library takes before `C::update`: Escape or a cancelled press ends a running gesture,
/// and a reset becomes a `Set` of the binding's reset value.
pub(crate) fn edit_intents<C: Component>(comp: &mut C, cx: &mut Cx<'_, C>) {
    use super::cx::Edit as _;
    if cx.gesture().is_some_and(|phase| matches!(phase, GesturePhase::Begin | GesturePhase::Update)) {
        let reason = if !cx.core.escape_to_list && cx.core.intents.take(Intent::Cancel) {
            Some(CancelReason::Escape)
        } else {
            cx.core.press_cancelled().map(|reason| match reason {
                PressCancel::Escape => CancelReason::Escape,
                PressCancel::Removed => CancelReason::Removed,
                _ => CancelReason::Blur,
            })
        };
        if let Some(reason) = reason {
            cx.cancel(reason);
        }
    }
    let reset = cx.core.intents.take_reset();
    let double = double_click(&cx.core).filter(|_| !matches!(C::INPUT, Input::Text | Input::Editor));
    if reset.is_some() || double.is_some() {
        let part = match reset {
            Some(whole) => cx.core.active.filter(|_| !whole),
            None => double.flatten(),
        };
        if let Some(part) = part.filter(|part| cx.core.own_reset_parts.contains(&(part.key, part.index))) {
            cx.core.reset_part = Some(part);
        } else if let Some(reset) = comp.bind().and_then(|bind| bind.reset_value().cloned()) {
            cx.set(reset);
        }
    }
}

/// The value a label scrub started from.
#[derive(Clone, Default)]
struct ScrubStart(f64);

/// The pointer warps of the running label scrub.
#[derive(Clone, Default)]
struct ScrubWarp(DragWarp);

/// The total warp of the running scrub. A scrub that warps moves the pointer to the other side of the window when
/// it reaches an edge, once the pointer from the last warp has shown up at its new place; a platform that cannot
/// warp leaves the drag as it is.
fn scrub_warp<C: Component>(cx: &mut Cx<'_, C>, drag: &ValueDrag, pos: Pos2) -> Vec2 {
    if !drag.warps() {
        return Vec2::ZERO;
    }
    let id = sub(cx.core.id, "scrub_warp");
    let ctx = cx.core.ctx.clone();
    let warp = &mut cx.core.memory.shown::<ScrubWarp>(id).0;
    let shift = warp.shift(pos);
    if warp.pending() {
        return shift;
    }
    let Some(screen) = crate::runtime::pointer::frame(&ctx).pos else { return shift };
    let hosts = WindowHost::of(&ctx);
    if let (Some(by), Some(desktop)) = (drag.warp_by(screen, ctx.content_rect()), hosts.desktop_pointer())
        && hosts.warp_pointer(desktop + by)
    {
        warp.sent(pos, by);
    }
    shift
}

/// Step 9, before the component: a drag on the side label (or on the component itself, for a scrub that
/// `Scrub::on_field` declares) scrubs the value of a component that opts in with `Component::scrub`. A label scrub
/// registers the control with its label scope so the label senses the drag from the next pass on.
pub(crate) fn scrub_by_label<C: Component>(comp: &C, cx: &mut Cx<'_, C>) {
    use super::cx::Edit as _;
    use craft_behavior::{PressCancel, PressPhase};
    let Some(scrub) = comp.scrub() else { return };
    let running = cx.gesture().is_some_and(|phase| matches!(phase, GesturePhase::Begin | GesturePhase::Update));
    if cx.core.resolved.read_only || !cx.core.resolved.enabled {
        if running {
            cx.cancel(CancelReason::Dismissed);
        }
        return;
    }
    if !scrub.on_field && scope::field_label(&cx.core.ctx).is_some() {
        scope::register_scrub(&cx.core.ctx);
    }
    let Some(press) = scrub_press(comp, cx) else { return };
    let start_id = sub(cx.core.id, "scrub_start");
    let rtl = cx.core.env.direction().is_rtl();
    match press.phase {
        PressPhase::DragStart { .. } => {
            cx.core.memory.shown::<ScrubStart>(start_id).0 = cx.value().map_or(0.0, |value| (scrub.read)(value));
            *cx.core.memory.shown::<ScrubWarp>(sub(cx.core.id, "scrub_warp")) = ScrubWarp::default();
            hide_scrub_cursor(cx, &scrub.drag);
            cx.begin();
        }
        PressPhase::Drag { pos, .. } => {
            hide_scrub_cursor(cx, &scrub.drag);
            let start = cx.core.memory.shown::<ScrubStart>(start_id).0;
            let shift = scrub_warp(cx, &scrub.drag, pos);
            if let Some(next) = scrub.drag.value_shifted(start, &press, rtl, shift) {
                cx.propose((scrub.write)(next));
            }
        }
        PressPhase::Release { .. } if running => cx.commit(),
        PressPhase::Cancel { reason, .. } if running => cx.cancel(match reason {
            PressCancel::Escape => CancelReason::Escape,
            PressCancel::Removed => CancelReason::Removed,
            _ => CancelReason::Blur,
        }),
        _ => {}
    }
}

/// A scrub that hides the pointer asks for no cursor while it runs.
fn hide_scrub_cursor<C: Component>(cx: &Cx<'_, C>, drag: &ValueDrag) {
    if drag.hides_cursor() {
        crate::runtime::cursor::request(&cx.core.ctx, Cursor::Painted(PaintedCursor::Hidden), CursorPriority::Drag);
    }
}

/// A second press of a double click went down on the component's root (`Some(None)`) or one of its value parts
/// (`Some(Some(part))`) this pass (10.2).
fn double_click(core: &CxCore<'_>) -> Option<Option<PartRef>> {
    if core.press.double_pressed() {
        return Some(None);
    }
    core.value_parts
        .iter()
        .find(|key| core.results.get(key).is_some_and(|result| result.press.double_pressed()))
        .map(|&(key, index)| Some(PartRef { key, index }))
}

/// Step 12: a control with a reset value tells its label scope whether its value differs from it (10.2).
pub(crate) fn register_reset<C: Component>(comp: &mut C, cx: &Cx<'_, C>) {
    if matches!(C::INPUT, Input::Text | Input::Editor) {
        return;
    }
    let Some(reset) = comp.bind().and_then(|bind| bind.reset_value().cloned()) else { return };
    let differs = cx.mixed || cx.shown.as_deref() != Some(&reset);
    scope::register_reset(&cx.core.ctx, cx.core.id, differs);
}

/// What the component left untaken that a scroll region serves: page and end moves go to the innermost scroll
/// owner around the component, and an AccessKit request to show the component reveals its rect there. Both are
/// applied at the owner's step 4 of the next pass.
fn route_to_scroll_owner(core: &mut CxCore<'_>) {
    let mut moves = Vec::new();
    while let Some(mv) = core.intents.take_scroll_move() {
        moves.push(mv);
    }
    let reveal = core.intents.take_scroll_access(true).is_some();
    if moves.is_empty() && !reveal {
        return;
    }
    let Some(owner) = core.env.scroll_owner().or_else(|| wheel::nearest_scroll_owner(&core.ctx, core.rect)) else { return };
    store::Service::<ScrollMoves>::run(&core.ctx, |queue| queue.0.extend(moves.into_iter().map(|mv| (owner, mv))));
    if reveal {
        store::Service::<Reveals>::run(&core.ctx, |reveals| reveals.0.push(Reveal { owner, rect: core.rect }));
    }
    repaint::request(&core.ctx);
}

/// Step 9, after `update`: a secondary release on the component, or a `ContextMenu` intent it did not take, records a
/// context request that the overlay step and the popups shown after it read in this pass (9.2 rule 3).
pub(crate) fn record_context(core: &CxCore<'_>) {
    if core.intents.untaken(&Intent::ContextMenu)
        || (core.response.hovered() && crate::runtime::pointer::frame(&core.ctx).release == Some(craft_behavior::PointerButton::Secondary))
    {
        let part = focus::active_part(&core.ctx, core.id).and_then(|part| core.results.get(&(part.key, part.index)));
        keys::request_context(&core.ctx, core.id, part.map_or(core.rect, |part| part.rect));
    }
}

/// Writes back what the component left untaken, and its state-dependent claims for the next frames.
pub(crate) fn after_update(core: &mut CxCore<'_>) {
    route_to_scroll_owner(core);
    keys::settle(&core.ctx, Some(core.id), &core.intents);
    for chord in std::mem::take(&mut core.claims) {
        keys::claim(&core.ctx, core.id, chord);
    }
}

/// Steps 11 and 15, apply: the events not yet applied are checked and written to the binding. A read-only component refuses
/// every one. A given caller gets `Begin` in the pass a gesture began and its first `Update` in the next, because it has
/// no source to hold the value now.
pub(crate) fn apply_values<C: Component>(comp: &mut C, cx: &mut Cx<'_, C>) {
    let from = cx.applied;
    cx.applied = cx.events.len();
    if cx.core.resolved.read_only {
        // A gesture's end still reaches the app; only events that carry a value are refused.
        let mut at = 0;
        cx.events.retain(|event| {
            let keep = at < from || matches!(event, EditEvent::Cancel { .. });
            at += 1;
            keep
        });
        cx.applied = cx.events.len();
        return;
    }
    let Some(mut bind) = comp.bind() else { return };
    // The folded `Begin`: the updates after a `Begin` of this pass wait for the next pass of a given binding.
    let mut held = None;
    if bind.is_given()
        && let Some(begin) = cx.events[from..].iter().rposition(|event| matches!(event, EditEvent::Begin { .. }))
    {
        let after = &cx.events[from + begin + 1..];
        if !after.is_empty() && after.iter().all(|event| matches!(event, EditEvent::Update { .. })) {
            held = after.last().and_then(|event| event.value().cloned());
            cx.events.truncate(from + begin + 1);
            cx.applied = cx.events.len();
        }
    }
    let slot = gesture_key(cx.core.id);
    let (mut emitted, mut began, mut landing, mut last_update) = (None, false, None, None);
    let mut changed = false;
    let mut at = from;
    while at < cx.events.len() {
        match bind.checked(&mut cx.events[at]) {
            Checked::Keep => {}
            Checked::Cancel => {
                if let EditEvent::Commit { id, before, .. } = cx.events[at].clone() {
                    cx.events[at] = EditEvent::cancel(id, CancelReason::Refused, Some(before));
                    let dead = cx.core.press_held();
                    let gesture = cx.core.memory.shown::<Gesture<ValueOf<C>>>(slot);
                    *gesture = gesture.ended(dead);
                    if matches!(cx.events[at], EditEvent::Cancel { before: Some(Mixed::Mixed), .. }) {
                        cx.shown = None;
                        cx.mixed = true;
                    }
                }
            }
            Checked::Drop => {
                cx.events.remove(at);
                cx.applied = cx.events.len();
                continue;
            }
        }
        let event = &cx.events[at];
        match event {
            EditEvent::Begin { before: Mixed::Same(value), .. } => {
                emitted = Some(value.clone());
                began = true;
            }
            EditEvent::Update { value, .. } => {
                last_update = Some(value.clone());
                if bind.writes(event) || bind.is_given() {
                    emitted = Some(value.clone());
                }
            }
            _ => {}
        }
        if let EditEvent::Update { value, .. } | EditEvent::Commit { after: value, .. } = event {
            changed |= bind.get() != Some(Mixed::Same(value.clone()));
        }
        bind.apply(event);
        if let Some(value) = landed(event) {
            landing = Some(value.clone());
        }
        at += 1;
    }
    // The given value of the pass a gesture began in, for step 4's host-change comparison next pass.
    let given_now = if began && bind.is_given() {
        match bind.get() {
            Some(Mixed::Same(value)) => Some(value),
            _ => None,
        }
    } else {
        None
    };
    // A pass that changes the value, or holds an update for the next pass, runs again before painting, so a control drawn
    // above the field shows the edit in the frame it happens. egui gives the rerun no input events and caps a frame at
    // two passes, so nothing is delivered twice.
    if changed || held.is_some() {
        crate::runtime::egui::sizing::request_discard(&cx.core.ctx, "craft_ui: an edit changed the value");
    }
    let repaint_next = held.is_some();
    let gesture = cx.core.memory.shown::<Gesture<ValueOf<C>>>(slot);
    let (running, watched) = (gesture.is_running().then_some(gesture.id), gesture.watched);
    if gesture.is_running() {
        // The value held for the next pass is already readable from the response's `shown` in this one, so a host write of
        // it is the gesture's own.
        gesture.emitted = held.clone().or(emitted).or(gesture.emitted.take());
        gesture.last_given = given_now.or(gesture.last_given.take());
        gesture.held = held.or(gesture.held.take());
        // A value the check replaced is the gesture's proposal, so the next pass shows it.
        if let Some(value) = last_update {
            gesture.proposal = Some(value);
        }
    }
    // The owner's eviction while the gesture runs ends it as removed (7.2).
    if running.is_some() || watched {
        gesture.watched = running.is_some();
        let ctx = cx.core.ctx.clone();
        let watch = cx.core.memory.shown::<crate::runtime::edit::RemovalWatch>(slot);
        match running {
            Some(id) => watch.arm(&ctx, id),
            None => watch.disarm(),
        }
    }
    if repaint_next {
        repaint::request(&cx.core.ctx);
    }
    if let Some(value) = landing {
        if bind.is_kept() {
            let id = cx.core.id;
            bind.keep(&mut *cx.core.memory, id, value.clone());
        }
        cx.shown = Some(Arc::new(value));
        cx.mixed = false;
    }
    if let Some(value) = cx.shown.as_deref() {
        cx.validity = bind.validity(value);
    }
}

/// The `(hovered, focus_visible)` a component showed last pass, which `PartsCx::hovered` and `focus_visible` read.
#[derive(Clone, Copy, Default)]
pub(crate) struct LastState(pub bool, pub bool);

/// The eased hover, press and on values of a component; a face shows its targets.
fn ease(core: &CxCore<'_>, targets: [bool; 3]) -> StateMix {
    let [hover, press, on] = targets;
    if core.face_mode {
        return StateMix::new(f32::from(u8::from(hover)), f32::from(u8::from(press)), f32::from(u8::from(on)));
    }
    let driver = core.driver();
    let [hover, press, on] = driver.state_anims([hover, press, on], [spec::HOVER, spec::HOVER, spec::CHECK]);
    StateMix::new(hover, press, on)
}

/// Step 12, state: flags first (OPEN from a popup shown this pass, EXPANDED from the container's own open body; CHECKED from a shown `bool` or a command's checked state, MIXED from a mixed binding), then `state`; animates hover, press and on; the ink of the surface, else `inherited`.
pub(crate) fn state<C: Component>(comp: &C, cx: &mut Cx<'_, C>, open: bool, expanded: bool, inherited: Ink) {
    let mut flags = comp.flags();
    if open {
        flags |= Flags::OPEN;
    }
    if expanded {
        flags |= Flags::EXPANDED;
    }
    if cx.core.resolved.read_only {
        flags |= Flags::READ_ONLY;
    }
    if cx.core.resolved.selected == Some(true) {
        flags |= Flags::SELECTED;
    }
    let shown_on = cx.shown.as_deref().and_then(|value| (value as &dyn std::any::Any).downcast_ref::<bool>()).copied();
    let command_on = cx.core.command.as_deref().and_then(|command| command.checked);
    if shown_on == Some(true) || command_on == Some(true) {
        flags |= Flags::CHECKED;
    }
    if let Some(facts) = cx.core.env.facts_of(cx.core.id) {
        flags |= facts.flags;
    }
    if cx.mixed {
        flags |= Flags::MIXED;
    }
    if cx.core.text_invalid {
        flags |= Flags::INVALID;
    }
    match &cx.validity {
        Validity::Refuse(_) => flags |= Flags::INVALID,
        Validity::Warn(_) => flags |= Flags::WARN,
        _ => {}
    }
    let core = &mut cx.core;
    core.sync_focus();
    let (hovered, pressed) = (core.response.hovered(), core.press.phase.is_press());
    let (focused, visible) = (core.focus.focused, core.focus.visible);
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
        Some(kind) => {
            let mut shapes = Surface::shapes(&surface_style(&env.tokens, kind), Some(fill.egui()), cx.core.rect, look.corners, env.direction, ppp);
            if let Some(border) = &look.border {
                let radius = look.corners.apply(look.radius, env.direction);
                let color = cx.core.resolve(border);
                shapes.extend(StateLayer::shapes(StateLayerKind::Outline, color, cx.core.rect, radius, look.border_width));
            }
            shapes
        }
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
    match cx.core.surface_slot {
        Some(slot) => ui.painter().set(slot, egui::Shape::Vec(shapes)),
        None => {
            ui.painter().extend(shapes);
        }
    }
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
pub(crate) fn take_reveal(ctx: &Context, owner: Id) -> Option<Rect> {
    store::Service::<Reveals>::run(ctx, |reveals| {
        let at = reveals.0.iter().position(|reveal| reveal.owner == owner)?;
        Some(reveals.0.remove(at).rect)
    })
}

/// A page or end move a focused component left untaken, addressed to the scroll owner around it (9.2 rule 5).
#[derive(Default)]
struct ScrollMoves(Vec<(Id, NavMove)>);

/// The scroll moves addressed to `owner`, in the order they arrived.
pub(crate) fn take_scroll_moves(ctx: &Context, owner: Id) -> Vec<NavMove> {
    store::Service::<ScrollMoves>::run(ctx, |moves| {
        let (mine, rest) = std::mem::take(&mut moves.0).into_iter().partition(|(id, _)| *id == owner);
        moves.0 = rest;
        mine.into_iter().map(|(_, mv)| mv).collect()
    })
}

/// Step 15, settle: child edits adopted with `cx.adopt` join the pass's events and are applied, and the open state of the
/// component's popups is written. A reveal request goes to the nearest scroll owner; anything queued for the next pass requests a repaint.
pub(crate) fn settle<C: Component>(comp: &mut C, cx: &mut Cx<'_, C>) {
    let adopted = std::mem::take(&mut cx.core.paint.borrow_mut().adopted);
    for event in adopted.into_iter().filter_map(|adopted| adopted.take::<ValueOf<C>>()) {
        if let Some(event) = cx.absorb(event) {
            cx.events.push(event);
        }
    }
    apply_values(comp, cx);
    let owner = cx.core.id;
    if C::OVERLAYS {
        super::overlay::write_open(&mut *cx.core.memory, owner);
    }
    if focus::take_reveal_of(&cx.core.ctx, owner) {
        cx.core.reveal.get_or_insert(cx.core.rect);
    }
    if let Some(rect) = cx.core.reveal.take()
        && let Some(owner) = cx.core.env.scroll_owner().or_else(|| wheel::nearest_scroll_owner(&cx.core.ctx, rect))
    {
        store::Service::<Reveals>::run(&cx.core.ctx, |reveals| reveals.0.push(Reveal { owner, rect }));
        repaint::request(&cx.core.ctx);
    }
}

/// What step 16 reads that the pass computed before: whether step 8 made the node and the part declarations.
pub(crate) struct ReportIn<'a> {
    /// The node step 8 created, `None` for a role with no node or while AccessKit is off.
    pub node: bool,
    pub decls: &'a [(PartDecl, PartExtras)],
    pub sense: Sense,
    /// The container's `ContainerProps.live`.
    pub live: Option<Politeness>,
    /// The axes the container's body scrolls on.
    pub scrolls: (bool, bool),
    /// The roving group the component's own stop is in.
    pub group: Option<Id>,
    /// The expanded state of a collapsible container, unless `Component::access` says otherwise.
    pub expanded: Option<bool>,
    /// The position the component reports itself, ahead of its group's and its item facts'.
    pub position: Option<SetPosition>,
}

/// The bound command's hint and shortcut text, which describe and key its node.
fn command_facts(command: &ResolvedCommand, platform: craft_keymap::Platform) -> (Option<&str>, Option<String>) {
    (command.hint.as_deref(), command.chord.as_ref().map(|chord| chord.display(platform)))
}

/// The message of a validity that warns or refuses.
fn validity_of<'c, C: Component>(cx: &'c Cx<'_, C>) -> Option<&'c Message> {
    match &cx.validity {
        Validity::Warn(message) | Validity::Refuse(message) => Some(message),
        _ => None,
    }
}

/// The message of a validity that warns or refuses as text; the node's description when the component gives none.
fn validity_message<C: Component>(cx: &Cx<'_, C>) -> Option<String> {
    validity_of(cx).map(|message| cx.core.env.resolve(&message.text).as_str().to_owned())
}

/// Step 16, report: the AccessKit node filled with its sub-nodes, the label-scope patch and the merge, then the
/// cursor request.
pub(crate) fn report<C: Component>(comp: &C, cx: &Cx<'_, C>, input: &ReportIn<'_>) {
    report_access(comp, cx, input);
    super::cx::write_cursor(&cx.core);
}

fn report_access<C: Component>(comp: &C, cx: &Cx<'_, C>, input: &ReportIn<'_>) {
    let core = &cx.core;
    let (ctx, id) = (&core.ctx, core.id);
    let mut access = Access::default();
    comp.access(&mut access, cx);
    for node in &mut access.nodes {
        if let Some(toggle) = core.toggles.iter().find(|toggle| toggle.part == node.part) {
            node.expanded = Some(toggle.open);
        }
    }
    let role = access.role.unwrap_or(C::ROLE);
    access.access_key = access.access_key.or(cx.core.resolved.access_key);
    if access.grid == access::GridPlace::default() {
        access.grid = cx.core.grid;
    }
    debug_assert!(access.role.is_none_or(|alt| C::ALT_ROLES.contains(&alt)), "`Access::role` picks one of `ALT_ROLES`");
    let flags = core.state.flags;
    let resolved = core.resolved;
    let platform = commands::platform(ctx);
    let (hint, shortcut) = core.command.as_deref().map_or((None, None), |command| command_facts(command, platform));
    let shortcut = shortcut.as_deref();
    for (_, label) in &mut access.custom_actions {
        *label = core.env.resolve(label);
    }
    let validity = validity_message(cx);
    if let (Some(message), Some(text)) = (validity_of(cx), &validity) {
        let tone = message.effective_tone(matches!(cx.validity, Validity::Warn(_)));
        scope::register_message(ctx, message, tone, text.clone());
    }
    let tooltip_text = resolved.tooltip.map(|tooltip| core.env.resolve(&tooltip.spec().text));
    let tooltip = tooltip_text.as_ref().map(Label::as_str);
    if access.live.is_none() {
        access.live = input.live;
    }
    if access.expanded.is_none() {
        access.expanded = input.expanded;
    }
    if access.position.is_none() {
        access.position = input.position;
    }
    if access.position.is_none() {
        access.position =
            input.group.and_then(|group| focus::group_position(ctx, id, group)).or_else(|| core.env.facts_of(id).map(|facts| facts.position));
    }
    let paint = core.paint.borrow();
    if role == Role::Merged {
        let facts = scope::Merged {
            name: resolved.label.to_owned(),
            description: access.description.clone().or_else(|| resolved.description.map(str::to_owned)).or_else(|| validity.clone()),
            flags,
            enabled: resolved.enabled,
        };
        scope::finish(ctx, id, Some(facts));
        return;
    }
    scope::finish(ctx, id, None);
    let carried = scope::carried_name(ctx, id);
    let Role::Node(role) = role else { return };
    if !input.node {
        return;
    }
    let inner_of = access::fold::owner_of(ctx, id);
    let folded = access::fold::take(ctx, id);
    if access.value.is_none() {
        access.value = folded.as_ref().and_then(|folded| folded.value.clone());
    }
    let focusable = takes_focus(resolved) && (input.sense.contains(Sense::FOCUS) || folded.is_some());
    let own = Own {
        id,
        component: std::any::type_name::<C>(),
        role,
        rect: core.rect,
        label: resolved.label,
        names_nothing: access.value.is_none() && carried.as_deref() == Some(resolved.label),
        tooltip,
        elided: paint.elided.as_deref(),
        description: resolved.description,
        validity: validity.as_deref(),
        hint,
        shortcut,
        display: C::INPUT == craft_behavior::Input::Display,
        flags,
        enabled: resolved.enabled,
        read_only: resolved.read_only,
        focusable,
        clickable: resolved.enabled && input.sense.contains(Sense::CLICK),
        keeps_name: resolved.keeps_name,
        scrolls: input.scrolls,
        access: &access,
        popups: &if C::OVERLAYS { super::overlay::popups_shown(ctx, id) } else { Vec::new() },
        popups_by_part: super::overlay::has_toggle_nodes(&core.toggles, &access.nodes),
        ltr_axis: resolved.ltr_axis,
    };
    if let Some(owner) = inner_of {
        // Shown as the owner's control: the owner's node is the one node, and stands for this root's key focus.
        access::hide(ctx, id);
        access::fold::offer(ctx, owner, access::fold::Folded { inner: id, value: access.value.clone() });
        if flags.contains(Flags::FOCUSED) {
            access::set_focus(ctx, owner);
        }
    } else {
        access::report(ctx, &own);
    }
    if let Some(folded) = &folded {
        keys::register_node(ctx, id, folded.inner, None, focusable);
    }
    if let Some(facts) = core.text_access.take() {
        crate::runtime::access::text::report_text(ctx, id, &facts);
    }
    if core.member_of.is_some() && flags.contains(Flags::FOCUSED) {
        access::set_focus(ctx, id);
    }
    let mut seen = Vec::new();
    let facts = core.env.facts_of(id);
    for node in &mut access.nodes {
        // A cell of an item row knows its row; its column is the position of its column key among the columns the
        // collection shows, or, for a cell keyed by position, its repeat index.
        let (Some(facts), Role::Node(crate::runtime::egui::access::NodeRole::Cell | crate::runtime::egui::access::NodeRole::GridCell)) =
            (facts, node.role)
        else {
            continue;
        };
        let column = match &node.column {
            Some(key) => facts.columns.iter().position(|column| column == key),
            None => node.part.index.map(|index| index as usize),
        };
        node.grid = access::GridPlace {
            row_index: node.grid.row_index.or(Some(facts.position.index)),
            column_index: node.grid.column_index.or(column),
            ..node.grid
        };
    }
    for node in &access.nodes {
        seen.push(node.part);
        report_sub(core, input, node, paint.faces.iter().find(|face| face.part == node.part));
    }
    for face in paint.faces.iter().filter(|face| !seen.contains(&face.part)) {
        report_sub(core, input, &SubNode::new(face.part, face.role), Some(face));
    }
}

fn report_sub(core: &CxCore<'_>, input: &ReportIn<'_>, node: &SubNode, face: Option<&FaceRecord>) {
    let key = (node.part.key, node.part.index);
    let result = core.results.get(&key);
    let declared = input.decls.iter().find(|(decl, _)| (decl.key, decl.index) == key);
    let mut node = node.clone();
    node.flags |= result.map_or(Flags::empty(), |result| result.flags);
    let decl = SubDecl {
        owner: core.id,
        component: "",
        node: &node,
        rect: result.map_or(core.rect, |result| result.rect),
        enabled: core.resolved.enabled && face.is_none_or(|face| face.enabled),
        focusable: takes_focus(core.resolved) && declared.is_some_and(|(_, extras)| extras.focusable),
        clickable: core.resolved.enabled && declared.is_some_and(|(decl, extras)| decl.sense.contains(Sense::CLICK) && !extras.no_access_click),
        active: core.active == Some(node.part),
        face: face.map(|face| FaceFacts { name: &face.name, flags: face.flags, numeric: face.numeric.as_ref(), expanded: face.expanded }),
        label: core.resolved.label,
        parent: node.under.map_or_else(|| access::fold::owner_of(&core.ctx, core.id).unwrap_or(core.id), |under| access::sub_node_id(core.id, under)),
    };
    access::report_sub(&core.ctx, &decl);
}

/// A node of a component whose size failed the guard: it shows nothing, so it is an unnamed group. The component's own
/// role would promise what the role rules require (a slider's range, a tab's position) of a node that has none.
pub(crate) fn report_inert<C: Component>(cx: &Cx<'_, C>) {
    let (Role::Node(_), Role::Node(role)) = (C::ROLE, Role::GROUP) else { return };
    let core = &cx.core;
    let mut access = Access::default();
    access.position = core.env.facts_of(core.id).map(|facts| facts.position);
    access::report(
        &core.ctx,
        &Own {
            id: core.id,
            component: std::any::type_name::<C>(),
            role,
            rect: core.rect,
            label: "",
            names_nothing: false,
            tooltip: None,
            elided: None,
            description: None,
            validity: None,
            hint: None,
            shortcut: None,
            display: false,
            flags: Flags::empty(),
            enabled: core.resolved.enabled,
            read_only: core.resolved.read_only,
            focusable: false,
            clickable: false,
            keeps_name: false,
            scrolls: (false, false),
            access: &access,
            popups: &[],
            popups_by_part: false,
            ltr_axis: core.resolved.ltr_axis,
        },
    );
}

/// Step 17, finish, before `C::finish`: the click flags of the response are rewritten, so every output built from it
/// carries them, and the bound command of an activation fires, once: with the source `Cx::activate` injected, else
/// as a component activation.
pub(crate) fn before_finish<C: Component>(cx: &mut Cx<'_, C>, activated: bool, injected: Option<FireSource>) {
    let by_key = activated && cx.core.activation.is_some_and(|activation| activation.by_key);
    response::rewrite_clicks(&mut cx.core.response, by_key || cx.core.part_clicked, cx.core.command.is_some());
    // The response reports a change for a `Set` or a `Commit` that moved the value: the pass's last event decides, because
    // `EditEvent::fold` yields one of them exactly when the last event is one.
    let changed = match cx.events.last() {
        Some(EditEvent::Commit { before, after, .. } | EditEvent::Set { before, after, .. }) => !matches!(before, Mixed::Same(held) if held == after),
        _ => false,
    };
    if changed {
        response::mark_changed(&mut cx.core.response);
    }
    if activated && let Some(command) = &cx.core.command {
        match injected {
            Some(source) => {
                commands::fire_resolved(&cx.core.ctx, command, source);
            }
            None => commands::fire_bound(&cx.core.ctx, command),
        }
    }
}

/// The sizes a body or collection records at step 17.
pub(crate) fn record_size(ctx: &Context, id: Id, real: Vec2) {
    layout_memory::record(ctx, id, real);
}

/// The response of a component before any pass has created its root widget: inert, for the component's own id.
pub(crate) fn inert_response(ui: &Ui, id: Id) -> Response {
    response::inert(ui, id)
}
