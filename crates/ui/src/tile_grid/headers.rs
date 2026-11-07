//! The group headers as rows of the grid's outline: where the keyboard lead stands when it is on a header, and the
//! order of headers and tile rows that Up and Down walk.

use super::layout::TileArrangement;
use crate::runtime::egui::memory as store;
use crate::runtime::memory::{adopt, sub};
use egui::{Context, Id};

/// One stop of the outline: a group header, or a row of tiles (an index into `TileArrangement::rows`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Stop {
    Header(usize),
    Row(usize),
}

/// The outline top to bottom. A closed group is its header alone; its tiles have no row.
pub(super) fn outline(arrangement: &TileArrangement) -> Vec<Stop> {
    let mut stops = Vec::with_capacity(arrangement.headers.len() + arrangement.rows.len());
    let mut next = 0;
    for (index, row) in arrangement.rows.iter().enumerate() {
        while next < arrangement.headers.len() && arrangement.headers[next].top() <= row.top {
            stops.push(Stop::Header(next));
            next += 1;
        }
        stops.push(Stop::Row(index));
    }
    stops.extend((next..arrangement.headers.len()).map(Stop::Header));
    stops
}

/// The header that holds the keyboard lead, and where the tile lead stood when the header took it: a lead that moved
/// since (a press on a tile, a key the item step took) ends the header's turn.
#[derive(Clone, Copy, Default, PartialEq)]
pub(super) struct HeaderLead {
    pub(super) group: Option<usize>,
    pub(super) tile: Option<usize>,
}

fn slot(grid: Id) -> Id {
    sub(grid, "header_lead")
}

pub(super) fn lead(ctx: &Context, grid: Id) -> HeaderLead {
    store::peek_clone::<HeaderLead>(ctx, slot(grid)).unwrap_or_default()
}

pub(super) fn set_lead(ctx: &Context, grid: Id, lead: HeaderLead) {
    store::put(ctx, slot(grid), lead);
    adopt::<HeaderLead>(ctx, grid, slot(grid));
}
