//! Which graphics adapter a window or device draws with.
//!
//! The canvas is rasterized on the CPU in most apps, and the GPU composites and draws the UI, so
//! any adapter that can present to the window will do; the question is which one fails least.
//! [`adapter_order`] ranks the adapters wgpu lists:
//!
//! 1. Never: those that can't present to the window's surface, those an earlier start failed or
//!    crashed on ([`Ranking::skip`]), and the software adapters a hardware attempt leaves out
//!    (or the hardware ones a software attempt leaves out, [`SoftwareRule`]).
//! 2. The adapter [`Ranking::named`] names ([`env::adapter_matches`]).
//! 3. Native backends (Vulkan, Metal, DX12) before OpenGL.
//! 4. The GPU the Wayland compositor draws with ([`compositor_gpu`]).
//! 5. The GPUs that drive a display, the primary display's first ([`display_gpus`]).
//! 6. The power preference among the rest ([`Ranking::power`]).
//! 7. A GPU's DX12 adapter before its Vulkan one (#545).
//!
//! Equals keep the system's order; then, among the entries of one GPU listed through two versions
//! of one driver, the newer version comes first.
//!
//! A GPU without a monitor is the wrong one even when it can present: on a desktop whose monitors
//! all hang off the discrete GPU, power saving picked the Ryzen's integrated GPU, Windows had to
//! present every frame across adapters, and the integrated GPU's driver reset under that, which
//! lost the graphics device, or took the desktop's compositor and every monitor down for minutes
//! (pdfcraft#378). So the GPU that drives the display the user looks at comes first, and the power
//! preference only orders the rest. A user who picked a kind of GPU (an app's Graphics Processor
//! preference, [`env::POWER_PREF`]) gets that kind: the caller then passes no displays and no
//! compositor GPU.
//!
//! Under Wayland the compositor's own GPU comes before the displays: frames from another GPU may
//! never reach the screen, with no error, even from the GPU behind the built-in panel when the
//! compositor runs on the discrete one.
//!
//! eframe is handed [`selector`], which takes the first of the order and logs every adapter and
//! the choice: which GPU draws is the first question in every black-window report.

use std::path::Path;
use std::sync::Arc;

use wgpu::{Backend, DeviceType, PowerPreference};

use crate::env;
use crate::software;

mod compositor;
mod display;
mod driver;
#[cfg(test)]
mod tests;

pub use compositor::compositor_gpu;
pub use display::{DisplayGpu, display_gpus};

/// One adapter, as [`adapter_order`] sees it.
#[derive(Clone, Debug)]
pub struct Candidate {
    /// Identifies the adapter across starts and restarts ([`key`]).
    pub key: String,
    /// The name the driver reports.
    pub name: String,
    /// The backend it is listed through.
    pub backend: Backend,
    /// What kind of device it is.
    pub device_type: DeviceType,
    /// PCI vendor and device ids, to match a [`DisplayGpu`]; zero where the backend reports none
    /// (Metal, and OpenGL's device id).
    pub pci: (u32, u32),
    /// Reports that it can present to the window's surface (always, without a surface).
    pub presents: bool,
    /// The driver's name (Vulkan) or version (DX12): one GPU listed through two versions of one
    /// driver has two entries with the same `driver` and `key`.
    pub driver: String,
    /// The features the adapter exposes; a newer version of one driver exposes more.
    pub features: wgpu::Features,
}

impl Candidate {
    /// What `adapter` reports, and whether it can present to `surface`.
    pub fn of(adapter: &wgpu::Adapter, surface: Option<&wgpu::Surface<'_>>) -> Self {
        let info = adapter.get_info();
        Self {
            key: key(info.backend, info.vendor, info.device, &info.name),
            presents: surface.is_none_or(|s| adapter.is_surface_supported(s)),
            name: info.name,
            backend: info.backend,
            device_type: info.device_type,
            pci: (info.vendor, info.device),
            driver: info.driver,
            features: adapter.features(),
        }
    }

    /// Whether it runs on the CPU ([`software::software_rasterizer`]'s rule).
    fn software(&self) -> bool {
        software::is_software(&self.name, self.device_type)
    }

    /// Whether it is the GPU the compositor draws with (`compositor`, by PCI ids).
    fn composites(&self, compositor: Option<(u32, u32)>) -> bool {
        compositor == Some(self.pci)
    }

    /// Whether it drives one of `displays`, and whether that is the primary one.
    fn drives(&self, displays: &[DisplayGpu]) -> (bool, bool) {
        let shown = displays.iter().find(|g| g.pci == self.pci);
        (shown.is_some(), shown.is_some_and(|g| g.primary))
    }

