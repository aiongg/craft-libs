//! The mock app's data: plain values with no egui types, so the edits unit-test without a
//! context.

mod doc;
mod size;
mod tools;

pub use doc::{Blend, Doc, Layer, LayerId, LayerKind, adjust};
pub use size::{ImageSizeDraft, Resample, SizeUnit};
pub use tools::{
    BrushOptions, EraserOptions, EyedropperOptions, GradientOptions, MarqueeOptions, MarqueeStyle, MoveOptions, MoveTarget, SampleSize, Tool, Tools,
};

/// The foreground and background colours.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Colors {
    pub fg: [u8; 3],
    pub bg: [u8; 3],
    /// The background well is the active one.
    pub bg_active: bool,
}

impl Default for Colors {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl Colors {
    pub const DEFAULT: Colors = Colors { fg: [0, 0, 0], bg: [255, 255, 255], bg_active: false };

    pub fn swap(&mut self) {
        std::mem::swap(&mut self.fg, &mut self.bg);
    }

    /// Sets the active well's colour.
    pub fn set_active(&mut self, color: [u8; 3]) {
        *(if self.bg_active { &mut self.bg } else { &mut self.fg }) = color;
    }
}

/// The canvas view: where the document sits and how large.
#[derive(Clone, Debug, PartialEq)]
pub struct View {
    /// The document point at the middle of the canvas region.
    pub center: [f64; 2],
    /// 1.0 is 100%.
    pub zoom: f64,
    pub rulers: bool,
    /// Fit on Screen was asked for; the canvas answers it with [`View::fit`] once it knows its size.
    pub fit_pending: bool,
}

impl Default for View {
    fn default() -> Self {
        Self { center: [600.0, 400.0], zoom: 1.0, rulers: true, fit_pending: false }
    }
}

impl View {
    pub const ZOOM: std::ops::RangeInclusive<f64> = 0.01..=64.0;

    pub fn set_zoom(&mut self, zoom: f64) {
        self.zoom = zoom.clamp(*Self::ZOOM.start(), *Self::ZOOM.end());
    }

    /// Shows the whole document in a region of `region` points, with a margin of 5% round it.
    pub fn fit(&mut self, region: [f64; 2], doc: [u32; 2]) {
        let doc = doc.map(f64::from);
        self.set_zoom((region[0] / doc[0]).min(region[1] / doc[1]) * 0.95);
        self.center = [doc[0] / 2.0, doc[1] / 2.0];
        self.fit_pending = false;
    }
}

/// Everything the mock app edits.
#[derive(Clone, Debug, PartialEq)]
pub struct Mock {
    pub doc: Doc,
    pub tools: Tools,
    pub colors: Colors,
    pub view: View,
    /// What happened to the document, oldest first.
    pub history: Vec<String>,
    pub status: String,
    /// The Image Size dialog's draft while it is open.
    pub dialog: Option<ImageSizeDraft>,
}

impl Default for Mock {
    fn default() -> Self {
        Self {
            doc: Doc::default(),
            tools: Tools::default(),
            colors: Colors::default(),
            view: View::default(),
            history: vec!["Open".to_owned()],
            status: String::new(),
            dialog: None,
        }
    }
}

impl Mock {
    /// Makes `tool` current and says so in the status bar. A tool that does not work is ignored.
    pub fn select_tool(&mut self, tool: Tool) {
        if tool.is_working() && self.tools.current != tool {
            self.tools.current = tool;
            self.status = format!("{} tool", tool.label());
        }
    }

    pub fn record(&mut self, what: impl Into<String>) {
        self.history.push(what.into());
    }

    pub fn toggle_visible(&mut self, id: LayerId) {
        if self.doc.toggle_visible(id) {
            self.record("Layer visibility");
        }
    }

    pub fn rename_layer(&mut self, id: LayerId, name: &str) {
        if self.doc.rename(id, name) {
            self.record("Rename layer");
        }
    }

    pub fn move_layer(&mut self, id: LayerId, to: usize) {
        if self.doc.move_layer(id, to) {
            self.record("Move layer");
        }
    }

    pub fn add_layer(&mut self) {
        self.doc.add_layer();
        self.record("New layer");
    }

    pub fn add_adjustment(&mut self, name: &str) {
        self.doc.add_adjustment(name);
        self.record(name.to_owned());
    }

    pub fn delete_selected(&mut self) {
        let ids = self.doc.selected.clone();
        if self.doc.delete(&ids) > 0 {
            self.record("Delete layer");
        }
    }

