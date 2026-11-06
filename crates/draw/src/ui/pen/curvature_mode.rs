//! The pen draft's Curvature mode ([`PenMode::Curvature`](super::PenMode::Curvature)):
//! the Curvature Pen while a path is being drawn.

use egui::{Context, Modifiers, Response, Shape, Stroke, Ui, vec2};
use kurbo::{BezPath, PathEl};

use super::{EditSubpath, Knot, PenDraft, PenResponse, PenStyle, drag_key, outline};
use craft_ui::canvas::claim::PrimaryPress;
use craft_ui::canvas::{CanvasView, Claim, PartFocus, hit};
use craft_ui::edit::order::Turn;
use craft_ui::edit::{EditPolicy, Gesture};
use craft_ui::primitives::{Handle, HandleShape};
use craft_ui::theme::{Tokens, TokensOf};

/// A press the Curvature mode owns: it moves the anchor `index` once past the click slop.
#[derive(Clone, Debug)]
struct Grab {
    press: PrimaryPress,
    index: usize,
    /// Past the click slop.
    moved: bool,
    /// The press waits for another control's typing run to end ([`Claim::turn`]), with the
    /// modifiers at the press.
    waits: Option<Modifiers>,
}

impl PenDraft<'_> {
    /// One pass in Curvature mode, with the Pen's id, gesture, claim, waiting press
    /// ([`Claim::turn`](craft_ui::canvas::Claim::turn)), keys, AccessKit parts and response as
    /// [`show`](PenDraft::show) has them (share its private helpers rather than copy them):
    ///
    /// - Press on the first anchor of a draft with 3 or more anchors: closes at once (`closed`
    ///   set, Commit, [`finished`](PenResponse::finished)), as PhotoCraft and Photoshop; the Pen
    ///   closes on release instead (lead call 8).
    /// - Press on another anchor (within the hit): grabs it; past the click slop it
    ///   follows the pointer and the draft reflows on every move ([`EditSubpath::reflow`](super::EditSubpath::reflow)).
    /// - Press elsewhere: a new smooth anchor at the press (an Update); it follows the pointer
    ///   while the button is down, past the click slop (lead call 9: PhotoCraft moves it on
    ///   any move). The first two anchors give straight segments; from three on the handles
    ///   follow [`curvature_handles`](crate::geometry::curvature::curvature_handles).
    /// - The second click of a double-click ([`PrimaryPress::clicks`](craft_ui::canvas::claim::PrimaryPress::clicks)
    ///   == 2) on an anchor toggles it between smooth and corner and reflows; on empty canvas the
    ///   first click's anchor is under it, so a double-click places a corner point.
    /// - ⇧ and ⌥ change nothing; no resume from an open path's end (`ends` is ignored).
    /// - Keys: Enter and Esc end the draft as the Pen's; Backspace, Delete and
    ///   [`remove_last`](PenDraft::remove_last) remove the last anchor and reflow.
    /// - Look: the draft line, the anchors (squares, lead call 7; the last filled), no handles;
    ///   the hover preview: with 2 or more anchors the last segment, dashed (the line,
    ///   the rubber dash, the rubber gap), of the curve `curvature_handles` makes
    ///   through the anchors plus the pointer, from the last anchor to the pointer; with one
    ///   anchor a straight dashed line; none while a press is down; the close cue over the first
    ///   anchor with 3 or more anchors.
    pub(super) fn show_curvature(self, ui: &mut Ui, canvas: &Response, view: &CanvasView) -> PenResponse {
        let ctx = ui.ctx().clone();
        let id = ui.make_persistent_id(self.id_salt);
        let style = self.look.unwrap_or_else(|| PenDraft::style(&Tokens::of(ui)));
        let editing = ui.is_enabled();
        let host = self.draft.cloned().unwrap_or_default();
        let mut gesture = Gesture::load(ui, id, &host, EditPolicy::default().cancellable(false)).commit_on_interrupt(true).enabled(editing);
        if !gesture.is_active() && !host.knots.is_empty() {
            gesture.begin();
        }
        let mut out = PenResponse { id, event: None, finished: false, resumed: None, claimed: false, closes: false };

        let mut grab: Option<Grab> = ctx
            .data(|d| d.get_temp(drag_key(id)))
            .filter(|g: &Grab| g.press.holds(&ctx) || (g.waits.is_some() && Claim::is_owner(&ctx, canvas.id, id)));
        let waited = grab.as_ref().is_some_and(|g| g.waits.is_some());
        if let Some(g) = &grab
            && let Some(mods) = g.waits
        {
            grab = match Claim::turn(&ctx, canvas.id, id) {
                Turn::Waits => grab,
                Turn::Runs => self.curve_start(&ctx, view, &style, &mut gesture, &mut out, g.press, mods),
                Turn::Now => None,
            };
        }
        if grab.is_none() && !Claim::keeps_waiting(&ctx, canvas.id, id, waited) && editing && self.sensing {
            grab = self.curve_press(&ctx, canvas, view, &style, &mut gesture, &mut out);
        }
        if let Some(g) = grab.take() {
            grab = if g.waits.is_some() { Some(g) } else { self.curve_track(&ctx, view, &style, g, &mut gesture) };
        }
        if editing && grab.is_none() {
            let before = gesture.shown().knots.len();
            out.finished |= self.keys(ui, self.remove_last, &mut gesture);
            // Removing the last anchor changes the curve through the rest.
            if !out.finished && gesture.shown().knots.len() < before {
                let mut draft = gesture.shown().clone();
                draft.reflow();
                gesture.update(draft, ui.input(|i| i.modifiers));
            }
        }

        let draft = gesture.shown().clone();
        out.closes = self.closes_now(ui, canvas, view, &draft, &style);
        let parts = self.parts(&draft, view, &style);
        PartFocus::new(id, &self.names.group).parts(&parts).focusable(false).picked(parts.last().map(|p| p.key)).show(ui, canvas);

        out.event = gesture.finish();
        out.claimed = Claim::is_owner(&ctx, canvas.id, id);
        ctx.data_mut(|d| match &grab {
            Some(grab) => {
                d.insert_temp(drag_key(id), grab.clone());
            }
            None => d.remove::<Grab>(drag_key(id)),
        });
        self.curve_paint(ui, canvas, view, &style, &draft, grab.is_some());
        out
    }

    /// A press on the canvas, taken through the claim.
    fn curve_press(
        &self,
        ctx: &Context,
        canvas: &Response,
        view: &CanvasView,
        style: &PenStyle,
        gesture: &mut Gesture<EditSubpath>,
        out: &mut PenResponse,
    ) -> Option<Grab> {
        if !ctx.input(|i| i.pointer.primary_pressed()) {
            return None;
        }
        let press = PrimaryPress::current(ctx)?;
        if !(view.rect.contains(press.origin) && canvas.contains_pointer()) {
            return None;
        }
        if !Claim::take(ctx, canvas.id, out.id) {
            return None;
        }
        let mods = ctx.input(|i| i.modifiers);
        if Claim::turn(ctx, canvas.id, out.id) == Turn::Waits {
            return Some(Grab { press, index: 0, moved: false, waits: Some(mods) });
        }
        self.curve_start(ctx, view, style, gesture, out, press, mods)
    }

    /// The press `press` against the draft now: closes it, toggles or grabs the anchor under it,
    /// or adds a smooth anchor.
    #[expect(clippy::too_many_arguments, reason = "the Pen's `start` and this share one call shape")]
    fn curve_start(
        &self,
        ctx: &Context,
        view: &CanvasView,
        style: &PenStyle,
        gesture: &mut Gesture<EditSubpath>,
        out: &mut PenResponse,
        press: PrimaryPress,
        mods: Modifiers,
    ) -> Option<Grab> {
        let mut draft = gesture.shown().clone();
        let n = draft.knots.len();
        let clicks = press.clicks(ctx);
        let under = draft
            .knots
            .iter()
            .enumerate()
            .map(|(i, k)| (i, hit::point(press.origin, view.to_screen(k.anchor))))
            .filter(|&(_, d)| d <= style.hit)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i);
        if under == Some(0) && n >= 3 && !draft.closed {
            draft.closed = true;
            draft.reflow();
            gesture.begin();
            gesture.update(draft, mods);
            gesture.commit();
            out.finished = true;
            return None;
        }
        let index = match under {
            Some(i) => {
                if clicks == 2 {
                    let k = &mut draft.knots[i];
                    k.smooth = !k.smooth;
                    if !k.smooth {
                        k.in_ctrl = k.anchor;
                        k.out_ctrl = k.anchor;
                    }
                }
                i
            }
            None => {
                draft.knots.push(Knot { smooth: true, ..Knot::corner(view.to_doc(press.origin)) });
                n
            }
        };
        draft.reflow();
        gesture.begin();
        gesture.update(draft, mods);
        Some(Grab { press, index, moved: false, waits: None })
    }

    /// One pass of a press the mode owns: it goes on while the button is down; once past the
    /// click slop the anchor follows the pointer and the draft reflows.
    fn curve_track(&self, ctx: &Context, view: &CanvasView, style: &PenStyle, mut g: Grab, gesture: &mut Gesture<EditSubpath>) -> Option<Grab> {
        let (pointer, down, mods) = ctx.input(|i| (i.pointer.latest_pos(), i.pointer.primary_down(), i.modifiers));
        if let Some(pointer) = pointer {
            g.moved |= pointer.distance(g.press.origin) > style.click_slop;
            if g.moved {
                let mut draft = gesture.shown().clone();
                if let Some(k) = draft.knots.get_mut(g.index) {
                    let by = view.to_doc(pointer) - k.anchor;
                    k.anchor += by;
                    k.in_ctrl += by;
                    k.out_ctrl += by;
                    draft.reflow();
                    gesture.update(draft, mods);
                }
            }
        }
        down.then_some(g)
    }

    /// The draft line, the hover preview, the square anchors (the last filled) and the close cue.
    fn curve_paint(&self, ui: &Ui, canvas: &Response, view: &CanvasView, style: &PenStyle, draft: &EditSubpath, pressing: bool) {
        let n = draft.knots.len();
        if n == 0 {
            return;
        }
        let painter = ui.painter_at(view.rect);
        let ppp = ui.pixels_per_point();
        let to_screen = view.affine();
        let pointer = ui.input(|i| i.pointer.latest_pos()).filter(|p| view.rect.contains(*p) && canvas.contains_pointer() && self.sensing);
        outline(&painter, to_screen * draft.bez_path(), f64::from(style.flatten), Stroke::new(style.draft_line, style.color));
        if let Some(pointer) = pointer
            && !pressing
            && !draft.closed
        {
            // The segment the next click would add: the curve through the anchors and the pointer.
            let mut next = draft.clone();
            next.knots.push(Knot { smooth: true, ..Knot::corner(view.to_doc(pointer)) });
            next.reflow();
            if let [.., a, b] = next.knots.as_slice() {
                let mut segment = BezPath::new();
                segment.move_to(a.anchor);
                segment.curve_to(a.out_ctrl, b.in_ctrl, b.anchor);
                let mut line: Vec<egui::Pos2> = Vec::new();
                kurbo::flatten(to_screen * segment, f64::from(style.flatten), |el| match el {
                    PathEl::MoveTo(p) | PathEl::LineTo(p) => line.push(egui::pos2(p.x as f32, p.y as f32)),
                    _ => {}
                });
                painter.extend(Shape::dashed_line(&line, Stroke::new(style.rubber_width, style.color), style.rubber_dash, style.rubber_gap));
            }
        }
        for (i, k) in draft.knots.iter().enumerate() {
            let look = if i + 1 == n { &style.selected_anchor } else { &style.anchor };
            painter.extend(Handle::shapes_with(look, HandleShape::Square, view.to_screen(k.anchor), ppp));
        }
        if let Some(pointer) = pointer
            && n >= 3
            && !draft.closed
            && hit::point(pointer, view.to_screen(draft.knots[0].anchor)) <= style.hit
        {
            let first = view.to_screen(draft.knots[0].anchor);
            painter.extend(Handle::shapes_with(&style.close_anchor, HandleShape::Square, first, ppp));
            let ring = pointer + vec2(style.close_offset, style.close_offset);
            painter.circle_stroke(ring, style.close_ring, Stroke::new(style.draft_line, style.color));
        }
    }
}
