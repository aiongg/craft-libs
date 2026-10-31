//! The overlay's keyboard parts and AccessKit tree, and the focus marks drawn round the picked
//! part.

use egui::{Painter, Pos2, Rect, Shape, vec2};

use super::pointer::Pass;
use super::{GradientLineStyle, GradientOverlay};
use crate::canvas::gradient_line::{at_screen, midpoint_location};
use crate::canvas::{CanvasView, LineGradient, LinePart, Part, PartFocus};
use crate::composites::curve_editor::fill;
use crate::composites::gradient::marker_texts;
use crate::composites::{StopRef, StopRow};
use crate::localized::words::counts;
use crate::localized::{LocaleNumbers, Words};

/// The parts in keyboard order: the line, the start, the stops and midpoints by location, the end.
pub(super) struct Parts {
    pub list: Vec<Part>,
}

/// The key `PartFocus` knows `part` by.
pub(super) fn part_key(part: LinePart) -> u64 {
    match part {
        LinePart::Line => 0,
        LinePart::Start => 1,
        LinePart::End => 2,
        LinePart::Stop(i) => 3 + 2 * i as u64,
        LinePart::Midpoint(i) => 4 + 2 * i as u64,
    }
}

/// The part a [`part_key`] stands for.
pub(super) fn key_part(key: u64) -> Option<LinePart> {
    Some(match key {
        0 => LinePart::Line,
        1 => LinePart::Start,
        2 => LinePart::End,
        k if k % 2 == 1 => LinePart::Stop(((k - 3) / 2) as usize),
        k => LinePart::Midpoint(((k - 4) / 2) as usize),
    })
}

impl GradientOverlay<'_> {
    /// The part that shows `selected` in `value`, if the canvas shows it.
    pub(super) fn part_selected(&self, value: &LineGradient, selected: Option<StopRef>) -> Option<LinePart> {
        match selected? {
            StopRef::Color(i) if i < value.stops.color.len() => Some(LinePart::Stop(i)),
            StopRef::Midpoint(StopRow::Color, i) if self.midpoints && midpoint_location(&value.stops, i).is_some() => Some(LinePart::Midpoint(i)),
            _ => None,
        }
    }

    /// The group's accessible value.
    pub(super) fn group_value(&self, value: &LineGradient, ctx: &egui::Context) -> String {
        match &self.value {
            Some(text) => text.clone(),
            None => Words::of_ctx(ctx).counted(ctx, counts::BAR_STOPS, &self.labels.bar_stops, value.stops.color.len() as u64),
        }
    }

    /// The parts of `shown`, named and valued as the bar's markers and the host's units are;
    /// `selected` is the selection the canvas shows.
    pub(super) fn parts(&self, pass: &Pass<'_>, shown: &LineGradient, numbers: &LocaleNumbers, selected: Option<StopRef>) -> Parts {
        let (view, hit) = (pass.view, pass.style.hit);
        let (start, end) = (view.to_screen(shown.line.start), view.to_screen(shown.line.end));
        let around = |at: Pos2| Rect::from_center_size(at, vec2(2.0 * hit, 2.0 * hit));
        let end_value = |p: kurbo::Point| {
            self.units.map(|(x, y)| fill(&self.labels.point_value, &[("x", &x.format_value(p.x, 0.0)), ("y", &y.format_value(p.y, 0.0))]))
        };
        let group = self.group_value(shown, pass.ctx);
        let part = |part: LinePart, name: &str, value: Option<String>, rect: Rect, selected: bool| Part {
            key: part_key(part),
            name: name.to_owned(),
            value,
            rect,
            selected,
        };
        let mut list = vec![
            part(LinePart::Line, &self.names.line, Some(group), Rect::from_two_pos(start, end).expand(hit), false),
            part(LinePart::Start, &self.names.start, end_value(shown.line.start), around(start), false),
        ];
        for i in 0..shown.stops.color.len() {
            let marker = StopRef::Color(i);
            let (name, value) = marker_texts(&self.labels, numbers, self.location_decimals, self.rules.rows, &shown.stops, marker);
            let at = at_screen(start, end, shown.stops.color[i].location, self.reverse);
            list.push(part(LinePart::Stop(i), &name, Some(value), around(at), selected == Some(marker)));
            if self.midpoints
                && let Some(t) = midpoint_location(&shown.stops, i)
            {
                let marker = StopRef::Midpoint(StopRow::Color, i);
                let (name, value) = marker_texts(&self.labels, numbers, self.location_decimals, self.rules.rows, &shown.stops, marker);
                list.push(part(LinePart::Midpoint(i), &name, Some(value), around(at_screen(start, end, t, self.reverse)), selected == Some(marker)));
            }
        }
        list.push(part(LinePart::End, &self.names.end, end_value(shown.line.end), around(end), false));
        Parts { list }
    }

    /// The focus mark of the picked `part` of `shown`: two lines beside the line, or a ring round
    /// the handle, stop or diamond.
    pub(super) fn paint_focus(&self, painter: &Painter, style: &GradientLineStyle, view: &CanvasView, shown: &LineGradient, part: LinePart) {
        let (start, end) = (view.to_screen(shown.line.start), view.to_screen(shown.line.end));
        let ring = |at: Pos2, radius: f32| PartFocus::ring(painter, Rect::from_center_size(at, vec2(2.0 * radius, 2.0 * radius)));
        match part {
            LinePart::Line => {
                let along = (end - start).normalized();
                if !along.is_finite() {
                    return;
                }
                let normal = vec2(-along.y, along.x) * (style.line.width / 2.0 + style.halo.width + style.focus_offset + style.focus.width / 2.0);
                for side in [normal, -normal] {
                    painter.add(Shape::line_segment([start + side, end + side], style.focus_halo));
                    painter.add(Shape::line_segment([start + side, end + side], style.focus));
                }
            }
            LinePart::Start => ring(start, style.end_ring + style.end.width / 2.0 + style.end_halo.width),
            LinePart::End => ring(end, style.end_ring + style.end.width / 2.0 + style.end_halo.width),
            LinePart::Stop(i) => {
                if let Some(stop) = shown.stops.color.get(i) {
                    ring(at_screen(start, end, stop.location, self.reverse), style.stop + style.stop_ring.width + style.stop_halo.width);
                }
            }
            LinePart::Midpoint(i) => {
                if let Some(t) = midpoint_location(&shown.stops, i) {
                    ring(at_screen(start, end, t, self.reverse), style.diamond + style.diamond_halo.width);
                }
            }
        }
    }
}
