//! The tile grid's layout: justified rows fill the width with whole cards (picture and what the tile adds around it),
//! groups start new rows below their headers, a closed group keeps its header only, and a hit finds the card under the
//! point.

use craft_behavior::{ItemLayout as _, ScrollPos};
use craft_ui::tile_grid::{Around, ArrangeSpacing, GridLayout, TileGroup, Tiling, arrange};
use egui::pos2;

const WIDTH: f32 = 400.0;
const ROW: f32 = 100.0;
const GAP: f32 = 8.0;
const INSET: f32 = 4.0;
const HEADER: f32 = 30.0;
const EPS: f32 = 0.01;

fn spacing() -> ArrangeSpacing {
    ArrangeSpacing::new(GAP, INSET, HEADER)
}

#[craft_ui::testing::trial]
fn a_justified_row_but_the_last_fills_the_width_exactly() {
    let aspects = [1.5; 9];
    let around = Around::new(6.0, 2.0, 3.0, 20.0);
    let laid = arrange(&aspects, &[], WIDTH, Tiling::justified(ROW), spacing(), around);
    assert!(laid.rows.len() > 1, "nine wide tiles do not fit one row");
    let content = WIDTH - 2.0 * INSET;
    for row in &laid.rows[..laid.rows.len() - 1] {
        let (first, last) = (laid.tiles[row.tiles.start], laid.tiles[row.tiles.end - 1]);
        let span = (last.right() + around.end) - (first.left() - around.start);
        assert!((span - content).abs() < EPS, "a full row of cards spans the content width");
        assert!(row.height >= ROW + around.height() - EPS, "scaling a full row up never shrinks it below the row height");
    }
    let last = laid.rows.last().expect("rows");
    assert!((last.height - ROW - around.height()).abs() < EPS, "the last row keeps the row height");
}

#[craft_ui::testing::trial]
fn each_group_starts_a_row_below_its_header_and_a_closed_group_has_no_rows() {
    let groups = [TileGroup::new(0, 5, "A"), TileGroup::new(5, 5, "B").open(false), TileGroup::new(10, 4, "C")];
    let laid = arrange(&[1.0; 14], &groups, WIDTH, Tiling::cards(100.0, 1.0), spacing(), Around::default());
    assert_eq!(laid.headers.len(), 3);
    assert!(laid.rows.iter().all(|row| row.group != Some(1)), "a closed group has no rows");
    assert!(laid.tiles[5..10].iter().all(|tile| *tile == egui::Rect::NOTHING), "its tiles have no place");
    let first_of_a = laid.rows.iter().find(|row| row.group == Some(0)).expect("group A has rows");
    assert!(first_of_a.top >= laid.headers[0].bottom() - EPS, "tiles sit below their header");
    let first_of_c = laid.rows.iter().find(|row| row.group == Some(2)).expect("group C has rows");
    assert!(laid.headers[2].top() > laid.headers[1].top(), "headers keep display order");
    assert!(first_of_c.top >= laid.headers[2].bottom() - EPS);
}

#[craft_ui::testing::trial]
fn a_hit_finds_the_card_under_the_point_after_scrolling() {
    let around = Around::new(0.0, 0.0, 0.0, 20.0);
    let laid = arrange(&[1.0; 12], &[], WIDTH, Tiling::cards(100.0, 1.0), spacing(), around);
    let layout = GridLayout::arranged(laid.clone(), WIDTH, around);
    let offset = ScrollPos { x: 0.0, y: 50.0 };
    let second_row = layout.rect(laid.rows[1].tiles.start, offset);
    assert_eq!(layout.hit(second_row.center(), offset), Some(laid.rows[1].tiles.start));
    assert_eq!(layout.hit(pos2(-1.0, -1.0), offset), None, "the margin holds no tile");
}
