//! One test per rule class, an idempotence test and the do-not-touch list.

#![allow(clippy::unwrap_used, reason = "tests")]

use codemod_v4::{Options, Output, transform};

fn run(src: &str) -> Output {
    transform(src, &Options::default()).unwrap()
}

fn text(src: &str) -> String {
    run(src).text
}

fn skipped(out: &Output, class: &str) -> usize {
    out.skips.iter().filter(|s| s.class == class).count()
}

#[test]
fn use_lines_and_qualified_paths_move_to_craft_ui() {
    let out = text(
        "use egui::{Context, Foo, Id, Rect, Response, Ui};\nfn f(ui: &mut egui::Ui, r: egui::Response, c: &egui::Context) -> egui::Id { todo!() }\n",
    );
    assert_eq!(
        out,
        "use egui::Foo;\nuse craft_ui::{Context, Id, Response, Ui};\nuse craft_ui::geom::Rect;\nfn f(ui: &mut craft_ui::Ui, r: craft_ui::Response, c: &craft_ui::Context) -> craft_ui::Id { todo!() }\n"
    );
}

#[test]
fn geometry_colour_input_and_window_types() {
    let out = text(
        "use egui::{Key, Modifiers, PointerButton, ViewportCommand, ViewportId};\nfn f(c: &Ctx) {\n    let m = Modifiers::COMMAND | Modifiers::SHIFT;\n    let k = egui::Key::A;\n    let v = egui::Vec2::ZERO;\n    let id = ViewportId::ROOT;\n    c.send(ViewportCommand::InnerSize(s));\n    c.send(egui::ViewportCommand::StartDrag);\n    let col = egui::Color32::RED;\n}\n",
    );
    assert_eq!(
        out,
        "use craft_ui::input::{Key, Mods, PointerButton};\nuse craft_ui::{WindowCommand, WindowId};\nfn f(c: &Ctx) {\n    let m = Mods::COMMAND | Mods::SHIFT;\n    let k = craft_ui::input::Key::A;\n    let v = craft_ui::geom::Vec2::ZERO;\n    let id = WindowId::ROOT;\n    c.send(WindowCommand::Size(s));\n    c.send(craft_ui::WindowCommand::DragMove);\n    let col = craft_ui::Color32::RED;\n}\n"
    );
}

#[test]
fn ui_builder_chains_move_unless_they_carry_a_layout() {
    let plain = text("fn f() { let b = egui::UiBuilder::new().max_rect(r).id_salt(s); }\n");
    assert_eq!(plain, "fn f() { let b = craft_ui::UiBuilder::new().max_rect(r).id_salt(s); }\n");
    let laid = run("fn f() { let b = egui::UiBuilder::new().max_rect(r).layout(l); }\n");
    assert!(laid.text.contains("egui::UiBuilder"));
    assert_eq!(skipped(&laid, "uibuilder-layout"), 1);
}

#[test]
fn ui_methods_are_renamed_on_ui_receivers_only() {
    let out = run(
        "fn f(ui: &mut Ui, other: &X) {\n    if ui.is_sizing_pass() { return; }\n    let p = ui.painter();\n    let q = ui.painter_at(r);\n    let (id, rect) = ui.allocate_space(s);\n    let w = ui.ctx().viewport_id();\n    other.painter();\n}\n",
    );
    assert_eq!(
        out.text,
        "fn f(ui: &mut Ui, other: &X) {\n    if ui.is_measuring() { return; }\n    let p = ui.draw();\n    let q = ui.draw_at(r);\n    let (id, rect) = ui.allocate(s);\n    let w = ui.ctx().window();\n    other.painter();\n}\n"
    );
    assert_eq!(skipped(&out, "ui-method-other-receiver"), 1);
}

#[test]
fn service_calls_become_methods() {
    let out = run(
        "fn f(ui: &mut Ui, cx: &mut Cx<C>, ctx: &Context) {\n    frame::request_repaint(ui.ctx());\n    frame::request_repaint_after(ctx, 0.5);\n    let a = measure::active(ui.ctx());\n    let b = measure::active(&cx.ctx);\n    let w = pass::current_viewport(ctx);\n    focus::request(ui.ctx(), target);\n    overlay::raise(ui.ctx(), id);\n    let s = StyleCx::of_env(ui.ctx(), env);\n    let t = frame::clock(&cx.ctx);\n    toast::post(ui.ctx(), x);\n    overlay::raise(ctx, id);\n}\n",
    );
    assert_eq!(
        out.text,
        "fn f(ui: &mut Ui, cx: &mut Cx<C>, ctx: &Context) {\n    ui.ctx().request_repaint();\n    ctx.request_repaint_after(std::time::Duration::from_secs_f64(0.5));\n    let a = ui.is_measuring();\n    let b = cx.is_measuring();\n    let w = ctx.window();\n    ui.request_focus(target);\n    ui.raise_layer(id);\n    let s = StyleCx::of_env(env);\n    let t = cx.context().time();\n    toast::post(ui.ctx(), x);\n    overlay::raise(ctx, id);\n}\n"
    );
    assert_eq!(skipped(&out, "service-hand-written"), 1);
    assert_eq!(skipped(&out, "service-receiver-unknown"), 1);
}

