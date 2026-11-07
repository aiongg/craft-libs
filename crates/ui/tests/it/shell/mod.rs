//! The window skeleton's interface: what `ShellCx` answers by mode and reveal, and how a body reads it.

mod document_window;
mod window_parts;

use crate::contract::kit::Panel;
use craft_ui::contract::{PaneEdge, ShowContainer as _};
use craft_ui::runtime::env::{BodyMode, RevealEdge, RootEnv, ShellCx, ShellRegions, WindowKey};
use craft_ui::testing::{Frames, trial};
use egui::{Rect, pos2};
use std::cell::RefCell;

fn body() -> Rect {
    Rect::from_min_max(pos2(40.0, 30.0), pos2(640.0, 480.0))
}

fn detached(mode: BodyMode) -> ShellCx {
    ShellCx::detached(WindowKey::main(), mode, body())
}

#[trial]
fn normal_mode_shows_the_dock_and_floating_frames_and_no_edge_reveals() {
    let cx = detached(BodyMode::Normal);
    assert!(cx.dock_visible() && cx.floating_visible() && cx.tabs_visible());
    assert!(!cx.revealed());
    let cx = cx.with_edge_reveal(RevealEdge::Top, true);
    assert!(!cx.edge_revealed(RevealEdge::Top), "an edge reveal exists only in body-only mode");
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

#[trial]
fn only_the_top_start_and_end_edges_reveal() {
    assert_eq!(PaneEdge::Top.reveal_edge(), Some(RevealEdge::Top));
    assert_eq!(PaneEdge::Start.reveal_edge(), Some(RevealEdge::Start));
    assert_eq!(PaneEdge::End.reveal_edge(), Some(RevealEdge::End));
    assert_eq!((PaneEdge::Bottom.reveal_edge(), PaneEdge::Center.reveal_edge()), (None, None));
}

#[trial]
fn the_tab_reveal_replaces_the_edge_reveals() {
    let cx = detached(BodyMode::Only).with_edge_reveal(RevealEdge::Top, true).with_tab_reveal(true);
    assert!(cx.revealed() && !cx.edge_revealed(RevealEdge::Top));
}

#[trial]
fn a_body_reads_the_shell_the_root_values_hold() {
    let regions = ShellRegions::new(body()).with_slot("tools", Rect::from_min_max(pos2(0.0, 30.0), pos2(40.0, 480.0)));
    let shell = ShellCx::new(WindowKey::new("palette"), BodyMode::Normal, regions);
    let mut root = RootEnv::default();
    shell.put(&mut root.values);
    let mut frames = Frames::new();
    frames.root(root);
    let seen = RefCell::new(Vec::new());
    let mut show = |ui: &mut egui::Ui| {
        Panel::new().show(ui, |_, cx| {
            seen.borrow_mut().push(cx.shell().map(|shell| (shell.window().clone(), shell.slot_visible("tools"), shell.slot_visible("status"))));
        });
    };
    frames.idle(2, &mut show);
    assert_eq!(seen.borrow().last(), Some(&Some((WindowKey::new("palette"), true, false))));

    let mut bare = Frames::new();
    let seen = RefCell::new(None);
    bare.idle(2, |ui| {
        Panel::new().show(ui, |_, cx| *seen.borrow_mut() = Some(cx.shell().is_none()));
    });
    assert_eq!(*seen.borrow(), Some(true), "outside a shell a body has none");
}
