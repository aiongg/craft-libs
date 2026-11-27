//! The runtime's dependency boundary: windows, GPUs, text shaping and egui live in the renderer and
//! platform crates, so nothing this crate links may be one of them. `cargo tree -p` resolves the
//! features of this crate alone, as an app that depends only on it would build.

use std::process::Command;

/// Packages the runtime must not link, directly or through a dependency.
const DENIED: &[&str] = &[
    "egui",
    "eframe",
    "epaint",
    "egui-winit",
    "egui-wgpu",
    "egui_kittest",
    "winit",
    "wgpu",
    "skia-safe",
    "skia-bindings",
    "parley",
    "vello",
    "softbuffer",
    "accesskit_winit",
];

#[test]
fn the_runtime_links_no_window_gpu_text_or_egui_crate() {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let out = Command::new(cargo)
        .args(["tree", "--offline", "-p", "craft-ui-rt", "-e", "normal", "--prefix", "none", "--format", "{lib}"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("cargo tree runs");
    assert!(out.status.success(), "cargo tree failed: {}", String::from_utf8_lossy(&out.stderr));
    let tree = String::from_utf8_lossy(&out.stdout);
    let linked: Vec<&str> = tree.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    assert!(linked.contains(&"craft_ui_rt"), "the tree lists the crate itself: {linked:?}");
    let denied: Vec<&str> = DENIED.iter().map(|d| d.trim()).filter(|d| linked.contains(&d.replace('-', "_").as_str())).collect();
    assert!(denied.is_empty(), "craft-ui-rt links {denied:?}");
}
