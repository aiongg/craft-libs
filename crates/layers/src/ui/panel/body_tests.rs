//! Tests of the display order and the click and reveal rules (L5).

use craft_ui::behavior::RowInfo;
use egui::Modifiers;

use super::body::{display_rows, reveal_opens, thumb_click_replaces_select};
use crate::ui::filter::{FilterKind, LayerFilter};
use crate::ui::mock::{MockLayer, MockLayers};
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
    let mut tree = MockLayers::sample();
    let open = display_rows(&tree, &LayerFilter::default());
    assert_eq!(layers(&open), [(1, 0), (2, 1), (3, 1), (4, 0), (5, 0), (6, 0), (7, 0), (8, 0), (9, 0)]);
    assert!(open[0].has_children && open[0].open);

    tree.layers[0].open = false;
    let closed = display_rows(&tree, &LayerFilter::default());
    assert_eq!(layers(&closed), [(1, 0), (4, 0), (5, 0), (6, 0), (7, 0), (8, 0), (9, 0)]);
    assert!(closed[0].has_children && !closed[0].open);

    tree.layers[5].effects_open = true;
    tree.layers[6].effects_open = true;
    let keys: Vec<_> = display_rows(&tree, &LayerFilter::default()).into_iter().skip(3).map(|row| (row.key, row.depth)).collect();
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

// With an active filter, matches inside closed groups are listed at their depth and the groups are
// left out.
#[test]
fn filtered_view_keeps_matches_inside_closed_groups_at_their_depth() {
    let mut tree = MockLayers::sample();
    tree.layers[0].open = false;
    let filter = LayerFilter { kinds: vec![FilterKind::Pixel], ..LayerFilter::default() };
    let rows = display_rows(&tree, &filter);
    assert_eq!(layers(&rows), [(2, 1), (3, 1), (7, 0), (8, 0), (9, 0)]);
    assert!(rows.iter().all(|row| !row.has_children));
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
    let mut tree = MockLayers::sample();
    tree.layers[0].open = false;
    tree.layers[1].open = false;
    let child = MockLayer {
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
