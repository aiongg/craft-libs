//! The renderer's copy of the [`ImageStore`]: one Skia image per key.

use std::collections::{HashMap, HashSet};

use craft_ui_rt::{ImageChange, ImageKey, ImageOptions, ImageStore, Media, RgbaImage};
use skia_safe::{AlphaType, ColorType, Data, Image, ImageInfo, images};

pub(crate) struct Stored {
    pub image: Image,
    pub options: ImageOptions,
}

#[derive(Default)]
pub(crate) struct Images {
    map: HashMap<ImageKey, Stored>,
    seeded: bool,
}

fn make(image: &RgbaImage) -> Option<Image> {
    let info = ImageInfo::new((image.size.width as i32, image.size.height as i32), ColorType::RGBA8888, AlphaType::Unpremul, None);
    images::raster_from_data(&info, Data::new_copy(&image.pixels), image.size.width as usize * 4)
}

impl Images {
    pub fn get(&self, key: ImageKey) -> Option<&Stored> {
        self.map.get(&key)
    }

    /// Applies the store's changes. An image whose pixels changed is rebuilt whole from the store's
    /// copy; app surfaces that change often are [`crate::SurfaceProducer`] tiles. Returns the
    /// number of images (re)built and the media that changed.
    pub fn sync(&mut self, store: &ImageStore, changed: &mut HashSet<Media>) -> u32 {
        let mut built = 0;
        let mut set = |this: &mut Images, key: ImageKey, changed: &mut HashSet<Media>| {
            let (Some(src), options) = (store.get(key), store.options(key)) else { return };
            if let Some(image) = make(src) {
                this.map.insert(key, Stored { image, options });
                built += 1;
            }
            changed.insert(Media::Image(key));
        };
        if !self.seeded {
            self.seeded = true;
            for (key, _) in store.iter() {
                set(self, key, changed);
            }
        }
        for change in store.changes() {
            match *change {
                ImageChange::Set(key) | ImageChange::Pixels(key, _) => set(self, key, changed),
                ImageChange::Removed(key) => {
                    self.map.remove(&key);
                    changed.insert(Media::Image(key));
                }
            }
        }
        built
    }
}
