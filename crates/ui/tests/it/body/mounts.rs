//! The mount host: layers move and resize by pointer, a zoomed canvas body hits its members in canvas space,
//! and a viewport mount shows its body in the fallback layer where viewports are unavailable.

use super::kit::Slab;
use crate::contract::kit::{Probe, Seen};
use crate::windows_kit::Windows;
use craft_behavior::CanvasTransform;
use craft_host::target::{Caps, ShellInfo};
use craft_ui::contract::{Bind, SaveKey};
use craft_ui::contract::{ContainerProps, DragBand, HeaderRegion, Mount, Overlay, Props as _, Show as _, ShowContainer as _, Sizing};
use craft_ui::choice::Checkbox;
use craft_ui::runtime::InstallSpec;
use craft_ui::testing::input::key_press;
use craft_ui::testing::layout::{Block, in_region};
use craft_ui::testing::windows::NoWindows;
use craft_ui::testing::{Frames, pointer, trial};
use craft_window::restore::Store;
use egui::{Key, Modifiers, Pos2, Rect, Ui, Vec2, ViewportEvent, ViewportId, ViewportInfo};
use std::cell::{Cell, RefCell};
use std::sync::{Arc, Mutex};

/// What the pointer drags a layer by, and the most its position may differ from the drag.
const MOVE: Vec2 = Vec2::new(40.0, 30.0);
const GROW: Vec2 = Vec2::new(30.0, 20.0);
const SLACK: f32 = 1.5;
/// The inner size of the exported window, in points.
const INNER: [f32; 2] = [420.0, 310.0];
/// The body the layers show.
const BODY: Vec2 = Vec2::new(120.0, 60.0);

fn layer_props(overlay: Overlay) -> ContainerProps {
    ContainerProps::new().mount(Mount::Layer(overlay)).sizing(Sizing::Content).header(HeaderRegion::new().band(DragBand::Move))
}

/// Shows a layer container for `frames`, recording its rect.
fn show_layer(rect: &Cell<Rect>, overlay: &Overlay) -> impl FnMut(&mut Ui) {
    let overlay = overlay.clone();
    move |ui| {
        let out = Slab::new(layer_props(overlay.clone())).id_salt("layer").show(ui, |ui, _| {
            Block::new(BODY).show(ui);
        });
        rect.set(out.output.rect);
    }
}

#[trial]
fn a_layer_container_moves_by_its_header_band() {
    let rect = Cell::new(Rect::NOTHING);
    let mut frames = Frames::new();
    let overlay = Overlay::floating().movable(true);
    let mut show = show_layer(&rect, &overlay);
    frames.idle(4, &mut show);
    let before = rect.get();
    assert!(before.is_positive(), "the layer shows its body: {before:?}");
    let header = Pos2::new(before.center().x, before.min.y + 8.0);
    pointer::drag(&mut frames, header, MOVE, &mut show);
    frames.idle(2, &mut show);
    let moved = rect.get().min - before.min;
    assert!((moved - MOVE).abs().max_elem() < SLACK, "the layer followed the pointer: {moved:?}");
}

#[trial]
fn a_resizable_layer_resizes_by_its_corner_grip() {
    let rect = Cell::new(Rect::NOTHING);
    let mut frames = Frames::new();
    let overlay = Overlay::floating().resizable(true);
    let mut show = show_layer(&rect, &overlay);
    frames.idle(4, &mut show);
    let before = rect.get();
    pointer::drag(&mut frames, before.max - Vec2::splat(1.0), GROW, &mut show);
    frames.idle(2, &mut show);
    let grown = rect.get().size() - before.size();
    assert!((grown - GROW).abs().max_elem() < SLACK, "the layer grew by the drag: {grown:?}");
    assert!((rect.get().min - before.min).abs().max_elem() < SLACK, "the opposite corner stayed: {:?} {before:?}", rect.get());
}

