//! The rungs a window's graphics start tries, one table per platform, and what each one starts
//! with.
//!
//! | Platform | Rungs, in order |
//! |---|---|
//! | Windows | [`Rung::Dx12`] → [`Rung::Vulkan`] → [`Rung::Gl`] → [`Rung::Downlevel`] → [`Rung::Software`] (WARP) |
//! | Windows, DirectX 12 lists only WARP | [`Rung::Dx12`] → [`Rung::Vulkan`] → [`Rung::Software`] → [`Rung::Gl`] → [`Rung::Downlevel`] |
//! | Linux, BSD | [`Rung::Native`] (Vulkan and OpenGL) → [`Rung::Gl`] → [`Rung::Downlevel`] → [`Rung::Software`] (lavapipe, llvmpipe) |
//! | macOS | [`Rung::Metal`] → [`Rung::Downlevel`] |
//!
//! Windows starts on DirectX 12 alone: creating a Vulkan instance loads every installed Vulkan
//! driver into the process, and a faulty one crashed the app before its window appeared (Intel's
//! `igvk64.dll`: VectorCraft #806, FilmCraft #512, DesignCraft #120). OpenGL comes after Vulkan:
//! creating its instance crashed inside AMD's `atio6axx.dll` (FilmCraft #477, DesignCraft #167).
//! Without a DirectX 12 GPU, WARP comes before OpenGL: in a VMware VM the SVGA3D OpenGL adapter
//! drew a black window, with no error to move on from (PhotoCraft #5191, #1788), and slow but
//! correct beats black. A GPU with only a DirectX 11 driver and a working OpenGL one (VectorCraft)
//! gets OpenGL once WARP fails, or when `WGPU_ADAPTER_NAME` names it.
//! FilmCraft's low-power rung (#2553) is gone: after a failure the next start leaves the adapter
//! out, which moves to the other GPU. FilmCraft's rule for Intel gen9–11 and PhotoCraft's DirectX
//! 12 first rule for Intel and Qualcomm GPUs never apply, as Windows starts on DirectX 12 anyway.
//!
//! Every rung: wgpu's check of indirect-call arguments off ([`env::instance_flags`]); on Windows,
//! the DirectX 12 compiler of [`env::dx12_compiler`]. The hardware rungs leave software adapters
//! out, [`Rung::Software`] takes only them, so a software renderer draws only once no GPU could.

use std::sync::OnceLock;

use crate::adapter::{self, Ranking, Selector, SoftwareRule};
use crate::env;
use crate::limits::{DeviceRequest, LimitsTier};

use super::GraphicsStart;

/// One way to start the window's graphics device.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Rung {
    /// DirectX 12 alone (Windows), with OpenGL beside it only when an adapter is left out or
    /// named ([`gl_beside`]).
    Dx12,
    /// Vulkan alone (Windows).
    Vulkan,
    /// OpenGL alone.
    Gl,
    /// Vulkan, with OpenGL beside it (Linux, BSD).
    Native,
    /// Metal (macOS).
    Metal,
    /// The platform's own backend on the low-power GPU, asking only for what OpenGL ES 3 hardware
    /// has: the last hardware start, for drivers that fail with more (FigCraft #965, #1438).
    Downlevel,
    /// A software renderer: WARP on Windows, lavapipe or llvmpipe elsewhere.
    Software,
}

const RUNGS: [Rung; 7] = [Rung::Dx12, Rung::Vulkan, Rung::Gl, Rung::Native, Rung::Metal, Rung::Downlevel, Rung::Software];

impl Rung {
    /// The name in the start-up record and on the command line (`--graphics <name>`).
    pub fn name(self) -> &'static str {
        match self {
            Rung::Dx12 => "dx12",
            Rung::Vulkan => "vulkan",
            Rung::Gl => "gl",
            Rung::Native => "native",
            Rung::Metal => "metal",
            Rung::Downlevel => "downlevel",
            Rung::Software => "software",
        }
    }

    /// The English name shown to the user (translated by the app).
    pub fn label(self) -> &'static str {
        match self {
            Rung::Dx12 => "DirectX 12",
            Rung::Vulkan => "Vulkan",
            Rung::Gl => "OpenGL",
            Rung::Native => "Vulkan or OpenGL",
            Rung::Metal => "Metal",
            Rung::Downlevel => "basic graphics",
            Rung::Software => "software rendering",
        }
    }

    /// The rung [`Self::name`] names.
    pub fn parse(name: &str) -> Option<Self> {
        RUNGS.into_iter().find(|r| r.name() == name)
    }

    /// How much the device asks for.
    fn tier(self) -> LimitsTier {
        if self == Rung::Downlevel { LimitsTier::Downlevel } else { LimitsTier::Adapter }
    }

    /// Which adapters the rung may draw with.
    pub(crate) fn software_rule(self) -> SoftwareRule {
        if self == Rung::Software { SoftwareRule::Only } else { SoftwareRule::Exclude }
    }
}

