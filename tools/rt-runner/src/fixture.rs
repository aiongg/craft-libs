//! Fake layer document and its operations. No UI dependency: every variant renders and edits the same `Doc`.

use std::collections::BTreeSet;

pub type LayerId = u64;

/// Panel size in logical points (plan section 2).
pub const PANEL_WIDTH: f32 = 300.0;
pub const PANEL_HEIGHT: f32 = 640.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Pixel,
    Type,
    Fill,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Blend {
    Normal,
    Multiply,
}

impl Blend {
    pub fn label(self) -> &'static str {
        match self {
            Blend::Normal => "Normal",
            Blend::Multiply => "Multiply",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layer {
    pub id: LayerId,
    pub name: String,
    pub kind: Kind,
    pub visible: bool,
    pub locked: bool,
    pub blend: Blend,
    /// Drives the thumbnail colours.
    pub seed: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectMode {
    /// Plain click: the layer alone.
    Replace,
    /// Cmd/Ctrl click: toggle the layer in the selection.
    Toggle,
    /// Shift click: the range from the anchor to the layer.
    Range,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Op {
    ToggleVisible(LayerId),
    ToggleLock(LayerId),
    Rename(LayerId, String),
    Select(LayerId, SelectMode),
    /// The selection becomes `ids`, the last of them active. The range anchor stays: a caller that sets whole
    /// selections keeps its own.
    SetSelection(Vec<LayerId>),
    ClearSelection,
    /// Move (or, with `copy`, duplicate) `ids` next to `target`. `ids` keep their current relative order.
    Move {
        ids: Vec<LayerId>,
        target: LayerId,
        after: bool,
        copy: bool,
    },
    /// New layer above the active layer (top of the list when none).
    AddLayer,
    DeleteSelected,
}

/// Layers in panel order: index 0 is the top row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Doc {
    pub layers: Vec<Layer>,
    pub selection: BTreeSet<LayerId>,
    pub active: Option<LayerId>,
    anchor: Option<LayerId>,
    next_id: LayerId,
}

/// Names, not ids, so snapshots compare across variants and across `reset`s.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DocSnapshot {
    pub order: Vec<String>,
    pub selected: Vec<String>,
    pub active: Option<String>,
    pub hidden: Vec<String>,
    pub locked: Vec<String>,
}

impl Doc {
    /// `n` layers: "Layer 1" (top) .. "Layer n-1", then "Background" (locked, bottom, id 0).
    /// Ids 1.. follow a 20-layer pattern. "Layer 1" is selected and active.
    pub fn new(n: usize) -> Doc {
        let n = n.max(1);
        let mut layers: Vec<Layer> = (1..n as u64).map(pattern_layer).collect();
        layers.push(Layer { id: 0, name: "Background".into(), kind: Kind::Pixel, visible: true, locked: true, blend: Blend::Normal, seed: 0 });
        let first = layers.first().map(|l| l.id);
        Doc { layers, selection: first.into_iter().collect(), active: first, anchor: first, next_id: n as u64 }
    }

    pub fn index_of(&self, id: LayerId) -> Option<usize> {
        self.layers.iter().position(|l| l.id == id)
    }

    pub fn layer(&self, id: LayerId) -> Option<&Layer> {
        self.layers.iter().find(|l| l.id == id)
    }

    pub fn layer_by_name(&self, name: &str) -> Option<&Layer> {
        self.layers.iter().find(|l| l.name == name)
    }

    pub fn is_selected(&self, id: LayerId) -> bool {
        self.selection.contains(&id)
    }

    pub fn apply(&mut self, op: Op) {
        match op {
            Op::ToggleVisible(id) => {
                if let Some(l) = self.layer_mut(id) {
                    l.visible = !l.visible;
                }
            }
            Op::ToggleLock(id) => {
                if let Some(l) = self.layer_mut(id) {
                    l.locked = !l.locked;
                }
            }
            Op::Rename(id, name) => {
                if let Some(l) = self.layer_mut(id) {
                    l.name = name;
                }
            }
            Op::Select(id, mode) => self.select(id, mode),
            Op::SetSelection(ids) => {
                self.selection = ids.iter().copied().filter(|id| self.index_of(*id).is_some()).collect();
                self.active = ids.last().copied().filter(|id| self.selection.contains(id));
            }
            Op::ClearSelection => {
                self.selection.clear();
                self.active = None;
            }
            Op::Move { ids, target, after, copy } => self.move_layers(&ids, target, after, copy),
            Op::AddLayer => self.add_layer(),
            Op::DeleteSelected => self.delete_selected(),
        }
    }

    pub fn snapshot(&self) -> DocSnapshot {
        let name = |id: LayerId| self.layer(id).map(|l| l.name.clone());
        DocSnapshot {
            order: self.layers.iter().map(|l| l.name.clone()).collect(),
            selected: self.layers.iter().filter(|l| self.is_selected(l.id)).map(|l| l.name.clone()).collect(),
            active: self.active.and_then(name),
            hidden: self.layers.iter().filter(|l| !l.visible).map(|l| l.name.clone()).collect(),
            locked: self.layers.iter().filter(|l| l.locked).map(|l| l.name.clone()).collect(),
        }
    }

    fn layer_mut(&mut self, id: LayerId) -> Option<&mut Layer> {
        self.layers.iter_mut().find(|l| l.id == id)
    }

    fn select(&mut self, id: LayerId, mode: SelectMode) {
        let Some(idx) = self.index_of(id) else { return };
        match mode {
            SelectMode::Replace => {
                self.selection = BTreeSet::from([id]);
                self.active = Some(id);
                self.anchor = Some(id);
            }
            SelectMode::Toggle => {
                if !self.selection.remove(&id) {
                    self.selection.insert(id);
                    self.active = Some(id);
                } else if self.active == Some(id) {
                    self.active = self.selection.iter().next().copied();
                }
                self.anchor = Some(id);
            }
            SelectMode::Range => {
                let anchor = self.anchor.and_then(|a| self.index_of(a)).unwrap_or(idx);
                let (lo, hi) = (anchor.min(idx), anchor.max(idx));
                self.selection = self.layers[lo..=hi].iter().map(|l| l.id).collect();
                self.active = Some(id);
            }
        }
    }

    fn move_layers(&mut self, ids: &[LayerId], target: LayerId, after: bool, copy: bool) {
        if ids.contains(&target) && !copy {
            return;
        }
        let moving: Vec<Layer> = self.layers.iter().filter(|l| ids.contains(&l.id)).cloned().collect();
        let moving: Vec<Layer> = if copy {
            moving
                .into_iter()
                .map(|mut l| {
                    l.id = self.next_id;
                    self.next_id += 1;
                    l.name = format!("{} copy", l.name);
                    l
                })
                .collect()
        } else {
            self.layers.retain(|l| !ids.contains(&l.id));
            moving
        };
        let Some(t) = self.index_of(target) else { return };
        let at = if after { t + 1 } else { t };
        if copy {
            self.selection = moving.iter().map(|l| l.id).collect();
            self.active = moving.first().map(|l| l.id);
        }
        self.layers.splice(at..at, moving);
    }

    fn add_layer(&mut self) {
        let id = self.next_id;
        self.next_id += 1;
        let mut layer = pattern_layer(id);
        layer.visible = true;
        layer.locked = false;
        layer.blend = Blend::Normal;
        layer.kind = Kind::Pixel;
        layer.name = format!("Layer {id}");
        let at = self.active.and_then(|a| self.index_of(a)).unwrap_or(0);
        self.layers.insert(at, layer);
        self.selection = BTreeSet::from([id]);
        self.active = Some(id);
        self.anchor = Some(id);
    }

    fn delete_selected(&mut self) {
        let first = self.layers.iter().position(|l| self.selection.contains(&l.id));
        self.layers.retain(|l| !self.selection.contains(&l.id));
        self.selection.clear();
        let neighbour = first.and_then(|i| self.layers.get(i.min(self.layers.len().saturating_sub(1)))).map(|l| l.id);
        self.active = neighbour;
        self.anchor = neighbour;
        if let Some(id) = neighbour {
            self.selection.insert(id);
        }
    }
}

fn pattern_layer(id: LayerId) -> Layer {
    let p = id % 20;
    Layer {
        id,
        name: format!("Layer {id}"),
        kind: match p {
            5 => Kind::Type,
            9 => Kind::Fill,
            _ => Kind::Pixel,
        },
        visible: !matches!(p, 3 | 13),
        locked: p == 11,
        blend: if p == 7 { Blend::Multiply } else { Blend::Normal },
        seed: (id as u32).wrapping_mul(2654435761),
    }
}

impl Layer {
    /// Straight RGBA8, `px` x `px`, row-major. Deterministic: every variant uploads the same pixels.
    pub fn thumbnail(&self, px: u32) -> Vec<u8> {
        let mut out = Vec::with_capacity((px * px * 4) as usize);
        let base = [(self.seed >> 16) as u8, (self.seed >> 8) as u8, self.seed as u8];
        for y in 0..px {
            for x in 0..px {
                let rgb = match self.kind {
                    Kind::Pixel => {
                        let t = ((x + y) * 255 / (2 * px.max(2) - 2)) as u8;
                        [base[0] ^ t, base[1].wrapping_add(t / 2), base[2] ^ (255 - t)]
                    }
                    Kind::Fill => base,
                    Kind::Type => {
                        // Ink glyph block on paper.
                        let ink =
                            x > px / 4 && x < px * 3 / 4 && (y > px / 4 && y < px / 2 || (x > px * 7 / 16 && x < px * 9 / 16 && y < px * 3 / 4));
                        if ink { [24, 24, 28] } else { [244, 242, 236] }
                    }
                };
                out.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
            }
        }
        out
    }
}
