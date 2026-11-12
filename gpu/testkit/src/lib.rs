#![forbid(unsafe_code)]

//! GPU test support for the `gpu/` engine crates. Test support only.
//!
//! A GPU test target calls [`gpu`] first and returns when it gives `None`: machines and CI
//! runners without an adapter (or whose adapter can't run compute shaders) skip GPU tests. A
//! probe is a compute shader over two storage buffers, [`PROBE_BINDINGS`]: the test writes its
//! inputs as `f32`s, the shader evaluates the WGSL under test into `probe_out`, and the test
//! compares that with the Rust twin.

use wgpu::util::DeviceExt;

/// The probe's bindings: `probe_in` (read) and `probe_out` (written), both `array<f32>`.
pub const PROBE_BINDINGS: &str = "@group(0) @binding(0) var<storage, read> probe_in: array<f32>;\n\
@group(0) @binding(1) var<storage, read_write> probe_out: array<f32>;\n";

/// Invocations per workgroup the probe entry point must declare: `@workgroup_size(64)`.
pub const WORKGROUP: u32 = 64;

/// A headless device for probes.
pub struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    /// The adapter's name and backend, for failure messages.
    pub name: String,
}

/// A device on the default adapter, or `None` (with a note on stderr) when there is no adapter
/// that runs compute shaders.
pub fn gpu() -> Option<Gpu> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())) else {
        eprintln!("no GPU adapter: GPU test skipped");
        return None;
    };
    if !adapter.get_downlevel_capabilities().flags.contains(wgpu::DownlevelFlags::COMPUTE_SHADERS) {
        eprintln!("the adapter runs no compute shaders: GPU test skipped");
        return None;
    }
    let info = adapter.get_info();
    let desc = wgpu::DeviceDescriptor { label: Some("craft-gpu-testkit"), required_limits: adapter.limits(), ..Default::default() };
    match pollster::block_on(adapter.request_device(&desc)) {
        Ok((device, queue)) => Some(Gpu { device, queue, name: format!("{} ({:?})", info.name, info.backend) }),
        Err(e) => {
            eprintln!("no GPU device ({e}): GPU test skipped");
            None
        }
    }
}

impl Gpu {
    /// Runs entry point `entry` of `source` (which must declare [`PROBE_BINDINGS`]) over
    /// `invocations` threads with `input` in `probe_in`, and returns `outputs` values of
    /// `probe_out` (zeroed before the run). A shader that doesn't compile panics with wgpu's
    /// message.
    pub fn run(&self, source: &str, entry: &str, input: &[f32], outputs: usize, invocations: u32) -> Vec<f32> {
        let module =
            self.device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some(entry), source: wgpu::ShaderSource::Wgsl(source.into()) });
        let pipeline = self.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(entry),
            layout: None,
            module: &module,
            entry_point: Some(entry),
            compilation_options: Default::default(),
            cache: None,
        });
        // Storage bindings can't be empty.
        let bytes: Vec<u8> = input.iter().chain(input.is_empty().then_some(&0.0)).flat_map(|v| v.to_le_bytes()).collect();
        let input_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("probe_in"),
            contents: &bytes,
            usage: wgpu::BufferUsages::STORAGE,
        });
        let size = (outputs.max(1) * 4) as u64;
        let output_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("probe_out"),
            size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("probe readback"),
            size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("probe"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: input_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: output_buf.as_entire_binding() },
            ],
        });
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("probe") });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some(entry), timestamp_writes: None });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(invocations.div_ceil(WORKGROUP), 1, 1);
        }
        encoder.copy_buffer_to_buffer(&output_buf, 0, &readback, 0, size);
        self.queue.submit([encoder.finish()]);
        readback.map_async(wgpu::MapMode::Read, .., |r| {
            if let Err(e) = r {
                panic!("probe readback failed: {e}");
            }
        });
        if let Err(e) = self.device.poll(wgpu::PollType::wait_indefinitely()) {
            panic!("{}: waiting for the probe failed: {e}", self.name);
        }
        let out = {
            let view = readback.get_mapped_range(..).unwrap_or_else(|e| panic!("probe readback not mapped: {e}"));
            view.chunks_exact(4).take(outputs).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect()
        };
        readback.unmap();
        out
    }
}

/// Whether `gpu` is within `tol · max(1, |cpu|)` of `cpu`; two NaNs count as equal.
pub fn close(cpu: f32, gpu: f32, tol: f32) -> bool {
    if cpu.is_nan() || gpu.is_nan() {
        return cpu.is_nan() && gpu.is_nan();
    }
    if cpu == gpu {
        return true;
    }
    (cpu - gpu).abs() <= tol * cpu.abs().max(1.0)
}
