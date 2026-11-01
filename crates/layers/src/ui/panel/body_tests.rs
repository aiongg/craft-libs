//! Tests of the display order and the click and reveal rules (L5).

use craft_ui::behavior::RowInfo;
use craft_ui::edit::EditEvent;
use egui::Modifiers;

use super::body::{display_rows, reveal_opens, shared_display_rows, thumb_click_replaces_select, toggle_event_for_tests};
use crate::ui::filter::{FilterKind, LayerFilter};
use crate::ui::mock::{FakeLayer, FakeLayers};
use crate::ui::tree::{LayerInfo, LayerKind, LayerPart, LayerRowKey};

fn layers(rows: &[RowInfo<LayerRowKey<u32>>]) -> Vec<(u32, usize)> {
    rows.iter()
        .map(|row| match row.key {
            LayerRowKey::Layer(id) => (id, row.depth),
            _ => panic!("a sub-row"),
        })
        .collect()
}

// `display_rows` follows `children()` top first and leaves out closed groups' contents.
#[test]
fn display_order_lists_top_first_and_hides_closed_groups() {
    let mut tree = FakeLayers::sample();
    let open = display_rows(&tree, &LayerFilter::default(), &[]).rows;
    assert_eq!(layers(&open), [(1, 0), (2, 1), (3, 1), (4, 0), (5, 0), (6, 0), (7, 0), (8, 0), (9, 0)]);
    assert!(open[0].has_children && open[0].open);

    tree.layers[0].open = false;
    let closed = display_rows(&tree, &LayerFilter::default(), &[]).rows;
    assert_eq!(layers(&closed), [(1, 0), (4, 0), (5, 0), (6, 0), (7, 0), (8, 0), (9, 0)]);
    assert!(closed[0].has_children && !closed[0].open);

    tree.layers[5].effects_open = true;
    tree.layers[6].effects_open = true;
    let keys: Vec<_> = display_rows(&tree, &LayerFilter::default(), &[]).rows.into_iter().skip(3).map(|row| (row.key, row.depth)).collect();
    assert_eq!(
        keys[..7],
        [
            (LayerRowKey::Layer(6), 0),
            (LayerRowKey::Filters(6, None), 1),
            (LayerRowKey::Filters(6, Some(0)), 1),
            (LayerRowKey::Filters(6, Some(1)), 1),
            (LayerRowKey::Layer(7), 0),
            (LayerRowKey::Effects(7, None), 1),
            (LayerRowKey::Effects(7, Some(0)), 1),
        ]
    );
}

// With an active filter, matches inside closed groups are listed and the groups are left out. A
// row's list depth is its number of shown ancestors (so the list finds its parent); it paints at
// its true depth.
#[test]
fn filtered_view_lists_rows_by_shown_ancestors_and_paints_the_true_depth() {
    let mut tree = FakeLayers::sample();
    tree.layers[0].open = false;
    let filter = LayerFilter { kinds: vec![FilterKind::Pixel], ..LayerFilter::default() };
    let shown = display_rows(&tree, &filter, &[]);
    assert_eq!(layers(&shown.rows), [(2, 0), (3, 0), (7, 0), (8, 0), (9, 0)]);
    assert_eq!(shown.paint_depths, [1, 1, 0, 0, 0]);
    assert!(shown.rows.iter().all(|row| !row.has_children));
}

// `opened` lists a closed group's contents, as the reveal of a layer inside it needs in the same pass.
#[test]
fn groups_opened_this_pass_list_their_children() {
    let mut tree = FakeLayers::sample();
    tree.layers[0].open = false;
    let closed = display_rows(&tree, &LayerFilter::default(), &[]);
    let opened = display_rows(&tree, &LayerFilter::default(), &[1]);
    assert_eq!(layers(&closed.rows), [(1, 0), (4, 0), (5, 0), (6, 0), (7, 0), (8, 0), (9, 0)]);
    assert_eq!(layers(&opened.rows), [(1, 0), (2, 1), (3, 1), (4, 0), (5, 0), (6, 0), (7, 0), (8, 0), (9, 0)]);
}

