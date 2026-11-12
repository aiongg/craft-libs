//! The drivers' own reports of a card's memory: `nvidia-smi` (NVIDIA, Linux and Windows) and the
//! amdgpu driver's sysfs counters (Linux), read without `unsafe`.

use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

/// PCI vendor id of NVIDIA.
pub(super) const NVIDIA: u32 = 0x10de;
/// PCI vendor id of AMD.
pub(super) const AMD: u32 = 0x1002;

/// How long `nvidia-smi` gets to answer: the first question for an adapter runs on the caller's
/// thread, often the UI thread while the window opens, and a driver that hangs the query must not
/// keep the app from starting.
const NVIDIA_SMI_DEADLINE: Duration = Duration::from_secs(2);

/// A PCI address: domain, bus, device, function.
pub(super) type Pci = [u32; 4];

/// `0000:01:00.0` (Vulkan, sysfs) or `00000000:01:00.0` (`nvidia-smi`).
pub(super) fn pci_address(s: &str) -> Option<Pci> {
    let mut parts = s.trim().split(':');
    let (domain, bus, rest) = (parts.next()?, parts.next()?, parts.next()?);
    let (device, function) = rest.split_once('.')?;
    if parts.next().is_some() {
        return None;
    }
    let hex = |x: &str| u32::from_str_radix(x.trim(), 16).ok();
    Some([hex(domain)?, hex(bus)?, hex(device)?, hex(function)?])
}

/// Which card is asked about: vendor and device id, and the PCI address where the backend reports
/// it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Adapter {
    pub vendor: u32,
    pub device: u32,
    pub bus: Option<Pci>,
}

impl Adapter {
    pub fn of(info: &wgpu::AdapterInfo) -> Self {
        Self { vendor: info.vendor, device: info.device, bus: pci_address(&info.device_pci_bus_id) }
    }
}

/// One card a driver lists, with a memory size in bytes (free or total, as asked).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Card {
    pub bus: Option<Pci>,
    pub device: u32,
    pub bytes: u64,
}

/// What [`pick`] answers when the device id alone matches several cards.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Ambiguous {
    /// No answer: the caller keeps its fixed budget (free memory changes per card).
    Unknown,
    /// The smallest: a budget planned for it fits whichever card wgpu picked (total memory).
    Smallest,
}

/// The size of `want` among the listed `cards`: the card at its PCI address when both sides report
/// one, else the card with its device id, else the only card listed. When the address is listed by
/// no card, `None`; when the device id matches several cards, as `ambiguous` says.
pub(super) fn pick(cards: &[Card], want: &Adapter, ambiguous: Ambiguous) -> Option<u64> {
    if let Some(bus) = want.bus
        && cards.iter().any(|c| c.bus.is_some())
    {
        return cards.iter().find(|c| c.bus == Some(bus)).map(|c| c.bytes);
    }
    let same: Vec<u64> = cards.iter().filter(|c| c.device == want.device).map(|c| c.bytes).collect();
    match (same.as_slice(), ambiguous) {
        ([one], _) => Some(*one),
        ([], _) if cards.len() == 1 => cards.first().map(|c| c.bytes),
        ([], _) | (_, Ambiguous::Unknown) => None,
        (several, Ambiguous::Smallest) => several.iter().copied().min(),
    }
}

/// Which memory figure `nvidia-smi` is asked for.
#[derive(Clone, Copy, Debug)]
pub(super) enum Figure {
    Free,
    Total,
}

/// The cards `nvidia-smi --query-gpu=pci.bus_id,pci.device_id,memory.<figure>` lists; `None` when
/// it is missing, fails or misses the deadline.
pub(super) fn nvidia_cards(figure: Figure) -> Option<Vec<Card>> {
    let field = match figure {
        Figure::Free => "memory.free",
        Figure::Total => "memory.total",
    };
    let mut cmd = Command::new("nvidia-smi");
    cmd.args([format!("--query-gpu=pci.bus_id,pci.device_id,{field}").as_str(), "--format=csv,noheader,nounits"]);
    let out = bounded_output(cmd, NVIDIA_SMI_DEADLINE)?;
    Some(parse_nvidia(&String::from_utf8_lossy(&out)))
}

