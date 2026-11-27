use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use craft_ui_render_epaint::{SurfaceHook, SurfaceInfo, SurfacePrepare, wgpu};
use craft_ui_rt::{Rect, SizeU, SurfaceKey, Vec2, pos2, vec2};

use crate::common::{Painted, Scene, Stage, paint, renderer};

const KEY: SurfaceKey = SurfaceKey(5);

#[derive(Default)]
struct Counting {
    prepared: AtomicU32,
    painted: AtomicU32,
}

impl SurfaceHook for Counting {
    fn prepare(&self, _: &wgpu::Device, _: &wgpu::Queue, _: &mut wgpu::CommandEncoder, info: &SurfacePrepare) -> Vec<wgpu::CommandBuffer> {
        assert_eq!(info.rect, Rect::from_min_size(pos2(20.0, 10.0), vec2(40.0, 30.0)));
        self.prepared.fetch_add(1, Ordering::Relaxed);
        Vec::new()
    }

    fn paint(&self, info: &SurfaceInfo, _: &mut wgpu::RenderPass<'static>) {
        assert_eq!(info.viewport, Rect::from_min_size(pos2(20.0, 10.0), vec2(40.0, 30.0)));
        self.painted.fetch_add(1, Ordering::Relaxed);
    }
}

#[test]
fn a_surface_item_calls_the_apps_hook_inside_the_frame() {
    let hook = Arc::new(Counting::default());
    let mut s = Scene::new(renderer(), SizeU::new(80, 60), 1.0, |_, _| Stage {
        kids: vec![(
            Vec2::ZERO,
            Painted { id: 1, size: vec2(80.0, 60.0), paint: paint(|cx| cx.surface(KEY, Rect::from_min_size(pos2(20.0, 10.0), vec2(40.0, 30.0)))) },
        )],
    });
    s.r.set_surface_hook(KEY, hook.clone());
    // A hook the window was drawn without is still called when the surface is invalidated.
    s.frame();
    assert_eq!((hook.prepared.load(Ordering::Relaxed), hook.painted.load(Ordering::Relaxed)), (1, 1));
    // A frame with no change draws nothing.
    s.frame();
    assert_eq!(hook.painted.load(Ordering::Relaxed), 1);
    // The app says its surface changed: the surface's rect is damaged and drawn again.
    s.rt.invalidate_surface(KEY, None);
    s.frame();
    assert_eq!(hook.painted.load(Ordering::Relaxed), 2);
    assert!(s.damage.iter().all(|d| d.width() * d.height() <= 40.0 * 30.0 + 4.0 * 70.0), "{:?}", s.damage);
}
