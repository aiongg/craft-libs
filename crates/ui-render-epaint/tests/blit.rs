//! Risk R11 (spike plan 7.1): a window frame copies the whole canvas onto the swapchain image, on
//! top of the damage it drew. This measures that copy (the renderer's `TextureBlitter` pass,
//! submitted and waited for) at common window sizes. A measurement, so it runs only when asked,
//! in a release build: `cargo test --release -p craft-ui-render-epaint -- --ignored r11`.

use craft_ui_render_epaint::wgpu;

use crate::common::{gpu, renderer};

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

#[test]
#[ignore]
fn r11_full_window_blit_per_presented_frame() {
    let gpu = gpu();
    let r = renderer(&gpu);
    let (device, queue, format) = (r.device(), r.queue(), r.target_format());
    // An X11 or Windows swapchain is BGRA; the canvas keeps the renderer's format.
    let dst_format = wgpu::TextureFormat::Bgra8Unorm;
    let blitter = wgpu::util::TextureBlitter::new(device, dst_format);
    for (w, h) in [(1280u32, 800u32), (1920, 1080), (2560, 1440)] {
        let texture = |format, usage| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some("r11"),
                size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        let canvas = texture(format, wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT);
        let image = texture(dst_format, wgpu::TextureUsages::RENDER_ATTACHMENT);
        let (src, dst) = (canvas.create_view(&Default::default()), image.create_view(&Default::default()));
        let mut samples = Vec::new();
        for i in 0..40 {
            let t = web_time::Instant::now();
            let mut encoder = device.create_command_encoder(&Default::default());
            blitter.copy(device, &mut encoder, &src, &dst);
            queue.submit([encoder.finish()]);
            device.poll(wgpu::PollType::wait_indefinitely()).expect("poll");
            if i >= 5 {
                samples.push(t.elapsed().as_secs_f64() * 1000.0);
            }
        }
        eprintln!("R11 full-window blit {w}x{h}: median {:.2} ms", median(samples));
    }
}
