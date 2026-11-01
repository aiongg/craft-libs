//! The grid's keys: arrows, Home and End, page keys, rating keys, Space, Enter, Delete.

use crate::behavior::{GridKey, GridNav};
use egui::{Key, Modifiers};

use super::{RatingKeys, SpaceAction, ZoomStep};

/// What one key press asks of the grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GridKeyAction {
    /// Move the lead to `to`; `extend` with Shift, `lead_only` with Cmd/Ctrl. `goal` is the column
    /// that vertical moves keep through rows of unequal length.
    Move {
        to: usize,
        goal: usize,
        extend: bool,
        lead_only: bool,
        key: GridKey,
    },
    SelectAll,
    Activate,
    Preview,
    ToggleLead,
    Remove,
    Rate(u8),
    Zoom(ZoomStep),
    /// F2: rename the lead.
    Rename,
    Menu,
}

/// What the key rules read of the grid.
pub(super) struct KeyGrid<'a> {
    pub mac: bool,
    pub nav: &'a GridNav,
    pub lead: Option<usize>,
    pub goal: Option<usize>,
    pub space: SpaceAction,
    pub check_mode: bool,
    pub rating: RatingKeys,
    /// Whether the lead can be renamed (F2 is claimed only then).
    pub renamable: bool,
}

const DIGITS: [Key; 6] = [Key::Num0, Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5];

fn digit(key: Key) -> Option<u8> {
    DIGITS.iter().position(|d| *d == key).map(|n| n as u8)
}

/// The action for `key` with `modifiers` on `grid` (its lead at `grid.lead`): arrows and Home,
/// End, Page Up, Page Down through `nav.step`; Cmd/Ctrl+A; Enter; Space per `space` (Ctrl+Space
/// always `ToggleLead`; check mode always toggles); Delete and Backspace; the rating keys per
/// `rating` (digits 1–5 rate, 0 clears); Cmd/Ctrl + `=`/`+`, `-`, `0` zoom (checked before the
/// rating digits, so Cmd/Ctrl+0 is Reset only when `rating` is not `CommandDigits`, where it
/// clears the rating: Bridge's rule); the Menu key and Shift+F10.
pub(super) fn action(key: Key, modifiers: Modifiers, grid: &KeyGrid<'_>) -> Option<GridKeyAction> {
    // pdfcraft: crates/ui-egui/src/workflows/combine_ui/view.rs:519 (`zoom_keys`); lightcraft: crates/ui-egui/src/panels/grid.rs:536 (`vertical_neighbor`).
    let Modifiers { alt, ctrl, shift, mac_cmd, .. } = modifiers;
    let command = if grid.mac { mac_cmd && !ctrl } else { ctrl && !mac_cmd };
    let plain = !alt && !ctrl && !mac_cmd;

    if key == Key::Menu && !alt && !command {
        return Some(GridKeyAction::Menu);
    }
    if key == Key::F10 && shift && !alt && !ctrl && !mac_cmd {
        return Some(GridKeyAction::Menu);
    }
    if let Some(grid_key) = GridKey::from_key(key) {
        if alt || (ctrl && mac_cmd) || (!grid.mac && mac_cmd) || (grid.mac && ctrl) {
            return None;
        }
        return step(grid, grid_key).map(|(to, goal)| GridKeyAction::Move { to, goal, extend: shift && !command, lead_only: command, key: grid_key });
    }
    if command && !alt {
        match key {
            Key::A if !shift => return Some(GridKeyAction::SelectAll),
            Key::Equals | Key::Plus => return Some(GridKeyAction::Zoom(ZoomStep::In)),
            Key::Minus => return Some(GridKeyAction::Zoom(ZoomStep::Out)),
            Key::Num0 if grid.rating != RatingKeys::CommandDigits => return Some(GridKeyAction::Zoom(ZoomStep::Reset)),
            _ => {}
        }
        if grid.rating == RatingKeys::CommandDigits
            && !shift
            && let Some(rating) = digit(key)
        {
            return Some(GridKeyAction::Rate(rating));
        }
    }
    match key {
        Key::Enter if plain && !shift => Some(GridKeyAction::Activate),
        Key::F2 if plain && !shift && grid.renamable => Some(GridKeyAction::Rename),
        Key::Space if ctrl && !alt && !mac_cmd => Some(GridKeyAction::ToggleLead),
        Key::Space if plain => {
            Some(if grid.check_mode || grid.space == SpaceAction::Toggle { GridKeyAction::ToggleLead } else { GridKeyAction::Preview })
        }
        Key::Delete | Key::Backspace if plain => Some(GridKeyAction::Remove),
        _ if plain && !shift && grid.rating == RatingKeys::Digits => digit(key).map(GridKeyAction::Rate),
        _ => None,
    }
}

/// Where `key` takes the lead, and the column to keep. With no lead, the keys that go backward
/// start from the last tile and the others from the first.
fn step(grid: &KeyGrid<'_>, key: GridKey) -> Option<(usize, usize)> {
    if grid.nav.is_empty() {
        return None;
    }
    let from = match grid.lead {
        Some(lead) => lead,
        None => {
            let backward = matches!(key, GridKey::Left | GridKey::Up | GridKey::PageUp | GridKey::End);
            let to = if backward { grid.nav.len() - 1 } else { 0 };
            return Some((to, grid.nav.column_of(to)));
        }
    };
    let step = grid.nav.step(from, key, grid.goal);
    Some((step.index, step.goal))
}
