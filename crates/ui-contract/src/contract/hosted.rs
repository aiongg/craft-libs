//! A leaf a component hosts in a slot of its own (a row's field, a notice's drop hint), erased so the host can keep
//! leaves of different types: measured under the slot's key, then shown at the rect the host placed it in.

use super::{Component, MeasureCx, PaintUi, Role, Show};
use craft_behavior::Proposal;
use craft_ui_rt::{Rect, Vec2};

/// The key a hosted leaf is measured and shown under: the slot's name and its index among the host's slots.
pub type HostedKey = (&'static str, usize);

/// A hosted leaf of a host `H`, erased.
pub trait Hosted<H: Component> {
    /// The leaf's size at `proposal`, as `show` will show it under `key`.
    fn measure(&mut self, cx: &MeasureCx<'_, H>, key: HostedKey, proposal: Proposal) -> Vec2;

    /// Shows the leaf at `rect` under `key` and hands its output on. The host's paint calls it in each pointer look
    /// it records: the first call shows the leaf and hands its output on, a later one keeps the child the first
    /// showed (docs/contract-rt.md 15.6 rule 5).
    fn show(&mut self, ui: &mut PaintUi<'_>, key: HostedKey, rect: Rect);

    /// The leaf's kind of component, for a host that places some kinds apart (a link only as wide as its text).
    fn role(&self) -> Role;
}

/// A hosted leaf and what takes its output; a hidden one keeps its slot's room and shows nothing.
pub struct Hosting<C, F> {
    /// The leaf and its output's taker until the first `show` uses them.
    component: Option<(C, F)>,
    shown: bool,
}

impl<C: Show, F: FnOnce(C::Output)> Hosting<C, F> {
    /// `component`, whose output goes to `output` in the pass it is shown.
    pub fn new(component: C, output: F) -> Self {
        Hosting { component: Some((component, output)), shown: true }
    }

    /// Keeps the slot as wide as the component measures and shows nothing in it.
    #[must_use]
    pub fn hidden(mut self) -> Self {
        self.shown = false;
        self
    }
}

impl<H: Component, C: Show, F: FnOnce(C::Output)> Hosted<H> for Hosting<C, F> {
    fn measure(&mut self, cx: &MeasureCx<'_, H>, key: HostedKey, proposal: Proposal) -> Vec2 {
        match &mut self.component {
            Some((component, _)) => cx.child(key, component, proposal),
            None => Vec2::ZERO,
        }
    }

    fn show(&mut self, ui: &mut PaintUi<'_>, key: HostedKey, rect: Rect) {
        if !self.shown {
            return;
        }
        match self.component.take() {
            Some((component, output)) => output(ui.child(key, rect, component)),
            None => ui.keep_shown(key, rect),
        }
    }

    fn role(&self) -> Role {
        C::ROLE
    }
}
