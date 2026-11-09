//! Native dock windows: where a saved window opens on the connected monitors, what the dock records when the OS
//! moves it, which window a palette's level and a drop follow.

use crate::windows_kit::Windows;
use craft_behavior::dock::{Action, Floating, Layout, Permissions, Workspace};
use craft_ui::contract::{Bind, BodyCx};
use craft_ui::docking::{DockConfig, DockPanels, FloatingGroup, PanelSpec};
use craft_ui::patterns::dock::DockArea;
use craft_ui::runtime::windows::{WindowHost, WindowSpec};
use craft_ui::testing::trial;
use egui::{Pos2, Rect, Ui, Vec2, ViewportId, ViewportInfo};

/// The root window and the palette overlap at `OVERLAP`; the palette is the larger.
const ROOT_AT: Rect = Rect { min: Pos2::new(100.0, 100.0), max: Pos2::new(300.0, 300.0) };
const PALETTE_AT: Rect = Rect { min: Pos2::new(200.0, 150.0), max: Pos2::new(600.0, 450.0) };
const OVERLAP: Pos2 = Pos2::new(250.0, 200.0);
const PALETTE_TITLE: &str = "Tools";

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

fn palette_frame(windows: &Windows) -> crate::windows_kit::Out {
    windows.frame(vec![], vec![], |ui| {
        FloatingGroup::new(PALETTE_TITLE, PALETTE_AT).native(WindowSpec::tool()).show_group(ui, |_, _| {});
    })
}

fn info(focused: bool, rect: Rect) -> ViewportInfo {
    ViewportInfo { focused: Some(focused), inner_rect: Some(rect), outer_rect: Some(rect), ..ViewportInfo::default() }
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