#[trial]
fn a_zoomed_canvas_body_hits_its_members_in_canvas_space() {
    let seen = RefCell::new(Seen::default());
    let body = Cell::new(Rect::NOTHING);
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| {
        in_region(ui, Rect::from_min_size(Pos2::new(20.0, 20.0), Vec2::new(400.0, 300.0)), |ui| {
            let props = ContainerProps::new().arena(true).transform(CanvasTransform::new(2.0, Vec2::ZERO));
            Slab::new(props).id_salt("canvas").show(ui, |ui, cx| {
                body.set(cx.rect);
                Probe::new(&seen).size(Vec2::new(40.0, 20.0)).show(ui);
            });
        });
    };
    frames.idle(4, &mut show);
    let origin = body.get().min;
    // The member is 40 x 20 in canvas space and 80 x 40 on screen.
    pointer::click(&mut frames, origin + Vec2::new(70.0, 30.0), &mut show);
    assert_eq!(seen.borrow().activations, 1, "a click inside the scaled member activates it");
    pointer::click(&mut frames, origin + Vec2::new(100.0, 30.0), &mut show);
    assert_eq!(seen.borrow().activations, 1, "a click outside the scaled member does not");
}

#[trial]
fn a_viewport_mount_shows_its_body_in_the_fallback_layer_under_test() {
    let (ran, rect) = (Cell::new(0), Cell::new(Rect::NOTHING));
    let mut frames = Frames::new();
    let screen = Rect::from_min_max(Pos2::ZERO, Pos2::new(800.0, 600.0));
    frames.idle(4, |ui| {
        let props = ContainerProps::new().mount(Mount::viewport(Overlay::floating())).sizing(Sizing::Content);
        let out = Slab::new(props).id_salt("window").show(ui, |ui, _| {
            ran.set(ran.get() + 1);
            Block::new(BODY).show(ui);
        });
        rect.set(out.output.rect);
    });
    assert!(ran.get() > 0, "the body ran in the fallback layer");
    assert!(screen.contains_rect(rect.get()) && rect.get().width() >= BODY.x, "the fallback layer sits inside the window: {:?}", rect.get());
}

#[trial]
fn a_pane_holds_its_width_while_its_body_fills_it() {
    use craft_ui::contract::{PaneEdge, ShellPane};
    let width = Cell::new(0.0);
    let mut frames = Frames::new();
    let mut widths = Vec::new();
    for _ in 0..10 {
        frames.idle(1, |ui| {
            let props = ContainerProps::new().mount(Mount::Pane(ShellPane::new("side").edge(PaneEdge::Start))).pad(craft_ui::contract::Pad::Regular);
            let out = Slab::new(props).id_salt("pane").show(ui, |ui, _| {
                Block::new(BODY).show(ui);
            });
            width.set(out.output.rect.width());
        });
        widths.push(width.get());
    }
    assert!((widths[9] - widths[3]).abs() < SLACK, "a pane keeps its width from frame to frame: {widths:?}");
}

/// Where viewports are available a viewport mount opens a window with the container's title, and its body runs in
/// that window's passes, not the main window's.
#[trial]
fn a_viewport_mount_opens_a_window_with_its_title() {
    let windows = Windows::new();
    let in_window = Cell::new(false);
    let frame = || {
        windows.frame(vec![], vec![], |ui| {
            let props = ContainerProps::new().mount(Mount::viewport(Overlay::floating())).sizing(Sizing::Content);
            Slab::new(props).label("Export").id_salt("window").show(ui, |ui, _| {
                in_window.set(ui.ctx().viewport_id() != ViewportId::ROOT);
                Block::new(BODY).show(ui);
            });
        })
    };
    frame();
    let out = frame();
    assert!(in_window.get(), "the body ran in the child viewport");
    assert!(
        out.titles.iter().any(|(id, title)| *id != ViewportId::ROOT && title == "Export"),
        "the window is titled by the container: {:?}",
        out.titles
    );
}

