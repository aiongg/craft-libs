//! The window skeleton's interface: what `ShellCx` answers by mode and reveal, and how a body reads it.

mod caption_button;

mod document_window;
mod embedded_viewport;
mod screen_mode;
mod window_parts;

// Start screen.
mod start;

use craft_ui::runtime::env::{BodyMode, RevealEdge, ShellCx, WindowKey};
use craft_ui::testing::trial;
use egui::{Rect, pos2};

fn body() -> Rect {
    Rect::from_min_max(pos2(40.0, 30.0), pos2(640.0, 480.0))
}

fn detached(mode: BodyMode) -> ShellCx {
    ShellCx::detached(WindowKey::main(), mode, body())
}

#[trial]
fn body_only_mode_hides_the_dock_and_floating_frames_until_revealed() {
    let cx = detached(BodyMode::Only);
    assert!(!cx.dock_visible() && !cx.floating_visible() && !cx.tabs_visible());
    let tab = cx.clone().with_tab_reveal(true);
    assert!(tab.revealed() && tab.dock_visible() && tab.floating_visible());
    assert!(!tab.tabs_visible(), "document tabs never show in body-only mode");
    let end = cx.clone().with_edge_reveal(RevealEdge::End, true);
    assert!(end.revealed() && end.dock_visible() && end.edge_revealed(RevealEdge::End));
    assert!(!end.floating_visible(), "an edge reveal does not show floating frames");
    let start = cx.with_edge_reveal(RevealEdge::Start, true);
    assert!(start.revealed() && !start.dock_visible());
}

mod app_shell;

// Title bar.
mod title_bar;
