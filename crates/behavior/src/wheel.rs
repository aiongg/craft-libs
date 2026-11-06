//! Wheel gesture values, as craft-behavior's copy of egui's types so the wheel split is egui-free.

/// How far one wheel event reaches.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum WheelUnit {
    /// Points.
    #[default]
    Point,
    /// Lines of text.
    Line,
    /// Pages.
    Page,
}

/// A wheel gesture's phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum WheelPhase {
    /// The first event of a gesture.
    Start,
    /// A later event.
    #[default]
    Move,
    /// The fingers lifted.
    End,
    /// The system cancelled the gesture.
    Cancel,
}

use craft_keymap::Mods;
use emath::{Pos2, Rect, Vec2};

/// One wheel event, or the sum of one pass's, as a taker reads it with `take_wheel`.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Wheel {
    /// Scroll in points, wheel notches and lines already converted by the service.
    pub scroll: Vec2,
    /// Zoom factor; 1 for none.
    pub zoom: f32,
    /// The modifiers held.
    pub mods: Mods,
    /// The unit the device reported.
    pub unit: WheelUnit,
    /// The gesture phase.
    pub phase: WheelPhase,
}

impl Wheel {
    /// A scroll of `scroll` points with no zoom.
    pub const fn new(scroll: Vec2, mods: Mods, unit: WheelUnit, phase: WheelPhase) -> Self {
        Wheel { scroll, zoom: 1.0, mods, unit, phase }
    }

    /// The same gesture with zoom factor `zoom`.
    pub const fn zoom(mut self, zoom: f32) -> Self {
        self.zoom = zoom;
        self
    }
}

/// A component that takes wheel input, as registered last pass.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct WheelTaker<O> {
    /// The component.
    pub owner: O,
    /// Its view rect.
    pub rect: Rect,
    /// It takes horizontal scroll.
    pub horizontal: bool,
    /// It takes vertical scroll.
    pub vertical: bool,
    /// It takes zoom.
    pub zoom: bool,
    /// A value control: it takes vertical scroll only while focused or armed. Horizontal scroll needs no gate.
    pub gated: bool,
}

impl<O> WheelTaker<O> {
    /// A taker over `rect` that takes no axis yet.
    pub const fn new(owner: O, rect: Rect) -> Self {
        WheelTaker { owner, rect, horizontal: false, vertical: false, zoom: false, gated: false }
    }

    /// Whether it takes horizontal scroll.
    pub const fn horizontal(mut self, on: bool) -> Self {
        self.horizontal = on;
        self
    }

    /// Whether it takes vertical scroll.
    pub const fn vertical(mut self, on: bool) -> Self {
        self.vertical = on;
        self
    }

    /// Whether it takes zoom.
    pub const fn zoom(mut self, on: bool) -> Self {
        self.zoom = on;
        self
    }

    /// Whether vertical scroll needs focus or arming.
    pub const fn gated(mut self, on: bool) -> Self {
        self.gated = on;
        self
    }
}

/// The split rule: each axis of `wheel` goes to the innermost taker under `at` that takes it; a share no taker
/// takes passes to the next enclosing one. A gated taker takes vertical scroll only while it is `focused` or
/// `armed`. Shares of one owner merge.
pub fn split<O: Copy + PartialEq>(takers: &[WheelTaker<O>], at: Pos2, wheel: Wheel, focused: Option<O>, armed: Option<O>) -> Vec<(O, Wheel)> {
    let mut under: Vec<&WheelTaker<O>> = takers.iter().filter(|taker| taker.rect.contains(at)).collect();
    under.sort_by(|a, b| a.rect.area().total_cmp(&b.rect.area()));
    let open = |taker: &&WheelTaker<O>| !taker.gated || armed == Some(taker.owner) || focused == Some(taker.owner);
    let vertical = (wheel.scroll.y != 0.0).then(|| under.iter().find(|t| t.vertical && open(t))).flatten().map(|t| t.owner);
    let horizontal = (wheel.scroll.x != 0.0).then(|| under.iter().find(|t| t.horizontal)).flatten().map(|t| t.owner);
    let zoom = (wheel.zoom != 1.0).then(|| under.iter().find(|t| t.zoom)).flatten().map(|t| t.owner);
    let mut shares: Vec<(O, Wheel)> = Vec::new();
    let parts = [(vertical, Vec2::new(0.0, wheel.scroll.y), 1.0), (horizontal, Vec2::new(wheel.scroll.x, 0.0), 1.0), (zoom, Vec2::ZERO, wheel.zoom)];
    for (owner, scroll, zoom) in parts {
        let Some(owner) = owner else { continue };
        match shares.iter_mut().find(|(o, _)| *o == owner) {
            Some((_, share)) => {
                share.scroll += scroll;
                share.zoom *= zoom;
            }
            None => shares.push((owner, Wheel { scroll, zoom, ..wheel })),
        }
    }
    shares
}

#[cfg(test)]
mod tests {
    use super::*;
    use emath::pos2;

    fn wheel(x: f32, y: f32) -> Wheel {
        Wheel::new(Vec2::new(x, y), Mods::NONE, WheelUnit::Point, WheelPhase::Move)
    }

    fn rect(size: f32) -> Rect {
        Rect::from_min_size(pos2(0.0, 0.0), Vec2::splat(size))
    }

    #[test]
    fn a_share_no_inner_taker_takes_passes_to_the_enclosing_one() {
        let panel = WheelTaker::new(1u8, rect(200.0)).vertical(true).horizontal(true);
        let list = WheelTaker::new(2u8, rect(100.0)).vertical(true);
        let shares = split(&[panel, list], pos2(10.0, 10.0), wheel(5.0, 7.0), None, None);
        assert_eq!(shares.len(), 2);
        let scroll_of = |owner| shares.iter().find(|(o, _)| *o == owner).map(|(_, w)| w.scroll);
        assert_eq!(scroll_of(2), Some(Vec2::new(0.0, 7.0)), "the list takes the vertical share");
        assert_eq!(scroll_of(1), Some(Vec2::new(5.0, 0.0)), "the sideways share passes to the panel");
    }

    #[test]
    fn a_gated_control_takes_vertical_scroll_only_when_focused_or_armed_and_horizontal_always() {
        let panel = WheelTaker::new(1u8, rect(200.0)).vertical(true);
        let slider = WheelTaker::new(2u8, rect(100.0)).vertical(true).horizontal(true).gated(true);
        let takers = [panel, slider];
        let at = pos2(10.0, 10.0);
        let owner_of_vertical = |focused, armed| split(&takers, at, wheel(0.0, 3.0), focused, armed).first().map(|(o, _)| *o);
        assert_eq!(owner_of_vertical(None, None), Some(1));
        assert_eq!(owner_of_vertical(Some(2), None), Some(2));
        assert_eq!(owner_of_vertical(None, Some(2)), Some(2));
        assert_eq!(split(&takers, at, wheel(3.0, 0.0), None, None).first().map(|(o, _)| *o), Some(2));
    }

    #[test]
    fn the_zoom_share_goes_to_the_innermost_zooming_taker() {
        let scroller = WheelTaker::new(1u8, rect(200.0)).vertical(true);
        let canvas = WheelTaker::new(2u8, rect(100.0)).zoom(true);
        let shares = split(&[scroller, canvas], pos2(10.0, 10.0), wheel(0.0, 0.0).zoom(1.5), None, None);
        assert_eq!(shares.len(), 1);
        assert_eq!((shares[0].0, shares[0].1.zoom), (2, 1.5));
    }
}
