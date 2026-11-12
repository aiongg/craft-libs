//! What a device asks its adapter for.
//!
//! eframe asks for egui's default limits, which cap textures at 8192 px and buffers at 256 MiB
//! whatever the GPU offers. A window across monitors that add up to more (3440 + 3840 + 1080 px
//! side by side, or 9048×5757 after a mixed-DPI drag) then failed `Surface::configure`
//! (VectorCraft #1459, DesignCraft), a large document went to the CPU compositor (PhotoCraft), and a
//! frame with 137 MB of vertices panicked egui-wgpu when it asked for a 275 MB buffer
//! (DesignCraft). And a GPU short of any default limit got no device at all (VectorCraft #2105). So
//! a device asks for the defaults lowered to what the adapter offers, with the adapter's own largest
//! texture side and buffer.

/// How much a device asks for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LimitsTier {
    /// The defaults (WebGL2's on OpenGL), lowered to the adapter's, with the adapter's own largest
    /// texture side and buffer.
    #[default]
    Adapter,
    /// Only what OpenGL ES 3 hardware has (WebGL2's defaults), with the adapter's texture side: the
    /// last hardware start, for a driver that fails with more (FigCraft).
    Downlevel,
}

/// The limits to ask the device for, given the adapter's `backend` and `supported` limits.
pub fn device_limits(backend: wgpu::Backend, supported: &wgpu::Limits, tier: LimitsTier) -> wgpu::Limits {
    match tier {
        LimitsTier::Adapter => {
            let base = if backend == wgpu::Backend::Gl { wgpu::Limits::downlevel_webgl2_defaults() } else { wgpu::Limits::default() };
            let base = base.or_worse_values_from(supported);
            wgpu::Limits { max_texture_dimension_2d: supported.max_texture_dimension_2d, max_buffer_size: supported.max_buffer_size, ..base }
        }
        LimitsTier::Downlevel => wgpu::Limits::downlevel_webgl2_defaults().using_resolution(supported.clone()),
    }
}

/// What a device is created with, besides its adapter.
#[derive(Clone, Debug)]
pub struct DeviceRequest {
    /// The device's label in wgpu's messages.
    pub label: &'static str,
    /// How much it asks for ([`device_limits`]).
    pub tier: LimitsTier,
    /// Features asked for only when the adapter has all of them (timestamp queries for GPU
    /// timing): asking for one it lacks fails device creation, and a device without them is far
    /// better than none.
    pub optional_features: wgpu::Features,
    /// How the allocator trades memory for speed. A device that owns many short-lived textures
    /// asks for [`wgpu::MemoryHints::MemoryUsage`], so freed textures give their blocks back
    /// (EffectCraft #879).
    pub memory_hints: wgpu::MemoryHints,
}

impl DeviceRequest {
    /// A request for a device of its own (compositing, effects, export, tests): adapter limits,
    /// no optional features, [`wgpu::MemoryHints::MemoryUsage`].
    pub fn headless(label: &'static str) -> Self {
        Self { label, tier: LimitsTier::Adapter, optional_features: wgpu::Features::empty(), memory_hints: wgpu::MemoryHints::MemoryUsage }
    }
}

/// The descriptor for `req` on `adapter`.
pub fn device_descriptor(adapter: &wgpu::Adapter, req: &DeviceRequest) -> wgpu::DeviceDescriptor<'static> {
    let optional = if adapter.features().contains(req.optional_features) { req.optional_features } else { wgpu::Features::empty() };
    wgpu::DeviceDescriptor {
        label: Some(req.label),
        required_limits: device_limits(adapter.get_info().backend, &adapter.limits(), req.tier),
        required_features: optional,
        memory_hints: req.memory_hints.clone(),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A GL adapter weaker than wgpu's defaults still gets a device (VectorCraft #2105), and every
    /// adapter's own texture side and buffer size are asked for, above egui's 8192 px and 256 MiB
    /// (VectorCraft #1459, DesignCraft): the egui fork's `surface_fit` only scales a surface down to
    /// the device's side, it doesn't raise egui's request.
    #[test]
    fn requested_limits_fit_the_adapter() {
        let weak_gl =
            wgpu::Limits { max_texture_dimension_2d: 4096, max_uniform_buffer_binding_size: 16 << 10, ..wgpu::Limits::downlevel_webgl2_defaults() };
        let asked = device_limits(wgpu::Backend::Gl, &weak_gl, LimitsTier::Adapter);
        assert!(asked.check_limits(&weak_gl), "a GL device asks for no more than its adapter offers");
        assert_eq!(asked.max_texture_dimension_2d, 4096);

        let strong = wgpu::Limits { max_texture_dimension_2d: 16384, max_buffer_size: 4 << 30, ..wgpu::Limits::default() };
        let asked = device_limits(wgpu::Backend::Vulkan, &strong, LimitsTier::Adapter);
        assert!(asked.check_limits(&strong));
        assert_eq!((asked.max_texture_dimension_2d, asked.max_buffer_size), (16384, 4 << 30));
        assert!(asked.max_bind_groups >= wgpu::Limits::default().max_bind_groups, "a capable adapter keeps the backend defaults");

        let low = device_limits(wgpu::Backend::Metal, &strong, LimitsTier::Downlevel);
        assert!(low.check_limits(&strong));
        assert!(low.max_storage_buffers_per_shader_stage <= wgpu::Limits::downlevel_webgl2_defaults().max_storage_buffers_per_shader_stage);
    }
}
