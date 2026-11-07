//! Raw egui scenarios the `egui_surface` trials read: each runs a few frames on a bare context and returns what
//! egui did, so the trials pin the egui behaviours the runtime relies on without calling egui themselves.

use super::fixture;
use egui::{Area, Context, Event, Id, LayerId, Order, Pos2, RawInput, Rect, Sense, Ui, Vec2, ViewportBuilder, ViewportId};
use std::sync::{Arc, Mutex};

fn raw(time: f64, events: Vec<Event>) -> RawInput {
    RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))), time: Some(time), events, ..RawInput::default() }
}

/// What the plugin hooks of one context saw around a nested immediate viewport.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct HookFacts {
    /// Input hooks that ran for the child viewport's raw input.
    pub child_input_hooks: usize,
    /// Every one of them ran while the context's current viewport was still the parent.
    pub input_hooks_see_parent: bool,
    /// Begin-pass hooks that ran with the child as the current viewport.
    pub child_begin_hooks: usize,
}

enum Entry {
    Input { raw: ViewportId, current: ViewportId },
    Begin { current: ViewportId },
}

struct Recorder(Arc<Mutex<Vec<Entry>>>);

impl egui::Plugin for Recorder {
    fn debug_name(&self) -> &'static str {
        "craft_ui.surface.recorder"
    }

    fn input_hook(&mut self, ctx: &Context, input: &mut RawInput) {
        if let Ok(mut log) = self.0.lock() {
            log.push(Entry::Input { raw: input.viewport_id, current: ctx.viewport_id() });
        }
    }

    fn on_begin_pass(&mut self, ui: &mut Ui) {
        if let Ok(mut log) = self.0.lock() {
            log.push(Entry::Begin { current: ui.ctx().viewport_id() });
        }
    }
}

/// A renderer like eframe's: the child's pass runs nested inside the parent's `Ui` closure.
fn render_child(ctx: &Context, viewport: egui::ImmediateViewport<'_>) {
    let mut show = viewport.viewport_ui_cb;
    let mut input = raw(0.0, Vec::new());
    input.viewport_id = viewport.ids.this;
    input.viewports.insert(viewport.ids.this, egui::ViewportInfo { parent: Some(viewport.ids.parent), ..Default::default() });
    let mut output = ctx.run_ui(input, |ui| show(ui));
    output.textures_delta.clear();
}

/// Two frames of a root viewport that shows an immediate child viewport, with a recording plugin on the context.
pub fn child_viewport_hooks() -> HookFacts {
    let ctx = Context::default();
    ctx.set_embed_viewports(false);
    let log = Arc::new(Mutex::new(Vec::new()));
    ctx.add_plugin(Recorder(log.clone()));
    Context::set_immediate_viewport_renderer(render_child);
    let child = ViewportId::from_hash_of("craft_ui.surface.child");
    for frame in 0..2 {
        let mut output = ctx.run_ui(raw(f64::from(frame), Vec::new()), |ui| {
            ui.ctx().show_viewport_immediate(child, ViewportBuilder::default(), |_, _| {});
        });
        output.textures_delta.clear();
    }
    let log = log.lock().map(|log| log.iter().map(|e| match e {
        Entry::Input { raw, current } => (Some((*raw, *current)), None),
        Entry::Begin { current } => (None, Some(*current)),
    }).collect::<Vec<_>>()).unwrap_or_default();
    let inputs: Vec<(ViewportId, ViewportId)> = log.iter().filter_map(|(input, _)| *input).filter(|(raw, _)| *raw == child).collect();
    HookFacts {
        child_input_hooks: inputs.len(),
        input_hooks_see_parent: inputs.iter().all(|(_, current)| *current == ViewportId::ROOT),
        child_begin_hooks: log.iter().filter_map(|(_, begin)| *begin).filter(|current| *current == child).count(),
    }
}

/// The widget `ViewportState::hits` names for a press, by the order of the frame's pointer events.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PressHits {
    /// Events: pointer moved to A, then press at B. `hits.click` is the widget at B.
    pub press_last_is_press_target: bool,
    /// Events: press at B, then pointer moved to A. `hits.click` is the widget at A, where the pointer ended.
    pub move_last_is_move_target: bool,
}

