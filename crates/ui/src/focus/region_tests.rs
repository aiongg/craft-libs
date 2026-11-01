//! F6 region focus.

use egui::{Context, Event, Id, PointerButton, Popup, RawInput, Rect, pos2};

use super::{Region, RegionFocus};

const REGIONS: [(&str, f32); 4] = [("tools", 0.0), ("options", 100.0), ("document", 200.0), ("panels", 300.0)];

fn rect_of(x: f32) -> Rect {
    Rect::from_min_size(pos2(x, 0.0), egui::vec2(100.0, 100.0))
}

/// One frame: `act` runs first (where the host runs its commands), then `shown` regions register.
fn frame(ctx: &Context, shown: &[&str], events: Vec<Event>, act: impl FnOnce(&RegionFocus, &Context)) {
    let raw = RawInput { events, ..Default::default() };
    let mut act = Some(act);
    let mut out = ctx.run_ui(raw, |ui| {
        let ctx = ui.ctx();
        let focus = RegionFocus::new(ctx);
        (act.take().expect("once"))(&focus, ctx);
        for (id, x) in REGIONS.into_iter().filter(|(id, _)| shown.contains(id)) {
            focus.region(ctx, Region::new(id, id), rect_of(x), Some(Id::unique(("entry", id))));
        }
    });
    out.textures_delta.clear();
}

fn focused(ctx: &Context) -> Option<Id> {
    ctx.memory(|m| m.focused())
}

#[test]
fn f6_cycles_registered_regions_and_wraps() {
    let ctx = Context::default();
    let all = ["tools", "options", "document", "panels"];
    frame(&ctx, &all, vec![], |_, _| {});
    let mut visited = vec![];
    for _ in 0..5 {
        frame(&ctx, &all, vec![], |f, ctx| f.step(ctx, 1));
        visited.push(focused(&ctx));
    }
    let entry = |id: &str| Some(Id::unique(("entry", id)));
    assert_eq!(visited, [entry("tools"), entry("options"), entry("document"), entry("panels"), entry("tools")]);
    frame(&ctx, &all, vec![], |f, ctx| f.step(ctx, -1));
    assert_eq!(focused(&ctx), entry("panels"));

    // A region that stopped registering is skipped.
    let without_document = ["tools", "options", "panels"];
    frame(&ctx, &without_document, vec![], |_, _| {});
    frame(&ctx, &without_document, vec![], |f, ctx| f.step(ctx, 1));
    assert_eq!(focused(&ctx), entry("tools"));
    frame(&ctx, &without_document, vec![], |f, ctx| f.step(ctx, 1));
    frame(&ctx, &without_document, vec![], |f, ctx| f.step(ctx, 1));
    assert_eq!(RegionFocus::new(&ctx).current(&ctx).as_deref(), Some("panels"));
}

#[test]
fn pointer_press_sets_the_region_without_a_ring() {
    let ctx = Context::default();
    let all = ["tools", "options", "document", "panels"];
    frame(&ctx, &all, vec![], |_, _| {});
    frame(&ctx, &all, vec![], |f, ctx| f.step(ctx, 1));
    assert!(RegionFocus::new(&ctx).ring(&ctx).is_some());

    let press = |pressed| Event::PointerButton { pos: pos2(250.0, 50.0), button: PointerButton::Primary, pressed, modifiers: Default::default() };
    frame(&ctx, &all, vec![Event::PointerMoved(pos2(250.0, 50.0)), press(true)], |_, _| {});
    let focus = RegionFocus::new(&ctx);
    assert_eq!(focus.current(&ctx).as_deref(), Some("document"));
    assert!(focus.ring(&ctx).is_none());

    frame(&ctx, &all, vec![press(false)], |f, ctx| f.step(ctx, 1));
    let (rect, label) = RegionFocus::new(&ctx).ring(&ctx).expect("ring after F6");
    assert_eq!((rect, label.as_str()), (rect_of(300.0), "panels"));
}

#[test]
fn f6_does_nothing_under_a_modal_layer() {
    let ctx = Context::default();
    let all = ["tools", "options", "document", "panels"];
    frame(&ctx, &all, vec![], |_, _| {});
    frame(&ctx, &all, vec![], |f, ctx| f.step(ctx, 1));
    frame(&ctx, &all, vec![], |f, ctx| {
        Popup::open_id(ctx, Id::unique("popup"));
        f.step(ctx, 1);
    });
    assert_eq!(RegionFocus::new(&ctx).current(&ctx).as_deref(), Some("tools"));
}

// Entering a region without an entry widget focuses the first widget in layer order (back to
// front), not whichever layer the widget map yields first.
#[test]
fn a_region_without_an_entry_focuses_the_back_most_layer_first() {
    let ctx = Context::default();
    let mut first = None;
    let mut run = |events: Vec<Event>, act: &dyn Fn(&RegionFocus, &Context)| {
        let raw = RawInput { events, ..Default::default() };
        let mut out = ctx.run_ui(raw, |ui| {
            let ctx = ui.ctx();
            let focus = RegionFocus::new(ctx);
            act(&focus, ctx);
            for layer in 0..8 {
                egui::Area::new(Id::unique(("layer", layer))).fixed_pos(pos2(10.0, 10.0)).show(ctx, |ui| {
                    let response = ui.button("Go");
                    if layer == 0 {
                        first = Some(response.id);
                    }
                });
            }
            focus.region(ctx, Region::new("area", "Area"), rect_of(0.0), None);
        });
        out.textures_delta.clear();
    };
    run(vec![], &|_, _| {});
    run(vec![], &|_, _| {});
    run(vec![], &|f, ctx| f.step(ctx, 1));
    assert_eq!(focused(&ctx), first);
}
