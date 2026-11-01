//! Tests of the drag rules (L6).

use std::cell::RefCell;

use craft_ui::behavior::{DropPosition, ListDrag, Move, RowInfo, UniformSpans};
use craft_ui::edit::{CancelReason, EditEvent, GestureId};
use craft_ui::list::ListRow;
use craft_ui::theme::{Base, Tokens, TokensOf as _, install};
use egui::{Context, DragAndDrop, Modifiers};

use super::context::{PanelContext, RowSink};
use super::drag::{accepts_move, drop_event, drop_preview, layer_drop, reorder, start_mask_drag};
use super::events::{LayerDrop, LayersEvent, SubRowMove};
use super::labels::LayersLabels;
use super::style::LayersStyle;
use crate::ui::mock::MockLayers;
use crate::ui::options::LayersOptions;
use crate::ui::tree::{
    BlendChoice, LayerInfo, LayerPart, LayerProperties, LayerRowKey, LayerSelection, LayerTree, LayersFooterState, SubRowGroup, SubRows,
};

type Key = LayerRowKey<u32>;

fn mv(items: Vec<Key>, target: Key, position: DropPosition) -> Move<Key> {
    Move { items, target, position, copy: false }
}

/// The sample tree, refusing every drop below its Background (layer 9), as PhotoCraft does.
struct NothingBelowBackground(MockLayers);

impl LayerTree for NothingBelowBackground {
    type Id = u32;

    fn children(&self, parent: Option<&u32>) -> Vec<u32> {
        self.0.children(parent)
    }
    fn parent(&self, id: &u32) -> Option<u32> {
        self.0.parent(id)
    }
    fn is_open(&self, id: &u32) -> bool {
        self.0.is_open(id)
    }
    fn effects_open(&self, id: &u32) -> bool {
        self.0.effects_open(id)
    }
    fn info(&self, id: &u32) -> LayerInfo {
        self.0.info(id)
    }
    fn selection(&self) -> LayerSelection<u32> {
        self.0.selection()
    }
    fn sub_rows(&self, id: &u32, group: SubRowGroup) -> Option<SubRows> {
        self.0.sub_rows(id, group)
    }
    fn properties(&self) -> Option<LayerProperties> {
        self.0.properties()
    }
    fn blend_modes(&self, group: bool, current: &str) -> Vec<BlendChoice> {
        self.0.blend_modes(group, current)
    }
    fn footer(&self) -> LayersFooterState {
        self.0.footer()
    }
    fn accepts_drop(&self, mv: &Move<Key>, _modifiers: Modifiers) -> bool {
        !(mv.target == LayerRowKey::Layer(9) && mv.position == DropPosition::After)
    }
}

// `accepts_move` refuses what the tree's `accepts_drop` refuses (the Background rules), so no
// marker shows.
#[test]
fn accepts_drop_refusal_shows_no_marker() {
    let tree = NothingBelowBackground(MockLayers::sample());
    let ok = mv(vec![Key::Layer(4)], Key::Layer(9), DropPosition::Before);
    let below = mv(vec![Key::Layer(4)], Key::Layer(9), DropPosition::After);
    assert!(accepts_move(&tree, &ok, Modifiers::NONE));
    assert!(!accepts_move(&tree, &below, Modifiers::NONE));

    // The list's resolve asks the same rules: the space under the last row (the Background)
    // has no target.
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let rows = [RowInfo::leaf(Key::Layer(4)), RowInfo::leaf(Key::Layer(5)), RowInfo::leaf(Key::Layer(9))];
    let spans = UniformSpans { origin: 0.0, extent: 20.0, gap: 0.0, rows: 3 };
    let output = ctx.run_ui(egui::RawInput::default(), |ui| {
        with_context(ui, &tree, |cx| {
            let reorder = reorder(cx);
            assert!(reorder.resolve(&rows[..], &spans, 55.0, &[Key::Layer(4)], Modifiers::NONE).is_none());
            let before = reorder.resolve(&rows[..], &spans, 45.0, &[Key::Layer(4)], Modifiers::NONE).expect("Before the Background");
            assert_eq!((before.mv.target, before.mv.position), (Key::Layer(9), DropPosition::Before));
        });
    });
    output.drop_without_applying_deltas();
}

