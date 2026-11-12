use wgpu::Backend::{Dx12, Gl, Metal, Vulkan};
use wgpu::DeviceType::{Cpu, DiscreteGpu, IntegratedGpu, Other};
use wgpu::PowerPreference::{HighPerformance, LowPower};

use super::compositor::major_minor;
use super::display::{linux_display_gpus, pci_ids};
use super::*;

/// The order of `c` for a hardware attempt.
fn order(
    c: &[Candidate],
    power: PowerPreference,
    displays: &[DisplayGpu],
    compositor: Option<(u32, u32)>,
    named: Option<&str>,
    skip: &[String],
) -> Vec<usize> {
    let ranking = Ranking {
        power,
        displays: displays.to_vec(),
        compositor,
        named: named.map(String::from),
        skip: skip.to_vec(),
        software: SoftwareRule::Exclude,
    };
    adapter_order(c, &ranking)
}

fn names(c: &[Candidate], order: &[usize]) -> Vec<String> {
    order.iter().map(|&i| c[i].name.clone()).collect()
}

/// An adapter without PCI ids, keyed by name.
fn gpu(name: &str, backend: Backend, device_type: DeviceType) -> Candidate {
    pci_gpu(name, backend, device_type, (0, 0))
}

fn pci_gpu(name: &str, backend: Backend, device_type: DeviceType, pci: (u32, u32)) -> Candidate {
    Candidate {
        key: key(backend, pci.0, pci.1, name),
        name: name.into(),
        backend,
        device_type,
        pci,
        presents: true,
        driver: String::new(),
        features: wgpu::Features::empty(),
    }
}

const NVIDIA_3090: (u32, u32) = (0x10de, 0x2204);
const AMD_IGPU: (u32, u32) = (0x1002, 0x164e);
const INTEL_IGPU: (u32, u32) = (0x8086, 0xa780);
const WARP: (u32, u32) = (0x1414, 0x008c);

fn monitor(pci: (u32, u32)) -> DisplayGpu {
    DisplayGpu { pci, primary: false }
}

fn primary(pci: (u32, u32)) -> DisplayGpu {
    DisplayGpu { pci, primary: true }
}

/// The desktop of pdfcraft#378 as wgpu lists it on Windows: a Ryzen's integrated GPU without a
/// monitor and an RTX 3090 driving both monitors, each under Vulkan and DX12, then WARP and
/// OpenGL (which reports no device id).
fn ryzen_desktop() -> Vec<Candidate> {
    vec![
        pci_gpu("NVIDIA GeForce RTX 3090", Vulkan, DiscreteGpu, NVIDIA_3090),
        pci_gpu("AMD Radeon(TM) Graphics", Vulkan, IntegratedGpu, AMD_IGPU),
        pci_gpu("NVIDIA GeForce RTX 3090", Dx12, DiscreteGpu, NVIDIA_3090),
        pci_gpu("AMD Radeon(TM) Graphics", Dx12, IntegratedGpu, AMD_IGPU),
        pci_gpu("Microsoft Basic Render Driver", Dx12, Cpu, WARP),
        pci_gpu("NVIDIA GeForce RTX 3090/PCIe/SSE2", Gl, Other, (NVIDIA_3090.0, 0)),
    ]
}

/// The machine of VectorCraft #502 as Vulkan lists it with Mesa's device-select layer: the GPU
/// KDE's compositor runs on (NVIDIA) first, then the Ryzen's integrated GPU, Mesa's software
/// renderer, and OpenGL.
fn issue_502() -> Vec<Candidate> {
    vec![
        gpu("NVIDIA GeForce RTX 5070", Vulkan, DiscreteGpu),
        gpu("AMD Ryzen 9 7900X (RADV RAPHAEL_MENDOCINO)", Vulkan, IntegratedGpu),
        gpu("llvmpipe (LLVM 20.1.8, 256 bits)", Vulkan, Cpu),
        gpu("NVIDIA GeForce RTX 5070/PCIe/SSE2", Gl, Other),
    ]
}