// The cache serves the unfiltered order while the revision stands, and is not used under a filter,
// whose rows read fields the revision doesn't cover.
#[test]
fn the_display_cache_is_not_used_while_the_filter_is_active() {
    let ctx = egui::Context::default();
    let mut tree = FakeLayers::sample();
    tree.revision = Some(1);
    let id = egui::Id::NULL.with("cache");
    let none = LayerFilter::default();
    let before = shared_display_rows(&ctx, id, &tree, &none, &[]);
    tree.layers[0].open = false;
    assert_eq!(shared_display_rows(&ctx, id, &tree, &none, &[]).rows.len(), before.rows.len(), "the same revision serves the cache");

    let name = LayerFilter { kinds: vec![FilterKind::Pixel], ..LayerFilter::default() };
    let first = shared_display_rows(&ctx, id, &tree, &name, &[]);
    tree.layers[1].info.kind = LayerKind::Text;
    let second = shared_display_rows(&ctx, id, &tree, &name, &[]);
    assert_eq!(first.rows.len(), second.rows.len() + 1, "a kind change shows at the same revision");
}

// `thumb_click_replaces_select`: Cmd on a loadable thumbnail, Shift or Alt on a mask, Alt on the
// pixel thumbnail; Cmd on the Background's thumbnail is a plain click.
#[test]
fn modified_thumbnail_clicks_replace_the_select_event() {
    let layer = LayerInfo::new("Layer", LayerKind::Pixel);
    let mut background = LayerInfo::new("Background", LayerKind::Pixel);
    background.thumb_loads_selection = false;
    let replaces = |part, modifiers, info: &LayerInfo| thumb_click_replaces_select(part, modifiers, info);

    assert!(replaces(LayerPart::Pixels, Modifiers::COMMAND, &layer));
    assert!(replaces(LayerPart::Pixels, Modifiers::COMMAND | Modifiers::SHIFT, &layer));
    assert!(replaces(LayerPart::Pixels, Modifiers::ALT, &layer));
    assert!(!replaces(LayerPart::Pixels, Modifiers::NONE, &layer));
    assert!(!replaces(LayerPart::Pixels, Modifiers::SHIFT, &layer));
    assert!(!replaces(LayerPart::Pixels, Modifiers::COMMAND, &background));

    for mask in [LayerPart::LayerMask, LayerPart::VectorMask] {
        assert!(replaces(mask, Modifiers::COMMAND, &background));
        assert!(replaces(mask, Modifiers::SHIFT, &layer));
        assert!(replaces(mask, Modifiers::ALT, &layer));
        assert!(!replaces(mask, Modifiers::NONE, &layer));
    }
}

// `reveal_opens` opens the closed ancestors outermost first when the active layer changes, and not
// on the first pass.
#[test]
fn active_layer_change_opens_ancestors_and_scrolls_once() {
    let mut tree = FakeLayers::sample();
    tree.layers[0].open = false;
    tree.layers[1].open = false;
    let child = FakeLayer {
        id: 10,
        parent: Some(2),
        info: LayerInfo::new("Deep", LayerKind::Pixel),
        open: false,
        effects_open: false,
        effects: None,
        smart_filters: None,
    };
    tree.layers.push(child);

    let opens = |active: u32, previous: Option<Option<u32>>| {
        reveal_opens(&tree, Some(&active), previous.as_ref().map(Option::as_ref))
            .into_iter()
            .map(|change| (change.row, change.open))
            .collect::<Vec<_>>()
    };
    assert_eq!(opens(10, Some(Some(4))), [(1, true), (2, true)]);
    assert_eq!(opens(10, Some(Some(10))), []);
    assert_eq!(opens(10, None), []);
    assert_eq!(opens(4, Some(Some(10))), []);
}

// A gesture that starts on an eye with Alt held is one `ShowOnly`, however long it goes on, and the
// held `Begin` is never reported.
#[test]
fn an_alt_sweep_over_the_eyes_reports_show_only_once() {
    use craft_ui::edit::GestureId;
    use craft_ui::list::ToggleEdit;

    use crate::ui::panel::events::LayersEvent;

    let value = |layer: u32| ToggleEdit { column: 0, row: LayerRowKey::Layer(layer), on: false };
    let id = GestureId::next();
    let edits = [
        EditEvent::<ToggleEdit<LayerRowKey<u32>>>::begin(id),
        EditEvent::update(id, value(4), Modifiers::ALT),
        EditEvent::update(id, value(5), Modifiers::ALT),
        EditEvent::commit(id, value(5), true, Modifiers::ALT),
    ];
    let mut starts = Vec::new();
    let mut events = Vec::new();
    for edit in &edits {
        toggle_event_for_tests(edit, &mut starts, &mut events);
    }
    assert_eq!(events, [LayersEvent::ShowOnly(LayerRowKey::Layer(4))]);
    assert!(starts.is_empty(), "the gesture's record ends with it");
}
