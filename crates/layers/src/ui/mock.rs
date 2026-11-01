//! `MockLayers`: a `LayerTree` for stories and tests.

use std::cell::Cell;

use craft_theme::LabelColor;
use craft_ui::menu::{MenuEntry, MenuItem};

use super::tree::{
    BlendChoice, LayerInfo, LayerKind, LayerLocks, LayerPart, LayerProperties, LayerSelection, LayerTree, LayersFooterState, MaskInfo, SubRow,
    SubRowGroup, SubRows,
};

/// One layer of a [`MockLayers`] tree.
#[derive(Clone, Debug)]
pub(crate) struct MockLayer {
    pub(crate) id: u32,
    pub(crate) parent: Option<u32>,
    pub(crate) info: LayerInfo,
    pub(crate) open: bool,
    pub(crate) effects_open: bool,
    pub(crate) effects: Option<SubRows>,
    pub(crate) smart_filters: Option<SubRows>,
}

/// A fake document's layers, kept in plain vectors: the tests' and stories' `LayerTree`. Ids are
/// `u32`; the app's state (open groups, selection, effects open) is a field the test sets.
#[derive(Clone, Debug, Default)]
pub(crate) struct MockLayers {
    /// Every layer, each group before its children, topmost first among siblings.
    pub(crate) layers: Vec<MockLayer>,
    pub(crate) selection: LayerSelection<u32>,
    pub(crate) hovered: Option<u32>,
    pub(crate) revision: Option<u64>,
    info_calls: Cell<usize>,
}

impl MockLayers {
    /// A small tree for tests: a group with two pixel layers (one clipped, one with a pixel mask),
    /// a type layer, an adjustment with a disabled mask, a smart object with two Smart Filters,
    /// a pixel layer with effects, a hidden layer, and the Background; the top layer active.
    pub(crate) fn sample() -> Self {
        // new: the spec's `MockLayers` (`step6a-layers.md`, Stories and tests).
        let mut tree = Self::default();
        tree.push(1, None, LayerInfo::new("Group 1", LayerKind::Group), true);
        let mut clipped = LayerInfo::new("Clipped", LayerKind::Pixel);
        clipped.clipped = true;
        tree.push(2, Some(1), clipped, false);
        let mut masked = LayerInfo::new("Masked", LayerKind::Pixel);
        masked.masks.push(MaskInfo { part: LayerPart::LayerMask, enabled: true, linked: true });
        tree.push(3, Some(1), masked, false);
        tree.push(4, None, LayerInfo::new("Title", LayerKind::Text), false);
        let mut adjustment = LayerInfo::new("Levels 1", LayerKind::Adjustment);
        adjustment.masks.push(MaskInfo { part: LayerPart::LayerMask, enabled: false, linked: true });
        tree.push(5, None, adjustment, false);
        let mut smart = LayerInfo::new("Smart Object", LayerKind::SmartObject);
        smart.has_smart_filters = true;
        tree.push(6, None, smart, false);
        tree.layers[5].smart_filters = Some(SubRows {
            header: SubRow { label: String::from("Smart Filters"), on: true, eye_shown: true, icon: None, mask: true },
            items: vec![sub_row("Gaussian Blur"), sub_row("Sharpen")],
        });
        let mut styled = LayerInfo::new("Styled", LayerKind::Pixel);
        styled.has_effects = true;
        tree.push(7, None, styled, false);
        tree.layers[6].effects = Some(SubRows {
            header: SubRow { label: String::from("Effects"), on: true, eye_shown: true, icon: None, mask: false },
            items: vec![sub_row("Drop Shadow"), sub_row("Stroke")],
        });
        let mut hidden = LayerInfo::new("Hidden", LayerKind::Pixel);
        hidden.visible = false;
        hidden.dimmed = true;
        tree.push(8, None, hidden, false);
        let mut background = LayerInfo::new("Background", LayerKind::Pixel);
        background.background = true;
        background.renamable = false;
        background.thumb_loads_selection = false;
        background.locks = LayerLocks { all: true, ..LayerLocks::default() };
        tree.push(9, None, background, false);
        tree.selection = LayerSelection { layers: vec![1], active: Some(1), target: LayerPart::Pixels };
        tree
    }

