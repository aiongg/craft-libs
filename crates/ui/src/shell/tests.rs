use std::cell::RefCell;

use craft_theme::{AppDefaults, ModeChoice, Size, SuiteChoice, SystemAppearance, ThemePair, builtins};
use egui::{Color32, Context, Event, FullOutput, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, Shape, Ui, pos2, vec2};

use super::*;
use crate::direction::{self, Direction};
use crate::theme::{Base, ToEgui as _, Tokens, TokensOf as _, install};

const W: f32 = 800.0;
const H: f32 = 600.0;

/// Which slots the host has on, and the shell's mode.
#[derive(Clone)]
struct Cfg {
    key: &'static str,
    mode: BodyMode,
    allow_reveal: bool,
    options: bool,
    status: bool,
    tools: bool,
    /// Slots take the token sizes, not the fixed ones below.
    token_sizes: bool,
}

impl Cfg {
    fn new() -> Self {
        Self { key: "main", mode: BodyMode::Normal, allow_reveal: true, options: true, status: true, tools: true, token_sizes: false }
    }

    fn only(mut self) -> Self {
        self.mode = BodyMode::Only;
        self
    }
}

type Calls = RefCell<Vec<&'static str>>;

fn stub<'a>(calls: &'a Calls, key: &'static str, color: Color32) -> impl FnOnce(&mut Ui) + 'a {
    move |ui| {
        calls.borrow_mut().push(key);
        ui.painter().rect_filled(ui.max_rect(), 0, color);
    }
}

/// The skeleton of a Canvas-like window with stub slots: title, options, a second bottom bar
/// ("hint"), status, tools at the start and "panels" at the end.
fn shell<'a>(cfg: &Cfg, calls: &'a Calls) -> AppShell<'a> {
    let sized = |slot: Slot, size: f32| if cfg.token_sizes { slot } else { slot.size(size) };
    AppShell::new(WindowKey::new(cfg.key))
        .title(sized(Slot::new("title"), 32.0).reveal().label("Title"), stub(calls, "title", Color32::RED))
        .top(sized(Slot::new("options"), 30.0).visible(cfg.options).reveal().label("Options"), stub(calls, "options", Color32::GREEN))
        .bottom(sized(Slot::new("status"), 22.0).visible(cfg.status).label("Status"), stub(calls, "status", Color32::BLUE))
        .bottom(sized(Slot::new("hint"), 20.0), stub(calls, "hint", Color32::YELLOW))
        .start(sized(Slot::new("tools"), 40.0).visible(cfg.tools).reveal().label("Tools"), stub(calls, "tools", Color32::BROWN))
        .end(sized(Slot::new("panels"), 50.0).reveal(), stub(calls, "panels", Color32::PURPLE))
        .body_mode(cfg.mode)
        .allow_reveal(cfg.allow_reveal)
}

struct Win {
    ctx: Context,
    time: f64,
    /// The host's `raw_input_hook` toggles the reveal on Tab.
    host_tab: bool,
    toggles: u32,
}

impl Win {
    fn new() -> Self {
        Self::with_base(&Base::suite_default())
    }

    fn with_base(base: &Base) -> Self {
        let win = Self { ctx: Context::default(), time: 0.0, host_tab: false, toggles: 0 };
        install(&win.ctx, base);
        win
    }

    fn frame(&mut self, events: Vec<Event>, mut add: impl FnMut(&mut Ui)) -> FullOutput {
        self.time += 0.1;
        let mut raw =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(W, H))), time: Some(self.time), events, ..Default::default() };
        if self.host_tab && take_tab_toggle(&self.ctx, &mut raw) {
            self.toggles += 1;
            AppShell::toggle_reveal(&self.ctx, &main_key());
        }
        let mut out = self.ctx.run_ui(raw, |ui| add(ui));
        out.textures_delta.clear();
        out
    }

    /// One frame of `cfg`; the output and the keys of the slot closures that ran.
    fn show(&mut self, cfg: &Cfg, events: Vec<Event>) -> (ShellOutput, Vec<&'static str>) {
        let calls = Calls::default();
        let mut shown = None;
        self.frame(events, |ui| shown = Some(shell(cfg, &calls).show(ui, |_, _| ())));
        let out = shown.expect("the shell ran");
        (out, calls.take())
    }

    fn pointer(&mut self, cfg: &Cfg, at: Pos2) -> ShellOutput {
        self.show(cfg, vec![Event::PointerMoved(at)]).0
    }
}

