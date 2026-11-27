//! The layer document the panel shows and the operations it sends. The panel never mutates it: the host
//! applies each `Op` (`Doc::apply` is the reference model hosts and the runner's fixture use).

use std::collections::BTreeSet;

use craft_ui_rt::{ImageKey, Pos2, Rect};

pub type LayerId = u64;

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

#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    pub id: LayerId,
    pub name: String,
    pub kind: Kind,
    pub visible: bool,
    pub locked: bool,
    pub blend: Blend,
    /// The thumbnail's picture in the runtime's image store, set by the host; none draws no picture.
    pub thumb: Option<ImageKey>,
    /// The thumbnail's region of that picture, in 0..1 of each side.
    pub thumb_uv: Rect,
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
#[derive(Clone, Debug, PartialEq)]
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
    /// A document of `layers` in panel order (index 0 is the top row) with `selection` selected and `active` the
    /// active layer, which is also the range anchor. New layers get ids above the largest.
    pub fn from_rows(layers: Vec<Layer>, selection: impl IntoIterator<Item = LayerId>, active: Option<LayerId>) -> Doc {
        let next_id = layers.iter().map(|l| l.id + 1).max().unwrap_or(0);
        Doc { layers, selection: selection.into_iter().collect(), active, anchor: active, next_id }
    }

    /// The thumbnail region that shows a whole picture.
    pub const FULL_UV: Rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));

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
        let layer = Layer {
            id,
            name: format!("Layer {id}"),
            kind: Kind::Pixel,
            visible: true,
            locked: false,
            blend: Blend::Normal,
            thumb: None,
            thumb_uv: Doc::FULL_UV,
        };
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
