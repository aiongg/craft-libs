//! The overlay's pointer: the press it takes, the drag of an end, the line, a stop or a midpoint,
//! the click that adds a stop, double-clicks, secondary clicks and the hover.

use egui::{Context, Id, Key, Modifiers, Pos2, Rect, Response, Vec2};
use kurbo::Point;

use super::{GradientLineStyle, GradientOverlay, GradientOverlayResponse, LineCursor};
use crate::ui::gradient::gradient_line::{self, MIN_MIDPOINT_GAP, at_screen, project, snap45};
use crate::ui::gradient::stops::{self, round_location};
use crate::ui::gradient::{GradientLine, LineGradient, LinePart};
use crate::ui::gradient::{Sampler, StopRef, StopRow};
use craft_ui::canvas::claim::PrimaryPress;
use craft_ui::canvas::{CanvasView, Claim};
use craft_ui::edit::order::Turn;
use craft_ui::edit::{CancelReason, Gesture, GestureId};

/// What a pass works with.
pub(super) struct Pass<'p> {
    pub ctx: &'p Context,
    pub id: Id,
    pub canvas: &'p Response,
    pub view: &'p CanvasView,
    pub style: &'p GradientLineStyle,
    pub sampler: &'p Sampler,
}

/// The selected stop or midpoint and the part the keys act on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Selection {
    pub selected: Option<StopRef>,
    pub picked: Option<LinePart>,
}

impl Selection {
    /// Selects `selected` and picks the part that shows it, so the keys act on what the pointer
    /// moved, added or selected. Nothing selected picks nothing.
    pub(super) fn select(&mut self, selected: Option<StopRef>) {
        self.selected = selected;
        self.picked = match selected {
            Some(StopRef::Color(i)) => Some(LinePart::Stop(i)),
            Some(StopRef::Midpoint(StopRow::Color, i)) => Some(LinePart::Midpoint(i)),
            Some(_) => self.picked,
            None => None,
        };
    }
}

/// A press the overlay owns, kept between passes.
#[derive(Clone, Debug)]
pub(super) struct Drag {
    pub press: PrimaryPress,
    /// The part the press took; a stop's or midpoint's index is into `base`.
    pub part: LinePart,
    /// The gradient the drag edits afresh in every pass: the host's at the press, or at the turn
    /// of a press that waited, or when a gesture the host ended carries the drag on.
    pub base: LineGradient,
    /// The modifiers at the press.
    mods: Modifiers,
    /// An Alt-drag of a stop: the drag moves a copy of it.
    copy: bool,
    /// The dragged stop is torn off far from the line; not an Alt copy.
    tears: bool,
    /// Past the click slop: the gesture began.
    pub began: bool,
    /// Esc, an outside change or a missed pass ended it: the rest of the press does nothing.
    pub cancelled: bool,
    /// The press waits for another control's typing run to end ([`Claim::turn`]); nothing of it
    /// has run.
    pub waits: bool,
    /// The selection and pick before the press, restored when the drag is cancelled.
    before: Selection,
    /// The gesture the drag's last update went out in; another one means the host ended it and
    /// the gesture carries the drag on.
    gesture: Option<GestureId>,
    /// The dragged stop is torn off the line.
    torn: bool,
}

impl Drag {
    fn new(press: PrimaryPress, part: LinePart, base: &LineGradient, mods: Modifiers, before: Selection) -> Self {
        Self {
            press,
            part,
            base: base.clone(),
            mods,
            copy: false,
            tears: true,
            began: false,
            cancelled: false,
            waits: false,
            before,
            gesture: None,
            torn: false,
        }
    }
}

pub(super) fn drag_key(id: Id) -> Id {
    id.with("craft_ui::canvas::GradientOverlay::drag")
}

pub(super) fn pick_key(id: Id) -> Id {
    id.with("craft_ui::canvas::GradientOverlay::picked")
}

pub(super) fn seen_key(id: Id) -> Id {
    id.with("craft_ui::canvas::GradientOverlay::seen")
}

pub(super) fn selection_of(part: LinePart) -> Option<StopRef> {
    match part {
        LinePart::Stop(i) => Some(StopRef::Color(i)),
        LinePart::Midpoint(i) => Some(StopRef::Midpoint(StopRow::Color, i)),
        LinePart::Line | LinePart::Start | LinePart::End => None,
    }
}