fn rect(out: &ShellOutput, key: &str) -> Rect {
    out.slots.iter().find(|(k, _)| *k == key).unwrap_or_else(|| panic!("slot {key} is not shown: {:?}", out.slots)).1
}

fn keys(out: &ShellOutput) -> Vec<&'static str> {
    out.slots.iter().map(|(k, _)| *k).collect()
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.5
}

/// The relations that make the order, as booleans so a theme loop can compare them.
fn order_holds(out: &ShellOutput, rtl: bool) -> bool {
    let r = |k| rect(out, k);
    let (title, options, status, hint, tools, panels) = (r("title"), r("options"), r("status"), r("hint"), r("tools"), r("panels"));
    near(title.top(), 0.0)
        && near(options.top(), title.bottom())
        && near(status.bottom(), H)
        && near(hint.bottom(), status.top())
        && near(tools.top(), options.bottom())
        && near(tools.bottom(), hint.top())
        && near(panels.top(), options.bottom())
        && near(panels.bottom(), hint.top())
        && near(out.body.top(), options.bottom())
        && near(out.body.bottom(), hint.top())
        && if rtl {
            near(out.body.left(), panels.right()) && near(out.body.right(), tools.left())
        } else {
            near(out.body.left(), tools.right()) && near(out.body.right(), panels.left())
        }
}

#[test]
fn slots_claim_space_in_order_and_the_body_takes_the_rest() {
    let (out, calls) = Win::new().show(&Cfg::new(), vec![]);
    assert_eq!(keys(&out), ["title", "options", "status", "hint", "tools", "panels"]);
    assert_eq!(calls, keys(&out));
    assert!(order_holds(&out, false), "{:?} body {:?}", out.slots, out.body);
    assert_eq!((rect(&out, "tools").width(), rect(&out, "panels").width(), rect(&out, "options").height()), (40.0, 50.0, 30.0));
}

#[test]
fn start_and_end_swap_sides_in_rtl_and_top_and_bottom_do_not() {
    let mut win = Win::new();
    direction::apply(&win.ctx, Direction::Rtl);
    let (out, _) = win.show(&Cfg::new(), vec![]);
    assert!(rect(&out, "tools").left() > W / 2.0 && rect(&out, "panels").right() < W / 2.0);
    assert!(order_holds(&out, true), "{:?} body {:?}", out.slots, out.body);
}

#[test]
fn the_order_is_the_same_in_every_theme_and_size() {
    for theme in builtins() {
        for size in [Size::Compact, Size::Comfortable] {
            let defaults = AppDefaults { theme: ThemePair::same(theme.id.clone()), mode: ModeChoice::System, size };
            let base = Base::resolve(&SuiteChoice::default().with_defaults(&defaults), SystemAppearance::default(), &defaults, None, &mut |id| {
                builtins().iter().find(|t| t.id == *id).cloned()
            });
            let cfg = Cfg { token_sizes: true, ..Cfg::new() };
            let (out, _) = Win::with_base(&base).show(&cfg, vec![]);
            assert_eq!(keys(&out), ["title", "options", "status", "hint", "tools", "panels"], "{:?} {size:?}", theme.id);
            assert!(order_holds(&out, false), "{:?} {size:?}: {:?}", theme.id, out.slots);
        }
    }
}

#[test]
fn token_sizes_come_from_the_size_preset() {
    let mut win = Win::new();
    let mut tokens = None;
    win.frame(vec![], |ui| tokens = Some(Tokens::of(ui)));
    let tokens = tokens.expect("ran");
    let out = win.show(&Cfg { token_sizes: true, ..Cfg::new() }, vec![]).0;
    let heights = (rect(&out, "title").height(), rect(&out, "options").height(), rect(&out, "status").height(), rect(&out, "tools").width());
    assert_eq!(heights, (tokens.size.bar.title, tokens.size.bar.options, tokens.size.bar.status, tokens.size.rail.width));
}

#[test]
fn a_slot_that_is_not_visible_is_not_created_and_the_body_grows() {
    let mut win = Win::new();
    let (all, _) = win.show(&Cfg::new(), vec![]);
    let off = Cfg { options: false, status: false, tools: false, ..Cfg::new() };
    let (out, calls) = win.show(&off, vec![]);
    assert_eq!(keys(&out), ["title", "hint", "panels"]);
    assert_eq!(calls, keys(&out));
    assert!(out.body.height() > all.body.height() && out.body.width() > all.body.width());
    assert!(near(out.body.top(), rect(&out, "title").bottom()));
    assert!(near(out.body.left(), 0.0));
}

