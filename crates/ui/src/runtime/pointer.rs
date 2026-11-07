//! The pointer snapshot, press targets, the shape record and press cancels (8.1, 8.3).
//!
//! The begin hook converts the pass's pointer input once. The sensing core in `primitives` runs the press
//! lifecycle over it and writes the shapes it registers into the record the focus service reads.

use crate::runtime::egui::areas::{self, Layer};
use crate::runtime::egui::{input, pass};
use crate::runtime::frame::{PerViewport, Stamped};
use craft_behavior::{
    Buttons, MultiTouch, PointerButton, PointerEvent, PointerKind, PointerSample, PressCancel, PressStart, ResolveFor, Shape, Winner, count_clicks,
    resolve,
};
use craft_keymap::Mods;
use craft_theme::HitSizes;
use egui::{Context, Id, Pos2, Vec2};
use std::collections::HashMap;

/// Points one wheel line scrolls.
// F5: `Scroll.notch`
pub(crate) const WHEEL_LINE_POINTS: f32 = 40.0;

/// Points one wheel page scrolls.
// F5: `Scroll.notch`
pub(crate) const WHEEL_PAGE_POINTS: f32 = 400.0;

/// The pointer input of one pass of one viewport, egui-converted.
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
    /// The button released in the pass.
    pub release: Option<PointerButton>,
    /// The modifiers held.
    pub mods: Mods,
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
    /// The movement that turns a press into a drag.
    pub slop: f32,
    /// The least hit size across for this pointer kind.
    pub target: f32,
}

/// egui's click and drag widgets for the latest press.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct PressTargets {
    pub click: Option<Id>,
    pub drag: Option<Id>,
}

/// What the begin hook read of the pass, for the sensing core and the services.
#[derive(Clone, Debug, Default)]
pub(crate) struct PassPointer {
    pub frame: Option<PointerFrame>,
    /// The primary events, presses carrying their click counts.
    pub events: Vec<PointerEvent>,
    pub hover: Option<Pos2>,
    pub primary_down: bool,
    pub press_targets: PressTargets,
    pub focus_lost: bool,
}

#[derive(Default)]
struct Memory {
    last_press: Option<(Pos2, f64, u32)>,
    press_targets: PressTargets,
    /// A cancel a service requested for the running pass.
    requested: Option<(u64, PressCancel)>,
    /// Whether any arena of the viewport holds a dragging press; sensing registers it, as the layer above.
    drag_probe: Option<fn(&Context) -> bool>,
}

/// What one root registered on one layer: its shapes and its egui widgets.
#[derive(Clone, Default)]
struct RootRecord {
    shapes: Vec<Shape<Id>>,
    /// The root's egui widgets: the click and drag targets that stand for it.
    widgets: Vec<Id>,
    /// The root's widgets sense a click.
    click: bool,
}

type LayerRecord = HashMap<Layer, HashMap<Id, RootRecord>>;

#[derive(Default)]
struct Shapes {
    this: LayerRecord,
    last: LayerRecord,
}

/// The begin hook (H5): converts the pass's pointer input.
pub(crate) fn begin(ctx: &Context, hit: HitSizes) {
    let viewport = pass::current_viewport(ctx);
    let pass_nr = pass::pass_nr(ctx, viewport);
    let read = input::pointer_read(ctx);
    let wheel = input::wheel_read(ctx, WHEEL_LINE_POINTS, WHEEL_PAGE_POINTS);
    let hits = input::hit_targets(ctx);
    let middle_pressed = read.events.iter().any(|event| matches!(event, PointerEvent::Pressed { button: PointerButton::Middle, .. }));
    let (events, targets) = PerViewport::<Memory>::with(ctx, viewport, |memory| {
        let mut events = read.events.clone();
        for event in &mut events {
            if let PointerEvent::Pressed { button: PointerButton::Primary, pos, time, clicks, .. } = event {
                *clicks = count_clicks(memory.last_press, *pos, *time, read.slop);
                memory.last_press = Some((*pos, *time, *clicks));
            }
        }
        if read.primary_pressed || middle_pressed {
            memory.press_targets = PressTargets { click: hits.0, drag: hits.1 };
        }
        (events, memory.press_targets)
    });
    let press = events.iter().rev().find_map(|event| match event {
        PointerEvent::Pressed { button, pos, time, clicks, .. } => Some(PressStart::new(*pos, *button, *time, *clicks)),
        _ => None,
    });
    let press = press.or_else(|| read.other_press.map(|(button, at)| PressStart::new(at, button, read.time, 1)));
    let (scroll, wheel_zoom) = (wheel.map_or(Vec2::ZERO, |(scroll, ..)| scroll), read.zoom);
    let frame = PointerFrame {
        pos: read.pos,
        delta: read.delta,
        buttons: read.buttons,
        press,
        release: read.release,
        mods: read.mods,
        time: read.time,
        kind: read.kind,
        sample: read.sample,
        multi_touch: read.multi_touch.map(|(translation, zoom, rotation, center)| MultiTouch::new(translation, zoom, rotation, center)),
        scroll,
        zoom: wheel_zoom,
        slop: read.slop,
        target: hit.target,
    };
    let pass_pointer = PassPointer {
        frame: Some(frame),
        events,
        hover: read.hover,
        primary_down: read.primary_down,
        press_targets: targets,
        focus_lost: read.focus_lost,
    };
    PerViewport::<Stamped<PassPointer>>::with_pass(ctx, viewport, pass_nr, |_, _| {}, |record| *record = pass_pointer);
    PerViewport::<Shapes>::with(ctx, viewport, |shapes| shapes.last = std::mem::take(&mut shapes.this));
}

