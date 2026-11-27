//! `ResetMark`: the mark the label step shows beside a side label: a dot while the value differs from its reset value,
//! a ring while it is at it.

use super::items;
use crate::style::ResetMarkLook;
use craft_ui_rt::Item;
use craft_ui_rt::Pos2;

/// The reset mark painter.
pub struct ResetMark;

impl ResetMark {
    /// The mark centred on `center`: the dot while the value `differs`, else the ring.
    pub fn items(look: &ResetMarkLook, center: Pos2, differs: bool) -> Vec<Item> {
        let radius = look.diameter / 2.0;
        if differs {
            vec![items::circle_filled(center, radius, look.color.egui())]
        } else {
            vec![items::circle_stroke(center, radius - look.ring_width, look.ring_width, look.ring.egui())]
        }
    }
}