fn with_context<T: LayerTree>(ui: &egui::Ui, tree: &T, body: impl FnOnce(&PanelContext<'_, T>)) {
    let tokens = Tokens::of(ui);
    let labels = LayersLabels::default();
    let options = LayersOptions::default();
    let cx = PanelContext {
        ctx: ui.ctx().clone(),
        id: egui::Id::unique("layers"),
        tree,
        selection: tree.selection(),
        options: &options,
        thumbs: None,
        labels: &labels,
        style: LayersStyle::of(&tokens),
        row_style: ListRow::<u8>::style(&tokens),
        lock_column: false,
        visibility_key: None,
        trailing: None,
        sink: RefCell::new(RowSink::default()),
    };
    body(&cx);
}

// Effects rows `Into` another layer are a `Transfer` (Alt copies); onto their own layer refused.
#[test]
fn effects_drag_onto_a_layer_outlines_it_and_reports_a_transfer() {
    let tree = MockLayers::sample();
    let effects = Key::Effects(7, None);
    let onto = mv(vec![effects.clone()], Key::Layer(4), DropPosition::Into);
    assert!(accepts_move(&tree, &onto, Modifiers::NONE));
    let Some(LayerDrop::Transfer(transfer)) = layer_drop(&Move { copy: true, ..onto }) else { panic!("a transfer") };
    assert_eq!((transfer.rows, transfer.to, transfer.copy), (vec![effects.clone()], 4, true));

    let own = mv(vec![effects.clone()], Key::Layer(7), DropPosition::Into);
    assert!(!accepts_move(&tree, &own, Modifiers::NONE), "onto their own layer");
    let between = mv(vec![effects.clone()], Key::Layer(4), DropPosition::Before);
    assert!(!accepts_move(&tree, &between, Modifiers::NONE), "sub-rows between layers");
    let filters_onto_text = mv(vec![Key::Filters(6, None)], Key::Layer(4), DropPosition::Into);
    assert!(!accepts_move(&tree, &filters_onto_text, Modifiers::NONE), "Smart Filters go onto smart objects only");

    // The middle of a childless layer row is an `Into` target for the effects, with Alt a copy;
    // the same drag of a layer row keeps the halves.
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let rows = [RowInfo::leaf(Key::Layer(7)), RowInfo::leaf(effects.clone()), RowInfo::leaf(Key::Layer(4))];
    let spans = UniformSpans { origin: 0.0, extent: 20.0, gap: 0.0, rows: 3 };
    let output = ctx.run_ui(egui::RawInput::default(), |ui| {
        with_context(ui, &tree, |cx| {
            let reorder = reorder(cx);
            let middle = reorder.resolve(&rows[..], &spans, 50.0, std::slice::from_ref(&effects), Modifiers::ALT).expect("Into the Title layer");
            assert_eq!((middle.mv.target.clone(), middle.mv.position, middle.mv.copy), (Key::Layer(4), DropPosition::Into, true));
            let layer = reorder.resolve(&rows[..], &spans, 50.0, &[Key::Layer(7)], Modifiers::NONE).expect("a half of the Title layer");
            assert_ne!(layer.mv.position, DropPosition::Into);
        });
    });
    output.drop_without_applying_deltas();
}

// A Smart Filter row among its layer's filters is a `SubRows` move with `to` counted after taking
// it out; among another layer's filters refused.
#[test]
fn smart_filter_drag_reorders_within_its_object() {
    let tree = MockLayers::sample();
    let filter = |layer, at| Key::Filters(layer, Some(at));
    let sub_rows = |from: usize, target: Key, position| match layer_drop(&mv(vec![filter(6, from)], target, position)) {
        Some(LayerDrop::SubRows(SubRowMove { layer: 6, group: SubRowGroup::SmartFilters, from: f, to, copy: false })) => Some((f, to)),
        None => None,
        other => panic!("{other:?}"),
    };
    assert_eq!(sub_rows(0, filter(6, 1), DropPosition::After), Some((0, 1)));
    assert_eq!(sub_rows(1, filter(6, 0), DropPosition::Before), Some((1, 0)));
    assert_eq!(sub_rows(1, Key::Filters(6, None), DropPosition::After), Some((1, 0)), "just under the header is the top");
    assert_eq!(sub_rows(0, filter(6, 1), DropPosition::Before), None, "where it already is");
    assert_eq!(sub_rows(0, Key::Filters(6, None), DropPosition::Before), None, "nothing goes above the header");

    let other = mv(vec![filter(6, 0)], filter(5, 0), DropPosition::After);
    assert!(!accepts_move(&tree, &other, Modifiers::NONE), "another layer's filters");
    let effect_row = mv(vec![Key::Effects(7, Some(0))], Key::Effects(7, Some(1)), DropPosition::After);
    assert!(!accepts_move(&tree, &effect_row, Modifiers::NONE), "effects keep their order");
    assert!(accepts_move(&tree, &mv(vec![filter(6, 0)], filter(6, 1), DropPosition::After), Modifiers::NONE));
}

// `drop_preview` is `Some` only while `preview_on_canvas_when_reordering` is on and the target is a
// layer move.
#[test]
fn drop_preview_is_reported_only_when_the_setting_is_on() {
    let layers = mv(vec![Key::Layer(4), Key::Layer(5)], Key::Layer(1), DropPosition::Into);
    let transfer = mv(vec![Key::Effects(7, None)], Key::Layer(4), DropPosition::Into);
    let option = |on| LayersOptions { preview_on_canvas_when_reordering: on, ..LayersOptions::default() };
    let (on, off) = (option(true), option(false));
    let preview = drop_preview(Some(&layers), &on).expect("a layer move previews");
    assert_eq!((preview.layers, preview.target, preview.position), (vec![4, 5], 1, DropPosition::Into));
    assert!(drop_preview(Some(&layers), &off).is_none());
    assert!(drop_preview::<u32>(None, &on).is_none());
    assert!(drop_preview(Some(&transfer), &on).is_none());
}

// The list's gesture maps one to one; a move that means nothing is not reported.
#[test]
fn drop_events_carry_the_layer_drop_of_the_move() {
    let id = GestureId::next();
    let good = mv(vec![Key::Layer(4)], Key::Layer(1), DropPosition::Before);
    let nothing = mv(vec![Key::Layer(4)], Key::Effects(7, None), DropPosition::Into);
    let Some(LayersEvent::Drop(EditEvent::Commit { value: LayerDrop::Layers(m), changed: true, .. })) =
        drop_event(EditEvent::commit(id, good.clone(), true, Modifiers::NONE))
    else {
        panic!("a commit with the drop")
    };
    assert_eq!((m.layers, m.target), (vec![4], 1));
    // A commit that means nothing still ends the gesture the app began.
    assert!(matches!(
        drop_event(EditEvent::commit(id, nothing.clone(), true, Modifiers::NONE)),
        Some(LayersEvent::Drop(EditEvent::Cancel { reason: CancelReason::Dismissed, .. }))
    ));
    assert!(drop_event(EditEvent::set(nothing, Modifiers::NONE)).is_none());
    assert!(matches!(drop_event(EditEvent::<Move<Key>>::begin(id)), Some(LayersEvent::Drop(EditEvent::Begin { value: None, .. }))));
    assert!(drop_event(EditEvent::update(id, good, Modifiers::NONE)).is_none());
}

// Layers dropped before or after one of a layer's effects or Smart Filter rows, or under the last
// of them, land below that layer; onto one (`Into`) they go nowhere.
#[test]
fn layers_dropped_beside_sub_rows_land_after_their_layer() {
    let tree = MockLayers::sample();
    let landing = |target: Key, position| match layer_drop(&mv(vec![Key::Layer(4)], target, position)) {
        Some(LayerDrop::Layers(m)) => Some((m.target, m.position)),
        _ => None,
    };
    for target in [Key::Effects(7, None), Key::Effects(7, Some(1)), Key::Filters(6, Some(0))] {
        let layer = *target.layer();
        for position in [DropPosition::Before, DropPosition::After] {
            assert_eq!(landing(target.clone(), position), Some((layer, DropPosition::After)), "{target:?} {position:?}");
            assert!(accepts_move(&tree, &mv(vec![Key::Layer(4)], target.clone(), position), Modifiers::NONE));
        }
        assert_eq!(landing(target.clone(), DropPosition::Into), None);
    }
}

// A mask drag's payload holds only the mask, labelled for the ghost.
#[test]
fn a_mask_drag_carries_only_the_mask() {
    let ctx = Context::default();
    let list = egui::Id::unique("list");
    let output = ctx.run_ui(egui::RawInput::default(), |ui| {
        start_mask_drag(ui, list, 3_u32, LayerPart::LayerMask, "Layer Mask");
    });
    output.drop_without_applying_deltas();
    let payload = DragAndDrop::payload::<ListDrag<Key>>(&ctx).expect("a payload");
    assert_eq!((payload.list, payload.items.clone(), payload.label.as_str()), (list, vec![Key::Mask(3, LayerPart::LayerMask)], "Layer Mask"));
}
