//! The document: its layers and the pure edits on them.

/// Identifies a layer for as long as the document lives.
pub type LayerId = u64;

/// How a layer's pixels combine with the ones below. Listed in the blend select; only the opacity
/// changes the picture.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Blend {
    #[default]
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
}

impl Blend {
    pub const ALL: [Blend; 6] = [Blend::Normal, Blend::Multiply, Blend::Screen, Blend::Overlay, Blend::Darken, Blend::Lighten];

    pub fn label(self) -> &'static str {
        match self {
            Blend::Normal => "Normal",
            Blend::Multiply => "Multiply",
            Blend::Screen => "Screen",
            Blend::Overlay => "Overlay",
            Blend::Darken => "Darken",
            Blend::Lighten => "Lighten",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum LayerKind {
    /// A filled block.
    Pixel { color: [u8; 3] },
    /// A transparent pixel layer: nothing is painted.
    Empty,
    /// A Brightness/Contrast layer; `clip` limits it to the layer below.
    Adjustment { brightness: i32, contrast: i32, clip: bool },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    pub id: LayerId,
    pub name: String,
    pub kind: LayerKind,
    pub visible: bool,
    pub locked: bool,
    /// Percent.
    pub opacity: u8,
    /// Percent.
    pub fill: u8,
    pub blend: Blend,
    /// `[x, y, width, height]` in document pixels.
    pub rect: [f32; 4],
}

impl Layer {
    fn new(id: LayerId, name: impl Into<String>, kind: LayerKind, rect: [f32; 4]) -> Self {
        Self { id, name: name.into(), kind, visible: true, locked: false, opacity: 100, fill: 100, blend: Blend::Normal, rect }
    }

    pub fn is_adjustment(&self) -> bool {
        matches!(self.kind, LayerKind::Adjustment { .. })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Doc {
    /// Width and height in pixels.
    pub size: [u32; 2],
    /// Top first.
    pub layers: Vec<Layer>,
    pub selected: Vec<LayerId>,
    pub next_id: LayerId,
    pub name: String,
    pub modified: bool,
    /// Pixels per inch.
    pub resolution: f64,
    /// The marquee selection as `[x0, y0, x1, y1]` in document pixels.
    pub selection: Option<[f64; 4]>,
}

impl Default for Doc {
    /// The document of the PhotoCraft captures: a locked white background, a hidden sky and the
    /// selected orange block. It has never been saved, so it counts as modified.
    fn default() -> Self {
        let mut background = Layer::new(1, "Background", LayerKind::Pixel { color: [255, 255, 255] }, [0.0, 0.0, 1200.0, 800.0]);
        background.locked = true;
        let mut sky = Layer::new(2, "Sky", LayerKind::Pixel { color: [92, 160, 232] }, [0.0, 0.0, 1200.0, 400.0]);
        sky.visible = false;
        let shapes = Layer::new(3, "Shapes", LayerKind::Pixel { color: [240, 140, 40] }, [500.0, 300.0, 500.0, 400.0]);
        Self {
            size: [1200, 800],
            layers: vec![shapes, sky, background],
            selected: vec![3],
            next_id: 4,
            name: "Untitled".into(),
            modified: true,
            resolution: 72.0,
            selection: None,
        }
    }
}

/// `brightness` (-150..=150) and `contrast` (-50..=100) applied to a colour.
pub fn adjust(color: [u8; 3], brightness: i32, contrast: i32) -> [u8; 3] {
    let k = f64::from(contrast.clamp(-50, 100));
    let factor = (259.0 * (k + 255.0)) / (255.0 * (259.0 - k));
    color.map(|c| {
        let c = f64::from(c) + f64::from(brightness.clamp(-150, 150));
        (factor * (c - 128.0) + 128.0).round().clamp(0.0, 255.0) as u8
    })
}

impl Doc {
    pub fn layer(&self, id: LayerId) -> Option<&Layer> {
        self.layers.iter().find(|l| l.id == id)
    }

    pub fn layer_mut(&mut self, id: LayerId) -> Option<&mut Layer> {
        self.layers.iter_mut().find(|l| l.id == id)
    }

    pub fn index_of(&self, id: LayerId) -> Option<usize> {
        self.layers.iter().position(|l| l.id == id)
    }

    /// The selected layer when exactly one is selected.
    pub fn selected_layer(&self) -> Option<&Layer> {
        match self.selected.as_slice() {
            [id] => self.layer(*id),
            _ => None,
        }
    }

    pub fn toggle_visible(&mut self, id: LayerId) -> bool {
        let Some(layer) = self.layer_mut(id) else { return false };
        layer.visible = !layer.visible;
        self.modified = true;
        true
    }

    /// Renames a layer; a locked layer keeps its name. Returns whether the name changed.
    pub fn rename(&mut self, id: LayerId, name: &str) -> bool {
        let name = name.trim();
        match self.layer_mut(id) {
            Some(layer) if !layer.locked && !name.is_empty() && layer.name != name => {
                layer.name = name.to_owned();
                self.modified = true;
                true
            }
            _ => false,
        }
    }

    /// The index a new layer takes: above the topmost selected layer, else at the top.
    fn insert_at(&self) -> usize {
        self.selected.iter().filter_map(|id| self.index_of(*id)).min().unwrap_or(0)
    }

    fn add(&mut self, name: String, kind: LayerKind, rect: [f32; 4]) -> LayerId {
        let id = self.next_id;
        self.next_id += 1;
        let at = self.insert_at();
        self.layers.insert(at, Layer::new(id, name, kind, rect));
        self.selected = vec![id];
        self.modified = true;
        id
    }

    /// An empty layer named "Layer N", N counting the layers made so far.
    pub fn add_layer(&mut self) -> LayerId {
        let name = format!("Layer {}", self.next_id);
        let rect = [0.0, 0.0, self.size[0] as f32, self.size[1] as f32];
        self.add(name, LayerKind::Empty, rect)
    }

    /// A Brightness/Contrast layer named `name`.
    pub fn add_adjustment(&mut self, name: &str) -> LayerId {
        let rect = [0.0, 0.0, self.size[0] as f32, self.size[1] as f32];
        self.add(name.to_owned(), LayerKind::Adjustment { brightness: 0, contrast: 0, clip: false }, rect)
    }

    /// Deletes the unlocked layers among `ids`; the selection falls to the layer now at the
    /// first deleted position. Returns how many went.
    pub fn delete(&mut self, ids: &[LayerId]) -> usize {
        let first = ids.iter().filter_map(|id| self.index_of(*id)).min();
        let before = self.layers.len();
        self.layers.retain(|l| l.locked || !ids.contains(&l.id));
        let removed = before - self.layers.len();
        if removed > 0 {
            self.selected.retain(|id| self.layers.iter().any(|l| l.id == *id));
            if self.selected.is_empty()
                && let Some(layer) = first.and_then(|i| self.layers.get(i.min(self.layers.len().saturating_sub(1))))
            {
                self.selected = vec![layer.id];
            }
            self.modified = true;
        }
        removed
    }

    /// The colour the pixel layer at `index` shows: its fill with the visible adjustment layers
    /// above it applied, the nearest first. A clipped adjustment applies to the layer directly
    /// below it only. `None` for an adjustment layer and an empty one.
    pub fn shown_color(&self, index: usize) -> Option<[u8; 3]> {
        let LayerKind::Pixel { color } = self.layers.get(index)?.kind else {
            return None;
        };
        let mut shown = color;
        for above in (0..index).rev() {
            let layer = &self.layers[above];
            if let LayerKind::Adjustment { brightness, contrast, clip } = layer.kind
                && layer.visible
                && (!clip || above + 1 == index)
            {
                shown = adjust(shown, brightness, contrast);
            }
        }
        Some(shown)
    }
}
