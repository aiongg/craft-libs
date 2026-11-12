//! [`GpuAccess`]: a device, its queue, and what everything that draws or computes on it needs.

use crate::env;
use crate::health::DeviceHealth;
use crate::limits::{DeviceRequest, device_descriptor};
use crate::software::software_rasterizer;

/// A wgpu device with its queue, adapter and health. Every GPU user in the suite (the canvas,
/// the compositor, effects, 3D, bakers, export) takes one of these instead of a bare device, so
/// faults reach it through the device's one [`DeviceHealth`].
///
/// The window's device comes from eframe ([`GpuAccess::adopt`]); a device of its own, for a
/// command-line tool, a test, a worker or an export, from [`GpuAccess::request`] or
/// [`GpuAccess::headless`]. Cheap to clone; clones share the device.
#[derive(Clone, Debug)]
pub struct GpuAccess {
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    info: wgpu::AdapterInfo,
    limits: wgpu::Limits,
    health: DeviceHealth,
    software: Option<String>,
}

impl GpuAccess {
    /// Take a device someone else created (the window's, from eframe's render state) and watch it:
    /// this installs the device's one fault handler ([`DeviceHealth::watch`]).
    pub fn adopt(adapter: wgpu::Adapter, device: wgpu::Device, queue: wgpu::Queue) -> Self {
        let health = DeviceHealth::watch(&device);
        let info = adapter.get_info();
        let software = software_rasterizer(&info);
        let limits = device.limits();
        Self { adapter, device, queue, info, limits, health, software }
    }

    /// A device of its own on the best adapter, without blocking (a browser worker: WebGPU only;
    /// natively any backend). The instance honours wgpu's variables ([`env`]); [`env::ADAPTER_NAME`]
    /// picks the adapter by name, else [`env::POWER_PREF`] does, high performance by default.
    /// `Err` says why there is none.
    pub async fn request(req: &DeviceRequest) -> Result<Self, String> {
        let instance = wgpu::Instance::new(instance_descriptor());
        let (adapter, by_name) = match adapter_by_name(&instance).await {
            Some(found) => (found?, true),
            None => {
                let options = wgpu::RequestAdapterOptions {
                    power_preference: env::power_preference().unwrap_or(wgpu::PowerPreference::HighPerformance),
                    ..Default::default()
                };
                (instance.request_adapter(&options).await.map_err(|e| format!("no graphics adapter: {e}"))?, false)
            }
        };
        let (device, queue) = adapter.request_device(&device_descriptor(&adapter, req)).await.map_err(|e| match by_name {
            true => format!("no device on {} (picked by {}): {e}", adapter.get_info().name, env::ADAPTER_NAME),
            false => format!("no device: {e}"),
        })?;
        // This path owns the device: its handler is installed once the device exists.
        Ok(Self::adopt(adapter, device, queue))
    }

    /// [`Self::request`], blocking (command-line tools, tests, benchmarks, export). `None`, with
    /// the reason in the log, when no adapter qualifies.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn headless(req: &DeviceRequest) -> Option<Self> {
        pollster::block_on(Self::request(req)).map_err(|e| log::info!("gpu: {e}")).ok()
    }

    /// The device.
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// The device's queue.
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// The adapter the device was created on.
    pub fn adapter(&self) -> &wgpu::Adapter {
        &self.adapter
    }

    /// The adapter's name, backend, type and ids.
    pub fn info(&self) -> &wgpu::AdapterInfo {
        &self.info
    }

    /// The limits the device was created with.
    pub fn limits(&self) -> &wgpu::Limits {
        &self.limits
    }

    /// The features the device was created with.
    pub fn features(&self) -> wgpu::Features {
        self.device.features()
    }

    /// The device's health: check it before issuing work, subscribe to hear of faults.
    pub fn health(&self) -> &DeviceHealth {
        &self.health
    }

    /// A description when the adapter is a software rasterizer ([`software_rasterizer`]).
    pub fn software(&self) -> Option<&str> {
        self.software.as_deref()
    }
}

/// The instance for a device of its own: the native backends (Vulkan, Metal, DX12), and OpenGL
/// except on Windows, unless [`env::BACKEND`] names others; wgpu's variables, without the
/// indirect-call validation ([`env::instance_flags`]); on Windows, the DirectX 12 compiler of
/// [`env::dx12_compiler`]. WebGPU only in the browser.
///
/// OpenGL stays out on Windows because it starts a WGL context thread inside the vendor driver
/// (NVIDIA's `nvoglv64` is both its GL driver and its Vulkan ICD), and instances created and
/// devices dropped on several threads at once then deadlocked in that driver (FilmCraft #3418).
fn instance_descriptor() -> wgpu::InstanceDescriptor {
    #[allow(unused_mut)]
    let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
    #[cfg(not(target_arch = "wasm32"))]
    {
        // `with_env` reads WGPU_BACKEND, WGPU_DX12_COMPILER, WGPU_VALIDATION and WGPU_DEBUG.
        desc = desc.with_env();
        desc.backends = env::backends().unwrap_or(if cfg!(windows) { wgpu::Backends::PRIMARY } else { wgpu::Backends::PRIMARY | wgpu::Backends::GL });
        desc.flags = env::instance_flags(desc.flags);
        if cfg!(windows) && std::env::var_os(env::DX12_COMPILER).is_none() {
            let exe = std::env::current_exe().ok();
            desc.backend_options.dx12.shader_compiler = env::dx12_compiler(exe.as_deref().and_then(std::path::Path::parent));
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        desc.backends = wgpu::Backends::BROWSER_WEBGPU;
    }
    desc
}

/// The adapter [`env::ADAPTER_NAME`] names ([`env::adapter_matches`]); `None` when it is unset.
/// `Err` lists each distinct adapter once as `name (backend)`, so a typo or an OS- or
/// driver-specific name is easy to spot.
#[cfg(not(target_arch = "wasm32"))]
async fn adapter_by_name(instance: &wgpu::Instance) -> Option<Result<wgpu::Adapter, String>> {
    let wanted = env::adapter_name()?;
    let mut all = instance.enumerate_adapters(wgpu::Backends::all()).await;
    let listed: Vec<(String, String)> = all
        .iter()
        .map(|a| {
            let i = a.get_info();
            (i.name, format!("{:?}", i.backend))
        })
        .collect();
    Some(match_adapter_name(&wanted, &listed).map(|i| all.swap_remove(i)))
}

#[cfg(target_arch = "wasm32")]
async fn adapter_by_name(_: &wgpu::Instance) -> Option<Result<wgpu::Adapter, String>> {
    None
}

/// The index of the first adapter in `adapters` (name, backend) that `wanted` names.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn match_adapter_name(wanted: &str, adapters: &[(String, String)]) -> Result<usize, String> {
    if adapters.is_empty() {
        return Err(format!("{}={wanted:?}: no adapters on the enabled backends", env::ADAPTER_NAME));
    }
    adapters.iter().position(|(n, _)| env::adapter_matches(n, wanted)).ok_or_else(|| {
        let mut list: Vec<String> = Vec::new();
        for l in adapters.iter().map(|(n, b)| format!("{n} ({b})")) {
            if !list.contains(&l) {
                list.push(l);
            }
        }
        format!(
            "no adapter name matches {}={wanted:?} (matched against the name only; select a backend with {}); available: [{}]",
            env::ADAPTER_NAME,
            env::BACKEND,
            list.join(", ")
        )
    })
}
