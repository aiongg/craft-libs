//! How much memory a GPU has: a discrete card's own memory ([`video_memory`]), how much of it is
//! free right now ([`free_video_memory`]), and the machine's physical memory
//! ([`physical_memory`]). Budgets built on them live with their users: the canvas residency in
//! craft-ui-gpu, the back-pressure of frames in flight in the effects runtime.
//!
//! wgpu has no query for an adapter's memory (`max_buffer_size` is capped at `i32::MAX` on DirectX
//! 12 and Mesa's Vulkan), so this asks the platform where it can without `unsafe`: DXGI on Windows,
//! the amdgpu driver's sysfs counters on Linux, and `nvidia-smi` for NVIDIA's driver, which writes
//! no sysfs counters. Everything else reports `None`, and callers keep a fixed budget.

use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use smi::{AMD, Adapter, Ambiguous, Card, Figure, NVIDIA};

use crate::env;

mod smi;

/// The memory of a discrete card itself, in bytes, when the platform reports it: DXGI's dedicated
/// video memory on Windows; amdgpu's VRAM size, else `nvidia-smi`'s, on Linux. `None` for
/// integrated and software adapters, whose textures live in system memory (RAM-based budgets
/// cover them), and where nothing reports it. Asked once per adapter (a DXGI factory each time on
/// Windows, a process on NVIDIA).
///
/// [`env::VRAM_MB`] overrides it for any adapter, simulating a discrete card of that size: a
/// budget on an integrated GPU then treats it like a card of its own too (a development switch).
pub fn video_memory(info: &wgpu::AdapterInfo) -> Option<u64> {
    if let Some(mb) = std::env::var(env::VRAM_MB).ok().and_then(|v| v.trim().parse::<u64>().ok()) {
        return mb.checked_mul(1 << 20).filter(|b| *b > 0);
    }
    if info.device_type != wgpu::DeviceType::DiscreteGpu {
        return None;
    }
    /// The adapters asked about, with the memory found for each.
    type Seen = Vec<(Adapter, Option<u64>)>;
    static SEEN: Mutex<Seen> = Mutex::new(Vec::new());
    let adapter = Adapter::of(info);
    let mut seen = SEEN.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some((_, bytes)) = seen.iter().find(|(a, _)| *a == adapter) {
        return *bytes;
    }
    let bytes = detect_video_memory(&adapter).filter(|b| *b > 0);
    seen.push((adapter, bytes));
    bytes
}

#[cfg(windows)]
fn detect_video_memory(adapter: &Adapter) -> Option<u64> {
    use winsafe::prelude::*;
    let factory = winsafe::CreateDXGIFactory1().ok()?;
    factory
        .EnumAdapters1()
        .filter_map(|a| a.ok()?.GetDesc1().ok())
        .find(|d| d.VendorId == adapter.vendor && d.DeviceId == adapter.device)
        .and_then(|d| u64::try_from(d.DedicatedVideoMemory).ok())
}

#[cfg(target_os = "linux")]
fn detect_video_memory(adapter: &Adapter) -> Option<u64> {
    drm_total(std::path::Path::new("/sys/class/drm"), adapter).or_else(|| match adapter.vendor {
        NVIDIA => nvidia_total(adapter),
        _ => None,
    })
}

#[cfg(not(any(windows, target_os = "linux")))]
fn detect_video_memory(_adapter: &Adapter) -> Option<u64> {
    None
}

/// amdgpu's VRAM size of `adapter` among the cards under `drm`.
#[cfg_attr(not(any(target_os = "linux", test)), allow(dead_code))]
fn drm_total(drm: &std::path::Path, adapter: &Adapter) -> Option<u64> {
    let cards: Vec<Card> = smi::drm_cards(drm, adapter.vendor)
        .into_iter()
        .filter_map(|c| Some(Card { bus: c.bus, device: c.device, bytes: c.total.filter(|b| *b > 0)? }))
        .collect();
    smi::pick(&cards, adapter, Ambiguous::Smallest)
}

/// `nvidia-smi`'s report of `adapter`'s memory size; the smallest when several cards share its ids
/// and no address tells them apart.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn nvidia_total(adapter: &Adapter) -> Option<u64> {
    let mut cards = smi::nvidia_cards(Figure::Total)?;
    cards.retain(|c| c.bytes > 0);
    smi::pick(&cards, adapter, Ambiguous::Smallest)
}

