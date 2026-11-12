//! craft-gpu-host on real devices. Every test skips when no adapter is present.

use std::sync::{Mutex, PoisonError};

use craft_gpu_host::{DeviceRequest, Fault, GpuAccess, ScopeKind, scoped};

/// One device test at a time: destroying devices while others are created is its own test
/// (`many_threads_create_use_and_drop_devices_at_once`), not a side effect of running in parallel.
static GPU: Mutex<()> = Mutex::new(());

fn device() -> Option<GpuAccess> {
    let access = GpuAccess::headless(&DeviceRequest::headless("craft-gpu-host test"));
    if access.is_none() {
        eprintln!("skipping: no GPU adapter");
    }
    access
}

/// Polling a destroyed device panics inside wgpu; the health's waits record a loss instead, and a
/// faulted device is never waited on again.
#[test]
fn health_wait_on_a_destroyed_device_is_safe() {
    let _gpu = GPU.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(gpu) = device() else { return };
    let (device, health) = (gpu.device(), gpu.health());
    assert!(health.wait(device, None));
    device.destroy();
    health.wait(device, None);
    health.poll(device);
    assert!(!health.is_ok());
    assert!(!health.wait(device, None));
}

/// PhotoCraft #3438: only wgpu reporting the device lost counts as a real loss. A fault recorded by
/// hand leaves the device usable, and `probe` keeps polling it, so a later real loss is still
/// noticed.
#[test]
fn only_wgpu_reports_a_real_device_loss() {
    let _gpu = GPU.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(gpu) = device() else { return };
    let (device, health) = (gpu.device(), gpu.health());
    health.probe(device);
    assert!(health.is_ok() && !health.device_lost());
    health.mark(Fault::Lost("simulated".into()));
    health.probe(device);
    assert!(!health.is_ok());
    assert!(!health.device_lost(), "a recorded fault is not a real loss");
    device.destroy();
    health.probe(device);
    assert!(health.device_lost(), "a destroyed device is lost: {:?}", health.fault());
    health.probe(device);
    assert!(health.device_lost(), "once lost, probing is a no-op rather than another caught panic");
}

/// A buffer past the device's limit is a validation error. Inside a scope it comes back as an
/// `Err`: observed, the device stays usable; caught, the device is retired. Outside a scope it
/// reaches the device's one handler, which faults the health instead of panicking as wgpu's default
/// handler does (PhotoCraft #243).
#[test]
fn a_watched_validation_error_faults_health() {
    let _gpu = GPU.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(gpu) = device() else { return };
    let (device, health) = (gpu.device(), gpu.health());
    let invalid = || {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("synthetic_invalid_buffer"),
            size: device.limits().max_buffer_size + 1,
            usage: wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    };
    let error = scoped(device, "invalid buffer", invalid).expect_err("the buffer is past the device's limit");
    assert_eq!(error.kind, ScopeKind::Validation);
    health.observe(&error);
    assert!(health.is_ok(), "an observed error leaves the device usable");
    health.caught(device, &error);
    assert!(health.fault().is_some_and(|f| !f.is_lost()), "a caught error retires the device: {:?}", health.fault());

    let Some(other) = crate::device() else { return };
    let _ = other.device().create_buffer(&wgpu::BufferDescriptor {
        label: Some("synthetic_invalid_buffer"),
        size: other.device().limits().max_buffer_size + 1,
        usage: wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    assert!(!other.health().is_ok(), "an uncaptured error faults the health instead of panicking");
    assert_eq!(other.health().errors(), 1);
}

/// A shader the compiler rejects and a pipeline built from it (on some Windows machines FXC failed
/// on egui's own shaders, and wgpu's default handler killed the app at start-up: LightCraft) come
/// back from a scope as an error, not a panic; a valid pipeline passes.
#[test]
fn pipeline_errors_are_captured_not_panics() {
    let _gpu = GPU.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(gpu) = device() else { return };
    let device = gpu.device();
    let pipeline = |source: &str| {
        let module =
            device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("test"), source: wgpu::ShaderSource::Wgsl(source.into()) });
        drop(device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("test_pipeline"),
            layout: None,
            vertex: wgpu::VertexState { module: &module, entry_point: Some("vs_main"), compilation_options: Default::default(), buffers: &[] },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: None,
            multiview_mask: None,
            cache: None,
        }));
    };
    let broken = scoped(device, "broken pipeline", || pipeline("@vertex fn vs_main() -> @builtin(position) vec4<f32> { return undefined_thing; }"));
    assert!(broken.is_err(), "{broken:?}");
    let valid = scoped(device, "valid pipeline", || pipeline("@vertex fn vs_main() -> @builtin(position) vec4<f32> { return vec4<f32>(0.0); }"));
    assert!(valid.is_ok(), "{valid:?}");
    assert!(gpu.health().is_ok(), "scoped errors never reach the device's handler");
}

/// FilmCraft #3418: several threads creating instances and devices, using them and dropping them at
/// once deadlocked inside the NVIDIA driver (the Vulkan loader's critical section against WGL
/// threads that GL-enabled instances start). Each thread makes its own device, as concurrent windows
/// and exports do.
#[test]
fn many_threads_create_use_and_drop_devices_at_once() {
    use std::sync::mpsc;
    use std::time::{Duration, Instant};
    const THREADS: usize = 16;
    const ROUNDS: usize = 4;
    let _gpu = GPU.lock().unwrap_or_else(PoisonError::into_inner);
    if device().is_none() {
        return;
    }
    let (tx, rx) = mpsc::channel();
    for _ in 0..THREADS {
        let tx = tx.clone();
        std::thread::spawn(move || {
            let mut used = 0usize;
            for _ in 0..ROUNDS {
                let Some(gpu) = GpuAccess::headless(&DeviceRequest::headless("craft-gpu-host thread test")) else { break };
                let (device, queue) = (gpu.device(), gpu.queue());
                let tex = device.create_texture(&wgpu::TextureDescriptor {
                    label: None,
                    size: wgpu::Extent3d { width: 64, height: 64, depth_or_array_layers: 1 },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                    view_formats: &[],
                });
                let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
                let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                drop(enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::RED), store: wgpu::StoreOp::Store },
                    })],
                    ..Default::default()
                }));
                queue.submit([enc.finish()]);
                let _ = device.poll(wgpu::PollType::wait_indefinitely());
                used += 1;
                drop((view, tex, gpu));
            }
            let _ = tx.send(used);
        });
    }
    drop(tx);
    let deadline = Instant::now() + Duration::from_secs(120);
    for i in 0..THREADS {
        let left = deadline.saturating_duration_since(Instant::now());
        assert!(rx.recv_timeout(left).is_ok(), "only {i} of {THREADS} threads finished within 120 s: device setup deadlocked");
    }
}
