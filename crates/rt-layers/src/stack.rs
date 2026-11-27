//! `Stack`: children in a column, each a fixed height or a share of what is left.

use std::hash::Hash;

use craft_ui_rt::{BuildCx, Constraints, Element, LayoutCx, Vec2, hash_key};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Slot {
    /// An equal share of the height the fixed slots leave.
    Fill,
    Fixed(f32),
}

/// A column whose children `f` creates (like `memo`, `f` reruns when `props` change). The
/// n-th child takes the n-th slot; children past the slots fill. Neighbours are `gap` apart.
pub struct Stack<P, F> {
    props: P,
    slots: Vec<Slot>,
    gap: f32,
    f: F,
}

impl<P: Hash + 'static, F: Fn(&P, &mut BuildCx<'_>) + 'static> Stack<P, F> {
    pub fn column(props: P, slots: impl Into<Vec<Slot>>, f: F) -> Self {
        Stack { props, slots: slots.into(), gap: 0.0, f }
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }
}

impl<P: Hash + 'static, F: Fn(&P, &mut BuildCx<'_>) + 'static> Element for Stack<P, F> {
    type Output = ();
    type Local = ();

    fn props_key(&self) -> u64 {
        let slots: Vec<(bool, u32)> = self
            .slots
            .iter()
            .map(|s| match s {
                Slot::Fill => (true, 0),
                Slot::Fixed(h) => (false, h.to_bits()),
            })
            .collect();
        hash_key(&(hash_key(&self.props), slots, self.gap.to_bits()))
    }

    fn init(&self, _: &mut BuildCx<'_>) {}

    fn build(&self, _: &mut (), cx: &mut BuildCx<'_>) {
        (self.f)(&self.props, cx);
    }

    fn layout(&self, _: &mut (), cx: &mut LayoutCx<'_>, c: Constraints) -> Vec2 {
        let size = Vec2::new(if c.max.x.is_finite() { c.max.x } else { c.min.x }, if c.max.y.is_finite() { c.max.y } else { c.min.y });
        let n = cx.child_count();
        let slot = |i: usize| self.slots.get(i).copied().unwrap_or(Slot::Fill);
        let gaps = self.gap * n.saturating_sub(1) as f32;
        let fixed: f32 = gaps + (0..n).filter_map(|i| if let Slot::Fixed(h) = slot(i) { Some(h) } else { None }).sum::<f32>();
        let fills = (0..n).filter(|&i| slot(i) == Slot::Fill).count().max(1);
        let share = ((size.y - fixed) / fills as f32).max(0.0);
        let mut y = 0.0;
        for i in 0..n {
            let h = match slot(i) {
                Slot::Fill => share,
                Slot::Fixed(h) => h.min(size.y),
            };
            let child = cx.child_at(i);
            cx.layout_child(child, Constraints::tight(Vec2::new(size.x, h)));
            cx.place_child(child, Vec2::new(0.0, y));
            y += h + self.gap;
        }
        size
    }

    fn relayout_boundary(&self) -> bool {
        true
    }
}
