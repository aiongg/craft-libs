//! The edits of the Properties panel: a layer's rectangle, its alignment to the page and an
//! adjustment layer's values.

use super::{Doc, LayerId, LayerKind, Mock};

/// Where an align button puts the layer on the page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Left,
    Centre,
    Right,
    Top,
    Middle,
    Bottom,
}

/// One number of a layer's `[x, y, width, height]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RectField {
    X,
    Y,
    W,
    H,
}

/// One value of an adjustment layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdjustField {
    Brightness(i32),
    Contrast(i32),
    Clip(bool),
}

impl Doc {
    /// Puts the layer's edge or centre on the page's. A locked layer stays. Returns whether the
    /// layer moved.
    pub fn align(&mut self, id: LayerId, how: Align) -> bool {
        let [page_w, page_h] = self.size.map(|s| s as f32);
        let Some(layer) = self.layer_mut(id).filter(|l| !l.locked) else { return false };
        let [x, y, w, h] = layer.rect;
        let moved = match how {
            Align::Left => [0.0, y],
            Align::Centre => [(page_w - w) / 2.0, y],
            Align::Right => [page_w - w, y],
            Align::Top => [x, 0.0],
            Align::Middle => [x, (page_h - h) / 2.0],
            Align::Bottom => [x, page_h - h],
        };
        let changed = moved != [x, y];
        layer.rect = [moved[0], moved[1], w, h];
        self.modified |= changed;
        changed
    }

    /// Sets one number of the layer's rectangle. Sizes stay at 1 or more. With `keep_ratio` a
    /// width or height change scales the other to keep the ratio; the position stays. A locked
    /// layer stays. Returns whether the rectangle changed.
    pub fn set_rect_field(&mut self, id: LayerId, field: RectField, value: f64, keep_ratio: bool) -> bool {
        let Some(layer) = self.layer_mut(id).filter(|l| !l.locked) else { return false };
        let before = layer.rect;
        let [x, y, w, h] = before;
        let value = value as f32;
        let size = value.max(1.0);
        layer.rect = match field {
            RectField::X => [value, y, w, h],
            RectField::Y => [x, value, w, h],
            RectField::W if keep_ratio && w > 0.0 => [x, y, size, (h * size / w).max(1.0)],
            RectField::H if keep_ratio && h > 0.0 => [x, y, (w * size / h).max(1.0), size],
            RectField::W => [x, y, size, h],
            RectField::H => [x, y, w, size],
        };
        let changed = layer.rect != before;
        self.modified |= changed;
        changed
    }

    /// Sets one value of an adjustment layer, clamped to its range. Another kind of layer and a
    /// locked one stay. Returns whether the value changed.
    pub fn set_adjustment(&mut self, id: LayerId, change: AdjustField) -> bool {
        let Some(layer) = self.layer_mut(id).filter(|l| !l.locked) else { return false };
        let LayerKind::Adjustment { brightness, contrast, clip } = &mut layer.kind else { return false };
        let before = (*brightness, *contrast, *clip);
        match change {
            AdjustField::Brightness(v) => *brightness = v.clamp(-150, 150),
            AdjustField::Contrast(v) => *contrast = v.clamp(-50, 100),
            AdjustField::Clip(on) => *clip = on,
        }
        let changed = before != (*brightness, *contrast, *clip);
        self.modified |= changed;
        changed
    }
}

impl Mock {
    /// Aligns the selected layer to the page, as the Properties panel and the Move options bar
    /// ask. Records "Align" when the layer moved.
    pub fn align_selected(&mut self, how: Align) {
        let Some(id) = self.doc.selected_layer().map(|l| l.id) else { return };
        if self.doc.align(id, how) {
            self.record("Align");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_linked_width_scales_the_height_and_moves_nothing_else() {
        let mut doc = Doc::default();
        assert!(doc.set_rect_field(3, RectField::W, 250.0, true));
        assert_eq!(doc.layer(3).unwrap().rect, [500.0, 300.0, 250.0, 200.0]);
        assert!(doc.set_rect_field(3, RectField::H, 100.0, false));
        assert_eq!(doc.layer(3).unwrap().rect, [500.0, 300.0, 250.0, 100.0]);
        assert!(!doc.set_rect_field(1, RectField::X, 5.0, false), "the locked background stays");
    }

    #[test]
    fn align_puts_an_edge_or_the_centre_on_the_page() {
        let mut doc = Doc::default();
        doc.align(3, Align::Right);
        doc.align(3, Align::Bottom);
        assert_eq!(doc.layer(3).unwrap().rect, [700.0, 400.0, 500.0, 400.0]);
        doc.align(3, Align::Centre);
        doc.align(3, Align::Middle);
        assert_eq!(doc.layer(3).unwrap().rect, [350.0, 200.0, 500.0, 400.0]);
        assert!(!doc.align(3, Align::Centre), "already there");
    }

    #[test]
    fn adjustment_values_are_clamped_and_other_layers_ignore_them() {
        let mut doc = Doc::default();
        let id = doc.add_adjustment("Brightness/Contrast");
        assert!(doc.set_adjustment(id, AdjustField::Brightness(400)));
        assert!(doc.set_adjustment(id, AdjustField::Contrast(-90)));
        assert_eq!(doc.layer(id).unwrap().kind, LayerKind::Adjustment { brightness: 150, contrast: -50, clip: false });
        assert!(!doc.set_adjustment(3, AdjustField::Clip(true)));
    }
}
