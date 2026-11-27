use std::sync::Arc;

use craft_ui_rt::{ImageKey, Rect, RectU, RgbaImage, SizeU, Vec2, pos2, vec2};

use crate::common::{Painted, Scene, Stage, paint, px, renderer, rgba};

const KEY: ImageKey = ImageKey(3);

fn scene() -> Scene {
    Scene::new(renderer(), SizeU::new(80, 60), 1.0, |rt, _| {
        let mut image = RgbaImage::new(SizeU::new(32, 32));
        for y in 0..32 {
            for x in 0..32 {
                image.write(RectU::new(x, y, x + 1, y + 1), &[0, 0, 255, 255]);
            }
        }
        rt.images_mut().insert(KEY, Arc::new(image));
        Stage {
            kids: vec![(
                Vec2::ZERO,
                Painted {
                    id: 1,
                    size: vec2(80.0, 60.0),
                    paint: paint(|cx| cx.image(KEY, Rect::from_min_size(pos2(10.0, 10.0), vec2(32.0, 32.0)), rgba(255, 255, 255, 255))),
                },
            )],
        }
    })
}

#[test]
fn a_sub_rect_image_update_uploads_only_that_rect() {
    let mut s = scene();
    s.frame();
    assert_eq!((s.r.last_sync().full, s.r.last_sync().partial, s.r.last_sync().texels), (1, 0, 32 * 32));
    assert_eq!(px(&s.pixels(), 16, 16), [0, 0, 255, 255]);

    let rect = RectU::new(4, 4, 12, 12);
    let red: Vec<u8> = [255, 0, 0, 255].repeat(64);
    assert!(s.rt.images_mut().update(KEY, rect, &red));
    let stats = s.frame();
    let sync = s.r.last_sync();
    assert_eq!((sync.full, sync.partial, sync.texels), (0, 1, 64), "one 8 x 8 rect went up, not the 32 x 32 image");
    assert_eq!(stats.prepared, 0, "the display list is not tessellated again");
    // The image sits at (10, 10): its texel (6, 6) shows at (16, 16).
    let img = s.pixels();
    assert_eq!(px(&img, 16, 16), [255, 0, 0, 255], "inside the updated rect");
    assert_eq!(px(&img, 11, 11), [0, 0, 255, 255], "texel (1, 1) outside it");
    assert_eq!(px(&img, 30, 30), [0, 0, 255, 255], "texel (20, 20) outside it");
}