#[test]
fn body_only_shows_no_slot_and_paints_the_black_backdrop() {
    let mut win = Win::new();
    let calls = Calls::default();
    let mut seen = None;
    let out = win.frame(vec![], |ui| {
        let out =
            shell(&Cfg::new().only(), &calls).show(ui, |_, cx| (cx.floating_visible(), cx.tabs_visible(), cx.slot_visible("title"), cx.body_mode()));
        seen = Some(out);
    });
    let out_shell = seen.expect("ran");
    assert!(out_shell.slots.is_empty() && calls.take().is_empty() && !out_shell.revealed);
    assert_eq!(out_shell.inner, (false, false, false, BodyMode::Only));
    assert!(near(out_shell.body.width(), W) && near(out_shell.body.height(), H));
    assert!(has_fill(&out, Color32::BLACK));

    let (normal, _) = win.show(&Cfg::new(), vec![]);
    let _ = normal;
    let out = win.frame(vec![], |ui| {
        shell(&Cfg::new(), &calls).show(ui, |_, _| ());
    });
    assert!(!has_fill(&out, Color32::BLACK));
}

fn has_fill(out: &FullOutput, color: Color32) -> bool {
    out.shapes.iter().any(|s| matches!(&s.shape, Shape::Rect(r) if r.fill == color))
}

#[test]
fn the_backdrop_is_the_hosts_pasteboard_or_the_token() {
    let mut win = Win::new();
    let pasteboard = Color32::from_rgb(1, 2, 3);
    let calls = Calls::default();
    let out = win.frame(vec![], |ui| {
        shell(&Cfg::new(), &calls).backdrop(Some(pasteboard)).show(ui, |_, _| ());
    });
    assert!(has_fill(&out, pasteboard));
    let mut tokens = None;
    win.frame(vec![], |ui| tokens = Some(Tokens::of(ui)));
    let token = tokens.expect("ran").component.app_shell.backdrop.to_egui();
    let out = win.frame(vec![], |ui| {
        shell(&Cfg::new(), &calls).show(ui, |_, _| ());
    });
    assert!(has_fill(&out, token));
}

fn main_key() -> WindowKey {
    WindowKey::main()
}

#[test]
fn toggle_reveal_shows_the_reveal_slots_and_never_the_status_bar() {
    let mut win = Win::new();
    let cfg = Cfg::new().only();
    let (out, _) = win.show(&cfg, vec![]);
    assert!(out.slots.is_empty());
    assert!(AppShell::toggle_reveal(&win.ctx, &main_key()));
    let (out, calls) = win.show(&cfg, vec![]);
    assert_eq!(keys(&out), ["title", "options", "tools", "panels"]);
    assert_eq!(calls, keys(&out));
    assert!(out.revealed);
    // The reveal ignores the visible flag.
    let hidden = Cfg { options: false, tools: false, ..cfg.clone() };
    assert_eq!(keys(&win.show(&hidden, vec![]).0), ["title", "options", "tools", "panels"]);
    assert!(!AppShell::toggle_reveal(&win.ctx, &main_key()));
    assert!(win.show(&cfg, vec![]).0.slots.is_empty());
}

#[test]
fn ending_body_only_mode_clears_the_reveal() {
    let mut win = Win::new();
    win.show(&Cfg::new().only(), vec![]);
    assert!(AppShell::toggle_reveal(&win.ctx, &main_key()));
    win.show(&Cfg::new(), vec![]);
    assert!(!AppShell::reveal_active(&win.ctx, &main_key()));
    assert!(win.show(&Cfg::new().only(), vec![]).0.slots.is_empty());
}

#[test]
fn clear_reveal_ends_it_for_a_screen_mode_change() {
    let mut win = Win::new();
    let cfg = Cfg::new().only();
    win.show(&cfg, vec![]);
    AppShell::toggle_reveal(&win.ctx, &main_key());
    AppShell::clear_reveal(&win.ctx, &main_key());
    assert!(win.show(&cfg, vec![]).0.slots.is_empty());
}

#[test]
fn a_presentation_ignores_the_reveal_and_the_edge() {
    let mut win = Win::new();
    let cfg = Cfg { allow_reveal: false, ..Cfg::new().only() };
    win.show(&cfg, vec![]);
    assert!(!AppShell::toggle_reveal(&win.ctx, &main_key()));
    assert!(win.pointer(&cfg, pos2(1.0, 300.0)).slots.is_empty());
    assert!(win.pointer(&cfg, pos2(1.0, 300.0)).slots.is_empty());
}

