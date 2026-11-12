//! Stub of the adapter ranking (H2 writes the real module): only the signatures `start` and
//! `probe` code against. Dropped at the merge.
#![allow(missing_docs, dead_code, unused_variables)]

use std::sync::Arc;

pub struct Candidate {
    pub key: String,
    pub name: String,
    pub backend: wgpu::Backend,
    pub device_type: wgpu::DeviceType,
    pub pci: (u32, u32),
    pub presents: bool,
    pub driver: String,
    pub features: wgpu::Features,
}

pub fn key(backend: wgpu::Backend, vendor: u32, device: u32, name: &str) -> String {
    format!("{backend:?}:{vendor:x}:{device:x}")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DisplayGpu {
    pub pci: (u32, u32),
    pub primary: bool,
}

pub fn display_gpus() -> Vec<DisplayGpu> {
    Vec::new()
}

pub fn compositor_gpu() -> Option<(u32, u32)> {
    None
}

#[derive(Clone, Debug)]
pub struct Ranking {
    pub power: wgpu::PowerPreference,
    pub displays: Vec<DisplayGpu>,
    pub compositor: Option<(u32, u32)>,
    pub named: Option<String>,
    pub skip: Vec<String>,
    pub software: SoftwareRule,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoftwareRule {
    Exclude,
    Only,
}

pub fn adapter_order(candidates: &[Candidate], ranking: &Ranking) -> Vec<usize> {
    (0..candidates.len()).collect()
}

pub type Selector = Arc<dyn Fn(&[wgpu::Adapter], Option<&wgpu::Surface<'_>>) -> Result<wgpu::Adapter, String> + Send + Sync>;

pub fn selector(
    ranking: Ranking,
    on_pick: impl Fn(&Candidate, bool) + Send + Sync + 'static,
    on_none: impl Fn() + Send + Sync + 'static,
) -> Selector {
    Arc::new(move |adapters: &[wgpu::Adapter], _surface: Option<&wgpu::Surface<'_>>| match adapters.first() {
        Some(a) => Ok(a.clone()),
        None => {
            on_none();
            Err("no adapter".to_string())
        }
    })
}