/// pdfcraft#378: power saving picked the integrated GPU, whose driver reset when Windows had to
/// present its frames on the monitors of the NVIDIA card. The GPU that drives the displays comes
/// first, through DX12 (#545); the integrated GPU stays the fallback a restart reaches.
#[test]
fn a_desktop_draws_on_the_gpu_that_drives_its_monitors() {
    let c = ryzen_desktop();
    for displays in [vec![primary(NVIDIA_3090)], vec![monitor(NVIDIA_3090)]] {
        assert_eq!(order(&c, LowPower, &displays, None, None, &[]), [2, 0, 3, 1, 5], "{displays:?}");
        assert_eq!(order(&c, PowerPreference::None, &displays, None, None, &[]), [2, 0, 3, 1, 5], "{displays:?}");
    }
    // Without the display information the order is power saving's.
    assert_eq!(order(&c, LowPower, &[], None, None, &[]), [3, 1, 2, 0, 5]);
    // After the NVIDIA adapters failed, the integrated GPU is next.
    let skip = vec![c[2].key.clone(), c[0].key.clone()];
    assert_eq!(order(&c, LowPower, &[primary(NVIDIA_3090)], None, None, &skip), [3, 1, 5]);
}

/// A hybrid laptop: the panel (primary) on the integrated GPU, an external monitor on the
/// discrete one. Power saving and the display agree, in either enumeration order.
#[test]
fn a_hybrid_laptop_draws_on_the_integrated_gpu_that_drives_its_panel() {
    let c = ryzen_desktop();
    for displays in [vec![primary(AMD_IGPU), monitor(NVIDIA_3090)], vec![monitor(NVIDIA_3090), primary(AMD_IGPU)], vec![primary(AMD_IGPU)]] {
        assert_eq!(order(&c, LowPower, &displays, None, None, &[]), [3, 1, 2, 0, 5], "{displays:?}");
    }
    // Linux names no primary unless there is a built-in panel: a monitor on each GPU keeps the
    // system's order among them (Mesa puts the compositor's GPU first), and a GPU without one
    // comes after both.
    let linux = vec![
        pci_gpu("AMD Radeon Graphics (RADV RAPHAEL_MENDOCINO)", Vulkan, IntegratedGpu, AMD_IGPU),
        pci_gpu("NVIDIA GeForce RTX 3090", Vulkan, DiscreteGpu, NVIDIA_3090),
        pci_gpu("Intel(R) Arc(TM) A380", Vulkan, DiscreteGpu, INTEL_IGPU),
        pci_gpu("llvmpipe (LLVM 20.1.8, 256 bits)", Vulkan, Cpu, (0x10005, 0)),
    ];
    let both = [monitor(AMD_IGPU), monitor(NVIDIA_3090)];
    assert_eq!(order(&linux, PowerPreference::None, &both, None, None, &[]), [0, 1, 2]);
    assert_eq!(order(&linux, PowerPreference::None, &[monitor(INTEL_IGPU)], None, None, &[]), [2, 0, 1]);
    assert_eq!(order(&linux, PowerPreference::None, &[monitor(NVIDIA_3090), primary(AMD_IGPU)], None, None, &[]), [0, 1, 2]);
}