fn key_event(key: Key, pressed: bool) -> Event {
    Event::Key { key, physical_key: Some(key), pressed, repeat: false, modifiers: Modifiers::NONE }
}

fn click(at: Pos2) -> Vec<Event> {
    let button = |pressed| Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
    vec![Event::PointerMoved(at), button(true), button(false)]
}

#[test]
fn tab_toggles_the_chrome_only_while_the_canvas_has_focus() {
    let mut win = Win::new();
    win.host_tab = true;
    let cfg = Cfg::new().only();
    let mut text = String::new();
    let field = std::cell::Cell::new(Rect::NOTHING);
    // The host: Tab toggles when no control has focus; the body holds a text field.
    let mut run = |win: &mut Win, events: Vec<Event>| -> Vec<&'static str> {
        let calls = Calls::default();
        let mut out = None;
        win.frame(events, |ui| {
            out = Some(shell(&cfg, &calls).show(ui, |ui, cx| {
                cx.canvas(Id::unique("canvas"));
                field.set(ui.put(Rect::from_min_size(pos2(300.0, 300.0), vec2(120.0, 24.0)), egui::TextEdit::singleline(&mut text)).rect);
            }));
        });
        keys(&out.expect("ran"))
    };
    run(&mut win, vec![]);
    let tab = || vec![key_event(Key::Tab, true), key_event(Key::Tab, false)];
    assert!(tab_toggles_chrome(&win.ctx));
    assert_eq!(run(&mut win, tab()), ["title", "options", "tools", "panels"]);
    assert_eq!(run(&mut win, tab()), Vec::<&str>::new());
    let at = field.get().center();
    run(&mut win, click(at));
    assert!(!tab_toggles_chrome(&win.ctx), "the field has focus");
    let before = win.toggles;
    run(&mut win, tab());
    assert_eq!(win.toggles, before, "Tab inside a control is the control's");
    run(&mut win, click(at));
    run(&mut win, vec![key_event(Key::Escape, true), key_event(Key::Escape, false)]);
    assert!(tab_toggles_chrome(&win.ctx), "Esc returned focus to the canvas");
    assert_eq!(run(&mut win, tab()), ["title", "options", "tools", "panels"]);
}

/// A token-sized shell with the options bar and the tool rail built from the given choices.
fn pref_shell<'a>(narrow: bool, columns: usize, edge_reveal: bool, mode: BodyMode) -> AppShell<'a> {
    AppShell::new(WindowKey::new("main"))
        .top(Slot::new("options").narrow(narrow).reveal(), |_| ())
        .start(Slot::new("tools").columns(columns).reveal(), |_| ())
        .body_mode(mode)
        .edge_reveal(edge_reveal)
}

fn pref_show(win: &mut Win, events: Vec<Event>, narrow: bool, columns: usize, edge_reveal: bool, mode: BodyMode) -> ShellOutput {
    let mut shown = None;
    win.frame(events, |ui| shown = Some(pref_shell(narrow, columns, edge_reveal, mode).show(ui, |_, _| ())));
    shown.expect("the shell ran")
}

#[test]
fn the_narrow_options_bar_and_two_rail_columns_take_their_token_sizes() {
    for size in [Size::Compact, Size::Comfortable] {
        let defaults = AppDefaults { theme: ThemePair::same(builtins()[0].id.clone()), mode: ModeChoice::System, size };
        let base = Base::resolve(&SuiteChoice::default().with_defaults(&defaults), SystemAppearance::default(), &defaults, None, &mut |id| {
            builtins().iter().find(|t| t.id == *id).cloned()
        });
        let mut win = Win::with_base(&base);
        let mut tokens = None;
        win.frame(vec![], |ui| tokens = Some(Tokens::of(ui)));
        let tokens = tokens.expect("ran");
        let out = pref_show(&mut win, vec![], false, 1, true, BodyMode::Normal);
        assert_eq!(rect(&out, "options").height(), tokens.size.bar.options, "{size:?}");
        assert_eq!(rect(&out, "tools").width(), ToolRail::<()>::metrics(&tokens, 1).width, "{size:?}");
        let out = pref_show(&mut win, vec![], true, 2, true, BodyMode::Normal);
        assert_eq!(rect(&out, "options").height(), tokens.size.bar.options_narrow, "{size:?}");
        assert_eq!(rect(&out, "tools").width(), ToolRail::<()>::metrics(&tokens, 2).width, "{size:?}");
        let out = pref_show(&mut win, vec![], true, 5, true, BodyMode::Normal);
        assert_eq!(rect(&out, "tools").width(), ToolRail::<()>::metrics(&tokens, 2).width, "{size:?}: more than two columns count as two");
    }
}

