use std::sync::{Arc, Mutex};

use craft_window::Layout;
use craft_window::policy::{CaptionFamily, Captions};
use craft_window::restore::{Defaults, MAIN, Placement, Store, Tracker, cascade_window};
use egui::{Context, Event, FullOutput, Pos2, RawInput, Rect, ResizeDirection, ViewportCommand, ViewportId, pos2, vec2};

use super::*;
use crate::theme::{Base, install};
use crate::title_bar::{Caption, Frame, LeadingEnd, caption_id};

const W: f32 = 800.0;
const H: f32 = 600.0;
const BAR: f32 = 32.0;

fn windows() -> Frame {
    Frame::Integrated(Captions { family: CaptionFamily::Windows, layout: Layout::default() })
}

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() }
}

fn commands(out: &mut FullOutput, viewport: ViewportId) -> Vec<ViewportCommand> {
    out.textures_delta.clear();
    out.viewport_output.remove(&viewport).map(|v| v.commands).unwrap_or_default()
}

/// One window: a context that may be shared between windows, a viewport and a key.
struct Win {
    ctx: Context,
    viewport: ViewportId,
    key: WindowKey,
    time: f64,
    fullscreen: bool,
    mode: BodyMode,
    bar: bool,
    modal: bool,
    finish: bool,
    titled: bool,
    decoration: Frame,
}

impl Win {
    fn new(ctx: &Context, viewport: ViewportId, key: &str) -> Self {
        install(ctx, &Base::suite_default());
        Self {
            ctx: ctx.clone(),
            viewport,
            key: WindowKey::new(key),
            time: 0.0,
            fullscreen: false,
            mode: BodyMode::Normal,
            bar: true,
            modal: false,
            finish: true,
            titled: false,
            decoration: windows(),
        }
    }

    fn raw(&mut self, events: Vec<Event>) -> RawInput {
        self.time += 0.05;
        let mut raw =
            RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(W, H))), time: Some(self.time), events, ..Default::default() };
        raw.viewport_id = self.viewport;
        raw.viewports.entry(self.viewport).or_default().fullscreen = Some(self.fullscreen);
        raw
    }

    fn frame_with(&mut self, events: Vec<Event>, frame: &Frame) -> Vec<ViewportCommand> {
        let raw = self.raw(events);
        let viewport = self.viewport;
        let (key, mode, bar, modal, finish, titled) = (self.key.clone(), self.mode, self.bar, self.modal, self.finish, self.titled);
        let mut out = self.ctx.run_ui(raw, |ui| {
            let mut shell = AppShell::new(key.clone()).frame(frame).body_mode(mode).allow_reveal(false);
            if bar {
                shell = shell.title_bar(Slot::new("title").height(BAR), move |ui, bar| {
                    if titled {
                        bar.show_titled(
                            ui,
                            |ui, _| {
                                ui.allocate_exact_size(vec2(100.0, BAR), egui::Sense::click());
                                LeadingEnd::from_ui(ui)
                            },
                            |ui, _| {
                                ui.allocate_exact_size(vec2(100.0, BAR), egui::Sense::click());
                            },
                        );
                    } else {
                        bar.show(ui, |_, _| {});
                    }
                });
            }
            shell.show(ui, |ui, _| {
                ui.label("Body");
            });
            if modal {
                egui::Modal::new(egui::Id::unique(("test-modal", viewport))).show(ui.ctx(), |ui| {
                    ui.label("A dialog");
                });
            }
            if finish {
                AppShell::finish_frame(ui.ctx(), &key, modal);
            }
        });
        commands(&mut out, viewport)
    }

    fn frame(&mut self, events: Vec<Event>) -> Vec<ViewportCommand> {
        let decoration = self.decoration.clone();
        self.frame_with(events, &decoration)
    }

    fn gesture(&mut self, path: &[Pos2]) -> Vec<ViewportCommand> {
        let mut cmds = self.frame(vec![Event::PointerMoved(path[0])]);
        cmds.extend(self.frame(vec![button(path[0], true)]));
        for p in &path[1..] {
            cmds.extend(self.frame(vec![Event::PointerMoved(*p)]));
        }
        cmds.extend(self.frame(vec![button(path[path.len() - 1], false)]));
        cmds
    }
}