fn point(p: Pos2) -> Point {
    Point::new(p.x.into(), p.y.into())
}

impl GradientOverlay<'_> {
    /// The press the overlay holds in this pass: one taken earlier (a waiting one when its turn
    /// comes), else a new press on one of its parts.
    pub(super) fn press_of(&self, pass: &Pass<'_>, value: &LineGradient, editable: bool, sel: &mut Selection) -> Option<Drag> {
        let (ctx, canvas, id) = (pass.ctx, pass.canvas, pass.id);
        // A press whose release the overlay missed is dropped, its gesture with it.
        let held = ctx.data(|d| d.get_temp::<Drag>(drag_key(id))).filter(|d| d.press.holds(ctx) || (d.waits && Claim::is_owner(ctx, canvas.id, id)));
        match held {
            Some(d) if d.waits => match Claim::turn(ctx, canvas.id, id) {
                Turn::Waits => Some(d),
                Turn::Runs => Some(self.start(pass, value, d.press, d.mods, sel)),
                Turn::Now => None,
            },
            Some(d) => Some(d),
            None if editable => self.press(pass, value, sel),
            None => None,
        }
    }

    /// A new press on the canvas: the part under it, claimed for the overlay. A press with ⌘
    /// (Ctrl) is the host's.
    fn press(&self, pass: &Pass<'_>, value: &LineGradient, sel: &mut Selection) -> Option<Drag> {
        let ctx = pass.ctx;
        if !ctx.input(|i| i.pointer.primary_pressed()) {
            return None;
        }
        let press = PrimaryPress::current(ctx)?;
        let mods = ctx.input(|i| i.modifiers);
        if mods.command || !(pass.view.rect.contains(press.origin) && pass.canvas.contains_pointer()) {
            return None;
        }
        let part = self.hit(pass, value, press.origin)?;
        if !Claim::take(ctx, pass.canvas.id, pass.id) {
            return None;
        }
        if Claim::turn(ctx, pass.canvas.id, pass.id) == Turn::Waits {
            return Some(Drag { waits: true, ..Drag::new(press, part, value, mods, *sel) });
        }
        Some(self.start(pass, value, press, mods, sel))
    }

    /// The press `press` made with `mods`, against the host's gradient now: the part it takes and
    /// the selection it makes. A press that finds no part any more does nothing.
    fn start(&self, pass: &Pass<'_>, value: &LineGradient, press: PrimaryPress, mods: Modifiers, sel: &mut Selection) -> Drag {
        let before = *sel;
        let Some(part) = self.hit(pass, value, press.origin) else {
            return Drag { cancelled: true, ..Drag::new(press, LinePart::Line, value, mods, before) };
        };
        if let Some(s) = selection_of(part) {
            sel.selected = Some(s);
        }
        sel.picked = Some(part);
        let copy = mods.alt && self.alt_copies && matches!(part, LinePart::Stop(_));
        Drag { copy, tears: !copy, ..Drag::new(press, part, value, mods, before) }
    }

    /// One pass of a press the overlay owns: the press that goes on, if it does.
    pub(super) fn track(
        &self,
        pass: &Pass<'_>,
        mut d: Drag,
        gesture: &mut Gesture<LineGradient>,
        reported: Option<&LineGradient>,
        sel: &mut Selection,
    ) -> Option<Drag> {
        let ctx = pass.ctx;
        if d.began && !d.cancelled {
            if !gesture.is_active() {
                // Esc, `cancel_all`, an outside change or disabling ended it.
                d.cancelled = true;
                *sel = d.before;
            } else if gesture.id() != d.gesture
                && let Some(start) = gesture.start()
                && Some(start) != reported
            {
                // The host ended the drag for another control's change and the gesture carries
                // it on: it goes on from the host's gradient now.
                let start = start.clone();
                self.carry_on(&mut d, &start, gesture, sel);
            }
        }
        // Esc before the first move ends the press as it would end its drag.
        if !d.began && !d.cancelled && !d.waits && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            d.cancelled = true;
            *sel = d.before;
        }
        if d.waits {
            return Some(d);
        }
        let (pointer, down, mods) = ctx.input(|i| (i.pointer.latest_pos(), i.pointer.primary_down(), i.modifiers));
        if d.cancelled {
            return down.then_some(d);
        }
        if let Some(pointer) = pointer {
            if !d.began && pointer.distance(d.press.origin) > pass.style.click_slop {
                gesture.begin();
                d.began = gesture.is_active();
            }
            if d.began {
                let value = self.dragged(pass, &mut d, pointer, mods, sel);
                gesture.update(value, mods);
            }
        }
        d.gesture = gesture.id();
        if d.began
            && let Some(cursor) = &self.cursor
        {
            ctx.set_cursor_icon(cursor(if d.torn { LineCursor::TornOff } else { LineCursor::Drag(d.part) }));
        }
        if down {
            return Some(d);
        }
        if d.began {
            gesture.commit();
        } else if d.part == LinePart::Line {
            let (t, _) = self.located(pass.view, &d.base.line, d.press.origin);
            if let Some((stops, i)) = stops::add(&d.base.stops, StopRow::Color, round_location(t), pass.sampler, &self.rules) {
                sel.select(Some(StopRef::Color(i)));
                gesture.set(LineGradient { line: d.base.line, stops });
            }
        }
        None
    }

    /// Carries the drag on from the host's gradient `start`, which another control's change left.
    /// The dragged stop or midpoint is identified in it by the colour row's length and the stops'
    /// tags: a change that adds or removes a stop, or leaves a different stop (by tag) at the
    /// dragged one's place, ends the drag with `Cancel(External)`, unannounced. A stop is found
    /// by the selection, which follows it here and, shared, in a gradient editor whose change
    /// ended the drag; a midpoint by its index; an Alt copy goes on as a move of the copy; a stop
    /// torn off is not in the host's gradient, so the drag ends. The line's ends and the line
    /// itself need no identity.
    fn carry_on(&self, d: &mut Drag, start: &LineGradient, gesture: &mut Gesture<LineGradient>, sel: &mut Selection) {
        let (old, new) = (&d.base.stops.color, &start.stops.color);
        let found = match d.part {
            LinePart::Start | LinePart::End | LinePart::Line => Some(d.part),
            _ if old.len() != new.len() => None,
            LinePart::Stop(i) => match sel.selected {
                Some(StopRef::Color(k)) if k < new.len() && (d.copy || stops::same_stop(old, i, new, k)) => Some(LinePart::Stop(k)),
                _ => None,
            },
            LinePart::Midpoint(i) => (stops::same_stop(old, i, new, i) && stops::same_stop(old, i + 1, new, i + 1)).then_some(d.part),
        };
        match found {
            Some(part) => {
                d.base = start.clone();
                d.part = part;
                if matches!(part, LinePart::Stop(_)) {
                    d.copy = false;
                    if let LinePart::Stop(k) = part {
                        sel.select(Some(StopRef::Color(k)));
                    }
                }
            }
            None => {
                gesture.cancel(CancelReason::External);
                d.cancelled = true;
            }
        }
    }

    /// The drag's gradient with its part at `pointer`, and the selection that follows a stop.
    fn dragged(&self, pass: &Pass<'_>, d: &mut Drag, pointer: Pos2, mods: Modifiers, sel: &mut Selection) -> LineGradient {
        let view = pass.view;
        let snapped = |part: LinePart, p: Point| self.snap.as_ref().map_or(p, |snap| snap(part, p, mods));
        let base = &d.base;
        let mut out = base.clone();
        d.torn = false;
        match d.part {
            LinePart::Start | LinePart::End => {
                let (fixed, moved) = match d.part {
                    LinePart::Start => (base.line.end, &mut out.line.start),
                    _ => (base.line.start, &mut out.line.end),
                };
                let to = view.to_doc(pointer);
                let to = if mods.shift { snap45(fixed, to, self.angle_base) } else { to };
                *moved = snapped(d.part, to);
            }
            LinePart::Line => {
                let by = view.to_doc(pointer) - view.to_doc(d.press.origin);
                let by = snapped(LinePart::Line, base.line.start + by) - base.line.start;
                out.line = GradientLine { start: base.line.start + by, end: base.line.end + by };
            }
            LinePart::Stop(i) => {
                let (t, off) = self.located(view, &base.line, pointer);
                let at = round_location(t);
                // At the row's minimum a stop dragged off the line stays and moves.
                let torn = (d.tears && off > pass.style.tear_off).then(|| stops::remove(&base.stops, StopRow::Color, i, &self.rules)).flatten();
                if let Some((stops, _)) = torn {
                    out.stops = stops;
                    sel.select(None);
                    d.torn = true;
                } else {
                    let edited = match d.copy {
                        true => stops::copy(&base.stops, StopRow::Color, i, at, &self.rules),
                        false => stops::move_to(&base.stops, StopRow::Color, i, at, &self.rules),
                    };
                    let (stops, j) = edited.unwrap_or_else(|| (base.stops.clone(), i));
                    out.stops = stops;
                    sel.select(Some(StopRef::Color(j)));
                }
            }
            LinePart::Midpoint(i) => {
                let (t, _) = self.located(view, &base.line, pointer);
                if let (Some(a), Some(b)) = (base.stops.color.get(i), base.stops.color.get(i + 1))
                    && b.location - a.location > MIN_MIDPOINT_GAP
                    && let Some((stops, _)) =
                        stops::set_midpoint(&base.stops, StopRow::Color, i, (t - a.location) / (b.location - a.location), &self.rules)
                {
                    out.stops = stops;
                }
            }
        }
        out
    }

    /// Double-clicks and secondary clicks on the parts of `shown`.
    pub(super) fn clicks(&self, pass: &Pass<'_>, shown: &LineGradient, sel: &mut Selection, out: &mut GradientOverlayResponse) {
        let canvas = pass.canvas;
        if canvas.secondary_clicked()
            && let Some(part) = canvas.interact_pointer_pos().and_then(|p| self.hit(pass, shown, p))
        {
            if let Some(s) = selection_of(part) {
                sel.selected = Some(s);
            }
            sel.picked = Some(part);
            out.context = Some(part);
        }
        if !canvas.double_clicked() || !Claim::is_owner(pass.ctx, canvas.id, pass.id) {
            return;
        }
        let part = canvas.interact_pointer_pos().and_then(|p| self.hit(pass, shown, p));
        out.color_requested = part.and_then(|part| self.color_request(pass, shown, part));
    }

    /// The colour a part asks for: a colour stop's, or the one at the end handle (stop 0 or the
    /// last, swapped under `.reverse`, PC `click`), with the screen rect of the stop's marker.
    pub(super) fn color_request(&self, pass: &Pass<'_>, shown: &LineGradient, part: LinePart) -> Option<(usize, Rect)> {
        let len = shown.stops.color.len();
        let i = match part {
            LinePart::Stop(i) => i,
            LinePart::Start | LinePart::End if len > 0 => {
                if (part == LinePart::Start) != self.reverse {
                    0
                } else {
                    len - 1
                }
            }
            _ => return None,
        };
        let (start, end) = (pass.view.to_screen(shown.line.start), pass.view.to_screen(shown.line.end));
        let at = at_screen(start, end, shown.stops.color.get(i)?.location, self.reverse);
        let size = 2.0 * (pass.style.stop + pass.style.stop_ring.width);
        Some((i, Rect::from_center_size(at, Vec2::splat(size))))
    }

    /// The part a press at the pointer would take, with the host's cursor over it.
    pub(super) fn hover(&self, pass: &Pass<'_>, shown: &LineGradient) -> Option<LinePart> {
        let (ctx, canvas) = (pass.ctx, pass.canvas);
        if Claim::owner(ctx, canvas.id).is_some_and(|owner| owner != pass.id) || ctx.input(|i| i.modifiers.command) {
            return None;
        }
        let part = canvas.hover_pos().and_then(|p| self.hit(pass, shown, p))?;
        if let Some(cursor) = &self.cursor {
            ctx.set_cursor_icon(cursor(LineCursor::Hover(part)));
        }
        Some(part)
    }

    /// The part of `value` under screen point `p`.
    fn hit(&self, pass: &Pass<'_>, value: &LineGradient, p: Pos2) -> Option<LinePart> {
        let (start, end) = (pass.view.to_screen(value.line.start), pass.view.to_screen(value.line.end));
        gradient_line::hit(start, end, &value.stops, self.reverse, self.midpoints, p, pass.style.hit)
    }

    /// The location on `line` nearest screen point `p`, and the screen distance from it. Measured
    /// on the screen, so a stretched view tears off at the distance it shows.
    fn located(&self, view: &CanvasView, line: &GradientLine, p: Pos2) -> (f32, f32) {
        let screen = GradientLine { start: point(view.to_screen(line.start)), end: point(view.to_screen(line.end)) };
        let (t, distance) = project(&screen, point(p), self.reverse);
        (t, distance as f32)
    }
}