#[test]
fn turning_the_edge_reveal_off_hides_slots_the_pointer_revealed_and_on_shows_them_again() {
    let mut win = Win::new();
    let only = BodyMode::Only;
    let top = vec![Event::PointerMoved(pos2(400.0, 1.0))];
    pref_show(&mut win, vec![], false, 1, true, only);
    assert_eq!(keys(&pref_show(&mut win, top, false, 1, true, only)), ["options"]);
    assert!(pref_show(&mut win, vec![], false, 1, false, only).slots.is_empty());
    assert_eq!(keys(&pref_show(&mut win, vec![], false, 1, true, only)), ["options"]);
}

#[test]
fn with_the_edge_reveal_off_the_pointer_reveals_nothing_but_tab_does() {
    let mut win = Win::new();
    win.host_tab = true;
    let only = BodyMode::Only;
    pref_show(&mut win, vec![], false, 1, false, only);
    assert!(pref_show(&mut win, vec![Event::PointerMoved(pos2(400.0, 1.0))], false, 1, false, only).slots.is_empty());
    assert!(pref_show(&mut win, vec![Event::PointerMoved(pos2(1.0, 300.0))], false, 1, false, only).slots.is_empty());
    let tab = vec![Event::Key { key: Key::Tab, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE }];
    assert_eq!(keys(&pref_show(&mut win, tab, false, 1, false, only)), ["options", "tools"]);
}

#[test]
fn the_pointer_at_an_edge_shows_that_sides_reveal_slots_and_leaving_hides_them() {
    let cfg = Cfg::new().only();
    for (at, shown) in [(pos2(1.0, 300.0), vec!["tools"]), (pos2(W - 1.0, 300.0), vec!["panels"]), (pos2(400.0, 1.0), vec!["title", "options"])] {
        let mut win = Win::new();
        win.show(&cfg, vec![]);
        let out = win.pointer(&cfg, at);
        assert_eq!(keys(&out), shown, "pointer at {at:?}");
        assert!(out.revealed);
        // Still over the zone, then over the revealed slot: stays.
        let over = match shown[0] {
            "tools" => pos2(20.0, 300.0),
            "panels" => pos2(W - 20.0, 300.0),
            _ => pos2(400.0, 20.0),
        };
        assert_eq!(keys(&win.pointer(&cfg, over)), shown, "pointer over the revealed slot {over:?}");
        // Leaving both hides.
        assert!(win.pointer(&cfg, pos2(400.0, 300.0)).slots.is_empty());
    }
    // The bottom edge reveals nothing.
    let mut win = Win::new();
    win.show(&cfg, vec![]);
    assert!(win.pointer(&cfg, pos2(400.0, H - 1.0)).slots.is_empty());
}

#[test]
fn the_start_edge_is_the_right_edge_in_rtl() {
    let mut win = Win::new();
    direction::apply(&win.ctx, Direction::Rtl);
    let cfg = Cfg::new().only();
    win.show(&cfg, vec![]);
    assert_eq!(keys(&win.pointer(&cfg, pos2(W - 1.0, 300.0))), ["tools"]);
    assert!(win.pointer(&cfg, pos2(400.0, 300.0)).slots.is_empty());
    assert_eq!(keys(&win.pointer(&cfg, pos2(1.0, 300.0))), ["panels"]);
}

