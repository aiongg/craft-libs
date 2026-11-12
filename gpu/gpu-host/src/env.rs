//! The one set of environment variables that steers the GPU, for every app.
//!
//! The user-facing names are wgpu's own, so a variable a user already knows from wgpu, or from
//! another Craft app, means the same everywhere:
//!
//! | Variable | Effect |
//! |---|---|
//! | [`BACKEND`] (`WGPU_BACKEND`) | `dx12`, `vulkan`, `metal`, `gl` or a comma list: one start with these backends; the start-up record is left alone |
//! | [`ADAPTER_NAME`] (`WGPU_ADAPTER_NAME`) | the adapter whose name holds every word of the value (any case) comes first ([`adapter_matches`]) |
//! | [`POWER_PREF`] (`WGPU_POWER_PREF`) | `low`, `high` or `none`: the power preference among adapters that drive no display |
//! | [`DX12_COMPILER`] (`WGPU_DX12_COMPILER`) | the DirectX 12 shader compiler ([`dx12_compiler`]) |
//! | [`VALIDATION_INDIRECT_CALL`] (`WGPU_VALIDATION_INDIRECT_CALL`) | `1` turns wgpu's check of indirect-call arguments back on ([`instance_flags`]) |
//! | [`VRAM_MB`] (`CRAFT_GPU_VRAM_MB`) | development: report a discrete card of this many MiB (`memory::video_memory`) |
//!
//! Physical memory has its own override, `CRAFT_RAM_MB`, in `craft_platform::usage`. The start-up
//! ladder passes its state to a restarted process as an argument, not as a variable: a variable
//! can't be removed from the process again without `unsafe`, and helper processes would inherit it.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

/// wgpu's variable naming the backends to use.
pub const BACKEND: &str = "WGPU_BACKEND";
/// wgpu's variable naming the adapter to use, by (part of) its name.
pub const ADAPTER_NAME: &str = "WGPU_ADAPTER_NAME";
/// wgpu's variable for the power preference.
pub const POWER_PREF: &str = "WGPU_POWER_PREF";
/// wgpu's variable for the DirectX 12 shader compiler.
pub const DX12_COMPILER: &str = "WGPU_DX12_COMPILER";
/// wgpu's variable that turns the indirect-call validation back on.
pub const VALIDATION_INDIRECT_CALL: &str = "WGPU_VALIDATION_INDIRECT_CALL";
/// The video memory, in MiB, that `memory::video_memory` reports for any adapter.
pub const VRAM_MB: &str = "CRAFT_GPU_VRAM_MB";

/// The longest adapter name taken from the environment or a preference.
const MAX_NAME: usize = 256;

/// The backends [`BACKEND`] names; `None` when it is unset.
pub fn backends() -> Option<wgpu::Backends> {
    wgpu::Backends::from_env()
}

/// The adapter name [`ADAPTER_NAME`] asks for, trimmed; `None` when unset, empty or only
/// whitespace (an empty name would match the first adapter and skip the power preference).
pub fn adapter_name() -> Option<String> {
    wanted_name(std::env::var(ADAPTER_NAME).ok().as_deref())
}

fn wanted_name(var: Option<&str>) -> Option<String> {
    var.map(str::trim).filter(|w| !w.is_empty()).map(|w| w.chars().take(MAX_NAME).collect())
}

/// Whether an adapter called `name` is the one `wanted` asks for: every whitespace-separated word
/// of `wanted` appears in the name, ignoring case ("nvidia" finds "NVIDIA GeForce RTX 4070",
/// "arc a370m" finds "Intel(R) Arc(tm) A370M Graphics"). An empty `wanted` matches nothing.
pub fn adapter_matches(name: &str, wanted: &str) -> bool {
    let name = name.to_lowercase();
    let mut words = wanted.split_whitespace().peekable();
    words.peek().is_some() && words.all(|w| name.contains(&w.to_lowercase()))
}

/// The power preference [`POWER_PREF`] names; `None` when it is unset or unknown. wgpu ignores an
/// unknown value silently, so this warns once, and a typo is noticed.
pub fn power_preference() -> Option<wgpu::PowerPreference> {
    static WARNED: AtomicBool = AtomicBool::new(false);
    let pref = wgpu::PowerPreference::from_env();
    if pref.is_none()
        && let Ok(v) = std::env::var(POWER_PREF)
        && !WARNED.swap(true, Ordering::Relaxed)
    {
        log::warn!("gpu: ignoring {POWER_PREF}={v:?} (expected low, high or none)");
    }
    pref
}