/// What the begin hook read of the running pass.
pub(crate) fn pass_pointer(ctx: &Context) -> PassPointer {
    let viewport = pass::current_viewport(ctx);
    let pass_nr = pass::pass_nr(ctx, viewport);
    PerViewport::<Stamped<PassPointer>>::with_pass(ctx, viewport, pass_nr, |_, _| {}, |record| record.clone())
}

/// The running pass's pointer input.
pub fn frame(ctx: &Context) -> PointerFrame {
    pass_pointer(ctx).frame.unwrap_or_else(|| PointerFrame {
        pos: None,
        delta: Vec2::ZERO,
        buttons: Buttons::empty(),
        press: None,
        release: None,
        mods: Mods::NONE,
        time: 0.0,
        kind: PointerKind::Mouse,
        sample: None,
        multi_touch: None,
        scroll: Vec2::ZERO,
        zoom: 1.0,
        slop: 0.0,
        target: 0.0,
    })
}

/// Ends every captured press of the running viewport at its next lifecycle run in this pass (Escape, a part
/// turning disabled). The next begin hook ends egui's own drag so hover reaches other widgets again.
pub(crate) fn cancel_press(ctx: &Context, reason: PressCancel) {
    let viewport = pass::current_viewport(ctx);
    let pass_nr = pass::pass_nr(ctx, viewport);
    PerViewport::<Memory>::with(ctx, viewport, |memory| memory.requested = Some((pass_nr, reason)));
}

/// Registers how the viewport tells that a press is captured by a drag. Sensing owns the arenas' machines.
pub(crate) fn register_drag_probe(ctx: &Context, probe: fn(&Context) -> bool) {
    PerViewport::<Memory>::with(ctx, pass::current_viewport(ctx), |memory| memory.drag_probe = Some(probe));
}

/// Whether a press of the running viewport is captured by a drag: Escape cancels it before anything else.
pub(crate) fn press_captured(ctx: &Context) -> bool {
    let probe = PerViewport::<Memory>::with(ctx, pass::current_viewport(ctx), |memory| memory.drag_probe);
    probe.is_some_and(|probe| probe(ctx))
}

/// Why captured presses end in the running pass: a cancel a service requested, else the window losing focus.
pub(crate) fn cancel_of_pass(ctx: &Context) -> Option<PressCancel> {
    let viewport = pass::current_viewport(ctx);
    let pass_nr = pass::pass_nr(ctx, viewport);
    let requested = PerViewport::<Memory>::with(ctx, viewport, |memory| memory.requested.filter(|(nr, _)| *nr == pass_nr).map(|(_, reason)| reason));
    requested.or_else(|| pass_pointer(ctx).focus_lost.then_some(PressCancel::FocusLost))
}

/// Writes `shapes` of the arena rooted at `root` into the running pass's shape record under `layer`. Stack
/// positions are per arena, so each root's shapes stay apart.
pub(crate) fn record_shapes(ctx: &Context, layer: Layer, root: Id, shapes: impl IntoIterator<Item = Shape<Id>>) {
    let viewport = pass::current_viewport(ctx);
    PerViewport::<Shapes>::with(ctx, viewport, |record| record.this.entry(layer).or_default().entry(root).or_default().shapes.extend(shapes));
}

/// Records the egui widgets that stand for the root at `root` (`click` when they sense a click).
pub(crate) fn record_widgets(ctx: &Context, layer: Layer, root: Id, widgets: &[Id], click: bool) {
    let viewport = pass::current_viewport(ctx);
    PerViewport::<Shapes>::with(ctx, viewport, |record| {
        let entry = record.this.entry(layer).or_default().entry(root).or_default();
        entry.widgets.extend_from_slice(widgets);
        entry.click = click;
    });
}

/// The winner of a press at `at` among last pass's shapes of the root egui hands the press to, on the topmost
/// layer under it, with that root: what the focus service classifies the press by (8.3, H7). A press whose egui
/// target is no root's widget has no winner. Shapes of transformed bodies lie on their own layers.
pub(crate) fn press_winner(ctx: &Context, at: Pos2, targets: PressTargets) -> Option<(Winner<Id>, Id)> {
    let layer = areas::layer_at(ctx, at)?;
    let viewport = pass::current_viewport(ctx);
    let (root, shapes) = PerViewport::<Shapes>::with(ctx, viewport, |record| {
        let roots = || record.last.values().flat_map(|roots| roots.iter());
        let root_of = |widget: Option<Id>, drag_only: bool| {
            let widget = widget?;
            roots().find(|(_, root)| root.widgets.contains(&widget) && (!drag_only || !root.click)).map(|(id, _)| *id)
        };
        let root = root_of(targets.click, false).or_else(|| root_of(targets.drag, true))?;
        let shapes = record.last.get(&layer)?.get(&root)?.shapes.clone();
        Some((root, shapes))
    })?;
    let map = areas::layer_map(ctx, layer);
    resolve(&shapes, map.map_or(at, |map| map.to_layer(at)), ResolveFor::Press).map(|winner| (winner, root))
}