/// Without the viewport capability the same mount shows its body in the main window, on the fallback layer.
#[trial]
fn a_viewport_mount_without_the_capability_uses_the_fallback_overlay() {
    let shell = ShellInfo::default().caps(Caps::web());
    let windows = Windows::with(InstallSpec::new(shell, Arc::new(NoWindows)));
    let (ran, in_window) = (Cell::new(0), Cell::new(false));
    for _ in 0..3 {
        windows.frame(vec![], vec![], |ui| {
            let props = ContainerProps::new().mount(Mount::viewport(Overlay::floating())).sizing(Sizing::Content);
            Slab::new(props).label("Export").id_salt("window").show(ui, |ui, _| {
                ran.set(ran.get() + 1);
                in_window.set(ui.ctx().viewport_id() != ViewportId::ROOT);
                Block::new(BODY).show(ui);
            });
        });
    }
    assert!(ran.get() > 0 && !in_window.get(), "the body ran in the main window's fallback layer");
}

/// A viewport mount's window placement goes to the placement store under the container's save key, and the next
/// run opens the window with it.
#[trial]
fn a_viewport_mount_saves_its_window_placement_in_the_store() {
    let store = Arc::new(Mutex::new(Store::in_memory()));
    let spec = InstallSpec::new(ShellInfo::default(), Arc::new(NoWindows)).placement(store.clone());
    let windows = Windows::with(spec);
    windows.child_info(ViewportInfo {
        inner_rect: Some(Rect::from_min_size(Pos2::new(120.0, 90.0), Vec2::from(INNER))),
        outer_rect: Some(Rect::from_min_size(Pos2::new(118.0, 60.0), Vec2::new(424.0, 342.0))),
        native_pixels_per_point: Some(2.0),
        ..ViewportInfo::default()
    });
    for _ in 0..3 {
        windows.frame(vec![], vec![], |ui| {
            let props = ContainerProps::new()
                .mount(Mount::viewport(Overlay::floating()))
                .sizing(Sizing::Content)
                .save(SaveKey::new("craft_ui_tests.export_window"));
            Slab::new(props).label("Export").id_salt("window").show(ui, |ui, _| {
                Block::new(BODY).show(ui);
            });
        });
    }
    let saved = store.lock().unwrap().get("craft_ui_tests.export_window").expect("the window's placement was saved");
    assert_eq!(saved.normal.size, INNER, "the inner size in points");
    assert_eq!(saved.position_px, Some([236, 120]), "the outer position in physical pixels");
}

/// The OS asking a viewport mount's window to close closes the container: its body stops running.
#[trial]
fn the_os_closing_a_viewport_mount_window_closes_the_container() {
    let windows = Windows::new();
    let ran = Cell::new(0);
    let frame = || {
        windows.frame(vec![], vec![], |ui| {
            let props = ContainerProps::new().mount(Mount::viewport(Overlay::floating())).sizing(Sizing::Content);
            Slab::new(props).label("Export").id_salt("window").show(ui, |ui, _| {
                ran.set(ran.get() + 1);
                Block::new(BODY).show(ui);
            });
        });
    };
    for _ in 0..3 {
        frame();
    }
    windows.child_info(ViewportInfo { events: vec![ViewportEvent::Close], ..ViewportInfo::default() });
    frame();
    windows.child_info(ViewportInfo::default());
    frame();
    let after_close = ran.get();
    for _ in 0..3 {
        frame();
    }
    assert_eq!(ran.get(), after_close, "the body no longer runs once the window was closed");
}