/// Whether [`BACKEND`] or [`ADAPTER_NAME`] is set: then a start uses what they name, and leaves out
/// no adapter or backend that earlier starts failed on.
pub fn explicit_choice() -> bool {
    std::env::var_os(BACKEND).is_some() || std::env::var_os(ADAPTER_NAME).is_some()
}

/// The DirectX 12 shader compiler (PhotoCraft #712). wgpu's default loads `dxcompiler.dll` by
/// name, and the Windows DLL search falls through to the current directory and `PATH`, so another
/// program's DXC build (a browser's, an SDK's) got loaded and failed device creation or crashed the
/// first shader compile (GridCraft #339, #354; FigCraft #965). So: a `dxcompiler.dll` shipped beside
/// the executable (`exe_dir`), by its full path; otherwise FXC (`d3dcompiler_47.dll`, part of
/// Windows). [`DX12_COMPILER`] still picks one explicitly.
pub fn dx12_compiler(exe_dir: Option<&Path>) -> wgpu::Dx12Compiler {
    dx12_compiler_for(std::env::var(DX12_COMPILER).ok().as_deref(), exe_dir)
}

fn dx12_compiler_for(env: Option<&str>, exe_dir: Option<&Path>) -> wgpu::Dx12Compiler {
    if let Some(c) = env.and_then(|v| v.trim().parse().ok()) {
        return c;
    }
    let beside_exe = exe_dir.map(|d| d.join("dxcompiler.dll")).filter(|p| p.is_file());
    match beside_exe.as_deref().and_then(Path::to_str) {
        Some(p) => wgpu::Dx12Compiler::DynamicDxc { dxc_path: p.to_string() },
        None => wgpu::Dx12Compiler::Fxc,
    }
}

/// `flags` without wgpu's check of indirect-call arguments, then wgpu's own variables applied
/// (`WGPU_VALIDATION`, `WGPU_DEBUG`, [`VALIDATION_INDIRECT_CALL`]). No Craft app draws or dispatches
/// indirectly, and the check costs a compute shader at device creation that Metal can't compile on
/// older Intel Macs (Iris Pro: PhotoCraft #3107, DesignCraft #334, FigCraft #1438): the device was
/// lost and the window never opened.
pub fn instance_flags(flags: wgpu::InstanceFlags) -> wgpu::InstanceFlags {
    flags.difference(wgpu::InstanceFlags::VALIDATION_INDIRECT_CALL).with_env()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PhotoCraft #712: a `dxcompiler.dll` found through the DLL search path is never loaded; one
    /// beside the executable is taken by its full path, and the variable still wins.
    #[test]
    fn dx12_compiler_is_fxc_or_dxc_beside_the_exe() {
        let dir = std::env::temp_dir().join(format!("craft-gpu-host-dxc-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(matches!(dx12_compiler_for(None, Some(&dir)), wgpu::Dx12Compiler::Fxc));
        assert!(matches!(dx12_compiler_for(None, None), wgpu::Dx12Compiler::Fxc));
        std::fs::write(dir.join("dxcompiler.dll"), b"").unwrap();
        match dx12_compiler_for(None, Some(&dir)) {
            wgpu::Dx12Compiler::DynamicDxc { dxc_path } => assert!(Path::new(&dxc_path).starts_with(&dir)),
            other => panic!("expected the DXC beside the executable, got {other:?}"),
        }
        assert!(matches!(dx12_compiler_for(Some("fxc"), Some(&dir)), wgpu::Dx12Compiler::Fxc));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A requested adapter is found by its words, in any order and case; an empty request picks
    /// nothing, so the automatic ranking stays in charge.
    #[test]
    fn adapter_names_match_loosely_and_an_empty_request_matches_nothing() {
        assert!(adapter_matches("Intel(R) Arc(tm) A370M Graphics", "arc a370m"));
        assert!(adapter_matches("NVIDIA GeForce RTX 4070", "4070 nvidia"));
        assert!(!adapter_matches("NVIDIA GeForce RTX 4070", "nvidia 4080"));
        assert!(!adapter_matches("NVIDIA GeForce RTX 4070", "  "));
        assert_eq!(wanted_name(Some("  ")), None);
    }
}