/// Three frames over two click widgets: one to register them, then the two event orders.
pub fn press_hits() -> PressHits {
    let ctx = fixture::bare_context();
    let (a, b) = (Id::new("craft_ui.surface.a"), Id::new("craft_ui.surface.b"));
    let rect_a = Rect::from_min_size(Pos2::new(10.0, 10.0), Vec2::splat(40.0));
    let rect_b = Rect::from_min_size(Pos2::new(100.0, 10.0), Vec2::splat(40.0));
    let mut show = |ui: &mut Ui| {
        ui.interact(rect_a, a, Sense::click());
        ui.interact(rect_b, b, Sense::click());
    };
    let mut run = |time: f64, events: Vec<Event>| {
        let mut output = ctx.run_ui(raw(time, events), &mut show);
        output.textures_delta.clear();
        ctx.viewport(|viewport| viewport.hits.click.map(|widget| widget.id))
    };
    run(1.0, Vec::new());
    let first = run(2.0, vec![Event::PointerMoved(rect_a.center()), fixture::primary_button(rect_b.center(), true)]);
    let second = run(3.0, vec![fixture::primary_button(rect_b.center(), true), Event::PointerMoved(rect_a.center())]);
    PressHits { press_last_is_press_target: first == Some(b), move_last_is_move_target: second == Some(a) }
}

/// The layer order after `Context::set_layer_order`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LayerOrder {
    /// The listed layer is the topmost of the layers shown.
    pub listed_on_top: bool,
    /// The unlisted layers keep their relative order.
    pub unlisted_keep_order: bool,
    /// A listed id that is no layer does not become one.
    pub unknown_ignored: bool,
}

/// Two frames over three areas; the second lists the first area and an unknown layer.
pub fn layer_order() -> LayerOrder {
    let ctx = fixture::bare_context();
    let ids = [Id::new("craft_ui.surface.la"), Id::new("craft_ui.surface.lb"), Id::new("craft_ui.surface.lc")];
    let layers = ids.map(|id| LayerId::new(Order::Middle, id));
    let ghost = LayerId::new(Order::Middle, Id::new("craft_ui.surface.ghost"));
    let mut frame = |time: f64, list: &[LayerId]| {
        let mut output = ctx.run_ui(raw(time, Vec::new()), |ui| {
            let ctx = ui.ctx().clone();
            for id in ids {
                Area::new(id).show(&ctx, |ui| {
                    ui.label("layer");
                });
            }
            if !list.is_empty() {
                ctx.set_layer_order(list);
            }
        });
        output.textures_delta.clear();
        ctx.memory(|memory| memory.layer_ids().filter(|layer| layers.contains(layer) || *layer == ghost).collect::<Vec<_>>())
    };
    let before = frame(1.0, &[]);
    let after = frame(2.0, &[layers[0], ghost]);
    let rest = |order: &[LayerId]| order.iter().copied().filter(|layer| *layer != layers[0]).collect::<Vec<_>>();
    LayerOrder {
        listed_on_top: after.last() == Some(&layers[0]),
        unlisted_keep_order: rest(&before) == rest(&after),
        unknown_ignored: !after.contains(&ghost),
    }
}

/// Whether a member that registered no egui widget, given `register_accesskit_parent`, has its node among the
/// children of its parent's node.
pub fn registered_parent_owns_member_node() -> bool {
    let ctx = fixture::bare_context();
    let (parent, member) = (Id::new("craft_ui.surface.parent"), Id::new("craft_ui.surface.member"));
    let rect = Rect::from_min_size(Pos2::new(10.0, 10.0), Vec2::splat(40.0));
    let mut output = ctx.run_ui(raw(1.0, Vec::new()), |ui| {
        let ctx = ui.ctx().clone();
        ui.interact(rect, parent, Sense::click());
        ctx.accesskit_node_builder(parent, |_| ());
        ctx.register_accesskit_parent(member, parent);
        ctx.accesskit_node_builder(member, |_| ());
    });
    output.textures_delta.clear();
    let Some(update) = output.platform_output.accesskit_update else { return false };
    update.nodes.iter().find(|(id, _)| *id == parent.accesskit_id()).is_some_and(|(_, node)| node.children().contains(&member.accesskit_id()))
}

/// What a pass saw of one frame that discarded its first pass.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PassFacts {
    /// Input events the pass received.
    pub events: usize,
    /// The cumulative frame number during the pass.
    pub frame: u64,
    /// The cumulative pass number during the pass.
    pub pass: u64,
}

/// One frame with a key event whose first pass asks for a discard.
pub fn discard_passes() -> Vec<PassFacts> {
    let ctx = fixture::bare_context();
    let mut seen = Vec::new();
    let event = fixture::key_press(egui::Key::A, egui::Modifiers::NONE);
    let mut output = ctx.run_ui(raw(1.0, vec![event]), |ui| {
        seen.push(PassFacts { events: ui.input(|input| input.events.len()), frame: ui.ctx().cumulative_frame_nr(), pass: ui.ctx().cumulative_pass_nr() });
        if seen.len() == 1 {
            ui.ctx().request_discard("surface test");
        }
    });
    output.textures_delta.clear();
    seen
}