fn resizes(cmds: &[ViewportCommand]) -> usize {
    cmds.iter().filter(|c| matches!(c, ViewportCommand::BeginResize(d) if *d == ResizeDirection::West)).count()
}

const EDGE: Pos2 = pos2(2.0, 300.0);

fn resize_gesture(w: &mut Win) -> usize {
    w.frame(vec![]);
    resizes(&w.gesture(&[EDGE, EDGE + vec2(6.0, 0.0)]))
}

#[test]
fn the_title_slot_is_a_title_bar_for_the_frame_and_the_window_key() {
    let mut w = Win::new(&Context::default(), ViewportId::ROOT, "doc");
    w.frame(vec![]);
    w.frame(vec![]);
    assert!(w.ctx.read_response(caption_id(&w.key, Caption::Close)).is_some(), "caption buttons keyed by the window key");
    assert!(w.ctx.read_response(caption_id(ViewportId::ROOT, Caption::Close)).is_none());
    // The bar's zones resize from the edge.
    assert_eq!(resize_gesture(&mut w), 1);
}

#[test]
fn an_undecorated_window_without_its_title_slot_still_resizes() {
    // Body-only mode shows no slot, so no bar draws the zones.
    let mut w = Win::new(&Context::default(), ViewportId::ROOT, "main");
    w.mode = BodyMode::Only;
    assert_eq!(resize_gesture(&mut w), 1);
    // A hidden slot, a plain title slot and a bare shell all fall back to the zones.
    let frame = windows();
    let mut seen = 0;
    for variant in 0..3 {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        let mut time = 0.0;
        let mut cmds = Vec::new();
        let steps = [vec![], vec![Event::PointerMoved(EDGE)], vec![button(EDGE, true)], vec![Event::PointerMoved(EDGE + vec2(6.0, 0.0))]];
        for events in steps {
            time += 0.05;
            let raw = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(W, H))), time: Some(time), events, ..Default::default() };
            let mut out = ctx.run_ui(raw, |ui| {
                let shell = match variant {
                    0 => AppShell::new(WindowKey::main()).title_bar(Slot::new("title").visible(false), |ui, bar| {
                        bar.show(ui, |_, _| {});
                    }),
                    1 => AppShell::new(WindowKey::main()).title(Slot::new("title").height(BAR), |ui| {
                        ui.label("Plain");
                    }),
                    _ => AppShell::bare(WindowKey::main()),
                };
                shell.frame(&frame).show(ui, |_, _| {});
            });
            cmds.extend(commands(&mut out, ViewportId::ROOT));
        }
        seen += resizes(&cmds);
    }
    assert_eq!(seen, 3);
}

#[test]
fn there_are_no_zones_in_full_screen_or_under_a_native_frame_or_hit_test() {
    let mut w = Win::new(&Context::default(), ViewportId::ROOT, "main");
    w.mode = BodyMode::Only;
    w.fullscreen = true;
    assert_eq!(resize_gesture(&mut w), 0);

    let mut w = Win::new(&Context::default(), ViewportId::ROOT, "main");
    w.mode = BodyMode::Only;
    w.decoration = Frame::Native;
    assert_eq!(resizes(&w.gesture(&[EDGE, EDGE + vec2(6.0, 0.0)])), 0);
}