/// `nvidia-smi`'s CSV: one `DDDDDDDD:BB:DD.F, 0xDDDDVVVV, <MiB>` line per card (device id the high
/// half of the hex, vendor the low, as `lspci -n` and wgpu read them). Lines of another shape are
/// skipped.
pub(super) fn parse_nvidia(out: &str) -> Vec<Card> {
    out.lines()
        .filter_map(|l| {
            let mut f = l.split(',');
            let (bus, id, mib) = (f.next()?, f.next()?, f.next()?);
            let id = u32::from_str_radix(id.trim().trim_start_matches("0x").trim_start_matches("0X"), 16).ok()?;
            let bytes = mib.trim().parse::<u64>().ok()?.checked_mul(1 << 20)?;
            Some(Card { bus: pci_address(bus), device: id >> 16, bytes })
        })
        .collect()
}

/// An amdgpu card under the sysfs `drm` folder, with its VRAM counters where it has them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct DrmCard {
    pub bus: Option<Pci>,
    pub device: u32,
    pub total: Option<u64>,
    pub used: Option<u64>,
}

/// The cards of `vendor` under `drm` (`/sys/class/drm`), each once (`card0` and `renderD128` are
/// the same device): ids from the hex files of `<entry>/device`, the PCI address from the name of
/// that directory, and amdgpu's `mem_info_vram_total` and `mem_info_vram_used` (bytes).
pub(super) fn drm_cards(drm: &Path, vendor: u32) -> Vec<DrmCard> {
    let read_hex = |p: &Path| -> Option<u32> { u32::from_str_radix(std::fs::read_to_string(p).ok()?.trim().trim_start_matches("0x"), 16).ok() };
    let read_u64 = |p: &Path| -> Option<u64> { std::fs::read_to_string(p).ok()?.trim().parse().ok() };
    let mut cards = Vec::new();
    let mut seen = Vec::new();
    let Ok(entries) = std::fs::read_dir(drm) else { return cards };
    // A machine has a handful of entries; the cap only bounds a pathological sysfs.
    for entry in entries.flatten().take(256) {
        let Ok(dev) = std::fs::canonicalize(entry.path().join("device")) else { continue };
        if seen.contains(&dev) || read_hex(&dev.join("vendor")) != Some(vendor) {
            continue;
        }
        seen.push(dev.clone());
        let Some(device) = read_hex(&dev.join("device")) else { continue };
        let bus = dev.file_name().and_then(|n| pci_address(&n.to_string_lossy()));
        cards.push(DrmCard { bus, device, total: read_u64(&dev.join("mem_info_vram_total")), used: read_u64(&dev.join("mem_info_vram_used")) });
    }
    cards
}

/// The first 64 KiB of `cmd`'s stdout when it exits successfully within `deadline`; `None` when
/// it can't start, fails or is still running by then. A late one is killed and reaped on a thread
/// of its own (a process stuck in the driver may not die at once), so the caller waits about
/// `deadline` at most.
pub(super) fn bounded_output(mut cmd: Command, deadline: Duration) -> Option<Vec<u8>> {
    use std::io::Read;
    use std::process::{Child, Stdio};
    fn abandon(mut child: Child) {
        let _ = child.kill();
        let _ = std::thread::Builder::new().name("reap".into()).spawn(move || child.wait());
    }
    // A release app has no console: without this, Windows opens one for each query.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let end = Instant::now().checked_add(deadline)?;
    let mut child = cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().ok()?;
    let Some(stdout) = child.stdout.take() else {
        abandon(child);
        return None;
    };
    let (tx, rx) = std::sync::mpsc::channel();
    let reader = std::thread::Builder::new().name("child-stdout".into()).spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.take(64 << 10).read_to_end(&mut buf);
        let _ = tx.send(buf);
    });
    if reader.is_err() {
        abandon(child);
        return None;
    }
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < end => std::thread::sleep(Duration::from_millis(10)),
            _ => {
                abandon(child);
                return None;
            }
        }
    };
    if !status.success() {
        return None;
    }
    // The child has exited, so its stdout is at EOF unless it handed the pipe on to a process of
    // its own: bounded too, rather than joining the reader.
    rx.recv_timeout(end.saturating_duration_since(Instant::now()).max(Duration::from_millis(50))).ok()
}