    /// Whether `rule` leaves it out.
    fn excluded_by(&self, rule: SoftwareRule) -> bool {
        match rule {
            SoftwareRule::Exclude => self.software(),
            SoftwareRule::Only => !self.software(),
        }
    }
}

/// `backend:vendor:device`, e.g. `Vulkan:1002:164e` (PCI ids, as `MESA_VK_DEVICE_SELECT` takes
/// them). Metal reports no ids (`0000:0000` for every GPU), so an adapter without them is
/// `backend:name` (`Metal:Intel Iris Pro Graphics`): otherwise leaving out the one that failed
/// would leave out every GPU of a dual-GPU Mac (#651). Commas, which separate keys in a list,
/// become spaces.
pub fn key(backend: Backend, vendor: u32, device: u32, name: &str) -> String {
    if vendor == 0 && device == 0 {
        format!("{backend:?}:{}", name.replace(',', " ").trim())
    } else {
        format!("{backend:?}:{vendor:04x}:{device:04x}")
    }
}

/// Whether an attempt takes hardware or software adapters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoftwareRule {
    /// Hardware only: a software adapter would draw every frame on the CPU while a later attempt
    /// may still find a GPU that works.
    Exclude,
    /// Software only (WARP, lavapipe, llvmpipe, SwiftShader): the attempt after every GPU failed.
    Only,
}

/// What [`adapter_order`] ranks by.
#[derive(Clone, Debug)]
pub struct Ranking {
    /// The power preference among adapters that neither the compositor nor a display decides.
    pub power: PowerPreference,
    /// The GPUs that drive a display ([`display_gpus`]); empty when the user chose a kind of GPU.
    pub displays: Vec<DisplayGpu>,
    /// The GPU the Wayland compositor draws with ([`compositor_gpu`]); `None` when the user chose
    /// a kind of GPU.
    pub compositor: Option<(u32, u32)>,
    /// The adapter asked for by name ([`env::adapter_name`] or an app preference), matched by
    /// words ([`env::adapter_matches`]); the others stay as fallbacks.
    pub named: Option<String>,
    /// The [`key`]s of the adapters earlier starts failed or crashed on.
    pub skip: Vec<String>,
    /// Hardware or software adapters.
    pub software: SoftwareRule,
}

/// The order to try `candidates` in (their indices): only those that present to the window, aren't
/// in [`Ranking::skip`] and fit [`Ranking::software`]; the one whose name matches
/// [`Ranking::named`] first; native backends before OpenGL; the GPU the compositor draws with; the
/// GPUs that drive one of the displays, the primary display's first; then by power preference;
/// then DX12 before Vulkan, keeping the system's order among equals (all of it for
/// [`PowerPreference::None`]). Windows lists each GPU under both, and Intel's Vulkan driver made
/// the whole window flicker black where DX12 and OpenGL didn't (#545); elsewhere there is no DX12
/// adapter. Last, one GPU listed through two versions of one driver is tried through the newer one
/// first (`amdvlk64.dll`).
///
/// With no displays, or none a candidate's PCI ids match (OpenGL reports no device id, Metal no
/// ids at all), the order is the power preference's alone.
pub fn adapter_order(candidates: &[Candidate], ranking: &Ranking) -> Vec<usize> {
    let named = ranking.named.as_deref();
    let mut order: Vec<(usize, &Candidate)> =
        candidates.iter().enumerate().filter(|(_, c)| c.presents && !ranking.skip.contains(&c.key) && !c.excluded_by(ranking.software)).collect();
    // Stable, so equals keep the system's order.
    order.sort_by_key(|(_, c)| {
        let unnamed = named.is_some_and(|n| !env::adapter_matches(&c.name, n));
        let (drives_display, drives_primary) = c.drives(&ranking.displays);
        (
            unnamed,
            c.backend == Backend::Gl,
            !c.composites(ranking.compositor),
            !drives_display,
            !drives_primary,
            power_rank(c.device_type, ranking.power),
            c.backend == Backend::Vulkan,
        )
    });
    let mut order: Vec<usize> = order.into_iter().map(|(i, _)| i).collect();
    driver::newer_versions_first(&mut order, candidates);
    order
}

/// wgpu's own ranking for a power preference: the preferred kind of GPU, the other kind, unknown,
/// virtual, software.
fn power_rank(t: DeviceType, power: PowerPreference) -> u8 {
    match (power, t) {
        (PowerPreference::None, _) => 0,
        (PowerPreference::LowPower, DeviceType::IntegratedGpu) | (PowerPreference::HighPerformance, DeviceType::DiscreteGpu) => 0,
        (_, DeviceType::IntegratedGpu | DeviceType::DiscreteGpu) => 1,
        (_, DeviceType::Other) => 2,
        (_, DeviceType::VirtualGpu) => 3,
        (_, DeviceType::Cpu) => 4,
    }
}

