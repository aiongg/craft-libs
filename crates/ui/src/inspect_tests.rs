use egui::accesskit::Role;
use egui::{Context, RawInput, Ui, vec2};

use super::*;
use crate::primitives::{Gap, Handle, HandleShape, Interactive, Pad, Row, Stack, Surface, SurfaceKind};
use crate::theme::{Base, Tokens, TokensOf, install};

/// One of each recorded primitive; the stack and the row are padded.
fn primitives(ui: &mut Ui) {
    install(ui.ctx(), &Base::suite_default());
    Surface::new(SurfaceKind::Raised).show(ui, |ui| {
        Stack::new(Gap::Stack).pad(Pad::Panel).show(ui, |ui| {
            Row::new(Gap::Inline).pad(Pad::Control).show(ui, |ui| {
                Interactive::new("item", Role::Button, "Item").show(ui, vec2(40.0, 20.0));
            });
            let centre = ui.cursor().min + vec2(20.0, 20.0);
            Handle::new(HandleShape::Square, "Corner").show(ui, centre);
            ui.add_space(40.0);
        });
    });
}

fn frame(ctx: &Context) {
    ctx.run_ui(RawInput::default(), primitives).drop_without_applying_deltas();
}

/// Runs a frame and returns whether its output carried an AccessKit update.
fn accesskit_on(ctx: &Context) -> bool {
    let output = ctx.run_ui(RawInput::default(), primitives);
    let on = output.platform_output.accesskit_update.is_some();
    output.drop_without_applying_deltas();
    on
}

#[test]
fn nothing_is_recorded_while_inspection_is_off() {
    let ctx = Context::default();
    for _ in 0..2 {
        assert!(!accesskit_on(&ctx), "AccessKit stays off");
    }
    assert!(!enabled(&ctx));
    assert!(records(&ctx).is_empty());
    assert!(tree(&ctx).is_none());
}

#[test]
fn each_primitive_records_its_box_with_the_content_inside() {
    let ctx = Context::default();
    set_enabled(&ctx, true);
    let mut pads = (0.0, 0.0);
    for _ in 0..2 {
        ctx.run_ui(RawInput::default(), |ui| {
            primitives(ui);
            let tokens = Tokens::of(ui);
            pads = (Pad::Panel.points(&tokens), Pad::Control.points(&tokens));
        })
        .drop_without_applying_deltas();
    }
    let records = records(&ctx);
    let of = |kind: RecordKind| records.iter().find(|r| r.kind == kind).copied().unwrap_or_else(|| panic!("no {kind:?} record in {records:?}"));
    let surface = of(RecordKind::Surface(SurfaceKind::Raised));
    let stack = of(RecordKind::Stack);
    let row = of(RecordKind::Row);
    let item = of(RecordKind::Interactive(Role::Button));
    let handle = of(RecordKind::Handle);
    for record in &records {
        assert!(record.outer.contains_rect(record.content), "{record:?}");
    }
    assert_eq!(surface.outer, surface.content, "a surface has no padding");
    assert_eq!(surface.outer, stack.outer, "the stack fills the surface");
    assert_eq!(stack.content, stack.outer.shrink(pads.0.round()));
    assert_eq!(row.content, row.outer.shrink(pads.1.round()));
    assert!(stack.content.contains_rect(row.outer));
    assert_eq!(item.outer, item.content);
    assert_eq!(item.outer.size(), vec2(40.0, 20.0));
    assert!(row.content.contains_rect(item.outer));
    assert!(handle.outer.width() > handle.content.width(), "the hit square is larger than the body");
    assert_eq!(handle.outer.center(), handle.content.center());
}

#[test]
fn accesskit_is_on_only_while_inspecting_unless_it_was_on_before() {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    frame(&ctx);
    set_enabled(&ctx, true);
    assert!(accesskit_on(&ctx));
    assert!(tree(&ctx).is_some());
    set_enabled(&ctx, false);
    assert!(tree(&ctx).is_none() && records(&ctx).is_empty());
    assert!(!accesskit_on(&ctx), "inspection turned AccessKit off again");

    // A platform (screen reader, test harness) had it on: it stays on.
    ctx.enable_accesskit();
    frame(&ctx);
    set_enabled(&ctx, true);
    frame(&ctx);
    set_enabled(&ctx, false);
    assert!(accesskit_on(&ctx), "AccessKit stays on for the platform");
}

