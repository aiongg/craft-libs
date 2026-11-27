//! A component's run inside its scope's build, and the keep decision before it
//! (docs/contract-rt.md sections 3.3, 4, 7 and 8).
//!
//! The skeleton's [`Component`] has the hooks of `contract::Component` the bridge's stand-ins use,
//! over contexts on craft-ui-rt. The pipeline tasks move the real trait's contexts (`Cx`,
//! `MeasureCx`, `PaintUi`, `Draw`) onto these and retire this cut.

use std::cell::{Cell, RefCell};
use std::sync::Arc;

use craft_behavior::{EditEvent, EditEvents, GestureId, GesturePhase, Mixed, PartRef, PressPhase, PressState};
use craft_i18n::Direction;
use craft_theme::Tokens;
use craft_ui_rt::{BuildCx, Cursor, Gesture, HitShape, InlineState, Interact, PointerButton, Pos2, Rect, Sense, Vec2};
use egui::Id;
use smallvec::SmallVec;

use super::access::Access;
use super::env::{EnvSnapshot, LayerFrame};
use super::key::{ComponentKey, component_key, fold, type_key};
use super::memory::Memory;
use super::motion::{Level, MotionSpec};
use super::node::{ComponentNode, Kept, KeptPart, NodeInput, Place, Pointer, Variant, part_id};
use super::response::{EditResponse, InputFacts, Outcome, Response};
use super::sweep;
use super::ui::{Flow, Ui};
use crate::contract::{BindMut, IdSource};
use crate::primitives::{Canvas, Draw, DrawEnv, FrameServices, PaintRecord, PartResult};
use crate::runtime::memory::Slot;
use crate::style::{Ink, UiTokens};

/// The value type of `C`'s output.
pub type ValueOf<C> = <<C as Component>::Output as Outcome>::Value;

/// What `measure` sees: the room the component is shown in.
pub struct MeasureCx<'a> {
    /// The room the component is shown in.
    pub room: Vec2,
    /// The tokens the node's run read.
    pub tokens: &'a Tokens,
    env: &'a EnvSnapshot,
}

impl MeasureCx<'_> {
    /// The flow direction. Reading it makes a direction change run the node again.
    pub fn direction(&self) -> Direction {
        self.env.direction()
    }

    /// The density preset. Reading it makes a density change run the node again.
    pub fn density(&self) -> craft_theme::Size {
        self.env.density()
    }
}

/// The facts of one run, or of a kept node (settled: no input, the kept facts).
pub struct Cx<'a, C: Component> {
    /// The node's craft `Id`.
    pub id: Id,
    /// The node's rect in this frame, in window coordinates.
    pub rect: Rect,
    /// The tokens of the run, or of the scope that shows a kept node.
    pub tokens: &'a Tokens,
    /// The response `finish` returns: the run's input facts and the node's interaction state.
    pub response: Response,
    input: &'a [NodeInput],
    parts: &'a [(PartRef, PartResult)],
    own: PartResult,
    memory: &'a mut Memory,
    edit: Edit<ValueOf<C>>,
    swept: Option<bool>,
    settled: bool,
    env: &'a EnvSnapshot,
    level: Cell<Option<(f32, MotionSpec)>>,
}

/// The value a run shows and the edits it made.
struct Edit<V> {
    shown: Option<Mixed<V>>,
    events: EditEvents<V>,
}

/// The value gesture a press carries across runs, in the node's memory.
#[derive(Clone)]
struct HeldGesture<V> {
    id: Option<GestureId>,
    before: Option<Mixed<V>>,
    next: u64,
}

impl<V> Default for HeldGesture<V> {
    fn default() -> Self {
        HeldGesture { id: None, before: None, next: 0 }
    }
}

impl<'a, C: Component> Cx<'a, C> {
    /// The inputs queued since the last run, oldest first.
    pub fn input(&self) -> &[NodeInput] {
        self.input
    }

    /// The node did not run: its outputs are the kept ones.
    pub fn is_settled(&self) -> bool {
        self.settled
    }

