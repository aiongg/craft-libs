//! `MockLayers`: a `LayerTree` for stories and tests.

#![expect(dead_code, clippy::todo, reason = "skeleton: `fifty` (ST2) and the not yet used helpers")]

use std::cell::Cell;

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
        // new: the spec's story tree.
        todo!()
    }

    /// `count` pixel layers at the top level, for the 10,000-row test; counts the `info` calls.
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
    pub(crate) fn take_info_calls(&self) -> usize {
        // new: a `Cell<usize>` counter.
        self.info_calls.replace(0)
    }
}

impl MockLayers {
    fn push(&mut self, id: u32, parent: Option<u32>, info: LayerInfo, open: bool) {
        self.layers.push(MockLayer { id, parent, info, open, effects_open: false, effects: None, smart_filters: None });
    }

    fn layer(&self, id: &u32) -> Option<&MockLayer> {
        self.layers.iter().find(|layer| layer.id == *id)
    }
}

fn sub_row(label: &str) -> SubRow {
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
