//! Native dock windows: where a saved window opens on the connected monitors, what the dock records when the OS
//! moves it, which window a palette's level and a drop follow.

use crate::windows_kit::Windows;
use craft_behavior::Monitor;
use craft_behavior::dock::{Action, Floating, Layout, NativeSpace, Permissions, Workspace};
use craft_ui::contract::{Bind, BodyCx};
use craft_ui::docking::native::restore;
use craft_ui::docking::{DockConfig, DockPanels, FloatingGroup, PanelSpec, native};
use craft_ui::patterns::dock::DockArea;
use craft_ui::runtime::windows::{WindowHost, WindowSpec};
use craft_ui::testing::trial;
use egui::{Pos2, Rect, Ui, Vec2, ViewportBuilder, ViewportId, ViewportInfo};

const BAR: f32 = 24.0;
const PRIMARY: Rect = Rect { min: Pos2::new(0.0, 0.0), max: Pos2::new(1920.0, 1080.0) };
const GONE: Rect = Rect { min: Pos2::new(1920.0, 0.0), max: Pos2::new(3840.0, 1080.0) };
const SAVED: Rect = Rect { min: Pos2::new(2000.0, 100.0), max: Pos2::new(2400.0, 400.0) };
/// The root window and the palette overlap at `OVERLAP`; the palette is the larger.
const ROOT_AT: Rect = Rect { min: Pos2::new(100.0, 100.0), max: Pos2::new(300.0, 300.0) };
const PALETTE_AT: Rect = Rect { min: Pos2::new(200.0, 150.0), max: Pos2::new(600.0, 450.0) };
const OVERLAP: Pos2 = Pos2::new(250.0, 200.0);
const PALETTE_TITLE: &str = "Tools";

fn monitor(name: &str, rect: Rect) -> Monitor {
    Monitor::new(Some(name.to_owned()), rect, 1.0)
}

fn native_window() -> Floating<u32> {
    let mut window = Floating::single(vec![1], 0, Rect::from_min_size(Pos2::new(40.0, 40.0), Vec2::new(300.0, 200.0)));
    window.native = Some(SAVED);
    window.monitor = Some(monitor("B", GONE));
    window
}

/// A window saved on a monitor that is no longer connected opens on the monitor that is, whole.
#[trial]
fn a_window_saved_on_a_missing_monitor_opens_on_a_connected_one() {
    let monitors = [monitor("A", PRIMARY)];
    let window = native_window();
    let restored = restore(&window, NativeSpace::new(&monitors, BAR)).expect("the window is native");
    assert!(PRIMARY.contains_rect(restored), "{restored:?} is on the connected monitor");
    assert_eq!(restored.size(), SAVED.size(), "the window keeps its size");
    let mut inside = native_window();
    inside.native = None;
    assert_eq!(restore(&inside, NativeSpace::new(&monitors, BAR)), None, "a window in the app window has no desktop rect");
}

struct Palettes;

impl DockPanels<u32> for Palettes {
    fn spec(&self, _: &u32) -> PanelSpec {
        PanelSpec::new(PALETTE_TITLE)
    }

    fn permissions(&self, _: &u32) -> Permissions {
        Permissions::ALL
    }