#[test]
fn the_tree_gives_bounds_interactive_nodes_and_labels() {
    let ctx = Context::default();
    set_enabled(&ctx, true);
    for _ in 0..2 {
        frame(&ctx);
    }
    let tree = tree(&ctx).expect("a tree after two passes");
    let record = records(&ctx).into_iter().find(|r| matches!(r.kind, RecordKind::Interactive(_))).expect("record");
    let item = tree.of(record.id).expect("the item has a node");
    let node = tree.node(item).expect("node");
    assert!(is_interactive(node));
    assert_eq!(tree.name(item).as_deref(), Some("Item"));
    assert_eq!(tree.bounds(item), Some(record.outer));
    let interactive: Vec<_> = tree.interactive(tree.root()).into_iter().filter_map(|id| tree.node(id)?.label().map(str::to_owned)).collect();
    assert_eq!(interactive, ["Item", "Corner"], "the handle takes clicks but not focus");
    assert_eq!(tree.focus_order(tree.root()), [item]);
    assert_eq!(tree.walk(tree.root()).first(), Some(&(tree.root(), 0)));
}

#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
mod tab {
    use egui::Ui;
    use egui::accesskit::Role;
    use egui::vec2;
    use egui_kittest::Harness;

    use crate::primitives::{Gap, Interactive, Row};
    use crate::testing::a11y;
    use crate::theme::{Base, install};

    fn harness(show: impl FnMut(&mut Ui) + 'static) -> Harness<'static> {
        Harness::builder()
            .with_size([640.0, 240.0])
            .with_context_setup(|ctx| {
                install(ctx, &Base::suite_default());
                super::set_enabled(ctx, true);
            })
            .build_ui(show)
    }

    /// The labels of the inspector's focus order, then of the Tab key's.
    fn orders(harness: &mut Harness<'_>) -> (Vec<String>, Vec<String>) {
        harness.run();
        let tree = super::tree(&harness.ctx).expect("a tree while inspecting");
        let inspected =
            tree.focus_order(tree.root()).into_iter().map(|id| tree.node(id).and_then(|n| n.label()).unwrap_or_default().to_owned()).collect();
        let tabbed = a11y::tab_order(harness).into_iter().map(|stop| stop.label).collect();
        (inspected, tabbed)
    }

    #[test]
    fn the_focus_order_is_the_tab_order_on_the_focus_story() {
        let story = crate::story::registry().into_iter().find(|s| s.id == "primitives/focus-ring/focus").expect("the Focus story");
        let mut args = story.defaults();
        let mut harness = harness(move |ui| (story.show)(ui, &mut args));
        let (inspected, tabbed) = orders(&mut harness);
        assert_eq!(inspected, ["Item 1", "Item 2", "Item 3", "Item 4"]);
        assert_eq!(inspected, tabbed);
    }

    #[test]
    fn the_focus_order_is_the_tab_order_across_nested_and_disabled_widgets() {
        let mut harness = harness(|ui| {
            let mut on = false;
            Row::new(Gap::Inline).show(ui, |ui| {
                let _ = ui.button("First");
                ui.add_enabled_ui(false, |ui| ui.button("Disabled"));
                ui.vertical(|ui| {
                    ui.checkbox(&mut on, "Nested");
                    Interactive::new("inner", Role::Tab, "Inner").show(ui, vec2(40.0, 20.0));
                });
            });
            ui.horizontal(|ui| {
                let _ = ui.button("Last");
            });
        });
        let (inspected, tabbed) = orders(&mut harness);
        assert_eq!(inspected, ["First", "Nested", "Inner", "Last"]);
        assert_eq!(inspected, tabbed);
    }
}