    /// Opens the Image Size dialog's draft from the document.
    pub fn open_image_size(&mut self) {
        self.dialog = Some(ImageSizeDraft::new(self.doc.size));
    }

    /// Applies the draft and closes it. With resampling on the document takes the draft's pixel
    /// size and the layers scale with it; off, the pixel count stays and only the resolution
    /// changes, as PhotoCraft's Image Size does.
    pub fn apply_size(&mut self) {
        let Some(draft) = self.dialog.take() else { return };
        let size = draft.pixel_size();
        if size == self.doc.size && draft.resolution == self.doc.resolution {
            return;
        }
        if draft.resample && size != self.doc.size {
            let scale = [size[0] as f32 / self.doc.size[0] as f32, size[1] as f32 / self.doc.size[1] as f32];
            for layer in &mut self.doc.layers {
                let [x, y, w, h] = layer.rect;
                layer.rect = [x * scale[0], y * scale[1], w * scale[0], h * scale[1]];
            }
            self.view.center = size.map(|s| f64::from(s) / 2.0);
        }
        self.doc.size = size;
        self.doc.resolution = draft.resolution;
        self.doc.modified = true;
        self.record("Image size");
        self.status = format!("{} x {} px", size[0], size[1]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_seed_is_the_capture() {
        let doc = Doc::default();
        let names: Vec<_> = doc.layers.iter().map(|l| (l.name.as_str(), l.visible, l.locked)).collect();
        assert_eq!(names, [("Shapes", true, false), ("Sky", false, false), ("Background", true, true)]);
        assert_eq!(doc.selected_layer().map(|l| l.name.as_str()), Some("Shapes"));
    }

    #[test]
    fn visibility_toggles_and_is_recorded() {
        let mut mock = Mock::default();
        mock.toggle_visible(2);
        assert!(mock.doc.layer(2).unwrap().visible);
        assert_eq!(mock.history.last().map(String::as_str), Some("Layer visibility"));
        mock.toggle_visible(99);
        assert_eq!(mock.history.len(), 2, "an unknown layer records nothing");
    }

    #[test]
    fn a_locked_layer_keeps_its_name_and_survives_delete() {
        let mut doc = Doc::default();
        assert!(!doc.rename(1, "Floor"));
        assert!(doc.rename(3, "  Blocks "));
        assert_eq!(doc.layer(3).unwrap().name, "Blocks");
        assert!(!doc.rename(3, "   "), "an empty name is refused");
        assert_eq!(doc.delete(&[1]), 0);
        assert_eq!(doc.delete(&[3]), 1);
        assert_eq!(doc.selected, vec![2], "the selection falls to the layer now in its place");
    }

    #[test]
    fn layers_move_and_new_layers_go_above_the_selection() {
        let mut doc = Doc::default();
        assert!(doc.move_layer(3, 99));
        assert_eq!(doc.layers.iter().map(|l| l.id).collect::<Vec<_>>(), [2, 1, 3], "clamped to the bottom");
        assert!(!doc.move_layer(3, 2));
        let id = doc.add_layer();
        assert_eq!(doc.layers[2].id, id, "above the selected layer");
        assert_eq!(doc.layer(id).unwrap().name, "Layer 4");
        assert_eq!(doc.selected, vec![id]);
    }

    #[test]
    fn adjustments_change_the_layers_below_and_a_clip_limits_them() {
        assert_eq!(adjust([100, 100, 100], 0, 0), [100, 100, 100]);
        assert_eq!(adjust([100, 100, 100], 50, 0), [150, 150, 150]);
        assert_eq!(adjust([250, 10, 128], 50, 0), [255, 60, 178], "clamped at white");
        assert!(adjust([200, 200, 200], 0, 50)[0] > 200, "contrast pushes light colours up");
        assert!(adjust([60, 60, 60], 0, 50)[0] < 60, "and dark colours down");

        let mut doc = Doc::default();
        let id = doc.add_adjustment("Brightness/Contrast");
        if let Some(Layer { kind: LayerKind::Adjustment { brightness, .. }, .. }) = doc.layer_mut(id) {
            *brightness = 20;
        }
        // The adjustment is on top: every pixel layer below shows brighter.
        assert_eq!(doc.shown_color(0), None);
        assert_eq!(doc.shown_color(1), Some([255, 160, 60]));
        assert_eq!(doc.shown_color(3), Some([255, 255, 255]));
        // Clipped, it reaches only the layer directly below.
        if let Some(Layer { kind: LayerKind::Adjustment { clip, .. }, .. }) = doc.layer_mut(id) {
            *clip = true;
        }
        assert_eq!(doc.shown_color(1), Some([255, 160, 60]));
        assert_eq!(doc.shown_color(2), Some([92, 160, 232]));
        // Hidden, it does nothing.
        doc.toggle_visible(id);
        assert_eq!(doc.shown_color(1), Some([240, 140, 40]));
    }

    #[test]
    fn a_linked_size_keeps_the_ratio_in_either_unit() {
        let mut draft = ImageSizeDraft::new([1200, 800]);
        draft.set_width(600.0);
        assert_eq!(draft.height, 400.0);
        draft.set_height(100.0);
        assert_eq!(draft.width, 150.0);
        draft.link = false;
        draft.set_width(10.0);
        assert_eq!(draft.height, 100.0);
        draft.link = true;
        draft.set_unit(0, SizeUnit::Percent);
        assert!((draft.width - 10.0 / 12.0).abs() < 1e-9);
        draft.set_width(50.0);
        assert_eq!(draft.pixel_size(), [600, 400]);
    }

    #[test]
    fn applying_the_size_resizes_the_document_and_cancel_leaves_it() {
        let mut mock = Mock::default();
        mock.open_image_size();
        mock.dialog.as_mut().unwrap().set_width(600.0);
        mock.dialog = None;
        assert_eq!(mock.doc.size, [1200, 800], "closing the draft applies nothing");

        mock.open_image_size();
        mock.dialog.as_mut().unwrap().set_width(600.0);
        mock.apply_size();
        assert_eq!(mock.doc.size, [600, 400]);
        assert!(mock.dialog.is_none());
        assert_eq!(mock.doc.layer(3).unwrap().rect, [250.0, 150.0, 250.0, 200.0], "resampled layers scale");
        assert_eq!(mock.status, "600 x 400 px");
        assert_eq!(mock.history.last().map(String::as_str), Some("Image size"));
    }

    #[test]
    fn without_resampling_only_the_resolution_changes() {
        let mut mock = Mock::default();
        mock.open_image_size();
        let draft = mock.dialog.as_mut().unwrap();
        draft.set_resample(false);
        draft.set_width(600.0);
        assert_eq!((draft.width, draft.height), (1200.0, 800.0), "the pixel fields are fixed");
        draft.resolution = 150.0;
        mock.apply_size();
        assert_eq!(mock.doc.size, [1200, 800]);
        assert_eq!(mock.doc.resolution, 150.0);
        assert_eq!(mock.doc.layer(3).unwrap().rect, [500.0, 300.0, 500.0, 400.0], "the layers stay");
        assert_eq!(mock.history.last().map(String::as_str), Some("Image size"));
    }

    #[test]
    fn a_new_layer_is_transparent() {
        let mut doc = Doc::default();
        let id = doc.add_layer();
        let index = doc.index_of(id).unwrap();
        assert_eq!(doc.layer(id).unwrap().kind, LayerKind::Empty);
        assert_eq!(doc.shown_color(index), None, "nothing to paint");
        assert!(!doc.layer(id).unwrap().is_adjustment());
    }

    #[test]
    fn choosing_a_tool_says_so_and_keys_find_working_tools_only() {
        let mut mock = Mock::default();
        mock.select_tool(Tool::Brush);
        assert_eq!((mock.tools.current, mock.status.as_str()), (Tool::Brush, "Brush tool"));
        mock.select_tool(Tool::Pen);
        assert_eq!(mock.tools.current, Tool::Brush, "a tool that does not work is ignored");
        assert_eq!(Tool::from_key('v'), Some(Tool::Move));
        assert_eq!(Tool::from_key('B'), Some(Tool::Brush));
        assert_eq!(Tool::from_key('P'), None);
    }

    #[test]
    fn fit_shows_the_whole_document_and_zoom_is_clamped() {
        let mut view = View { fit_pending: true, ..View::default() };
        view.fit([600.0, 800.0], [1200, 800]);
        assert!((view.zoom - 0.475).abs() < 1e-9);
        assert_eq!(view.center, [600.0, 400.0]);
        assert!(!view.fit_pending);
        view.set_zoom(1000.0);
        assert_eq!(view.zoom, 64.0);
    }

    #[test]
    fn colours_swap_and_the_active_well_takes_the_pick() {
        let mut colors = Colors::default();
        colors.set_active([1, 2, 3]);
        assert_eq!(colors.fg, [1, 2, 3]);
        colors.swap();
        assert_eq!((colors.fg, colors.bg), ([255, 255, 255], [1, 2, 3]));
        colors.bg_active = true;
        colors.set_active([9, 9, 9]);
        assert_eq!(colors.bg, [9, 9, 9]);
    }
}