    fn body(&mut self, _: &u32, _: &mut Ui, _: &mut BodyCx<'_>) {}
}

/// The dock records a move the OS made to its native window, and nothing while the OS reports what the model holds.
#[trial]
fn the_dock_records_where_the_os_put_its_native_window() {
    let windows = Windows::new();
    windows.root_info(info(true, ROOT_AT));
    let mut window = Floating::single(vec![1], 0, Rect::from_min_size(Pos2::new(40.0, 40.0), Vec2::new(300.0, 200.0)));
    window.native = Some(PALETTE_AT);
    let mut workspace = Workspace::new(Layout::new(None, vec![window]));
    let run = |workspace: &mut Workspace<u32>| {
        let mut actions = Vec::new();
        windows.frame(vec![], vec![], |ui| {
            let config = DockConfig::default().native_windows(true);
            let shown = DockArea::new("Native dock", Bind::bound(workspace), &mut Palettes, config).show(ui);
            actions.extend(shown.actions);
        });
        actions
    };
    windows.child_info(info(true, PALETTE_AT));
    for _ in 0..3 {
        run(&mut workspace);
    }
    let rest = run(&mut workspace);
    assert!(rest.iter().all(|action| !matches!(action, Action::SetNative { .. })), "at rest: {rest:?}");
    let moved = PALETTE_AT.translate(Vec2::new(-120.0, 40.0));
    windows.child_info(info(true, moved));
    let after = run(&mut workspace);
    assert!(after.contains(&Action::set_native(1, Some(moved))), "the OS move is recorded: {after:?}");
}

/// A group holding a document panel opens a document window; a tool panel, a palette without a taskbar entry.
#[trial]
fn the_window_kind_follows_the_panels() {
    use craft_host::target::WindowKind;
    use craft_ui::docking::PanelSpec;
    let tool = PanelSpec::new("Layers");
    let document = PanelSpec::new("Canvas").window_kind(WindowKind::Document);
    assert_eq!(native::spec_for(&[tool.clone()]).kind, WindowKind::Tool);
    assert!(!native::spec_for(&[tool.clone()]).taskbar);
    assert_eq!(native::spec_for(&[tool, document]).kind, WindowKind::Document);
}

fn palette_frame(windows: &Windows) -> crate::windows_kit::Out {
    windows.frame(vec![], vec![], |ui| {
        FloatingGroup::new(PALETTE_TITLE, PALETTE_AT).native(WindowSpec::tool()).show_group(ui, |_, _| {});
    })
}

fn palette_level(out: &crate::windows_kit::Out) -> Option<egui::WindowLevel> {
    let (_, builder): &(ViewportId, ViewportBuilder) =
        out.builders.iter().find(|(_, builder)| builder.title.as_deref() == Some(PALETTE_TITLE)).expect("the palette asked to open");
    builder.window_level
}

fn info(focused: bool, rect: Rect) -> ViewportInfo {
    ViewportInfo { focused: Some(focused), inner_rect: Some(rect), outer_rect: Some(rect), ..ViewportInfo::default() }
}

/// Focus hopping from the root window to a palette moves within one frame: the root window reports no focus, and the
/// palette's window reports it in its own pass of that frame. The palette keeps its level through the hop, and drops
/// it only when no window of the app keeps focus.
#[trial]
fn a_focus_hop_to_the_palette_keeps_its_level() {
    let windows = Windows::new();
    windows.root_info(info(true, ROOT_AT));
    windows.child_info(info(false, PALETTE_AT));
    let mut out = palette_frame(&windows);
    for _ in 0..3 {
        out = palette_frame(&windows);
    }
    assert_eq!(palette_level(&out), Some(egui::WindowLevel::AlwaysOnTop), "the root window has focus");
    windows.root_info(info(false, ROOT_AT));
    windows.advance(1.0);
    let hop = windows.frame(vec![], vec![], |ui| {
        windows.child_info(info(true, PALETTE_AT));
        FloatingGroup::new(PALETTE_TITLE, PALETTE_AT).native(WindowSpec::tool()).show_group(ui, |_, _| {});
    });
    assert_eq!(palette_level(&hop), Some(egui::WindowLevel::AlwaysOnTop), "the frame of the hop");
    let hopped = palette_frame(&windows);
    assert_eq!(palette_level(&hopped), Some(egui::WindowLevel::AlwaysOnTop), "the palette has focus");
    windows.child_info(info(false, PALETTE_AT));
    let mut away = palette_frame(&windows);
    for _ in 0..3 {
        away = palette_frame(&windows);
    }
    assert_eq!(palette_level(&away), Some(egui::WindowLevel::Normal), "no window of the app keeps focus");
}

/// Of two overlapping windows the one focused last is the one under the pointer, whichever is smaller.
#[trial]
fn the_window_focused_last_is_found_where_windows_overlap() {
    let windows = Windows::new();
    let run = |root_focused: bool, palette_focused: bool| {
        windows.root_info(info(root_focused, ROOT_AT));
        windows.child_info(info(palette_focused, PALETTE_AT));
        palette_frame(&windows);
        palette_frame(&windows);
    };
    run(true, false);
    run(false, true);
    run(false, false);
    let host = WindowHost::of(&windows.ctx);
    let found = host.viewport_at(OVERLAP).expect("a window lies there");
    assert_ne!(found, ViewportId::ROOT, "the palette was focused after the root window");
    run(true, false);
    run(false, false);
    assert_eq!(host.viewport_at(OVERLAP), Some(ViewportId::ROOT), "the root window was focused after the palette");
}

/// The native group of `id`'s window in one frame; returns its container id.
fn dock_window_frame(windows: &Windows, root: Vec<egui::Event>, own_drag: bool) -> egui::Id {
    let mut id = egui::Id::NULL;
    windows.frame(root, vec![], |ui| {
        let group = FloatingGroup::new(PALETTE_TITLE, PALETTE_AT).native(WindowSpec::tool());
        let group = if own_drag { group.window_of(1_u32) } else { group };
        id = group.show_group(ui, |_, _| {}).output.response.id;
    });
    id
}

fn pointer(pressed: bool) -> egui::Event {
    egui::Event::PointerButton { pos: Pos2::new(10.0, 10.0), button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE }
}

/// A native dock window fades while its drag shows a drop target, and is opaque after the drop and after the drag is
/// cancelled. The group runs in the parent's pass, so the window it fades is its own, not the running pass's.
#[trial]
fn a_native_dock_window_fades_while_its_drag_shows_a_drop_target() {
    use craft_theme::Opacity;
    use craft_ui::runtime::drag;
    use craft_ui::runtime::windows::WindowAction;
    use craft_ui::testing::windows::{applied_actions, begin_window_drag, show_drop_mark};

    let windows = Windows::new();
    windows.root_info(info(true, ROOT_AT));
    windows.child_info(info(true, PALETTE_AT));
    let mut id = dock_window_frame(&windows, vec![], true);
    for _ in 0..3 {
        id = dock_window_frame(&windows, vec![], true);
    }
    let viewport = WindowHost::of(&windows.ctx).viewport_of(id).expect("the group's window is open");
    applied_actions(&windows.ctx);
    let target = Rect::from_min_size(Pos2::new(50.0, 50.0), Vec2::splat(40.0));
    let sent = |opacity: f32| applied_actions(&windows.ctx).contains(&(viewport, WindowAction::SetOpacity(opacity)));

    // A drag ended by the drop.
    dock_window_frame(&windows, vec![pointer(true)], true);
    begin_window_drag(&windows.ctx, id);
    show_drop_mark(&windows.ctx, Some(target));
    dock_window_frame(&windows, vec![], true);
    assert!(sent(Opacity::MOVING), "the window fades over a drop target");
    show_drop_mark(&windows.ctx, None);
    dock_window_frame(&windows, vec![], true);
    assert!(sent(1.0), "the window is opaque over empty space");
    show_drop_mark(&windows.ctx, Some(target));
    dock_window_frame(&windows, vec![], true);
    assert!(sent(Opacity::MOVING));
    dock_window_frame(&windows, vec![pointer(false)], true);
    assert!(drag::active::<()>(&windows.ctx).is_none(), "the release ended the drag");
    dock_window_frame(&windows, vec![], true);
    assert!(sent(1.0), "the window is opaque after the drop");

    // A drag that is cancelled.
    dock_window_frame(&windows, vec![pointer(true)], true);
    begin_window_drag(&windows.ctx, id);
    show_drop_mark(&windows.ctx, Some(target));
    dock_window_frame(&windows, vec![], true);
    assert!(sent(Opacity::MOVING));
    drag::cancel(&windows.ctx, craft_behavior::CancelReason::Escape);
    dock_window_frame(&windows, vec![], true);
    assert!(sent(1.0), "the window is opaque after the drag is cancelled");
}

/// A dock window whose group runs its own drag from the title row keeps that row for the app, so the Windows hit test
/// is not routed over it; a window without that drag is not marked.
#[trial]
fn a_dock_window_dragged_by_the_app_keeps_its_title_row() {
    use craft_ui::testing::windows::keeps_app_drag;

    for (own_drag, expected) in [(true, true), (false, false)] {
        let windows = Windows::new();
        windows.root_info(info(true, ROOT_AT));
        windows.child_info(info(true, PALETTE_AT));
        let mut id = dock_window_frame(&windows, vec![], own_drag);
        for _ in 0..3 {
            id = dock_window_frame(&windows, vec![], own_drag);
        }
        let viewport = WindowHost::of(&windows.ctx).viewport_of(id).expect("the group's window is open");
        assert_eq!(keeps_app_drag(&windows.ctx, viewport), expected, "own drag: {own_drag}");
    }
}
