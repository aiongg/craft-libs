//! Software adapters: wgpu running entirely on the CPU.

/// A description when `info` is a software rasterizer (llvmpipe, lavapipe, SwiftShader, softpipe,
/// WARP: VMs and containers without GPU device access, missing or broken drivers); `None` for a
/// GPU. Compositing through one is slower than a native CPU compositor and hides the cause: GPU
/// usage stays at 0 % while the CPU saturates (FilmCraft). Matched by device type, and by name for
/// the rasterizers that report another type.
pub fn software_rasterizer(info: &wgpu::AdapterInfo) -> Option<String> {
    rasterizer_reason(&info.name, info.device_type)
}

fn rasterizer_reason(name: &str, device_type: wgpu::DeviceType) -> Option<String> {
    is_software(name, device_type).then(|| format!("software rasterizer ({name}, {device_type:?})"))
}

/// Whether an adapter of this name and type is a software rasterizer ([`software_rasterizer`]).
pub(crate) fn is_software(name: &str, device_type: wgpu::DeviceType) -> bool {
    let lower = name.to_ascii_lowercase();
    device_type == wgpu::DeviceType::Cpu || ["llvmpipe", "swiftshader", "softpipe"].iter().any(|s| lower.contains(s))
}
