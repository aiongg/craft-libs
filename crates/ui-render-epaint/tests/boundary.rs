//! The renderer links the GPU and text stack but no window system or egui itself: the platform
//! hands over raw window handles.

use std::process::Command;

const DENIED: &[&str] = &["egui", "eframe", "egui-winit", "egui_kittest", "winit", "accesskit_winit", "skia-safe", "skia-bindings"];

#[test]
fn the_renderer_links_no_window_system_or_egui() {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let out = Command::new(cargo)
        .args(["tree", "--offline", "-p", "craft-ui-render-epaint", "-e", "normal", "--prefix", "none", "--format", "{lib}"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("cargo tree runs");
    assert!(out.status.success(), "cargo tree failed: {}", String::from_utf8_lossy(&out.stderr));
    let tree = String::from_utf8_lossy(&out.stdout);
    let linked: Vec<&str> = tree.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    for needed in ["craft_ui_rt", "epaint", "egui_wgpu", "wgpu", "raw_window_handle"] {
        assert!(linked.contains(&needed), "the tree lists {needed}: {linked:?}");
    }
    let denied: Vec<&str> = DENIED.iter().copied().filter(|d| linked.contains(&d.replace('-', "_").as_str())).collect();
    assert!(denied.is_empty(), "craft-ui-render-epaint links {denied:?}");
}