#[test]
fn finish_frame_draws_the_overlay_only_with_a_modal_open() {
    let in_the_gap = pos2(300.0, BAR / 2.0);
    let drag = |w: &mut Win| {
        let cmds = w.gesture(&[in_the_gap, in_the_gap + vec2(12.0, 4.0), in_the_gap + vec2(30.0, 8.0)]);
        cmds.iter().filter(|c| matches!(c, ViewportCommand::StartDrag)).count()
    };
    // A modal is open and the host says so: the overlay moves the window.
    let mut w = Win::new(&Context::default(), ViewportId::ROOT, "main");
    w.titled = true;
    w.modal = true;
    for _ in 0..3 {
        w.frame(vec![]);
    }
    assert_eq!(drag(&mut w), 1);
    // The host does not call it: the modal takes the bar's input and nothing moves the window.
    let mut w = Win::new(&Context::default(), ViewportId::ROOT, "main");
    w.titled = true;
    w.modal = true;
    w.finish = false;
    for _ in 0..3 {
        w.frame(vec![]);
    }
    assert_eq!(drag(&mut w), 0);
    // No modal: `finish_frame` adds nothing and the bar drags by itself, once.
    let mut w = Win::new(&Context::default(), ViewportId::ROOT, "main");
    w.titled = true;
    for _ in 0..3 {
        w.frame(vec![]);
    }
    assert_eq!(drag(&mut w), 1);
}

#[test]
fn the_raw_input_hook_takes_tab_unless_an_app_modal_window_is_open() {
    let tab = Event::Key { key: egui::Key::Tab, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE };
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let mut raw = RawInput { events: vec![tab.clone()], ..Default::default() };
    assert!(AppShell::raw_input_hook(&ctx, &mut raw, &windows()));
    assert!(raw.events.is_empty());

    // An app-modal window of the root viewport blocks Tab in the second window as well.
    let second = ViewportId::from_hash_of("second");
    let mut win = Win::new(&ctx, ViewportId::ROOT, "main");
    win.bar = false;
    let raw = win.raw(vec![]);
    ctx.run_ui(raw, |ui| {
        crate::viewport::ViewportWindow::new(ViewportId::from_hash_of("settings"), egui::ViewportBuilder::default())
            .embedded(true)
            .app_modal(true)
            .show(ui.ctx(), |ui, _| {
                ui.label("Settings");
            });
    })
    .textures_delta
    .clear();
    let mut other = Win::new(&ctx, second, "second");
    let mut raw = other.raw(vec![tab.clone()]);
    raw.viewports.entry(second).or_default();
    assert!(!AppShell::raw_input_hook(&ctx, &mut raw, &windows()));
    assert_eq!(raw.events.len(), 1);
}

/// A window whose placement goes to `store` under its key.
fn tracked(store: &Arc<Mutex<Store>>, key: &str, viewport: ViewportId, size: [f32; 2], at: [f32; 2]) -> Tracker {
    let mut tracker = Tracker::new(store, key, &Defaults::new([800.0, 600.0], [200.0, 100.0]));
    let ctx = Context::default();
    let mut win = Win::new(&ctx, viewport, key);
    win.bar = false;
    let mut raw = win.raw(vec![]);
    let info = raw.viewports.entry(viewport).or_default();
    info.native_pixels_per_point = Some(1.0);
    info.outer_rect = Some(Rect::from_min_size(pos2(at[0], at[1]), vec2(size[0], size[1] + 30.0)));
    info.inner_rect = Some(Rect::from_min_size(pos2(at[0], at[1] + 30.0), vec2(size[0], size[1])));
    info.minimized = Some(false);
    info.maximized = Some(false);
    info.fullscreen = Some(false);
    let key = win.key.clone();
    ctx.run_ui(raw, |ui| {
        AppShell::new(key.clone()).tracker(&mut tracker).show(ui, |_, _| {});
    })
    .textures_delta
    .clear();
    tracker
}

#[test]
fn the_shell_stores_each_windows_placement_under_its_key_and_a_second_window_cascades_from_main() {
    let store = Arc::new(Mutex::new(Store::in_memory()));
    let mut main = tracked(&store, MAIN, ViewportId::ROOT, [1000.0, 700.0], [100.0, 50.0]);
    main.flush().ok();
    let saved: Placement = store.lock().unwrap().get(MAIN).expect("main is stored under its key");
    assert_eq!(saved.normal.size, [1000.0, 700.0]);

    // The app opens the second window cascaded from MAIN, and the shell then tracks it.
    let cascaded = cascade_window(&saved, 1, 28.0, None);
    store.lock().unwrap().set("doc-2", &cascaded);
    let second = ViewportId::from_hash_of("doc-2");
    let moved = [cascaded.normal.pos[0] + 40.0, cascaded.normal.pos[1] + 10.0];
    let mut tracker = tracked(&store, "doc-2", second, cascaded.normal.size, moved);
    tracker.flush().ok();
    let (main_now, second_now) = {
        let store = store.lock().unwrap();
        (store.get(MAIN).unwrap(), store.get("doc-2").unwrap())
    };
    assert_eq!(main_now.normal.size, [1000.0, 700.0], "the first window's entry is untouched");
    assert_ne!(second_now.normal.pos, saved.normal.pos);
    assert_eq!(second_now.normal.size, cascaded.normal.size);
    assert_eq!(second_now.normal.pos, moved);
}

