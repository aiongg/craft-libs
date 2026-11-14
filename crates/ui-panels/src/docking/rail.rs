//! What a rail icon does for a panel and whether it shows lit, from the dock layout.

use craft_behavior::dock::{Action, Layout};
use std::hash::Hash;

/// The actions a click on the rail icon of `panel` takes: minimize a docked and shown panel, restore and activate a
/// minimized one, close a floating one's window, open a hidden one.
pub fn rail_actions<P: Clone + Eq + Hash>(layout: &Layout<P>, panel: &P) -> Vec<Action<P>> {
    let _ = (layout, panel);
    todo!("pn.C3")
}

/// Whether the rail icon of `panel` shows lit: its group is in the layout and not minimized.
pub fn rail_lit<P: Clone + Eq + Hash>(layout: &Layout<P>, panel: &P) -> bool {
    let _ = (layout, panel);
    todo!("pn.C4")
}