    /// The stories' 50-layer tree: groups nested three deep, a clipped run, linked layers, pixel
    /// mask, vector mask, both, a disabled mask, adjustment, solid and gradient fill, type, shape,
    /// smart object with Smart Filters, effects open and closed, a locked layer, the Background,
    /// a hidden layer (story `composites/layers/panel/tree`).
    pub(crate) fn fifty() -> Self {
        let mut tree = Self::default();
        let header = tree.add(None, named("Header", LayerKind::Group, |_| {}), true);
        let logo = tree.add(Some(header), named("Logo", LayerKind::Group, |_| {}), true);
        let marks = tree.add(Some(logo), named("Marks", LayerKind::Group, |_| {}), true);
        tree.add(Some(marks), named("Mark A", LayerKind::Pixel, |i| i.linked = true), false);
        tree.add(Some(marks), named("Mark B", LayerKind::Pixel, |i| i.linked = true), false);
        tree.add(Some(logo), named("Ring", LayerKind::Shape, |i| i.masks.push(mask(LayerPart::VectorMask, true, true))), false);
        tree.add(Some(header), named("Title", LayerKind::Text, |_| {}), false);
        tree.add(Some(header), named("Tagline", LayerKind::Pixel, |i| i.masks.push(mask(LayerPart::LayerMask, true, false))), false);

        let photo = tree.add(None, named("Photo", LayerKind::Group, |_| {}), true);
        tree.add(
            Some(photo),
            named("Curves 1", LayerKind::Adjustment, |i| {
                i.masks.push(mask(LayerPart::LayerMask, false, true));
                i.blend_key = String::from("multiply");
                i.blend_label = Some(String::from("Multiply"));
            }),
            false,
        );
        tree.add(Some(photo), named("Retouch", LayerKind::Pixel, |i| i.clipped = true), false);
        tree.add(
            Some(photo),
            named("Dodge", LayerKind::Pixel, |i| {
                i.clipped = true;
                i.blend_key = String::from("colorDodge");
                i.blend_label = Some(String::from("Color Dodge"));
            }),
            false,
        );
        let portrait = tree.add(
            Some(photo),
            named("Portrait", LayerKind::Pixel, |i| {
                i.masks.push(mask(LayerPart::LayerMask, true, true));
                i.masks.push(mask(LayerPart::VectorMask, true, false));
            }),
            false,
        );
        let backdrop = tree.add(Some(photo), named("Backdrop", LayerKind::SmartObject, |i| i.has_smart_filters = true), false);
        tree.layer_mut(backdrop).effects_open = true;
        tree.layer_mut(backdrop).smart_filters = Some(SubRows {
            header: SubRow { label: String::from("Smart Filters"), on: true, eye_shown: true, icon: None, mask: true },
            items: vec![sub_row("Gaussian Blur"), SubRow { on: false, ..sub_row("Unsharp Mask") }, sub_row("Camera Raw Filter")],
        });
        let glow = tree.add(Some(photo), named("Glow", LayerKind::Pixel, |i| i.has_effects = true), false);
        tree.layer_mut(glow).effects_open = true;
        tree.layer_mut(glow).effects = Some(effects(&["Outer Glow", "Color Overlay"]));
        let outline = tree.add(Some(photo), named("Outline", LayerKind::Pixel, |i| i.has_effects = true), false);
        tree.layer_mut(outline).effects = Some(effects(&["Stroke"]));

        let details = tree.add(None, named("Details", LayerKind::Group, |_| {}), false);
        tree.add(Some(details), named("Dust", LayerKind::Pixel, |_| {}), false);
        tree.add(Some(details), named("Scratches", LayerKind::Pixel, |_| {}), false);
        let grain = tree.add(Some(details), named("Grain", LayerKind::Group, |_| {}), true);
        tree.add(Some(grain), named("Fine", LayerKind::Pixel, |_| {}), false);
        tree.add(Some(grain), named("Coarse", LayerKind::Pixel, |_| {}), false);

        tree.add(None, named("Solid Color 1", LayerKind::Fill, |i| i.masks.push(mask(LayerPart::LayerMask, true, true))), false);
        tree.add(None, named("Gradient Fill 1", LayerKind::Fill, |_| {}), false);
        tree.add(None, named("Hue/Saturation 1", LayerKind::Adjustment, |_| {}), false);
        tree.add(
            None,
            named("Levels 2", LayerKind::Adjustment, |i| {
                i.masks.push(mask(LayerPart::LayerMask, true, true));
                i.label = Some(LabelColor::Blue);
            }),
            false,
        );
        tree.add(None, named("Rectangle 1", LayerKind::Shape, |_| {}), false);
        tree.add(None, named("Ellipse 1", LayerKind::Shape, |i| i.locks.position = true), false);
        tree.add(
            None,
            named("Headline", LayerKind::Text, |i| i.warning = Some(String::from("This layer uses a font that is not installed."))),
            false,
        );
        tree.add(None, named("Paper", LayerKind::Pixel, |i| i.locks.transparency = true), false);
        tree.add(None, named("Stamp", LayerKind::Pixel, |i| i.locks.pixels = true), false);
        tree.add(
            None,
            named("Hidden Note", LayerKind::Pixel, |i| {
                i.visible = false;
                i.dimmed = true;
            }),
            false,
        );
        tree.add(None, named("Green Pass", LayerKind::Pixel, |i| i.label = Some(LabelColor::Green)), false);
        let archive = tree.add(None, named("Archive", LayerKind::Group, |_| {}), false);
        for n in 1..=3 {
            tree.add(Some(archive), named(&format!("Old {n}"), LayerKind::Pixel, |_| {}), false);
        }
        tree.add(None, named("Placed Photo", LayerKind::Image, |_| {}), false);
        let mut n = 1;
        while tree.layers.len() < 49 {
            tree.add(None, named(&format!("Sketch {n}"), LayerKind::Pixel, |_| {}), false);
            n += 1;
        }
        tree.add(
            None,
            named("Background", LayerKind::Pixel, |i| {
                i.background = true;
                i.renamable = false;
                i.thumb_loads_selection = false;
                i.locks = LayerLocks { all: true, ..LayerLocks::default() };
            }),
            false,
        );
        tree.selection = LayerSelection { layers: vec![portrait], active: Some(portrait), target: LayerPart::Pixels };
        tree
    }