/// Free memory of the adapter `info` describes, in bytes, or `None` where it can't be asked
/// (AMD and Intel on Windows need DXGI's `unsafe` budget query; integrated GPUs share system
/// memory). Renders ask often and the query starts a process on NVIDIA, so the answer is cached
/// for [`TTL`]: only the first question for an adapter waits; later ones get the last answer
/// while a background thread asks again, so no render, and no lock it holds, waits for
/// `nvidia-smi`. A card is found by its PCI address where the backend reports it, else by its
/// device id; when that matches several cards (two of one model), the answer is `None`.
pub fn free_video_memory(info: &wgpu::AdapterInfo) -> Option<u64> {
    static CACHE: Cache = Cache::new(TTL);
    CACHE.get(&Adapter::of(info), query_free)
}

/// Physical memory of this machine in bytes, if known (`craft_platform::usage::physical_memory`,
/// which `CRAFT_RAM_MB` overrides).
pub fn physical_memory() -> Option<u64> {
    craft_platform::usage::physical_memory()
}

fn query_free(adapter: &Adapter) -> Option<u64> {
    match adapter.vendor {
        NVIDIA => smi::pick(&smi::nvidia_cards(Figure::Free)?, adapter, Ambiguous::Unknown),
        AMD => {
            let cards: Vec<Card> = smi::drm_cards(std::path::Path::new("/sys/class/drm"), adapter.vendor)
                .into_iter()
                .filter_map(|c| Some(Card { bus: c.bus, device: c.device, bytes: c.total?.saturating_sub(c.used?) }))
                .collect();
            smi::pick(&cards, adapter, Ambiguous::Unknown)
        }
        _ => None,
    }
}

/// How long a free-memory answer is fresh; an older one is still returned while a newer one is
/// asked for.
const TTL: Duration = Duration::from_secs(3);

/// Adapters remembered (a renderer, a face finder and a model may each use another device).
const ADAPTERS: usize = 4;

/// A background query not done after this is taken as lost, and the next question asks again.
const LOST: Duration = Duration::from_secs(10);

/// The last answer for one adapter: when it was asked, the answer, and since when a newer one is
/// being asked for.
struct Answer {
    adapter: Adapter,
    at: Instant,
    free: Option<u64>,
    asking: Option<Instant>,
}

type Ask = fn(&Adapter) -> Option<u64>;

struct Cache {
    ttl: Duration,
    answers: Mutex<Vec<Answer>>,
}

impl Cache {
    const fn new(ttl: Duration) -> Self {
        Cache { ttl, answers: Mutex::new(Vec::new()) }
    }

    /// The answer of `ask` for `adapter`: cached, refreshed in the background once it is older
    /// than the TTL (in the caller where no thread can be started), and asked for directly the
    /// first time.
    fn get(&'static self, adapter: &Adapter, ask: Ask) -> Option<u64> {
        {
            let mut answers = self.answers.lock().unwrap_or_else(PoisonError::into_inner);
            if let Some(a) = answers.iter_mut().find(|a| a.adapter == *adapter) {
                let busy = a.asking.is_some_and(|t| t.elapsed() < LOST);
                if a.at.elapsed() >= self.ttl && !busy {
                    let adapter = *adapter;
                    let refresh = std::thread::Builder::new().name("vram-query".into()).spawn(move || self.store(adapter, ask(&adapter)));
                    if refresh.is_err() {
                        drop(answers);
                        return self.ask_now(&adapter, ask);
                    }
                    a.asking = Some(Instant::now());
                }
                return a.free;
            }
        }
        self.ask_now(adapter, ask)
    }

    fn ask_now(&self, adapter: &Adapter, ask: Ask) -> Option<u64> {
        let free = ask(adapter);
        self.store(*adapter, free);
        free
    }

    /// Keep `adapter`'s answer (each adapter has its own; the oldest one goes when the list is
    /// full).
    fn store(&self, adapter: Adapter, free: Option<u64>) {
        let mut answers = self.answers.lock().unwrap_or_else(PoisonError::into_inner);
        answers.retain(|a| a.adapter != adapter);
        if answers.len() >= ADAPTERS {
            answers.remove(0);
        }
        answers.push(Answer { adapter, at: Instant::now(), free, asking: None });
    }

    /// Make `adapter`'s answer as old as the TTL (tests: no waiting, so no answer goes stale by
    /// itself on a slow machine). `false` when there is no answer, or the clock doesn't reach that
    /// far back.
    #[cfg(test)]
    fn expire(&self, adapter: &Adapter) -> bool {
        let mut answers = self.answers.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(a) = answers.iter_mut().find(|a| a.adapter == *adapter) else { return false };
        let Some(at) = a.at.checked_sub(self.ttl) else { return false };
        a.at = at;
        true
    }

    #[cfg(test)]
    fn asking(&self, adapter: &Adapter) -> bool {
        self.answers.lock().unwrap_or_else(PoisonError::into_inner).iter().any(|a| a.adapter == *adapter && a.asking.is_some())
    }
}

#[cfg(test)]
mod tests;
