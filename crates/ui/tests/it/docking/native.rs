//! Native dock windows: where a saved window opens on the connected monitors, what the model records when the OS
//! moves it, which window a palette's level and a drop follow.

use crate::windows_kit::Windows;
use craft_behavior::Monitor;
use craft_behavior::dock::{Action, Floating, NativeSpace};
use craft_ui::docking::native::{reported, restore};
use craft_ui::docking::{FloatingGroup, native};
use craft_ui::runtime::windows::{WindowHost, WindowSpec};
use craft_ui::testing::trial;
use egui::{Pos2, Rect, Vec2, ViewportBuilder, ViewportId, ViewportInfo};

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

/// The model records a move the OS made, with the monitor it ended on, and nothing while the OS reports what the
/// model holds.
#[trial]
fn the_model_records_where_the_os_put_the_window() {
    let monitors = [monitor("A", PRIMARY), monitor("B", GONE)];
    let window = native_window();
    assert!(reported(&window, SAVED, &monitors).iter().all(|action| !matches!(action, Action::SetNative { .. })), "at rest");
    let moved = SAVED.translate(Vec2::new(-1500.0, 0.0));
    let actions = reported(&window, moved, &monitors);
    assert!(actions.contains(&Action::set_native(1, Some(moved))), "{actions:?}");
    assert!(actions.contains(&Action::set_monitor(1, Some(monitor("A", PRIMARY)), None)), "{actions:?}");
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

/// Focus hopping from the root window to a palette leaves a pass with no focused window; the palette keeps its
/// level however long that pass took, and drops it only when the passes go on without a focused window.
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
    let gap = palette_frame(&windows);
    assert_eq!(palette_level(&gap), Some(egui::WindowLevel::AlwaysOnTop), "one pass with no focused window");
    windows.child_info(info(true, PALETTE_AT));
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