    /// A primary click or an assistive technology's click reached the node since the last run.
    pub fn clicked(&self) -> bool {
        self.response.clicked()
    }

    /// The result of `part` in this run: its rect, its press as the run's input left it, and what
    /// it senses. A part `parts` did not declare reads idle.
    pub fn part(&self, part: impl Into<PartRef>) -> &PartResult {
        self.part_if_declared(part).unwrap_or(&PartResult::IDLE)
    }

    /// The result of `part`, `None` when `parts` did not declare it.
    pub fn part_if_declared(&self, part: impl Into<PartRef>) -> Option<&PartResult> {
        let part = part.into();
        self.parts.iter().find(|(p, _)| *p == part).map(|(_, r)| r)
    }

    /// The press on the component's own shape.
    pub fn press(&self) -> &PressState {
        &self.own.press
    }

    /// The flow direction. Reading it makes a direction change run the node again.
    pub fn direction(&self) -> Direction {
        self.env.direction()
    }

    /// The density preset. Reading it makes a density change run the node again.
    pub fn density(&self) -> craft_theme::Size {
        self.env.density()
    }

    /// The motion mode in effect. Reading it makes a change of the mode run the node again.
    pub fn motion_mode(&self) -> craft_behavior::MotionMode {
        self.env.motion()
    }

    /// The rounded frame of the layer the node is in. Reading it makes a change of the frame run the
    /// node again.
    pub fn frame(&self) -> Option<LayerFrame> {
        self.env.frame()
    }

    /// Aims the node's opacity at `target` (0 to 1): the paint phase tweens toward it over `spec`'s
    /// duration, reading the frame time while it runs, so no hook runs per frame. The first target a
    /// node names is shown at once.
    pub fn fade_to(&self, target: f32, spec: MotionSpec) {
        self.level.set(Some((target.clamp(0.0, 1.0), spec)));
    }

    /// The node's memory slot of type `T`, created with its default on first use.
    pub fn memory<T: Slot>(&mut self) -> &mut T {
        self.memory.shown::<T>(self.id)
    }

    /// A copy of the node's memory slot of type `T`, or its default.
    pub fn recall<T: Slot>(&self) -> T {
        self.memory.recall::<T>(self.id)
    }

    /// The value a primary-button sweep sets on this toggle in this run, `None` when no sweep
    /// reached it. `on` is the value the toggle shows. Only toggles that return their value from
    /// `Component::sweep_value` and share a layout take part; the press owner's run gets the value
    /// first, each toggle the sweep crosses gets it once.
    pub fn swept_toggle(&self, on: Option<bool>) -> Option<bool> {
        self.swept.filter(|&swept| on != Some(swept))
    }

    /// The value the component shows: its binding's, with this run's edits applied.
    pub fn value(&self) -> Option<&ValueOf<C>> {
        self.edit.shown.as_ref().and_then(Mixed::same)
    }

    /// The running value gesture's phase: `Begin` in the run that began it, `Commit` in the run
    /// that ended it, `Update` in between; `None` when none runs.
    pub fn gesture(&self) -> Option<GesturePhase> {
        let last = self.edit.events.iter().rev().find_map(EditEvent::phase);
        match last {
            Some(phase) => Some(phase),
            None => self.memory.recall::<HeldGesture<ValueOf<C>>>(self.id).id.map(|_| GesturePhase::Update),
        }
    }

    /// Begins a value gesture (one undo step) unless one runs.
    pub fn begin(&mut self) {
        let before = self.edit.shown.clone();
        let held = self.memory.shown::<HeldGesture<ValueOf<C>>>(self.id);
        if held.id.is_some() {
            return;
        }
        let id = GestureId::new(held.next);
        held.next += 1;
        held.id = Some(id);
        held.before = before.clone();
        if let Some(before) = before {
            self.edit.events.push(EditEvent::begin(id, before, Arc::from(""), None));
        }
    }

