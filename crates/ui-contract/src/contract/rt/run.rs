//! A component's run inside its scope's build, and the keep decision before it
//! (docs/contract-rt.md sections 3.3, 4, 7 and 8).
//!
//! The skeleton's [`Component`] has the hooks of `contract::Component` the bridge's stand-ins use,
//! over contexts on craft-ui-rt. The pipeline tasks move the real trait's contexts (`Cx`,
//! `MeasureCx`, `PaintUi`, `Draw`) onto these and retire this cut.

use std::cell::{Cell, RefCell};
use std::sync::Arc;

use craft_behavior::{EditEvent, EditEvents, EditorInput, GestureId, GesturePhase, Intent, Mixed, PartRef, PressPhase, PressState};
use craft_i18n::Direction;
use craft_theme::Tokens;
use craft_ui_rt::{BuildCx, Cursor, Gesture, HitShape, ImePurpose, InlineState, Interact, PointerButton, Pos2, Rect, Sense, Vec2};
use egui::Id;
use smallvec::SmallVec;

use super::access::Access;
use super::drag::{DragSource, DropRule};
use super::env::{EnvSnapshot, LayerFrame};
use super::key::{ComponentKey, component_key, fold, type_key};
use super::memory::Memory;
use super::motion::{Level, MotionSpec};
use super::node::{ComponentNode, Kept, KeptPart, NodeInput, Place, Pointer, Variant, part_id};
use super::overlay::OverlayUi;
use super::response::{EditResponse, InputFacts, Outcome, Response};
use super::sweep;
use super::tip;
use super::ui::{Flow, Ui};
use crate::contract::{BindMut, IdSource};
use crate::primitives::{Canvas, Draw, DrawEnv, FrameServices, PaintRecord, PartResult};
use crate::runtime::focus::{FocusPolicy, KeepKeys, KeyUse, Roving};
use crate::runtime::keys::Intents;
use crate::runtime::memory::Slot;
use crate::style::{Ink, UiTokens};

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
        cx.inline(
            ComponentNode { key: key.map_or(0, fold), tag, stop: component.keys().is_some(), text: component.text_input() },
            |kept, state, cx| {
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
                    kept.cache(room, size);
                    if state.path {
                        // A node in one of its layer children is marked: reach them.
                        reach_layers(&mut component, kept, cx, rect, &flow.tokens, flow.sweep);
                    } else {
                        cx.keep_children();
                    }
                    return settle(component, kept, cx, rect, &flow.tokens);
                }
                let tokens = super::env::tokens(cx);
                run(component, kept, cx, Ran { id, key, place: Place { room }, size, rect, tokens: &tokens, sweep: flow.sweep })
            },
        )
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
        holds_escape: Cell::new(false),
        intents: Intents::default(),
        ime_cursor: Cell::new(None),
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
    let intents = super::intents::of_queue(&input, || env.direction() == Direction::Rtl);
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
        holds_escape: Cell::new(false),
        intents,
        ime_cursor: Cell::new(None),
    };
    component.update(&mut rcx);
    kept.ime.caret = rcx.ime_cursor.get().map(|caret| caret.translate(Pos2::ZERO - ran.rect.min));
    let wrote = !rcx.edit.events.is_empty();
    if wrote && let Some(mut bind) = component.bind() {
        for event in rcx.edit.events.iter() {
            bind.apply(event);
        }
    }
    // `rcx` borrows `kept.memory`: the hooks below take Kept's other fields one by one.
    overlay_pass(&mut component, OverlayUi::new(cx, ran.id, ran.rect, &mut kept.popups, true, ran.tokens.clone(), ran.sweep));
    tip::show(&mut kept.tip, component.tooltip(), true, cx, ran.id, ran.rect, ran.tokens);
    kept.drag = component.drag_source();
    kept.drop = component.drop_target();
    let stop = component.keys();
    kept.focus.ring = stop.map(|_| craft_ui_rt::Stroke::new(ran.tokens.stroke.focus, ran.tokens.color.line.focus));
    kept.focus.stop = stop;
    kept.focus.scope = component.focus_scope();
    kept.focus.roving = component.roving();
    kept.focus.holds_escape = rcx.holds_escape.get();
    kept.close_guard = component.close_guard();
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
            NodeInput::Intent { intent: Intent::Activate, .. } => facts.clicked = true,
            _ => {}
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

/// Runs the component's `overlay` hook over its kept popups. Returns whether a popup opened or
/// closed.
pub(crate) fn overlay_pass<C: Component>(component: &mut C, mut ui: OverlayUi<'_, '_>) -> bool {
    component.overlay(&mut ui);
    ui.changed
}

/// The overlay and tooltip passes of a node that keeps its own outputs while a build reaches its
/// layer children (a marked node under one of them, or a container whose body runs): the popups
/// keep their bodies unless marked, and a popup that opened or closed here makes the node run in
/// this frame.
pub(crate) fn reach_layers<C: Component>(
    component: &mut C,
    kept: &mut Kept,
    cx: &mut BuildCx<'_>,
    rect: Rect,
    tokens: &Arc<Tokens>,
    sweep: Option<craft_ui_rt::State<sweep::Sweep>>,
) {
    let id = kept.id;
    if overlay_pass(component, OverlayUi::new(cx, id, rect, &mut kept.popups, false, tokens.clone(), sweep)) {
        cx.rebuild();
    }
    tip::show(&mut kept.tip, None, false, cx, id, rect, tokens);
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
