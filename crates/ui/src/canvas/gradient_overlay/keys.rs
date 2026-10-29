//! The overlay's keys while its group has focus: the picked part moved, removed, added after or
//! asked for its colour.

use egui::{Key, Modifiers, Ui};

use super::pointer::{Pass, Selection, selection_of};
use super::{GradientOverlay, GradientOverlayResponse};
use crate::behavior::nudge;
use crate::canvas::{GradientLine, LineGradient, LinePart};
use crate::composites::StopRow;
use crate::composites::curve_editor::take_exact;
use crate::composites::gradient::stops;
use crate::edit::Gesture;

impl GradientOverlay<'_> {
    /// Handles the keys of the picked part (the line when none is picked): Left and Right on a
    /// stop or midpoint, the arrows on an end or the line, Delete and Backspace on a stop, Insert
    /// and Cmd/Ctrl+Enter, Enter and Space. Each change is one Set, and the part it leaves is
    /// picked and, for a stop or midpoint, selected. Reads nothing while a text field has focus, a
    /// popup is open or a modal layer is above.
    pub(super) fn keys(&self, ui: &Ui, pass: &Pass<'_>, gesture: &mut Gesture<LineGradient>, sel: &mut Selection, out: &mut GradientOverlayResponse) {
        if !nudge::reads(ui) {
            return;
        }
        let shown = gesture.shown().clone();
        let mut value = shown.clone();
        let mut part = sel.picked.unwrap_or(LinePart::Line);
        let take = |modifiers, key| take_exact(ui, modifiers, key);

        if take(Modifiers::NONE, Key::Insert) + take(Modifiers::COMMAND, Key::Enter) > 0 {
            let after = match part {
                LinePart::Stop(i) | LinePart::Midpoint(i) => Some(i),
                _ => None,
            };
            let at = stops::halfway(&value.stops, StopRow::Color, after);
            if let Some((added, i)) = stops::add(&value.stops, StopRow::Color, at, pass.sampler, &self.rules) {
                value.stops = added;
                part = LinePart::Stop(i);
            }
        }
        if take(Modifiers::NONE, Key::Enter) + take(Modifiers::NONE, Key::Space) > 0 {
            out.color_requested = self.color_request(pass, &shown, part);
        }
        match part {
            LinePart::Stop(i) | LinePart::Midpoint(i) if i < value.stops.color.len() => {
                let delete = matches!(part, LinePart::Stop(_)) && take(Modifiers::NONE, Key::Delete) + take(Modifiers::NONE, Key::Backspace) > 0;
                if delete {
                    if let Some((removed, j)) = stops::remove(&value.stops, StopRow::Color, i, &self.rules) {
                        value.stops = removed;
                        part = LinePart::Stop(j);
                    }
                } else {
                    let steps = |key| 10 * take(Modifiers::SHIFT, key) as i32 + take(Modifiers::NONE, key) as i32;
                    let d = (steps(Key::ArrowRight) - steps(Key::ArrowLeft)) as f32 / 100.0;
                    let moved = match part {
                        _ if d == 0.0 => None,
                        LinePart::Midpoint(_) => {
                            let at = value.stops.color[i].midpoint + d;
                            stops::set_midpoint(&value.stops, StopRow::Color, i, at, &self.rules).map(|(s, _)| (s, part))
                        }
                        _ => {
                            let at = value.stops.color[i].location + d;
                            stops::move_to(&value.stops, StopRow::Color, i, at, &self.rules).map(|(s, j)| (s, LinePart::Stop(j)))
                        }
                    };
                    if let Some((s, p)) = moved {
                        value.stops = s;
                        part = p;
                    }
                }
            }
            LinePart::Line | LinePart::Start | LinePart::End => {
                if let Some(by) = nudge::read(ui, nudge::STEP, nudge::BIG_STEP) {
                    let GradientLine { start, end } = value.line;
                    value.line = match part {
                        LinePart::Start => GradientLine { start: start + by, end },
                        LinePart::End => GradientLine { start, end: end + by },
                        _ => GradientLine { start: start + by, end: end + by },
                    };
                }
            }
            _ => {}
        }
        if value != shown {
            if let Some(selected) = selection_of(part) {
                sel.selected = Some(selected);
            }
            sel.picked = Some(part);
            gesture.set(value);
        }
    }
}