    /// `count` pixel layers at the top level, for the 10,000-row test; counts the `info` calls.
    #[cfg(test)]
    pub(crate) fn flat(count: usize) -> Self {
        // new: `tree_with_ten_thousand_layers_builds_rows_in_view_only`.
        let mut tree = Self::default();
        for n in 0..count {
            let id = u32::try_from(n + 1).expect("the mock tree holds fewer than 2^32 layers");
            tree.push(id, None, LayerInfo::new(format!("Layer {id}"), LayerKind::Pixel), false);
        }
        tree
    }

    /// How many times `info` was called since the last [`take_info_calls`](Self::take_info_calls).
    #[cfg(test)]
    pub(crate) fn take_info_calls(&self) -> usize {
        // new: a `Cell<usize>` counter.
        self.info_calls.replace(0)
    }
}

impl MockLayers {
    /// Appends a layer under `parent` with the next id; returns the id.
    pub(crate) fn add(&mut self, parent: Option<u32>, info: LayerInfo, open: bool) -> u32 {
        let id = u32::try_from(self.layers.len() + 1).expect("the mock tree holds fewer than 2^32 layers");
        self.push(id, parent, info, open);
        id
    }

    /// The layer with `id`; the mock tree has it.
    pub(crate) fn layer_mut(&mut self, id: u32) -> &mut MockLayer {
        let at = self.layers.iter().position(|layer| layer.id == id).expect("the layer is in the mock tree");
        &mut self.layers[at]
    }

    fn push(&mut self, id: u32, parent: Option<u32>, info: LayerInfo, open: bool) {
        self.layers.push(MockLayer { id, parent, info, open, effects_open: false, effects: None, smart_filters: None });
    }

    fn layer(&self, id: &u32) -> Option<&MockLayer> {
        self.layers.iter().find(|layer| layer.id == *id)
    }
}

fn named(name: &str, kind: LayerKind, edit: impl FnOnce(&mut LayerInfo)) -> LayerInfo {
    let mut info = LayerInfo::new(name, kind);
    edit(&mut info);
    info
}

fn mask(part: LayerPart, enabled: bool, linked: bool) -> MaskInfo {
    MaskInfo { part, enabled, linked }
}

/// An "Effects" group with one row per name.
pub(crate) fn effects(names: &[&str]) -> SubRows {
    SubRows {
        header: SubRow { label: String::from("Effects"), on: true, eye_shown: true, icon: None, mask: false },
        items: names.iter().map(|name| sub_row(name)).collect(),
    }
}

pub(crate) fn sub_row(label: &str) -> SubRow {
    SubRow { label: label.to_owned(), on: true, eye_shown: true, icon: None, mask: false }
}