/// The body of a viewport mount runs under its own window's facts, not the parent window's.
#[trial]
fn a_viewport_mount_body_reads_its_own_windows_facts() {
    let windows = Windows::new();
    windows.root_info(ViewportInfo { focused: Some(true), ..ViewportInfo::default() });
    windows.child_info(ViewportInfo { focused: Some(false), ..ViewportInfo::default() });
    let seen = Cell::new(None);
    for _ in 0..3 {
        windows.frame(vec![], vec![], |ui| {
            let props = ContainerProps::new().mount(Mount::viewport(Overlay::floating())).sizing(Sizing::Content);
            Slab::new(props).label("Export").id_salt("window").show(ui, |ui, cx| {
                if ui.ctx().viewport_id() != ViewportId::ROOT {
                    seen.set(Some((cx.env.shell().window.active, cx.env.shell().window.kind)));
                }
                Block::new(BODY).show(ui);
            });
        });
    }
    assert_eq!(seen.get(), Some((false, craft_host::target::WindowKind::Tool)), "the unfocused tool window's own facts");
}

/// A dialog mounted in an OS window keeps the dialog's dismissal rules: Escape with the focus inside closes it.
#[trial]
fn escape_closes_a_dialog_mounted_in_an_os_window() {
    let windows = Windows::new();
    let ran = Cell::new(0);
    let frame = |child: Vec<egui::Event>| {
        windows.frame(vec![], child, |ui| {
            let props = ContainerProps::new().mount(Mount::viewport(Overlay::dialog())).sizing(Sizing::Content);
            Slab::new(props).label("Export").id_salt("window").show(ui, |ui, _| {
                ran.set(ran.get() + 1);
                let mut on = false;
                Checkbox::new(Bind::bound(&mut on)).label("Snap").show(ui);
            });
        });
    };
    for _ in 0..30 {
        frame(vec![]);
    }
    frame(vec![key_press(Key::Escape, Modifiers::NONE)]);
    frame(vec![]);
    let after_close = ran.get();
    for _ in 0..3 {
        frame(vec![]);
    }
    assert_eq!(ran.get(), after_close, "Escape closed the dialog in its window: the body no longer runs");
}

/// A modal fallback mounted in an OS window is a dialog window, where the same mount with a floating overlay is a tool
/// window.
#[trial]
fn a_modal_viewport_mount_opens_a_dialog_window() {
    let windows = Windows::new();
    let seen = Cell::new(None);
    for _ in 0..3 {
        windows.frame(vec![], vec![], |ui| {
            let props = ContainerProps::new().mount(Mount::viewport(Overlay::dialog())).sizing(Sizing::Content);
            Slab::new(props).label("Export").id_salt("window").show(ui, |ui, cx| {
                if ui.ctx().viewport_id() != ViewportId::ROOT {
                    seen.set(Some(cx.env.shell().window.kind));
                }
                Block::new(BODY).show(ui);
            });
        });
    }
    assert_eq!(seen.get(), Some(craft_host::target::WindowKind::Dialog), "the modal mount's window is a dialog window");
}

/// A close request the container did not take in its frame (it was shown in a parent's sizing pass, which runs no
/// update step) is dropped: showing the container in later frames does not close it.
#[trial]
fn a_close_request_not_taken_this_frame_does_not_close_the_container_later() {
    let windows = Windows::new();
    let ran = Cell::new(0);
    let show = |ui: &mut Ui| {
        let props = ContainerProps::new().mount(Mount::viewport(Overlay::floating())).sizing(Sizing::Content);
        Slab::new(props).label("Export").id_salt("window").show(ui, |ui, _| {
            ran.set(ran.get() + 1);
            Block::new(BODY).show(ui);
        });
    };
    let frame = || {
        windows.frame(vec![], vec![], |ui| show(ui));
    };
    for _ in 0..3 {
        frame();
    }
    windows.child_info(ViewportInfo { events: vec![ViewportEvent::Close], ..ViewportInfo::default() });
    windows.frame(vec![], vec![], |ui| {
        let mut sizing = ui.new_child(egui::UiBuilder::new().sizing_pass().invisible());
        show(&mut sizing);
    });
    windows.child_info(ViewportInfo::default());
    for _ in 0..3 {
        frame();
    }
    let before = ran.get();
    for _ in 0..3 {
        frame();
    }
    assert!(ran.get() > before, "the container stayed open: the stale request did not close it");
}
