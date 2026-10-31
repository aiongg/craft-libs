//! The scrub's pointer warp.

use std::sync::{Arc, Mutex};

use egui::{Context, Pos2, RawInput, ViewportCommand, pos2};

use super::Scrub;

fn cursor_commands(ctx: &Context, pos: Pos2) -> (bool, Vec<ViewportCommand>) {
    let mut moved = false;
    let output = ctx.run_ui(RawInput::default(), |ui| moved = Scrub::warp(ui.ctx(), pos));
    let commands = output.viewport_output.get(&egui::ViewportId::ROOT).map(|v| v.commands.clone()).unwrap_or_default();
    output.drop_without_applying_deltas();
    (moved, commands)
}

// After install_warp, warp calls the service with the point and returns its answer.
#[test]
fn an_installed_warp_service_moves_the_pointer() {
    let ctx = Context::default();
    let seen: Arc<Mutex<Vec<Pos2>>> = Arc::default();
    let log = seen.clone();
    Scrub::install_warp(&ctx, move |_, p| {
        log.lock().unwrap().push(p);
        p.x > 0.0
    });
    assert!(Scrub::warp(&ctx, pos2(5.0, 6.0)));
    assert!(!Scrub::warp(&ctx, pos2(-1.0, 6.0)), "the service's refusal is returned");
    assert_eq!(*seen.lock().unwrap(), [pos2(5.0, 6.0), pos2(-1.0, 6.0)]);
}

// warp sends ViewportCommand::CursorPosition and returns true.
#[test]
fn without_a_service_egui_moves_the_pointer() {
    let ctx = Context::default();
    let (moved, commands) = cursor_commands(&ctx, pos2(5.0, 6.0));
    assert!(moved);
    assert!(commands.iter().any(|c| matches!(c, ViewportCommand::CursorPosition(p) if *p == pos2(5.0, 6.0))), "{commands:?}");
}