    /// Proposes `v` in the running gesture, beginning one when none runs.
    pub fn propose(&mut self, v: ValueOf<C>) {
        self.begin();
        if let Some(id) = self.memory.recall::<HeldGesture<ValueOf<C>>>(self.id).id {
            self.edit.events.push(EditEvent::update(id, v.clone()));
            self.edit.shown = Some(Mixed::Same(v));
        }
    }

    /// Ends the running gesture with the shown value.
    pub fn commit(&mut self) {
        let held = std::mem::take(self.memory.shown::<HeldGesture<ValueOf<C>>>(self.id));
        let next = held.next;
        self.memory.shown::<HeldGesture<ValueOf<C>>>(self.id).next = next;
        if let (Some(id), Some(before), Some(after)) = (held.id, held.before, self.value().cloned()) {
            self.edit.events.push(EditEvent::commit(id, before, after));
        }
    }

    /// Sets `v` in one step, outside a gesture.
    pub fn set(&mut self, v: ValueOf<C>) {
        if let Some(before) = self.edit.shown.clone() {
            self.edit.events.push(EditEvent::set(before, v.clone(), Arc::from(""), None));
        }
        self.edit.shown = Some(Mixed::Same(v));
    }

    /// The output of a component that edits a value: the response, the run's edits and the shown
    /// value.
    pub fn edit_response(&self) -> EditResponse<ValueOf<C>> {
        EditResponse {
            response: self.response.clone(),
            events: self.edit.events.clone(),
            shown: self.edit.shown.clone().map(|m| match m {
                Mixed::Same(v) => Mixed::Same(Arc::new(v)),
                Mixed::Mixed => Mixed::Mixed,
            }),
        }
    }
}

/// One paint variant being recorded: the content-paint API (`Draw`, in window coordinates, recorded as display items
/// in the node's coordinates) and the pointer state the variant shows.
pub struct Paint<'a> {
    /// The tokens of the run.
    pub tokens: &'a Tokens,
    rect: Rect,
    pointer: Pointer,
    read: Cell<bool>,
    canvas: &'a Canvas<'a>,
    record: &'a RefCell<PaintRecord>,
    ui_tokens: &'a UiTokens,
    env: &'a EnvSnapshot,
}

impl Paint<'_> {
    /// The flow direction. Reading it makes a direction change run the node again.
    pub fn direction(&self) -> Direction {
        self.env.direction()
    }

    /// The rounded frame of the layer the node is in, in window coordinates. Reading it makes a
    /// change of the frame run the node again.
    pub fn frame(&self) -> Option<LayerFrame> {
        self.env.frame()
    }

    /// The pointer state this variant shows. Reading it makes the run record a variant per state.
    pub fn pointer(&self) -> Pointer {
        self.read.set(true);
        self.pointer
    }

    /// The node's rect in window coordinates, where `draw` paints.
    pub fn bounds(&self) -> Rect {
        self.rect
    }

    /// The content-paint API.
    pub fn draw(&self) -> Draw<'_> {
        Draw::new(self.canvas, self.record, DrawEnv { tokens: self.ui_tokens, direction: self.env.peek_direction(), ink: Ink::OnPanel })
    }
}

/// Where `parts` declares the component's named sub-rects.
pub struct PartsCx<'a, C: Component> {
    /// The tokens of the run.
    pub tokens: &'a Tokens,
    value: Option<&'a ValueOf<C>>,
    origin: Pos2,
    parts: &'a mut Vec<KeptPart>,
}

impl<C: Component> PartsCx<'_, C> {
    /// The value the component shows.
    pub fn value(&self) -> Option<&ValueOf<C>> {
        self.value
    }

    /// Declares `part` over `shape` (window coordinates). It senses nothing until
    /// [`PartBuilder::sense`] says what.
    pub fn part(&mut self, part: impl Into<PartRef>, shape: impl Into<HitShape>) -> PartBuilder<'_> {
        let part = part.into();
        let shape = translate(shape.into(), Pos2::ZERO - self.origin);
        self.parts.retain(|p| p.part != part);
        self.parts.push(KeptPart { part, id: part_id(part), shape, sense: Sense::empty(), cursor: None });
        PartBuilder { part: self.parts.last_mut().expect("the part was just pushed") }
    }
}