#[test]
fn no_edge_reveal_while_a_button_is_held_a_popup_is_open_or_the_tab_reveal_is_on() {
    let cfg = Cfg::new().only();
    let press = |at| Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE };

    let mut win = Win::new();
    win.show(&cfg, vec![]);
    let at = pos2(1.0, 300.0);
    assert!(win.show(&cfg, vec![Event::PointerMoved(pos2(400.0, 300.0)), press(pos2(400.0, 300.0))]).0.slots.is_empty());
    assert!(win.show(&cfg, vec![Event::PointerMoved(at)]).0.slots.is_empty(), "button held while the pointer enters the zone");

    let mut win = Win::new();
    win.show(&cfg, vec![]);
    let calls = Calls::default();
    let mut out = None;
    win.frame(vec![Event::PointerMoved(at)], |ui| {
        egui::Popup::open_id(ui.ctx(), Id::unique("popup"));
        out = Some(shell(&cfg, &calls).show(ui, |_, _| ()));
    });
    assert!(out.expect("ran").slots.is_empty(), "a popup is open");

    // The Tab reveal shows everything with `.reveal()` and the edge adds nothing.
    let mut win = Win::new();
    win.show(&cfg, vec![]);
    AppShell::toggle_reveal(&win.ctx, &main_key());
    assert_eq!(keys(&win.pointer(&cfg, at)), ["title", "options", "tools", "panels"]);
    AppShell::toggle_reveal(&win.ctx, &main_key());
    // Toggled off with the pointer in the zone: the edge reveal starts again from the zone.
    assert_eq!(keys(&win.pointer(&cfg, at)), ["tools"]);
}

#[test]
fn the_edge_zone_does_not_take_a_press() {
    // No start slot reveals, so nothing covers the body's left edge; a click there reaches it.
    let mut win = Win::new();
    let calls = Calls::default();
    let mut clicked = false;
    let run = |win: &mut Win, events: Vec<Event>, clicked: &mut bool| {
        win.frame(events, |ui| {
            AppShell::new(WindowKey::main()).body_mode(BodyMode::Only).show(ui, |ui, _| {
                let r = ui.put(Rect::from_min_size(pos2(0.0, 100.0), vec2(30.0, 30.0)), egui::Button::new("x"));
                *clicked |= r.clicked();
            });
        });
        let _ = &calls;
    };
    run(&mut win, vec![], &mut clicked);
    run(&mut win, click(pos2(1.0, 110.0)), &mut clicked);
    assert!(clicked);
}

#[test]
fn windows_with_different_keys_reveal_independently() {
    let mut win = Win::new();
    let (a, b) = (Cfg { key: "a", ..Cfg::new().only() }, Cfg { key: "b", ..Cfg::new().only() });
    let calls = Calls::default();
    let both = |win: &mut Win| {
        let mut shown = (Vec::new(), Vec::new());
        win.frame(vec![], |ui| {
            let (left, right) = ui.max_rect().split_left_right_at_fraction(0.5);
            let mut l = ui.new_child(egui::UiBuilder::new().max_rect(left));
            shown.0 = keys(&shell(&a, &calls).show(&mut l, |_, _| ()));
            let mut r = ui.new_child(egui::UiBuilder::new().max_rect(right));
            shown.1 = keys(&shell(&b, &calls).show(&mut r, |_, _| ()));
        });
        shown
    };
    both(&mut win);
    assert!(AppShell::toggle_reveal(&win.ctx, &WindowKey::new("a")));
    let (shown_a, shown_b) = both(&mut win);
    assert_eq!(shown_a, ["title", "options", "tools", "panels"]);
    assert!(shown_b.is_empty());
    assert!(!AppShell::reveal_active(&win.ctx, &WindowKey::new("b")));
    assert_ne!(ids::slot_salt(&WindowKey::new("a"), "tools"), ids::slot_salt(&WindowKey::new("b"), "tools"));
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "two AppShells")]
fn two_shells_with_one_key_in_one_pass_panic_in_debug() {
    let mut win = Win::new();
    win.frame(vec![], |ui| {
        AppShell::bare(WindowKey::main()).show(ui, |_, _| ());
        AppShell::bare(WindowKey::main()).show(ui, |_, _| ());
    });
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "two slots with the key")]
fn two_slots_with_one_key_panic_in_debug() {
    let mut win = Win::new();
    win.frame(vec![], |ui| {
        AppShell::new(WindowKey::main()).top(Slot::new("bar").height(20.0), |_| {}).bottom(Slot::new("bar").height(20.0), |_| {}).show(ui, |_, _| ());
    });
}

#[test]
fn a_bare_shell_is_only_a_body() {
    let mut win = Win::new();
    let mut out = None;
    win.frame(vec![], |ui| out = Some(AppShell::bare(WindowKey::new("torn")).show(ui, |_, cx| cx.window().clone())));
    let out = out.expect("ran");
    assert!(out.slots.is_empty() && near(out.body.width(), W) && near(out.body.height(), H));
    assert_eq!(out.inner, WindowKey::new("torn"));
    assert!(AppShell::bare(WindowKey::main()).is_bare());
}

