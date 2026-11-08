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

use crate::Sense;
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
    /// Where the pointer was, in screen points; a zoom is about this point.
    pub at: Option<Pos2>,
}

impl Wheel {
    /// A scroll of `scroll` points with no zoom.
    pub const fn new(scroll: Vec2, mods: Mods, unit: WheelUnit, phase: WheelPhase) -> Self {
        Wheel { scroll, zoom: 1.0, mods, unit, phase, at: None }
    }

    /// The same gesture with zoom factor `zoom`.
    pub const fn zoom(mut self, zoom: f32) -> Self {
        self.zoom = zoom;
        self
    }

    /// The same gesture with the pointer at `at`.
    pub const fn at(mut self, at: Pos2) -> Self {
        self.at = Some(at);
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
    /// A scroll region (a scrolling body or collection): Shift turns the vertical wheel sideways over it.
    pub scrolls: bool,
    /// The distance of one wheel line.
    pub notch: WheelNotch,
}

/// The distance one wheel line scrolls a taker.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum WheelNotch {
    /// This many points.
    Points(f32),
    /// This share of the taker's view along the axis.
    ViewShare(f32),
}

impl WheelNotch {
    /// The distance of a taker that states none.
    pub const DEFAULT: WheelNotch = WheelNotch::Points(40.0);
}

impl<O> WheelTaker<O> {
    /// A taker over `rect` that takes no axis yet.
    pub const fn new(owner: O, rect: Rect) -> Self {
        WheelTaker { owner, rect, horizontal: false, vertical: false, zoom: false, gated: false, scrolls: false, notch: WheelNotch::DEFAULT }
    }