/// A hybrid laptop whose Wayland compositor runs on the discrete GPU (Hyprland with
/// `AQ_DRM_DEVICES` naming it first), as Vulkan lists it there: the integrated GPU drives the
/// built-in panel, yet the compositor shows nothing it draws, and reports no error. The GPU the
/// compositor draws with comes first; without it, the panel's.
#[test]
fn the_gpu_the_compositor_draws_with_comes_before_the_panels() {
    const RTX_3050: (u32, u32) = (0x10de, 0x25ac);
    const INTEL_RPL: (u32, u32) = (0x8086, 0xa7a8);
    let mut c = vec![
        pci_gpu("NVIDIA GeForce RTX 3050 6GB Laptop GPU", Vulkan, DiscreteGpu, RTX_3050),
        pci_gpu("Intel(R) Graphics (RPL-P)", Vulkan, IntegratedGpu, INTEL_RPL),
        pci_gpu("NVIDIA GeForce RTX 3050 6GB Laptop GPU/PCIe/SSE2", Gl, Other, (RTX_3050.0, 0)),
    ];
    c[2].presents = false;
    let panel = [primary(INTEL_RPL)];
    assert_eq!(order(&c, PowerPreference::None, &panel, Some(RTX_3050), None, &[]), [0, 1]);
    assert_eq!(order(&c, PowerPreference::None, &panel, None, None, &[]), [1, 0]);
    // The usual hybrid laptop, with the compositor on the integrated GPU: the panel's GPU.
    assert_eq!(order(&c, PowerPreference::None, &[primary(INTEL_RPL), monitor(RTX_3050)], Some(INTEL_RPL), None, &[]), [1, 0]);
    // A named adapter still comes first, and a restart after a failure still moves on.
    assert_eq!(order(&c, PowerPreference::None, &panel, Some(RTX_3050), Some("intel"), &[]), [1, 0]);
    assert_eq!(order(&c, PowerPreference::None, &panel, Some(RTX_3050), None, &[c[0].key.clone()]), [1]);
    // A compositor GPU no adapter matches (a virtual one) leaves the displays in charge.
    assert_eq!(order(&c, PowerPreference::None, &panel, Some((0x1af4, 0x1050)), None, &[]), [1, 0]);
}

/// A Windows desktop with a monitor on each GPU: the window opens on the primary display, so its
/// GPU draws, whichever kind it is.
#[test]
fn the_primary_display_decides_between_gpus_that_both_drive_a_monitor() {
    let c = ryzen_desktop();
    assert_eq!(order(&c, LowPower, &[monitor(AMD_IGPU), primary(NVIDIA_3090)], None, None, &[]), [2, 0, 3, 1, 5]);
    assert_eq!(order(&c, LowPower, &[primary(AMD_IGPU), monitor(NVIDIA_3090)], None, None, &[]), [3, 1, 2, 0, 5]);
    // A GPU driving no display still loses to one that does.
    assert_eq!(order(&c, LowPower, &[monitor(AMD_IGPU), monitor(INTEL_IGPU)], None, None, &[]), [3, 1, 2, 0, 5]);
}

/// Display GPUs none of the adapters match (another driver's ids, OpenGL's missing device id,
/// Metal's zeros) change nothing: power saving's order.
#[test]
fn displays_no_adapter_matches_leave_the_power_preference_in_charge() {
    let c = ryzen_desktop();
    let low = order(&c, LowPower, &[], None, None, &[]);
    for displays in [vec![monitor(INTEL_IGPU)], vec![primary((0x8086, 0x1234)), monitor((0x8086, 0x5678))], vec![primary((0, 0))]] {
        assert_eq!(order(&c, LowPower, &displays, None, None, &[]), low, "{displays:?}");
    }
    assert_eq!(names(&c, &order(&c, HighPerformance, &[monitor(INTEL_IGPU)], None, None, &[]))[0], "NVIDIA GeForce RTX 3090");
    // The #502 machine's adapters report no ids here: the primary display never matches.
    let c = issue_502();
    assert_eq!(order(&c, PowerPreference::None, &[primary(NVIDIA_3090)], None, None, &[]), order(&c, PowerPreference::None, &[], None, None, &[]));
    assert!(order(&[], LowPower, &[primary(NVIDIA_3090)], None, None, &[]).is_empty());
}