#[test]
fn cx_ctx_passed_on_becomes_context() {
    let out = run(
        "fn f(cx: &mut Cx<C>, response: &R) {\n    g(&cx.ctx);\n    let c = cx.ctx.clone();\n    let d = response.ctx;\n    cx.ctx.input(|i| 1);\n}\n",
    );
    assert_eq!(
        out.text,
        "fn f(cx: &mut Cx<C>, response: &R) {\n    g(cx.context());\n    let c = cx.context().clone();\n    let d = response.ctx;\n    cx.ctx.input(|i| 1);\n}\n"
    );
    assert_eq!(skipped(&out, "ctx-field-other-receiver"), 1);
    assert_eq!(skipped(&out, "cx-ctx-other"), 1);
}

#[test]
fn fixture_becomes_harness() {
    let out = run(
        "use craft_ui::testing::{Fixture, Other};\nfn t() {\n    let mut fx = Fixture::new(show);\n    fx.frame(vec![]);\n    fx.frame(Vec::new());\n    fx.frame(vec![press(k)]);\n    fx.frame(vec![a, b]);\n    fx.click(a);\n    fx.take_paint();\n    fx.frame_shapes_with(go);\n}\nfn h(fx: &mut Fixture) { fx.frame(vec![]); }\n",
    );
    assert_eq!(
        out.text,
        "use craft_ui::testing::{Harness, Other};\nfn t() {\n    let mut fx = Harness::new_ui(show);\n    fx.step();\n    fx.step();\n    fx.send(press(k));\n    fx.frame(vec![a, b]);\n    fx.click(a);\n    fx.take_paint();\n    fx.frame_shapes_with(go);\n}\nfn h(fx: &mut Harness) { fx.step(); }\n"
    );
    assert_eq!(skipped(&out, "fixture-no-twin"), 1);
    assert_eq!(skipped(&out, "fixture-frame-args"), 1);
}

#[test]
fn contract_crate_targets_stay_inside_the_crate() {
    let out = transform("use egui::{Rect, Ui};\nfn f(ui: &mut egui::Ui) {}\n", &Options { inside_contract: true }).unwrap();
    assert_eq!(out.text, "use egui::Rect;\nuse crate::contract::Ui;\nfn f(ui: &mut crate::contract::Ui) {}\n");
}

#[test]
fn the_do_not_touch_list_is_reported_and_unchanged() {
    let src = "use egui::{ColorImage, Shape, TextureHandle};\nstruct S<'a> { ui: &'a mut egui::Ui }\nfn f(ui: &mut Ui, ctx: &Context) {\n    ui.label(\"a\");\n    ui.horizontal(|ui| {});\n    ctx.input(|i| i.time);\n    let _ = egui::Align2::CENTER;\n    r.hover_pos();\n    ctx.send_viewport_cmd_to(id, cmd);\n}\n";
    let out = run(src);
    assert_eq!(out.text, src);
    for class in ["textures", "raw-egui-paint", "ui-struct-field", "egui-layout-widgets", "ctx-input-read", "response-methods", "viewport-other"] {
        assert!(skipped(&out, class) > 0, "{class}: {:?}", out.skips);
    }
}

#[test]
fn a_second_run_changes_nothing() {
    let src = "use egui::{Context, Foo, Modifiers, Rect, Ui, ViewportCommand};\nuse craft_ui::testing::Fixture;\nfn f(ui: &mut Ui, cx: &mut Cx<C>, ctx: &Context) {\n    // kept\n    let p = ui.painter();\n    frame::request_repaint(&cx.ctx);\n    measure::active(ui.ctx());\n    let m = Modifiers::NONE;\n    ctx.send(ViewportCommand::Close);\n    let mut fx = Fixture::new(show);\n    fx.frame(vec![]);\n}\n";
    let once = run(src);
    assert_ne!(once.text, src);
    assert!(once.text.contains("// kept"));
    let twice = run(&once.text);
    assert_eq!(twice.text, once.text);
    assert!(twice.counts.is_empty());
}
