//! GPU timestamps around a frame's render pass (`EpaintOptions::gpu_timing`). The device asks for
//! the timestamp feature only where the adapter has it. Only pass-boundary timestamps are used,
//! the form Metal on Apple GPUs supports; clears and buffer uploads are outside the pass and not
//! in the time.

use egui_wgpu::{RenderState, wgpu};

/// Timestamps written at the start and end of the frame's render pass, resolved and read back.
pub(crate) struct Timer {
    set: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    read: wgpu::Buffer,
    /// Nanoseconds per timestamp tick.
    period: f64,
}

impl Timer {
    pub(crate) fn new(gpu: &RenderState) -> Option<Timer> {
        let device = &gpu.device;
        if !device.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
            return None;
        }
        let set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("craft-ui-render-epaint timer"),
            ty: wgpu::QueryType::Timestamp,
            count: 2,
        });
        let buffer = |usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("craft-ui-render-epaint timer"),
                size: 2 * wgpu::QUERY_SIZE as u64,
                usage,
                mapped_at_creation: false,
            })
        };
        Some(Timer {
            set,
            resolve: buffer(wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC),
            read: buffer(wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST),
            period: f64::from(gpu.queue.get_timestamp_period()),
        })
    }

    pub(crate) fn writes(&self) -> wgpu::RenderPassTimestampWrites<'_> {
        wgpu::RenderPassTimestampWrites { query_set: &self.set, beginning_of_pass_write_index: Some(0), end_of_pass_write_index: Some(1) }
    }

    pub(crate) fn resolve(&self, encoder: &mut wgpu::CommandEncoder) {
        encoder.resolve_query_set(&self.set, 0..2, &self.resolve, 0);
        encoder.copy_buffer_to_buffer(&self.resolve, 0, &self.read, 0, None);
    }

    /// Waits for the submitted pass and returns its GPU time in milliseconds.
    pub(crate) fn read(&self, device: &wgpu::Device) -> Option<f64> {
        Some(self.ticks(device)? as f64 * self.period / 1e6)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn ticks(&self, device: &wgpu::Device) -> Option<u64> {
        let slice = self.read.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        device.poll(wgpu::PollType::wait_indefinitely()).ok()?;
        let ticks = {
            let mapped = slice.get_mapped_range().ok()?;
            let stamp = |i: usize| u64::from_le_bytes(mapped[i * 8..][..8].try_into().unwrap_or_default());
            stamp(1).wrapping_sub(stamp(0))
        };
        self.read.unmap();
        Some(ticks)
    }

    /// A blocking wait, which the browser does not allow.
    #[cfg(target_arch = "wasm32")]
    fn ticks(&self, _device: &wgpu::Device) -> Option<u64> {
        None
    }
}