    /// The taker a shape with `sense` registers, `None` when `sense` has no wheel bit: `WHEEL` takes both scroll
    /// axes and zoom; `WHEEL_X`, `WHEEL_Y` and `WHEEL_ZOOM` take one share each and combine.
    pub fn for_sense(owner: O, rect: Rect, sense: Sense) -> Option<Self> {
        let all = sense.contains(Sense::WHEEL);
        let taker = WheelTaker::new(owner, rect)
            .horizontal(all || sense.contains(Sense::WHEEL_X))
            .vertical(all || sense.contains(Sense::WHEEL_Y))
            .zoom(all || sense.contains(Sense::WHEEL_ZOOM));
        (taker.horizontal || taker.vertical || taker.zoom).then_some(taker)
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

    /// The distance of one wheel line.
    pub const fn notch(mut self, notch: WheelNotch) -> Self {
        self.notch = notch;
        self
    }

    /// The taker is a scroll region (`WheelTaker::scrolls`).
    #[must_use]
    pub const fn scrolls(mut self, on: bool) -> Self {
        self.scrolls = on;
        self
    }

    /// `scroll`, given in `unit`s of the device, as points of this taker's view: a line is one notch, a page is
    /// the view.
    pub fn points(&self, scroll: Vec2, unit: WheelUnit) -> Vec2 {
        let size = self.rect.size();
        match unit {
            WheelUnit::Point => scroll,
            WheelUnit::Line => match self.notch {
                WheelNotch::Points(points) => scroll * points,
                WheelNotch::ViewShare(share) => scroll * size * share,
            },
            WheelUnit::Page => scroll * size,
        }
    }
}

/// The split rule: each axis of `wheel` goes to the innermost taker under `at` that takes it; a share no taker
/// takes passes to the next enclosing one. A gated taker takes vertical scroll only while it is `focused` or
/// `armed`. Shift turns a vertical wheel sideways when its vertical owner is a scroll region. Shares of one owner merge.
pub fn split<O: Copy + PartialEq>(takers: &[WheelTaker<O>], at: Pos2, wheel: Wheel, focused: Option<O>, armed: Option<O>) -> Vec<(O, Wheel)> {
    let mut under: Vec<&WheelTaker<O>> = takers.iter().filter(|taker| taker.rect.contains(at)).collect();
    under.sort_by(|a, b| a.rect.area().total_cmp(&b.rect.area()));
    let open = |taker: &&WheelTaker<O>| !taker.gated || armed == Some(taker.owner) || focused == Some(taker.owner);
    let vertical_taker = (wheel.scroll.y != 0.0).then(|| under.iter().find(|t| t.vertical && open(t))).flatten();
    // Shift turns the vertical wheel sideways over a scroll region (Photoshop); a value control that takes the
    // vertical wheel keeps Shift as its modifier, and a device that already sends a sideways axis keeps it.
    let wheel = if wheel.mods.shift && wheel.scroll.x == 0.0 && vertical_taker.is_some_and(|t| t.scrolls) {
        Wheel { scroll: Vec2::new(wheel.scroll.y, 0.0), ..wheel }
    } else {
        wheel
    };
    let vertical = if wheel.scroll.y != 0.0 { vertical_taker.map(|t| t.owner) } else { None };
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
    fn a_line_scrolls_the_takers_notch_and_a_page_scrolls_its_view() {
        let lines = WheelTaker::new(1u8, rect(200.0)).vertical(true).notch(WheelNotch::Points(30.0));
        assert_eq!(lines.points(Vec2::new(0.0, 2.0), WheelUnit::Line), Vec2::new(0.0, 60.0));
        let share = WheelTaker::new(1u8, rect(200.0)).vertical(true).notch(WheelNotch::ViewShare(0.25));
        assert_eq!(share.points(Vec2::new(0.0, 2.0), WheelUnit::Line), Vec2::new(0.0, 100.0));
        assert_eq!(share.points(Vec2::new(0.0, 1.0), WheelUnit::Page), Vec2::new(0.0, 200.0));
        assert_eq!(share.points(Vec2::new(0.0, 7.0), WheelUnit::Point), Vec2::new(0.0, 7.0));
    }

    #[test]
    fn shift_turns_the_wheel_sideways_over_a_scroll_region_and_not_over_a_value_control() {
        let shifted = Wheel::new(Vec2::new(0.0, 7.0), Mods { shift: true, ..Mods::NONE }, WheelUnit::Point, WheelPhase::Move);
        let region = WheelTaker::new(1u8, rect(200.0)).vertical(true).horizontal(true).scrolls(true);
        let sideways = split(&[region], pos2(10.0, 10.0), shifted, None, None);
        assert_eq!(sideways[0].1.scroll, Vec2::new(7.0, 0.0));
        let slider = WheelTaker::new(2u8, rect(100.0)).vertical(true).gated(true);
        let kept = split(&[region, slider], pos2(10.0, 10.0), shifted, Some(2), None);
        assert_eq!((kept[0].0, kept[0].1.scroll), (2, Vec2::new(0.0, 7.0)));
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
    fn a_zoom_only_taker_takes_zoom_and_passes_both_scroll_shares_on() {
        let panel = WheelTaker::new(1u8, rect(200.0)).vertical(true).horizontal(true);
        let pane = WheelTaker::for_sense(2u8, rect(100.0), Sense::WHEEL_ZOOM).expect("a wheel bit makes a taker");
        let at = pos2(10.0, 10.0);
        let shares = split(&[panel, pane], at, wheel(4.0, 6.0).zoom(1.5), None, None);
        let of = |owner| shares.iter().find(|(o, _)| *o == owner).map(|(_, w)| (w.scroll, w.zoom));
        assert_eq!(of(1), Some((Vec2::new(4.0, 6.0), 1.0)), "both scroll shares reach the panel");
        assert_eq!(of(2), Some((Vec2::ZERO, 1.5)), "the zoom share stays with the pane");
    }

    #[test]
    fn wheel_bits_combine_into_the_axes_a_taker_takes() {
        let axes = |sense| WheelTaker::for_sense(1u8, rect(10.0), sense).map(|t| (t.horizontal, t.vertical, t.zoom));
        assert_eq!(axes(Sense::FOCUS), None);
        assert_eq!(axes(Sense::WHEEL), Some((true, true, true)));
        assert_eq!(axes(Sense::WHEEL | Sense::WHEEL_X), Some((true, true, true)));
        assert_eq!(axes(Sense::WHEEL_X), Some((true, false, false)));
        assert_eq!(axes(Sense::WHEEL_Y | Sense::WHEEL_ZOOM), Some((false, true, true)));
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
