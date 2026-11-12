//! One GPU listed through two versions of one driver.
//!
//! On Windows the Vulkan loader takes one driver from each display adapter's driver package. A PC
//! whose GPUs were installed from different packages of one vendor (a laptop with an integrated
//! and a discrete AMD GPU, the first on the manufacturer's old package and the second on a current
//! one) loads two versions of that driver, and the newer GPU is listed once through each. wgpu
//! takes the first entry, whichever version the loader listed first; a window drawn through the
//! stale one kills the process inside the driver (`amdvlk64.dll`, 0xc0000005), where no Rust error
//! handling runs. So the entry exposing the most features, the newer version, is tried first.
//!
//! Entries of one GPU from different drivers (RADV and AMDVLK, a vendor's driver and Dozen) are
//! left alone: those are installed side by side on purpose, and the system's order between them
//! stays.

use std::cmp::Reverse;

use super::Candidate;

/// Whether `a` and `b` are one GPU listed twice by one driver, i.e. through two installed versions
/// of it: the same backend, device type, PCI ids and driver name. Adapters without PCI ids
/// (OpenGL, Metal) are never the same: two of them can't be told apart. Two cards of one model
/// on one driver count as one GPU too; they expose the same features, so their order stays.
pub(super) fn same_gpu_and_driver(a: &Candidate, b: &Candidate) -> bool {
    a.pci != (0, 0) && a.backend == b.backend && a.device_type == b.device_type && a.pci == b.pci && a.driver == b.driver
}

/// How many features an adapter exposes.
fn feature_count(features: wgpu::Features) -> u32 {
    features.bits().0.iter().map(|word| word.count_ones()).sum()
}

/// Within each set of entries in `order` that are one GPU through one driver, the entry exposing
/// the most features first, in the places the set already holds; entries exposing as many keep
/// their order. Everything else stays where it is.
pub(super) fn newer_versions_first(order: &mut [usize], candidates: &[Candidate]) {
    // The places in `order` of each GPU-and-driver set, the first place leading.
    let mut sets: Vec<Vec<usize>> = Vec::new();
    for (place, &i) in order.iter().enumerate() {
        let Some(c) = candidates.get(i) else { continue };
        let lead_of = |set: &&mut Vec<usize>| set.first().and_then(|&p| candidates.get(order[p])).is_some_and(|lead| same_gpu_and_driver(lead, c));
        match sets.iter_mut().find(lead_of) {
            Some(set) => set.push(place),
            None => sets.push(vec![place]),
        }
    }
    for places in sets.into_iter().filter(|set| set.len() > 1) {
        let mut entries: Vec<usize> = places.iter().map(|&p| order[p]).collect();
        // Stable: entries exposing as many features keep their order.
        entries.sort_by_key(|&i| Reverse(candidates.get(i).map_or(0, |c| feature_count(c.features))));
        for (p, i) in places.into_iter().zip(entries) {
            order[p] = i;
        }
    }
}