fn command(label: &str, command: &str) -> MenuEntry<String> {
    MenuItem::new(label, command.to_owned()).into()
}

impl LayerTree for MockLayers {
    type Id = u32;

    fn children(&self, parent: Option<&u32>) -> Vec<u32> {
        self.layers.iter().filter(|layer| layer.parent.as_ref() == parent).map(|layer| layer.id).collect()
    }

    fn parent(&self, id: &u32) -> Option<u32> {
        self.layer(id).and_then(|layer| layer.parent)
    }

    fn is_open(&self, id: &u32) -> bool {
        self.layer(id).is_some_and(|layer| layer.open)
    }

    fn effects_open(&self, id: &u32) -> bool {
        self.layer(id).is_some_and(|layer| layer.effects_open)
    }

    fn info(&self, id: &u32) -> LayerInfo {
        self.info_calls.set(self.info_calls.get() + 1);
        self.layer(id).map(|layer| layer.info.clone()).unwrap_or_else(|| LayerInfo::new(String::new(), LayerKind::Pixel))
    }

    fn selection(&self) -> LayerSelection<u32> {
        self.selection.clone()
    }

    fn sub_rows(&self, id: &u32, group: SubRowGroup) -> Option<SubRows> {
        let layer = self.layer(id)?;
        match group {
            SubRowGroup::Effects => layer.effects.clone(),
            SubRowGroup::SmartFilters => layer.smart_filters.clone(),
        }
    }

    fn properties(&self) -> Option<LayerProperties> {
        let active = self.selection.active.as_ref().and_then(|id| self.layer(id))?;
        Some(LayerProperties {
            blend_key: active.info.blend_key.clone(),
            group: active.info.kind == LayerKind::Group,
            opacity: 100.0,
            fill: 100.0,
            locks: active.info.locks,
            editable: !active.info.background,
        })
    }

    /// Photoshop's modes in its groups (Normal, Dissolve | Darken, Multiply, … ), Pass Through first
    /// for a group.
    fn blend_modes(&self, group: bool, _current: &str) -> Vec<BlendChoice> {
        const GROUPS: [&[(&str, &str)]; 6] = [
            &[("normal", "Normal"), ("dissolve", "Dissolve")],
            &[
                ("darken", "Darken"),
                ("multiply", "Multiply"),
                ("colorBurn", "Color Burn"),
                ("linearBurn", "Linear Burn"),
                ("darkerColor", "Darker Color"),
            ],
            &[
                ("lighten", "Lighten"),
                ("screen", "Screen"),
                ("colorDodge", "Color Dodge"),
                ("linearDodge", "Linear Dodge (Add)"),
                ("lighterColor", "Lighter Color"),
            ],
            &[
                ("overlay", "Overlay"),
                ("softLight", "Soft Light"),
                ("hardLight", "Hard Light"),
                ("vividLight", "Vivid Light"),
                ("linearLight", "Linear Light"),
                ("pinLight", "Pin Light"),
                ("hardMix", "Hard Mix"),
            ],
            &[("difference", "Difference"), ("exclusion", "Exclusion"), ("subtract", "Subtract"), ("divide", "Divide")],
            &[("hue", "Hue"), ("saturation", "Saturation"), ("color", "Color"), ("luminosity", "Luminosity")],
        ];
        let mut choices = Vec::new();
        if group {
            choices.push(BlendChoice { key: String::from("passThrough"), label: String::from("Pass Through"), separator_before: false });
        }
        for (n, modes) in GROUPS.iter().enumerate() {
            for (m, (key, label)) in modes.iter().enumerate() {
                choices.push(BlendChoice { key: (*key).to_owned(), label: (*label).to_owned(), separator_before: n > 0 && m == 0 });
            }
        }
        choices
    }

    fn hovered(&self) -> Option<u32> {
        self.hovered
    }

    fn revision(&self) -> Option<u64> {
        self.revision
    }

    fn footer(&self) -> LayersFooterState {
        let active = self.selection.active.is_some();
        LayersFooterState {
            link_enabled: self.selection.layers.len() > 1,
            mask_label: active.then(|| String::from("Add Layer Mask")),
            delete_label: String::from("Delete Layer"),
            delete_enabled: active,
            adjustment_menu: vec![command("Brightness/Contrast", "brightnessContrast"), command("Levels", "levels"), command("Curves", "curves")],
            style_menu: vec![command("Drop Shadow", "dropShadow"), MenuEntry::Separator, command("Stroke", "stroke")],
        }
    }
}
