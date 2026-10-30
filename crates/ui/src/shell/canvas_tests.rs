use egui::{Color32, Context, Pos2, RawInput, Rect, Ui, vec2};

use super::*;
use crate::direction::{self, Direction};
use crate::theme::{Base, Tokens, TokensOf as _, install};

const W: f32 = 800.0;
const H: f32 = 600.0;
const RAIL: f32 = 36.0;

#[derive(Clone)]
struct Cfg {
    rail: bool,
    rail_visible: bool,
    tabs: bool,
    opening: bool,
    mode: BodyMode,
}

impl Cfg {
    fn new() -> Self {
        Self { rail: true, rail_visible: true, tabs: true, opening: false, mode: BodyMode::Normal }
    }
}

struct Frame {
    shell: ShellOutput<CanvasOutput<()>>,
    tab_height: f32,
}

fn stub(color: Color32) -> impl FnOnce(&mut Ui) {
    move |ui| {
        ui.painter().rect_filled(ui.max_rect(), 0, color);
    }
}

fn run(ctx: &Context, cfg: &Cfg) -> Frame {
    let mut out = None;
    let mut tab_height = 0.0;
    let raw = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(W, H))), ..Default::default() };
    let mut full = ctx.run_ui(raw, |ui| {
        tab_height = Tokens::of(ui).size.bar.tab;
        let cfg = cfg.clone();
        out = Some(
            CanvasShell::new(WindowKey::main())
                .title(true, stub(Color32::RED))
                .options(true, stub(Color32::GREEN))
                .tools(true, stub(Color32::BROWN))
                .status(true, stub(Color32::BLUE))
                .body_mode(cfg.mode)
                .document_tabs(cfg.tabs)
                .show(ui, move |ui, cx| {
                    let rail = cfg.rail.then(|| PanelRail::new(RAIL, stub(Color32::PURPLE)).visible(cfg.rail_visible).label("Panels"));
                    cx.canvas_panel_with(ui, PanelOptions { rail, opening: cfg.opening }, stub(Color32::YELLOW), |_| ())
                }),
        );
    });
    full.textures_delta.clear();
    Frame { shell: out.expect("ran"), tab_height }
}

fn ctx() -> Context {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    ctx
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.5
}

fn slot(f: &Frame, key: &str) -> Rect {
    f.shell.slots.iter().find(|(k, _)| *k == key).unwrap_or_else(|| panic!("{key} not shown: {:?}", f.shell.slots)).1
}

#[test]
fn the_preset_keys_and_order() {
    let f = run(&ctx(), &Cfg::new());
    let keys: Vec<_> = f.shell.slots.iter().map(|(k, _)| *k).collect();
    assert_eq!(keys, ["title", "options", "status", "tools"]);
    let (title, options, status, tools) = (slot(&f, "title"), slot(&f, "options"), slot(&f, "status"), slot(&f, "tools"));
    assert!(near(options.top(), title.bottom()) && near(status.bottom(), H));
    assert!(near(tools.top(), options.bottom()) && near(tools.bottom(), status.top()));
    assert!(near(f.shell.body.left(), tools.right()));
}

#[test]
fn the_panel_rail_is_at_the_end_edge_and_spans_tabs_and_document() {
    let f = run(&ctx(), &Cfg::new());
    let (body, canvas) = (f.shell.body, f.shell.inner.clone());
    let (rail, tabs) = (canvas.rail.expect("rail"), canvas.tabs.expect("tabs"));
    assert!(near(rail.right(), body.right()) && near(rail.width(), RAIL));
    assert!(near(rail.top(), body.top()) && near(rail.bottom(), body.bottom()));
    assert!(near(tabs.top(), body.top()) && near(tabs.right(), rail.left()) && near(tabs.left(), body.left()));
    assert!(near(tabs.height(), f.tab_height));
    assert!(near(canvas.document.top(), tabs.bottom()) && near(canvas.document.right(), rail.left()));
    assert!(near(canvas.document.bottom(), body.bottom()) && near(canvas.document.left(), body.left()));
}

#[test]
fn without_a_panel_rail_the_tabs_and_document_fill_the_width() {
    for cfg in [Cfg { rail: false, ..Cfg::new() }, Cfg { rail_visible: false, ..Cfg::new() }] {
        let f = run(&ctx(), &cfg);
        let canvas = f.shell.inner;
        assert!(canvas.rail.is_none());
        let tabs = canvas.tabs.expect("tabs");
        assert!(near(tabs.width(), f.shell.body.width()) && near(canvas.document.width(), f.shell.body.width()));
    }
}

#[test]
fn rtl_mirrors_the_rail_and_the_tool_rail() {
    let ctx = ctx();
    direction::apply(&ctx, Direction::Rtl);
    let f = run(&ctx, &Cfg::new());
    let (body, canvas) = (f.shell.body, f.shell.inner.clone());
    let rail = canvas.rail.expect("rail");
    assert!(near(rail.left(), body.left()) && near(canvas.document.left(), rail.right()));
    assert!(slot(&f, "tools").left() > W / 2.0);
}

#[test]
fn tabs_hide_when_the_host_says_so_unless_a_file_is_opening() {
    let ctx = ctx();
    let f = run(&ctx, &Cfg { tabs: false, ..Cfg::new() });
    assert!(f.shell.inner.tabs.is_none());
    assert!(near(f.shell.inner.document.top(), f.shell.body.top()));
    let f = run(&ctx, &Cfg { tabs: false, opening: true, ..Cfg::new() });
    assert!(f.shell.inner.tabs.is_some());
}

#[test]
fn body_only_shows_neither_tabs_nor_rail() {
    let f = run(&ctx(), &Cfg { mode: BodyMode::Only, ..Cfg::new() });
    assert!(f.shell.slots.is_empty());
    assert!(f.shell.inner.rail.is_none() && f.shell.inner.tabs.is_none());
}

#[test]
fn the_reveal_shows_title_options_and_tools_but_not_status() {
    let ctx = ctx();
    let cfg = Cfg { mode: BodyMode::Only, ..Cfg::new() };
    run(&ctx, &cfg);
    assert!(AppShell::toggle_reveal(&ctx, &WindowKey::main()));
    let f = run(&ctx, &cfg);
    let keys: Vec<_> = f.shell.slots.iter().map(|(k, _)| *k).collect();
    assert_eq!(keys, ["title", "options", "tools"]);
}