/// A named adapter and "can't present" still come before the displays.
#[test]
fn a_named_adapter_comes_before_the_one_driving_the_display() {
    let mut c = ryzen_desktop();
    let displays = [primary(NVIDIA_3090)];
    assert_eq!(order(&c, LowPower, &displays, None, Some("radeon"), &[]), [3, 1, 2, 0, 5]);
    c[2].presents = false;
    c[0].presents = false;
    assert_eq!(order(&c, LowPower, &displays, None, None, &[]), [3, 1, 5]);
}

/// Whatever this machine has: every GPU listed is a PCI device, at most one drives the primary
/// display, and none is listed twice. (A headless CI runner may list none.)
#[cfg(windows)]
#[test]
fn windows_lists_each_gpu_with_a_monitor_once() {
    let gpus = display::windows_display_gpus();
    assert!(gpus.iter().filter(|g| g.primary).count() <= 1, "{gpus:?}");
    for (i, g) in gpus.iter().enumerate() {
        assert_ne!(g.pci.0, 0, "{gpus:?}");
        assert!(!gpus[..i].iter().any(|h| h.pci == g.pci), "{gpus:?}");
    }
}

/// A sysfs like a hybrid laptop's with an external monitor: the panel and a disconnected port on
/// the integrated GPU (card0), the monitor on the discrete one (card1), and a card without PCI
/// ids. Each GPU is listed once, the one with the panel as primary.
#[test]
fn linux_display_gpus_come_from_the_connected_connectors() -> std::io::Result<()> {
    let dir = std::env::temp_dir().join(format!("craft-gpu-host-drm-connectors-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let card = |name: &str, (vendor, device): (u32, u32)| -> std::io::Result<()> {
        std::fs::create_dir_all(dir.join(name).join("device"))?;
        std::fs::write(dir.join(name).join("device/vendor"), format!("{vendor:#06x}\n"))?;
        std::fs::write(dir.join(name).join("device/device"), format!("{device:#06x}\n"))
    };
    let connector = |name: &str, status: &str| -> std::io::Result<()> {
        std::fs::create_dir_all(dir.join(name))?;
        std::fs::write(dir.join(name).join("status"), format!("{status}\n"))
    };
    card("card0", AMD_IGPU)?;
    card("card1", NVIDIA_3090)?;
    std::fs::create_dir_all(dir.join("card2"))?;
    connector("card0-eDP-1", "connected")?;
    connector("card0-HDMI-A-1", "disconnected")?;
    connector("card1-DP-1", "connected")?;
    connector("card1-DP-2", "connected")?;
    connector("card2-Virtual-1", "connected")?;
    std::fs::write(dir.join("renderD128"), "")?;
    let mut gpus = linux_display_gpus(&dir);
    gpus.sort_by_key(|g| g.pci);
    let missing = linux_display_gpus(&dir.join("no such dir"));
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(gpus, [primary(AMD_IGPU), monitor(NVIDIA_3090)]);
    assert!(missing.is_empty());
    Ok(())
}

/// A restart leaves out the adapter the window failed on. A dual-GPU Mac reports both GPUs under
/// Metal with no PCI ids (#651): their keys still differ, so when the integrated GPU fails, the
/// restart renders on the discrete one.
#[test]
fn gpus_without_pci_ids_are_told_apart_by_name() {
    let c = vec![gpu("NVIDIA GeForce GT 750M", Metal, DiscreteGpu), gpu("Intel Iris Pro Graphics", Metal, IntegratedGpu)];
    assert_eq!((c[0].key.as_str(), c[1].key.as_str()), ("Metal:NVIDIA GeForce GT 750M", "Metal:Intel Iris Pro Graphics"));
    let first = order(&c, LowPower, &[], None, None, &[]);
    assert_eq!(names(&c, &first)[0], "Intel Iris Pro Graphics");
    let skip = vec![c[first[0]].key.clone()];
    assert_eq!(names(&c, &order(&c, LowPower, &[], None, None, &skip)), ["NVIDIA GeForce GT 750M"]);
    // A comma in a name can't split a list of keys.
    assert_eq!(key(Gl, 0, 0, "Mesa, llvmpipe"), "Gl:Mesa  llvmpipe");
}

#[test]
fn adapters_that_cannot_present_to_the_window_are_never_tried() {
    let mut c = issue_502();
    c[0].presents = false;
    c[3].presents = false;
    for power in [PowerPreference::None, LowPower, HighPerformance] {
        assert_eq!(order(&c, power, &[], None, None, &[]), [1], "{power:?}");
    }
    for p in &mut c {
        p.presents = false;
    }
    assert!(order(&c, PowerPreference::None, &[], None, None, &[]).is_empty());
    assert!(order(&[], HighPerformance, &[], None, None, &[]).is_empty());
}

/// The machine of VectorCraft #545: one Intel GPU, whose Vulkan driver made the window flicker
/// black. DX12 comes first whatever the preference; Vulkan stays a fallback, which a named adapter
/// or a restart leaving DX12 out still reaches.
#[test]
fn a_windows_gpu_renders_through_dx12_before_vulkan() {
    let c = vec![
        gpu("Intel(R) Graphics", Vulkan, IntegratedGpu),
        gpu("Intel(R) Graphics", Dx12, IntegratedGpu),
        gpu("Microsoft Basic Render Driver", Dx12, Cpu),
        gpu("Intel(R) Graphics", Gl, Other),
    ];
    for power in [LowPower, HighPerformance, PowerPreference::None] {
        assert_eq!(order(&c, power, &[], None, None, &[]), [1, 0, 3], "{power:?}");
    }
    assert_eq!(order(&c, LowPower, &[], None, None, &[c[1].key.clone()]), [0, 3], "DX12 failed: Vulkan next");
}

/// `u32::from_str_radix` accepts a leading `+`; a device id with one is not a PCI id.
#[test]
fn pci_ids_reject_a_sign_in_the_hex_digits() {
    assert_eq!(pci_ids("PCI\\VEN_10DE&DEV_2204&SUBSYS_40421458&REV_A1"), Some(NVIDIA_3090));
    assert_eq!(pci_ids("PCI\\VEN_+10DE&DEV_2204"), None);
    assert_eq!(pci_ids("PCI\\VEN_10DE&DEV_+2204"), None);
}

/// A hardware attempt never draws on the CPU while a GPU is left to try, and the software attempt
/// after every GPU failed takes only a software adapter (FilmCraft's software mode, DesignCraft's
/// software last). llvmpipe is found by name even where the driver calls it another kind.
#[test]
fn hardware_attempts_leave_software_out_and_the_software_attempt_takes_only_it() {
    let mut c = issue_502();
    c.push(gpu("llvmpipe (LLVM 20.1.8, 256 bits)", Gl, Other));
    let ranking = |software| Ranking { power: LowPower, displays: Vec::new(), compositor: None, named: None, skip: Vec::new(), software };
    assert_eq!(adapter_order(&c, &ranking(SoftwareRule::Exclude)), [1, 0, 3]);
    assert_eq!(adapter_order(&c, &ranking(SoftwareRule::Only)), [2, 4]);
    // No software adapter, or none that presents: the software attempt has nothing to try.
    c[2].presents = false;
    c[4].presents = false;
    assert!(adapter_order(&c, &ranking(SoftwareRule::Only)).is_empty());
}

/// An ASUS G513QY as wgpu 30 lists it on Windows: the integrated Radeon is on the manufacturer's
/// 2021 driver package and the RX 6800M on a 2026 one, so the Vulkan loader has both versions of
/// AMD's driver, and the 2021 one lists the RX 6800M first. A window drawn through that entry died
/// inside `amdvlk64.dll` (0xc0000005) on every launch (FilmCraft).
#[test]
fn a_gpu_listed_through_two_versions_of_its_driver_draws_with_the_newer_one() {
    const RX_6800M: (u32, u32) = (0x1002, 0x73df);
    const RADEON_IGPU: (u32, u32) = (0x1002, 0x1638);
    let old = wgpu::Features::DEPTH_CLIP_CONTROL;
    let new = old | wgpu::Features::TIMESTAMP_QUERY;
    let newest = new | wgpu::Features::SHADER_F16;
    let amd = |name: &str, backend, device_type, pci, driver: &str, features| Candidate {
        driver: driver.into(),
        features,
        ..pci_gpu(name, backend, device_type, pci)
    };
    let vulkan = "AMD proprietary driver";
    let g513qy = vec![
        amd("AMD Radeon RX 6800M", Vulkan, DiscreteGpu, RX_6800M, vulkan, old),
        amd("AMD Radeon RX 6800M", Vulkan, DiscreteGpu, RX_6800M, vulkan, new),
        amd("AMD Radeon(TM) Graphics", Vulkan, IntegratedGpu, RADEON_IGPU, vulkan, old),
        amd("AMD Radeon RX 6800M", Dx12, DiscreteGpu, RX_6800M, "32.0.21045.11001", old),
        amd("AMD Radeon(TM) Graphics", Dx12, IntegratedGpu, RADEON_IGPU, "30.0.13002.13003", old),
    ];
    // The Vulkan attempt lists Vulkan alone: the newer entry, whichever the loader lists first.
    let vulkan_only = &g513qy[..3];
    assert_eq!(order(vulkan_only, HighPerformance, &[], None, None, &[]), [1, 0, 2]);
    assert_eq!(order(vulkan_only, PowerPreference::None, &[], None, None, &[]), [1, 0, 2]);
    let newer_first = vec![g513qy[1].clone(), g513qy[0].clone(), g513qy[2].clone()];
    assert_eq!(order(&newer_first, HighPerformance, &[], None, None, &[]), [0, 1, 2]);
    // A newer version never overtakes another GPU or backend: DX12 first (#545), the integrated
    // GPU first for power saving.
    assert_eq!(order(&g513qy, HighPerformance, &[], None, None, &[]), [3, 1, 0, 4, 2]);
    assert_eq!(order(&g513qy, LowPower, &[], None, None, &[]), [4, 2, 3, 1, 0]);
    // Two entries exposing as much keep their order; of three versions, the one exposing the most.
    let mut equal = vulkan_only.to_vec();
    equal[1].features = old;
    assert_eq!(order(&equal, HighPerformance, &[], None, None, &[]), [0, 1, 2]);
    let mut three = vulkan_only.to_vec();
    three.insert(1, amd("AMD Radeon RX 6800M", Vulkan, DiscreteGpu, RX_6800M, vulkan, newest));
    assert_eq!(order(&three, HighPerformance, &[], None, None, &[]), [1, 2, 0, 3]);
}

/// The compositor names its GPU as a `dev_t`; glibc's and musl's split of it into major and minor
/// numbers finds the DRM node in sysfs.
#[test]
fn device_numbers_split_into_major_and_minor() {
    /// glibc's `makedev()`.
    fn makedev(major: u64, minor: u64) -> u64 {
        ((major & 0x0000_0fff) << 8) | ((major & 0xffff_f000) << 32) | (minor & 0x0000_00ff) | ((minor & 0xffff_ff00) << 12)
    }
    // DRM nodes: card1 is 226:1, renderD128 is 226:128.
    assert_eq!(major_minor(0xe201), (226, 1));
    assert_eq!(major_minor(0xe280), (226, 128));
    for (major, minor) in [(0, 0), (226, 129), (511, 1_048_575), (0xfff, 0xff), (0x1000, 0x100), (0xffff_ffff, 0xffff_ffff)] {
        assert_eq!(major_minor(makedev(major, minor)), (major, minor));
    }
}