/// Sets what a declared part senses and shows.
pub struct PartBuilder<'a> {
    part: &'a mut KeptPart,
}

impl PartBuilder<'_> {
    /// What the part senses.
    pub fn sense(self, sense: Sense) -> Self {
        self.part.sense = sense;
        self
    }

    /// The cursor over the part, kept while a press that went down on it runs.
    pub fn cursor(self, cursor: impl Into<Cursor>) -> Self {
        self.part.cursor = Some(cursor.into());
        self
    }
}

fn translate(shape: HitShape, by: Vec2) -> HitShape {
    match shape {
        HitShape::Rect(r) => HitShape::Rect(r.translate(by)),
        HitShape::RoundedRect(r, radius) => HitShape::RoundedRect(r.translate(by), radius),
        HitShape::Circle(c, radius) => HitShape::Circle(c + by, radius),
    }
}

fn bounds(shape: HitShape) -> Rect {
    match shape {
        HitShape::Rect(r) | HitShape::RoundedRect(r, _) => r,
        HitShape::Circle(c, radius) => Rect::from_center_size(c, Vec2::splat(2.0 * radius)),
    }
}
/// The skeleton's component-author trait: `contract::Component`'s hooks over craft-ui-rt contexts.
/// The hooks run in the build of the scope that shows the component, and only when the keep
/// decision says so; their outputs are kept per node.
pub trait Component: ComponentKey + Sized {
    /// The AccessKit role.
    const ROLE: accesskit::Role;
    /// What `show` returns.
    type Output: Outcome;

    /// Where the node's id comes from (`Common`'s id source).
    fn id_source(&self) -> IdSource {
        IdSource::Auto
    }

    /// The node's size in the room it has.
    fn measure(&mut self, _cx: &MeasureCx<'_>) -> Vec2 {
        Vec2::ZERO
    }

    /// How input reaches the component's own shape.
    fn sense(&self) -> Sense {
        Sense::empty()
    }

    /// The cursor over the component's own shape: kept with the node and picked by ui-rt on hover, so
    /// a hover runs no hook. `None` inherits the nearest cursor above.
    fn cursor(&self) -> Option<Cursor> {
        None
    }

    /// Named sub-rects (thumbs, tracks, handles), kept as hit shapes until the next run.
    fn parts(&mut self, _rect: Rect, _cx: &mut PartsCx<'_, Self>) {}

    /// Value components return their binding; the same binding on every call, with no side effects.
    fn bind(&mut self) -> Option<BindMut<'_, ValueOf<Self>>> {
        None
    }

    /// A toggle that takes part in primary-button sweeps returns the value it shows.
    fn sweep_value(&self) -> Option<bool> {
        None
    }

