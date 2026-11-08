//! `MarqueeSelect`: the rectangle a drag over list items sweeps and the selection it makes (8.7).

use craft_keymap::Mods;
use emath::{Pos2, Rect};

/// How the swept items combine with the selection the drag started from. Items follow the list rule (Cmd or Ctrl
/// toggles); pixels follow [`SelectionOperation`](crate::geometry::SelectionOperation) (Shift adds, Alt
/// subtracts, both intersect).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum MarqueeMode {
    /// The swept items become the selection.
    Replace,
    /// The swept items join it.
    Add,
    /// The swept items flip: selected ones leave, the others join.
    Toggle,
    /// The swept items leave it.
    Subtract,
}

impl MarqueeMode {
    /// The mode a press with `mods` starts: Shift adds, Cmd or Ctrl toggles, Alt subtracts, none replaces.
    pub const fn of(mods: Mods) -> MarqueeMode {
        if mods.alt {
            MarqueeMode::Subtract
        } else if mods.cmd || mods.ctrl {
            MarqueeMode::Toggle
        } else if mods.shift {
            MarqueeMode::Add
        } else {
            MarqueeMode::Replace
        }
    }
}

/// A marquee drag in progress. It holds the selection the drag started from, so a cancel restores it.
#[derive(Clone, Debug, PartialEq)]
pub struct MarqueeSelect<K> {
    mode: MarqueeMode,
    base: Vec<K>,
    origin: Pos2,
    pos: Pos2,
}

impl<K: Clone + PartialEq> MarqueeSelect<K> {
    /// A marquee that began at `origin` over `base`, in the mode `mods` pick.
    pub fn begin(origin: Pos2, mods: Mods, base: Vec<K>) -> Self {
        MarqueeSelect { mode: MarqueeMode::of(mods), base, origin, pos: origin }
    }

    /// The mode the drag runs in.
    pub fn mode(&self) -> MarqueeMode {
        self.mode
    }

    /// Moves the free corner to `pos`.
    pub fn update(&mut self, pos: Pos2) {
        self.pos = pos;
    }

    /// The swept rectangle.
    pub fn rect(&self) -> Rect {
        Rect::from_two_pos(self.origin, self.pos)
    }

    /// The selection while the marquee is live, from the items `items` (key and rect) it sweeps.
    pub fn selection(&self, items: impl IntoIterator<Item = (K, Rect)>) -> Vec<K> {
        let marquee = self.rect();
        let swept: Vec<K> = items.into_iter().filter(|(_, rect)| marquee.intersects(*rect)).map(|(key, _)| key).collect();
        match self.mode {
            MarqueeMode::Replace => swept,
            MarqueeMode::Add => {
                let mut keys = self.base.clone();
                keys.extend(swept.into_iter().filter(|key| !self.base.contains(key)));
                keys
            }
            MarqueeMode::Toggle => {
                let mut keys: Vec<K> = self.base.iter().filter(|key| !swept.contains(key)).cloned().collect();
                keys.extend(swept.into_iter().filter(|key| !self.base.contains(key)));
                keys
            }
            MarqueeMode::Subtract => self.base.iter().filter(|key| !swept.contains(key)).cloned().collect(),
        }
    }

    /// Ends the drag with Escape: the selection it started from.
    pub fn cancel(self) -> Vec<K> {
        self.base
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use emath::{Vec2, pos2};

    fn items() -> Vec<(u8, Rect)> {
        (0..4).map(|i| (i, Rect::from_min_size(pos2(f32::from(i) * 20.0, 0.0), Vec2::splat(10.0)))).collect()
    }

    fn sweep(mods: Mods) -> Vec<u8> {
        let mut marquee = MarqueeSelect::begin(pos2(-5.0, -5.0), mods, vec![1, 3]);
        marquee.update(pos2(45.0, 5.0));
        marquee.selection(items())
    }

    #[test]
    fn the_modes_combine_the_swept_items_with_the_starting_selection() {
        assert_eq!(sweep(Mods::NONE), vec![0, 1, 2]);
        assert_eq!(sweep(Mods::SHIFT), vec![1, 3, 0, 2]);
        assert_eq!(sweep(Mods::CMD), vec![3, 0, 2]);
        assert_eq!(sweep(Mods::ALT), vec![3]);
    }

    #[test]
    fn escape_restores_the_selection_the_drag_started_from() {
        let mut marquee = MarqueeSelect::begin(pos2(0.0, 0.0), Mods::NONE, vec![3u8]);
        marquee.update(pos2(100.0, 100.0));
        assert_eq!(marquee.cancel(), vec![3]);
    }
}
