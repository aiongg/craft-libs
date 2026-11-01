//! `MockLayers`: a `LayerTree` for stories and tests.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use super::tree::{BlendChoice, LayerInfo, LayerProperties, LayerSelection, LayerTree, LayersFooterState, SubRowGroup, SubRows};

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
}

impl MockLayers {
    /// A small tree for tests: a group with two pixel layers (one clipped, one with a pixel mask),
    /// a type layer, an adjustment with a disabled mask, a smart object with two Smart Filters,
    /// a pixel layer with effects, a hidden layer, and the Background; the top layer active.
    pub(crate) fn sample() -> Self {
        // new: the spec's `MockLayers` (`step6a-layers.md`, Stories and tests).
        todo!()
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
        todo!()
    }

    /// How many times `info` was called since the last [`take_info_calls`](Self::take_info_calls).
    pub(crate) fn take_info_calls(&self) -> usize {
        // new: a `Cell<usize>` counter.
        todo!()
    }
}

impl LayerTree for MockLayers {
    type Id = u32;

    fn children(&self, parent: Option<&u32>) -> Vec<u32> {
        // new: over `layers` (the fake document).
        todo!()
    }

    fn parent(&self, id: &u32) -> Option<u32> {
        // new: over `layers` (the fake document).
        todo!()
    }

    fn is_open(&self, id: &u32) -> bool {
        // new: over `layers` (the fake document).
        todo!()
    }

    fn effects_open(&self, id: &u32) -> bool {
        // new: over `layers` (the fake document).
        todo!()
    }

    fn info(&self, id: &u32) -> LayerInfo {
        // new: over `layers` (the fake document).
        todo!()
    }

    fn selection(&self) -> LayerSelection<u32> {
        // new: over `layers` (the fake document).
        todo!()
    }

    fn sub_rows(&self, id: &u32, group: SubRowGroup) -> Option<SubRows> {
        // new: over `layers` (the fake document).
        todo!()
    }

    fn properties(&self) -> Option<LayerProperties> {
        // new: over `layers` (the fake document).
        todo!()
    }

    /// Photoshop's modes in its groups (Normal, Dissolve | Darken, Multiply, … ), Pass Through first
    /// for a group.
    fn blend_modes(&self, group: bool, current: &str) -> Vec<BlendChoice> {
        // new: over `layers` (the fake document).
        todo!()
    }

    fn footer(&self) -> LayersFooterState {
        // new: over `layers` (the fake document).
        todo!()
    }
}