/// A platform's ladder; [`OS`] is the one this build runs on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Os {
    Windows,
    Mac,
    Other,
}

pub(crate) const OS: Os = if cfg!(windows) {
    Os::Windows
} else if cfg!(target_os = "macos") {
    Os::Mac
} else {
    Os::Other
};

/// This platform's rungs, in the order a start tries them. On Windows the order depends on whether
/// DirectX 12 lists a hardware adapter, which is asked once per process, the first time the order
/// matters: after a rung failed, when DirectX 12 is loaded already. Every order of a platform holds
/// the same rungs under the same top one, so the rung a launch starts on never needs the answer.
pub fn ladder() -> &'static [Rung] {
    ladder_for(OS, dx12_lists_hardware)
}

/// The ladder without asking DirectX 12: the order a test start ([`crate::probe`]) learned, else
/// the order for a DirectX 12 GPU. The probe exists so that the parent process loads no driver.
pub(crate) fn ladder_known() -> &'static [Rung] {
    ladder_for(OS, || DX12_HARDWARE.get().copied().unwrap_or(true))
}

/// `os`'s ladder; `dx12_hardware` is asked only on Windows.
pub(crate) fn ladder_for(os: Os, dx12_hardware: impl FnOnce() -> bool) -> &'static [Rung] {
    match os {
        Os::Windows if dx12_hardware() => &[Rung::Dx12, Rung::Vulkan, Rung::Gl, Rung::Downlevel, Rung::Software],
        Os::Windows => &[Rung::Dx12, Rung::Vulkan, Rung::Software, Rung::Gl, Rung::Downlevel],
        Os::Mac => &[Rung::Metal, Rung::Downlevel],
        Os::Other => &[Rung::Native, Rung::Gl, Rung::Downlevel, Rung::Software],
    }
}

/// The rung every start on this platform begins with, unless a record, restart or choice says
/// otherwise.
pub(crate) fn top() -> Rung {
    ladder_known().first().copied().unwrap_or(Rung::Software)
}

/// Whether `rung` is on this platform's ladder (in any of its orders).
pub(crate) fn on_ladder(rung: Rung) -> bool {
    ladder_known().contains(&rung)
}

/// The rung to try after `current` failed, given the rungs this start already `tried`: the next
/// one down `ladder` not tried yet, else the first untried one from the top, else none. A saved
/// fallback that no longer works (software rendering where no software adapter is installed) goes
/// back up instead of ending every later start (FilmCraft #2581).
pub(crate) fn next_untried(ladder: &[Rung], current: Rung, tried: &[Rung]) -> Option<Rung> {
    let after = ladder.iter().position(|r| *r == current).map_or(0, |i| i.saturating_add(1));
    ladder.iter().skip(after).chain(ladder.iter().take(after)).copied().find(|r| *r != current && !tried.contains(r))
}

/// The backends `rung` creates its instance with on `os`. `gl_beside` adds OpenGL to Windows'
/// DirectX 12 rung ([`gl_beside`]).
pub(crate) fn backends_for(rung: Rung, os: Os, gl_beside: bool) -> wgpu::Backends {
    use wgpu::Backends as B;
    let primary = match os {
        Os::Windows => B::DX12,
        Os::Mac => B::METAL,
        Os::Other => B::VULKAN | B::GL,
    };
    match rung {
        Rung::Dx12 if gl_beside => B::DX12 | B::GL,
        Rung::Dx12 => B::DX12,
        Rung::Vulkan => B::VULKAN,
        Rung::Gl => B::GL,
        Rung::Metal => B::METAL,
        // WARP is a DirectX 12 adapter; lavapipe a Vulkan one and llvmpipe an OpenGL one, and
        // Vulkan stays beside OpenGL for a window OpenGL can't present to (PhotoCraft #1337).
        Rung::Native | Rung::Downlevel | Rung::Software => primary,
    }
}

/// Whether Windows' DirectX 12 rung lists OpenGL's adapters too: only when it leaves an adapter
/// out (`leaving_out`: an earlier start crashed on it, or a restart failed on it), or when
/// [`env::ADAPTER_NAME`] names one (`named`; it may be an OpenGL adapter). wgpu's OpenGL backend
/// keeps a hidden window on a thread that pumps no messages, and Windows waits on it when the input
/// language changes, so the whole app hung on a switch to another keyboard layout (VectorCraft
/// #4150, gfx-rs/wgpu#10504), even drawing with DirectX 12. A machine whose DirectX 12 lists no
/// hardware adapter reaches OpenGL through its own rung, after WARP ([`ladder`]).
pub(crate) fn gl_beside(leaving_out: bool, named: bool) -> bool {
    leaving_out || named
}