/// Picks the adapter eframe draws the window with, from the adapters it lists and the window's
/// surface; the shape of egui-wgpu's `NativeAdapterSelectorMethod`.
pub type Selector = Arc<dyn Fn(&[wgpu::Adapter], Option<&wgpu::Surface<'_>>) -> Result<wgpu::Adapter, String> + Send + Sync>;

/// The selector that draws on the first adapter of [`adapter_order`]. `on_pick` hears the
/// candidate picked, and whether the listing found an adapter of a backend other than OpenGL (a
/// crash while listing is then not OpenGL's alone); `on_none` hears that no adapter is left, and
/// the selector returns why. Every adapter, the displays and the choice are logged.
pub fn selector(
    ranking: Ranking,
    on_pick: impl Fn(&Candidate, bool) + Send + Sync + 'static,
    on_none: impl Fn() + Send + Sync + 'static,
) -> Selector {
    Arc::new(move |adapters, surface| {
        let candidates: Vec<Candidate> = adapters.iter().map(|a| Candidate::of(a, surface)).collect();
        let order = adapter_order(&candidates, &ranking);
        let listed: Vec<String> = candidates.iter().map(|c| describe(c, &ranking)).collect();
        log::info!("graphics adapters: {}", if listed.is_empty() { "none".to_string() } else { listed.join("; ") });
        if !ranking.displays.is_empty() {
            let displays: Vec<String> = ranking.displays.iter().map(ToString::to_string).collect();
            log::info!("GPUs driving a display: {}", displays.join(", "));
        }
        let first = order.first().and_then(|&i| adapters.get(i).zip(candidates.get(i)));
        let Some((adapter, chosen)) = first else {
            on_none();
            return Err(format!(
                "no graphics adapter can show the window (power preference {:?}, {:?} software, {} adapters)",
                ranking.power,
                ranking.software,
                adapters.len()
            ));
        };
        let duplicates = candidates.iter().filter(|c| !std::ptr::eq(*c, chosen) && c.presents && driver::same_gpu_and_driver(chosen, c)).count();
        if duplicates > 0 {
            log::warn!(
                "{:?} lists \"{}\" {} times: several versions of its driver are installed, and a window drawn through a stale one can crash. Using the one with the most features; installing one driver package for every GPU removes the others.",
                chosen.backend,
                chosen.name.trim(),
                duplicates.saturating_add(1)
            );
        }
        log::info!("drawing on {} ({:?}, {:?}): {}", chosen.name.trim(), chosen.backend, chosen.device_type, why_chosen(chosen, &ranking));
        on_pick(chosen, candidates.iter().any(|c| c.backend != Backend::Gl));
        Ok(adapter.clone())
    })
}

/// One adapter for the log: name, backend, kind, key, and why it is left out or preferred.
fn describe(c: &Candidate, ranking: &Ranking) -> String {
    let (drives_display, drives_primary) = c.drives(&ranking.displays);
    let note = if !c.presents {
        ", can't show the window"
    } else if ranking.skip.contains(&c.key) {
        ", an earlier start failed on it"
    } else if c.excluded_by(ranking.software) {
        if c.software() { ", software, left out" } else { ", not software, left out" }
    } else if c.composites(ranking.compositor) {
        ", the compositor draws with it"
    } else if drives_primary {
        ", drives the primary display"
    } else if drives_display {
        ", drives a display"
    } else {
        ""
    };
    format!("{} ({:?}, {:?}, {}{note})", c.name.trim(), c.backend, c.device_type, c.key)
}

/// Why [`adapter_order`] put `chosen` first, for the log.
fn why_chosen(chosen: &Candidate, ranking: &Ranking) -> String {
    let power = ranking.power;
    match chosen.drives(&ranking.displays) {
        _ if ranking.named.as_deref().is_some_and(|n| env::adapter_matches(&chosen.name, n)) => "its name was asked for".to_string(),
        _ if chosen.composites(ranking.compositor) => "the compositor draws with it".to_string(),
        (_, true) => "it drives the primary display".to_string(),
        (true, false) => "it drives a display".to_string(),
        (false, false) if ranking.displays.is_empty() => format!("power preference {power:?}"),
        (false, false) => format!("no adapter matches a display GPU, so power preference {power:?}"),
    }
}

/// A PCI id from a sysfs file such as `/sys/class/drm/card0/device/vendor` (`0x10de`).
pub(crate) fn read_sysfs_id(path: &Path) -> Option<u32> {
    let s = std::fs::read_to_string(path).ok()?;
    u32::from_str_radix(s.trim().trim_start_matches("0x"), 16).ok()
}