    /// Sees the queued input ([`Cx::input`], [`Cx::part`]) and edits the value.
    fn update(&mut self, _cx: &mut Cx<'_, Self>) {}

    /// Paints in local coordinates; may run once per pointer state it reads.
    fn paint(&mut self, _paint: &Paint<'_>, _cx: &Cx<'_, Self>) {}

    /// Says what the node is to assistive technology: its name and facts beyond the role and the
    /// sense. Runs only while the window's access tree is on.
    fn access(&self, _access: &mut Access, _cx: &Cx<'_, Self>) {}

    /// Runs at every `show`: after the run, or on the settled `Cx` of a kept node.
    fn finish(self, cx: &Cx<'_, Self>) -> Self::Output;
}

/// Whether a node keeps its outputs: it ran before with this component key and carries no mark.
/// The bridge's own key decides (ui-rt's `key_moved` is not consulted), so a run that writes its
/// binding keeps the key taken after the write. Its room may have moved: what it shows depends on
/// its size, which the size cache or `measure` gives for the new room.
pub(crate) fn holds(kept: &Kept, key: Option<u128>, state: InlineState) -> bool {
    !state.new && !state.marked && key.is_some() && kept.key == key && !kept.variants.is_empty()
}

/// The sweep group of a sweeping toggle: the id of the layout it is shown in.
pub(crate) fn sweep_tag<C: Component>(component: &C, flow: &Flow) -> u64 {
    if component.sweep_value().is_some() { flow.id.value() | 1 } else { 0 }
}

/// Shows a leaf: keeps it, or runs its hooks. In a measuring run it only reports its size: a kept
/// node's from its size cache, otherwise `measure`'s.
pub fn show_leaf<C: Component>(component: C, ui: &mut Ui<'_, '_>) -> C::Output {
    let (id, child) = ui.flow.child(type_key::<C>(), component.id_source());
    let key = component_key(&component);
    let room = ui.flow.room();
    let tag = sweep_tag(&component, &ui.flow);
    let mut component = component;
    if let Some(under) = ui.flow.measuring {
        let cached =
            under.and_then(|u| ui.cx.peek::<ComponentNode>(u, child)).filter(|p| holds(p.local, key, p.state)).and_then(|p| p.local.cached(room));
        let tokens = ui.flow.tokens.clone();
        let env = EnvSnapshot::peek(ui.cx);
        let size = cached.unwrap_or_else(|| component.measure(&MeasureCx { room, tokens: &tokens, env: &env }));
        env.register(ui.cx);
        let rect = ui.flow.allocate(None, size, false);
        return finish_measured(component, id, rect, &tokens);
    }
    let Ui { cx, flow } = ui;
    cx.keyed(child, |cx| {
        cx.inline(ComponentNode { key: key.map_or(0, fold), tag }, |kept, state, cx| {
            let kept_outputs = holds(kept, key, state);
            let size = match kept.cached(room).filter(|_| kept_outputs) {
                Some(size) => size,
                None => {
                    let tokens = if kept_outputs { flow.tokens.clone() } else { super::env::tokens(cx) };
                    let env = EnvSnapshot::peek(cx);
                    let size = component.measure(&MeasureCx { room, tokens: &tokens, env: &env });
                    env.register(cx);
                    size
                }
            };
            let rect = flow.allocate(Some(cx.node()), size, false);
            if kept_outputs && size == kept.size {
                cx.keep_children();
                kept.cache(room, size);
                return settle(component, kept, cx, rect, &flow.tokens);
            }
            let tokens = super::env::tokens(cx);
            run(component, kept, cx, Ran { id, key, place: Place { room }, size, rect, tokens: &tokens, sweep: flow.sweep })
        })
    })
}

/// The facts of a run decided before its hooks.
pub(crate) struct Ran<'a> {
    pub(crate) id: Id,
    pub(crate) key: Option<u128>,
    pub(crate) place: Place,
    pub(crate) size: Vec2,
    pub(crate) rect: Rect,
    pub(crate) tokens: &'a Arc<Tokens>,
    pub(crate) sweep: Option<craft_ui_rt::State<sweep::Sweep>>,
}

/// The output of a kept node: `finish` on a settled `Cx` with the kept facts and no input.
pub(crate) fn settle<C: Component>(component: C, kept: &mut Kept, cx: &mut BuildCx<'_>, rect: Rect, tokens: &Tokens) -> C::Output {
    let node = cx.node();
    let response = Response::new(kept.id, rect, Some(node), cx.interaction(node), kept.dragging, InputFacts::default());
    finish_settled(component, kept.id, rect, tokens, response, &mut kept.memory)
}

/// The output of a component a measuring run reached: `finish` on a settled `Cx` with no node
/// behind it, so its response has no interaction state and logs no reads.
pub(crate) fn finish_measured<C: Component>(component: C, id: Id, rect: Rect, tokens: &Tokens) -> C::Output {
    let response = Response::new(id, rect, None, Interact::default(), false, InputFacts::default());
    finish_settled(component, id, rect, tokens, response, &mut Memory::default())
}

fn finish_settled<C: Component>(mut component: C, id: Id, rect: Rect, tokens: &Tokens, response: Response, memory: &mut Memory) -> C::Output {
    let shown = component.bind().and_then(|b| b.get());
    let env = EnvSnapshot::default();
    let rcx = Cx {
        id,
        rect,
        tokens,
        response,
        input: &[],
        parts: &[],
        own: PartResult::IDLE,
        memory,
        edit: Edit { shown, events: EditEvents::new() },
        swept: None,
        settled: true,
        env: &env,
        level: Cell::new(None),
    };
    component.finish(&rcx)
}

/// Runs the hooks after `measure`: parts, input, update and the binding's writes, paint variants,
/// access, finish. The node's layout, paint and access run in this frame.
pub(crate) fn run<C: Component>(mut component: C, kept: &mut Kept, cx: &mut BuildCx<'_>, ran: Ran<'_>) -> C::Output {
    cx.count_run();
    cx.mark_changed();
    let node = cx.node();
    let env = EnvSnapshot::peek(cx);
    let sense = component.sense();
    kept.cursor = component.cursor();
    let shown = component.bind().and_then(|b| b.get());
    let mut parts = Vec::new();
    component
        .parts(ran.rect, &mut PartsCx { tokens: ran.tokens, value: shown.as_ref().and_then(Mixed::same), origin: ran.rect.min, parts: &mut parts });
    kept.parts = parts;
    kept.sense = sense;
    kept.sweep = ran.sweep;
    kept.toggle = component.sweep_value();
    let input = std::mem::take(&mut kept.queue);
    let (facts, dragging) = input_facts(&input, ran.rect.min, kept.dragging);
    kept.dragging = dragging;
    let interact = cx.interaction(node);
    let (own, results) = part_results(&kept.parts, sense, &input, ran.rect, interact.hovered);
    let swept = match (kept.toggle, ran.sweep) {
        (Some(_), Some(cell)) => sweep::take(cx, cell, node),
        _ => None,
    };
    let response = Response::new(ran.id, ran.rect, Some(node), interact, dragging, facts);
    let mut rcx = Cx {
        id: ran.id,
        rect: ran.rect,
        tokens: ran.tokens,
        response,
        input: &input,
        parts: &results,
        own,
        memory: &mut kept.memory,
        edit: Edit { shown, events: EditEvents::new() },
        swept,
        settled: false,
        env: &env,
        level: Cell::new(None),
    };
    component.update(&mut rcx);
    let wrote = !rcx.edit.events.is_empty();
    if wrote && let Some(mut bind) = component.bind() {
        for event in rcx.edit.events.iter() {
            bind.apply(event);
        }
    }
    kept.variants = record_variants(&mut component, &rcx, cx, sense);
    // Nothing is built while the window's access tree is off; turning it on rebuilds every node.
    let built = cx.access_enabled().then(|| {
        let mut access = Access::default();
        component.access(&mut access, &rcx);
        access.finish(C::ROLE, sense)
    });
    let (access, subs) = built.flatten().map_or((None, Vec::new()), |b| (Some(b.node), b.subs));
    kept.access = access;
    kept.access_subs = subs;
    kept.id = ran.id;
    kept.key = if wrote { component_key(&component) } else { ran.key };
    kept.place = ran.place;
    kept.size = ran.size;
    // The size cache holds only this run's size (field access: `rcx` still borrows the memory).
    kept.sizes.clear();
    kept.sizes.push((ran.place.room, ran.size));
    if let Some((target, spec)) = rcx.level.take() {
        Level::aim(kept.level.get_mut(), target, spec);
    }
    let out = component.finish(&rcx);
    env.register(cx);
    out
}

/// The response's input facts from a run's queue, and whether a drag of the node's press runs after
/// it (`dragging` before it).
fn input_facts(input: &[NodeInput], origin: Pos2, mut dragging: bool) -> (InputFacts, bool) {
    let mut facts = InputFacts::default();
    for i in input {
        match i {
            NodeInput::Pointer { gesture, pos, button, .. } => {
                facts.pointer = Some(*pos + origin.to_vec2());
                match *gesture {
                    Gesture::LongPress => facts.long_pressed = true,
                    Gesture::DragStart { .. } => {
                        facts.drag_started = true;
                        dragging = true;
                    }
                    Gesture::Drag { delta } => facts.drag_delta += delta,
                    Gesture::Up { click, clicks } => {
                        if click && *button == PointerButton::Primary {
                            facts.clicked = true;
                            facts.double_clicked |= clicks == 2;
                        }
                        facts.secondary_clicked |= click && *button == PointerButton::Secondary;
                        facts.drag_stopped |= dragging;
                        dragging = false;
                    }
                    Gesture::Cancel(_) => {
                        facts.drag_stopped |= dragging;
                        dragging = false;
                    }
                    Gesture::Down { .. } => {}
                }
            }
            NodeInput::Access(a) => facts.clicked |= a.action == accesskit::Action::Click && a.sub.is_none(),
        }
    }
    (facts, dragging)
}

/// The press of the node's own shape and of each declared part, folded from the run's queue: the
/// last phase on it, drag moves summed. Positions are window coordinates.
fn part_results(parts: &[KeptPart], sense: Sense, input: &[NodeInput], rect: Rect, hovered: bool) -> (PartResult, Vec<(PartRef, PartResult)>) {
    let press_of = |id: u64| {
        let mut state = PressState::IDLE;
        for i in input {
            let NodeInput::Pointer { gesture, pos, part, start, mods, .. } = i else { continue };
            if *part != id {
                continue;
            }
            let at = *pos + rect.min.to_vec2();
            let mut start = *start;
            start.origin += rect.min.to_vec2();
            let phase = match *gesture {
                Gesture::Down { .. } => PressPhase::down(start),
                Gesture::LongPress => PressPhase::held(start, at),
                Gesture::DragStart { .. } => PressPhase::drag_start(start, at),
                Gesture::Drag { delta } => PressPhase::drag(start, at, drag_delta(state.phase) + delta),
                Gesture::Up { click, .. } => PressPhase::release(start, at, click),
                Gesture::Cancel(reason) => PressPhase::cancel(start, reason),
            };
            state = PressState::new(phase).mods(*mods).captured(phase.is_drag());
        }
        state
    };
    let own = PartResult { rect, press: press_of(0), hovered, sense, ..PartResult::IDLE };
    let results = parts
        .iter()
        .map(|p| {
            (p.part, PartResult { rect: bounds(p.shape).translate(rect.min.to_vec2()), press: press_of(p.id), sense: p.sense, ..PartResult::IDLE })
        })
        .collect();
    (own, results)
}

fn drag_delta(phase: PressPhase) -> Vec2 {
    match phase {
        PressPhase::Drag { delta, .. } => delta,
        _ => Vec2::ZERO,
    }
}

/// One paint, then one per further pointer state when the paint read it (`Press` only for a
/// component that takes presses).
fn record_variants<C: Component>(component: &mut C, cx: &Cx<'_, C>, build: &mut BuildCx<'_>, sense: Sense) -> SmallVec<[Variant; 1]> {
    let scale = build.scale();
    let (text, images) = build.paint_services();
    let services = FrameServices::new(text, images, scale);
    let ui_tokens = UiTokens::new(Arc::new(cx.tokens.clone()), craft_theme::Contrast::Standard, craft_theme::Size::Compact);
    let mut paint_as = |look: Pointer| {
        let canvas = Canvas::new(&services, cx.rect.min, Rect::EVERYTHING);
        let record = RefCell::new(PaintRecord::default());
        let p = Paint {
            tokens: cx.tokens,
            rect: cx.rect,
            pointer: look,
            read: Cell::new(false),
            canvas: &canvas,
            record: &record,
            ui_tokens: &ui_tokens,
            env: cx.env,
        };
        component.paint(&p, cx);
        let read = p.read.get();
        if record.borrow().direction_read {
            cx.env.mark_direction();
        }
        let (content, overlay) = canvas.finish();
        (Variant { look, content, overlay }, read)
    };
    let (rest, read) = paint_as(Pointer::Rest);
    let mut out = SmallVec::new();
    out.push(rest);
    if read {
        out.push(paint_as(Pointer::Hover).0);
        if sense.takes_press() {
            out.push(paint_as(Pointer::Press).0);
        }
    }
    out
}