#[test]
fn an_app_modal_window_blocks_every_window_of_the_context() {
    use crate::viewport::ViewportWindow;
    let ctx = Context::default();
    let second = ViewportId::from_hash_of("second");
    let mut root = Win::new(&ctx, ViewportId::ROOT, "main");
    let mut other = Win::new(&ctx, second, "second");
    root.bar = false;
    other.bar = false;
    let modal_in_root = |win: &mut Win| {
        let raw = win.raw(vec![]);
        ctx.run_ui(raw, |ui| {
            ViewportWindow::new(ViewportId::from_hash_of("settings"), egui::ViewportBuilder::default()).embedded(true).app_modal(true).show(
                ui.ctx(),
                |ui, _| {
                    ui.label("Settings");
                },
            );
        })
        .textures_delta
        .clear();
    };
    let in_viewport = |win: &mut Win| {
        let raw = win.raw(vec![]);
        let mut seen = (false, false);
        ctx.run_ui(raw, |ui| seen = (ViewportWindow::blocks_main(ui.ctx()), ViewportWindow::any_app_modal(ui.ctx()))).textures_delta.clear();
        seen
    };
    assert_eq!(in_viewport(&mut other), (false, false));
    modal_in_root(&mut root);
    assert_eq!(in_viewport(&mut root), (true, true));
    modal_in_root(&mut root);
    assert_eq!(in_viewport(&mut other), (false, true), "the other window is blocked but does not host the modal");

    // A close request of the other window is refused while the modal is open.
    let mut raw = other.raw(vec![]);
    raw.viewports.entry(second).or_default().events.push(egui::ViewportEvent::Close);
    let mut out = ctx.run_ui(raw, |ui| {
        AppShell::bare(WindowKey::new("second")).show(ui, |_, _| {});
    });
    assert!(commands(&mut out, second).iter().any(|c| matches!(c, ViewportCommand::CancelClose)));

    // The modal closes: it is no longer shown, and both windows are free again.
    for _ in 0..3 {
        in_viewport(&mut root);
    }
    assert_eq!(in_viewport(&mut other), (false, false));
}

#[test]
fn each_window_has_its_own_title_bar_and_overlay() {
    // Separate contexts: a context only counts passes of its root viewport outside a real viewport.
    let (ctx_a, ctx_b) = (Context::default(), Context::default());
    let mut a = Win::new(&ctx_a, ViewportId::ROOT, "main");
    let mut b = Win::new(&ctx_b, ViewportId::ROOT, "second");
    a.titled = true;
    b.titled = true;
    b.modal = true;
    for _ in 0..3 {
        a.frame(vec![]);
        b.frame(vec![]);
    }
    assert!(ctx_a.read_response(caption_id(&a.key, Caption::Close)).is_some());
    assert!(ctx_b.read_response(caption_id(&b.key, Caption::Close)).is_some());
    let in_the_gap = pos2(300.0, BAR / 2.0);
    // Only the window with a modal needs the overlay; the other window's bar drags by itself.
    let starts = |w: &mut Win| {
        w.gesture(&[in_the_gap, in_the_gap + vec2(12.0, 4.0), in_the_gap + vec2(30.0, 8.0)])
            .iter()
            .filter(|c| matches!(c, ViewportCommand::StartDrag))
            .count()
    };
    assert_eq!(starts(&mut b), 1);
    assert_eq!(starts(&mut a), 1);
}