#[test]
fn accesskit_has_a_main_body_and_named_regions_for_shown_slots_only() {
    use egui::accesskit::Role;
    let nodes = |out: &FullOutput| -> Vec<(Role, Option<String>)> {
        let update = out.platform_output.accesskit_update.as_ref().expect("accesskit is on");
        update.nodes.iter().map(|(_, n)| (n.role(), n.label().map(str::to_owned))).collect()
    };
    let mut win = Win::new();
    win.ctx.enable_accesskit();
    let calls = Calls::default();
    let cfg = Cfg { options: false, ..Cfg::new() };
    let out = win.frame(vec![], |ui| {
        shell(&cfg, &calls).body_label("Document area").show(ui, |_, _| ());
    });
    let all = nodes(&out);
    let region = |name: &str| all.iter().any(|(r, l)| *r == Role::Region && l.as_deref() == Some(name));
    assert!(all.iter().any(|(r, l)| *r == Role::Main && l.as_deref() == Some("Document area")), "{all:?}");
    assert!(region("Title") && region("Status") && region("Tools"), "{all:?}");
    assert!(!region("Options"), "a hidden slot has no node");
    // Slots without a label are still regions.
    assert!(all.iter().filter(|(r, _)| *r == Role::Region).count() >= 5, "{all:?}");
}

/// Runs `cfg` with `extra` drawn after the shell, as the host's dialogs are.
fn show_with(win: &mut Win, cfg: &Cfg, events: Vec<Event>, mut extra: impl FnMut(&Context)) -> ShellOutput {
    let calls = Calls::default();
    let mut out = None;
    win.frame(events, |ui| {
        out = Some(shell(cfg, &calls).show(ui, |_, cx| cx.canvas(Id::unique("canvas"))));
        extra(ui.ctx());
    });
    out.expect("ran")
}

fn tab() -> Vec<Event> {
    vec![key_event(Key::Tab, true), key_event(Key::Tab, false)]
}

#[test]
fn tab_belongs_to_an_open_dialog_menu_or_popup() {
    let cfg = Cfg::new().only();
    let modal = |ctx: &Context| {
        egui::Modal::new(Id::unique("dialog")).show(ctx, |ui| {
            let _ = ui.button("First");
            let _ = ui.button("Second");
        });
    };
    let mut win = Win::new();
    win.host_tab = true;
    show_with(&mut win, &cfg, vec![], modal);
    show_with(&mut win, &cfg, vec![], modal);
    assert!(!tab_toggles_chrome(&win.ctx), "a modal is on top");
    show_with(&mut win, &cfg, tab(), modal);
    assert_eq!(win.toggles, 0, "Tab is the dialog's");
    assert!(win.ctx.memory(|m| m.focused()).is_some(), "Tab reached the dialog's first control");
    assert!(!AppShell::reveal_active(&win.ctx, &main_key()));

    // An open popup or menu.
    let mut win = Win::new();
    win.host_tab = true;
    show_with(&mut win, &cfg, vec![], |_| ());
    egui::Popup::open_id(&win.ctx, Id::unique("popup"));
    assert!(!tab_toggles_chrome(&win.ctx));
    show_with(&mut win, &cfg, tab(), |_| ());
    assert_eq!(win.toggles, 0, "Tab is the popup's");
    egui::Popup::close_all(&win.ctx);
    assert!(tab_toggles_chrome(&win.ctx));
    show_with(&mut win, &cfg, tab(), |_| ());
    assert_eq!(win.toggles, 1);
}

#[test]
fn a_stale_canvas_id_does_not_count_after_a_document_switch() {
    let mut win = Win::new();
    let cfg = Cfg::new().only();
    show_with(&mut win, &cfg, vec![], |_| ());
    let canvas = Id::unique("canvas");
    win.ctx.memory_mut(|m| m.request_focus(canvas));
    assert!(tab_toggles_chrome(&win.ctx), "the registered canvas has focus");
    // The next documents' body registers no canvas for two passes (a start screen).
    for _ in 0..2 {
        win.frame(vec![], |ui| {
            shell(&cfg, &Calls::default()).show(ui, |_, _| ());
        });
    }
    win.ctx.memory_mut(|m| m.request_focus(canvas));
    assert!(!tab_toggles_chrome(&win.ctx), "the old canvas id is no longer the canvas");
}

fn bare_body(win: &mut Win, cfg: &Cfg, events: Vec<Event>) -> ShellOutput {
    win.show(cfg, events).0
}