/// Whether DirectX 12 lists a hardware adapter (WARP doesn't count), once known.
static DX12_HARDWARE: OnceLock<bool> = OnceLock::new();

/// What a test start learned about DirectX 12's adapters ([`crate::probe`]), so this process
/// never lists them itself.
pub(crate) fn learned_dx12_hardware(hardware: bool) {
    let _ = DX12_HARDWARE.set(hardware);
}

/// Whether DirectX 12 lists a hardware adapter; asked once per process.
fn dx12_lists_hardware() -> bool {
    *DX12_HARDWARE.get_or_init(|| {
        if !cfg!(windows) {
            return false;
        }
        let instance = wgpu::Instance::new(crate::access::instance_descriptor(wgpu::Backends::DX12));
        let kinds: Vec<wgpu::DeviceType> =
            pollster::block_on(instance.enumerate_adapters(wgpu::Backends::DX12)).iter().map(|a| a.get_info().device_type).collect();
        log::info!("gpu: DirectX 12 lists these adapters: {kinds:?}");
        kinds.iter().any(|k| *k != wgpu::DeviceType::Cpu)
    })
}

/// The preference used when neither [`env::POWER_PREF`] nor the app's Graphics Processor
/// preference names one. Windows and macOS show frames from any GPU, and the integrated one avoids
/// the flicker of presenting a discrete GPU's frames through it (VectorCraft #306). Elsewhere the
/// system's own order: Mesa's device-select layer puts the GPU the desktop runs on first, and a
/// Wayland compositor may not show frames from another GPU (VectorCraft #502).
const AUTOMATIC: wgpu::PowerPreference =
    if cfg!(any(windows, target_os = "macos")) { wgpu::PowerPreference::LowPower } else { wgpu::PowerPreference::None };

/// One start of the window's graphics: the rung, and the adapters ([`adapter::key`]s) it leaves
/// out, those earlier starts crashed on and those a restart failed on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attempt {
    /// The rung.
    pub rung: Rung,
    /// The adapters left out.
    pub skip: Vec<String>,
    /// The kind of GPU the user chose in the app's preferences; `None` is automatic.
    pub(crate) chosen: Option<wgpu::PowerPreference>,
}

impl Attempt {
    /// The instance descriptor: the rung's backends unless [`env::BACKEND`] names others, wgpu's
    /// variables, no indirect-call validation and, on Windows, the DirectX 12 compiler.
    pub fn instance_descriptor(&self) -> wgpu::InstanceDescriptor {
        let beside = self.rung == Rung::Dx12 && OS == Os::Windows && gl_beside(!self.skip.is_empty(), env::adapter_name().is_some());
        crate::access::instance_descriptor(backends_for(self.rung, OS, beside))
    }

    /// The power preference that orders adapters that drive no display: [`env::POWER_PREF`], else
    /// low power on [`Rung::Downlevel`], else the app's preference, else automatic.
    pub fn power_preference(&self) -> wgpu::PowerPreference {
        env::power_preference().or(self.low_power()).or(self.chosen).unwrap_or(AUTOMATIC)
    }

    fn low_power(&self) -> Option<wgpu::PowerPreference> {
        (self.rung == Rung::Downlevel).then_some(wgpu::PowerPreference::LowPower)
    }

    /// Whether the adapter is the start's to choose: no variable, rung or preference names a kind
    /// of GPU. Then the GPU the compositor draws with and the GPUs that drive a display come first;
    /// a user's own choice is kept as it is.
    fn automatic(&self) -> bool {
        env::power_preference().is_none() && self.low_power().is_none() && self.chosen.is_none()
    }

    /// What the window's device asks for: the rung's limits, egui's memory hints.
    pub fn device_request(&self) -> DeviceRequest {
        DeviceRequest {
            label: "window",
            tier: self.rung.tier(),
            optional_features: wgpu::Features::empty(),
            memory_hints: wgpu::MemoryHints::default(),
        }
    }

    /// How the adapters are ranked for this attempt.
    pub(crate) fn ranking(&self) -> Ranking {
        let automatic = self.automatic();
        Ranking {
            power: self.power_preference(),
            displays: if automatic { adapter::display_gpus() } else { Vec::new() },
            compositor: if automatic { adapter::compositor_gpu() } else { None },
            named: env::adapter_name(),
            skip: self.skip.clone(),
            software: self.rung.software_rule(),
        }
    }

    /// eframe's adapter choice for this attempt; the adapter it picks is recorded in `start`, so a
    /// crash on it leaves it out of the next start.
    pub fn selector(&self, start: &GraphicsStart) -> Selector {
        let picked = start.clone();
        let rung = self.rung;
        adapter::selector(
            self.ranking(),
            move |c, native| picked.picked(&c.key, &c.name, native),
            move || log::warn!("gpu: no adapter of {} is left to show the window", rung.label()),
        )
    }
}