#[test]
fn an_edge_reveal_overlays_the_body_and_the_tab_reveal_pushes_it() {
    let cfg = Cfg::new().only();
    let mut win = Win::new();
    let plain = bare_body(&mut win, &cfg, vec![]).body;
    for at in [pos2(1.0, 300.0), pos2(W - 1.0, 300.0), pos2(400.0, 1.0)] {
        let mut win = Win::new();
        win.show(&cfg, vec![]);
        let out = win.show(&cfg, vec![Event::PointerMoved(at)]).0;
        assert!(out.revealed && !out.slots.is_empty());
        assert_eq!(out.body, plain, "the body stays put for the reveal at {at:?}");
    }
    // The overlaid slot covers the body's edge and keeps the reveal while hovered.
    let mut win = Win::new();
    win.show(&cfg, vec![]);
    let out = win.show(&cfg, vec![Event::PointerMoved(pos2(1.0, 300.0))]).0;
    let tools = rect(&out, "tools");
    assert!(near(tools.left(), 0.0) && near(tools.width(), 40.0) && tools.intersects(out.body));
    let out = win.show(&cfg, vec![Event::PointerMoved(pos2(30.0, 300.0))]).0;
    assert_eq!(keys(&out), ["tools"], "hovering the overlaid slot keeps it");
    // A press on the overlaid slot goes to the slot, not the body underneath.
    let mut win = Win::new();
    let (hit, slot_hit) = (std::cell::Cell::new(false), std::cell::Cell::new(false));
    let run = |win: &mut Win, events: Vec<Event>| {
        win.frame(events, |ui| {
            AppShell::new(WindowKey::main())
                .start(Slot::new("tools").width(40.0).reveal(), |ui| {
                    slot_hit.set(slot_hit.get() | ui.put(Rect::from_min_size(pos2(0.0, 280.0), vec2(40.0, 40.0)), egui::Button::new("t")).clicked());
                })
                .body_mode(BodyMode::Only)
                .show(ui, |ui, _| {
                    hit.set(hit.get() | ui.put(Rect::from_min_size(pos2(0.0, 280.0), vec2(40.0, 40.0)), egui::Button::new("b")).clicked());
                });
        });
    };
    run(&mut win, vec![]);
    run(&mut win, vec![Event::PointerMoved(pos2(1.0, 300.0))]);
    run(&mut win, vec![Event::PointerMoved(pos2(20.0, 300.0))]);
    run(&mut win, click(pos2(20.0, 300.0)));
    assert!(slot_hit.get() && !hit.get(), "the overlaid slot takes the press");

    // The Tab reveal shrinks the body.
    let mut win = Win::new();
    win.show(&cfg, vec![]);
    AppShell::toggle_reveal(&win.ctx, &main_key());
    let out = win.show(&cfg, vec![]).0;
    assert!(out.body.width() < plain.width() && out.body.height() < plain.height());
}

#[test]
fn the_shell_context_tells_which_edge_is_revealed() {
    let cfg = Cfg::new().only();
    let probe = |win: &mut Win, events: Vec<Event>| {
        let calls = Calls::default();
        let mut seen = None;
        win.frame(events, |ui| {
            shell(&cfg, &calls).show(ui, |_, cx| {
                seen = Some((cx.edge_revealed(Edge::Start), cx.edge_revealed(Edge::End), cx.edge_revealed(Edge::Top), cx.dock_visible()));
            });
        });
        seen.expect("ran")
    };
    let mut win = Win::new();
    probe(&mut win, vec![]);
    assert_eq!(probe(&mut win, vec![]), (false, false, false, false));
    assert_eq!(probe(&mut win, vec![Event::PointerMoved(pos2(1.0, 300.0))]), (true, false, false, false));
    assert_eq!(probe(&mut win, vec![Event::PointerMoved(pos2(W - 1.0, 300.0))]), (false, true, false, true));
    assert_eq!(probe(&mut win, vec![Event::PointerMoved(pos2(400.0, 1.0))]), (false, false, true, false));
    probe(&mut win, vec![Event::PointerMoved(pos2(400.0, 300.0))]);
    AppShell::toggle_reveal(&win.ctx, &main_key());
    assert_eq!(probe(&mut win, vec![]), (false, false, false, true), "Tab reveal");
    let normal = Cfg::new();
    let mut win = Win::new();
    let mut dock = false;
    win.frame(vec![], |ui| {
        shell(&normal, &Calls::default()).show(ui, |_, cx| dock = cx.dock_visible());
    });
    assert!(dock);
}
