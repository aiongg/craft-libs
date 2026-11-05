//! [`CornerWidgets`]: the round widgets inside the corners of the host's shapes.

use egui::{AsIdSalt, Context, Id, IdSalt, Key, Modifiers, Rect, Response, Stroke, Ui, Vec2};

use super::geometry::{self, Widget};
use super::{CornerEdit, CornerKind, CornerRef, CornerStyle, CornerTarget, CornerWidgetsLabels};
use crate::geometry::fillet::fillet;
use craft_ui::behavior::nudge;
use craft_ui::canvas::claim::PrimaryPress;
use craft_ui::canvas::{CanvasView, Claim, Part, PartFocus, Readout, RulerUnits};
use craft_ui::edit::order::Turn;
use craft_ui::edit::{EditEvent, EditPolicy, Gesture};
use craft_ui::localized::Labels as _;
use craft_ui::theme::{Tokens, TokensOf as _};

/// What a pass of [`CornerWidgets::show`] reports.
#[derive(Clone, Debug, PartialEq)]
#[must_use = "apply the corner edit and answer cycle_kind and activate"]
pub struct CornerWidgetsResponse {
    /// The overlay's id, also its Tab stop's.
    pub id: Id,
    /// The radii: a drag reports Begin past `corner.click_slop`, Update every move, Commit on
    /// release (Esc cancels, the rest of the press does nothing), each value every changed
    /// target's radii from the press; an arrow key a Set. The host makes one history step for
    /// every target in the value.
    pub event: Option<EditEvent<CornerEdit>>,
    /// ⌥-click without drag on a widget, with [`kinds`](CornerWidgets::kinds) on: the corner and
    /// the kind it turns into ([`CornerKind::next`] of its kind). The host applies it to the
    /// corners shown with it.
    pub cycle_kind: Option<(CornerRef, CornerKind)>,
    /// A double-click on a widget, or Enter on the picked one: the host opens its Corners dialog
    /// for that corner (S7-shapes).
    pub activate: Option<CornerRef>,
    /// The pointer is over a widget or a drag is under way: the host shows its corner-radius
    /// cursor.
    pub hovered: bool,
    /// The widgets own the current press: the host's tool leaves it alone.
    pub claimed: bool,
}

/// One round widget inside each corner of the host's shapes, dragged to round them (PhotoCraft's
/// and Photoshop's live corner widgets, VectorCraft's Live Corners).
///
/// ```no_run
/// use craft_draw::ui::corner::{CornerTarget, CornerWidgets};
/// use craft_ui::canvas::{CanvasView, RulerUnits};
/// # fn demo(ui: &mut egui::Ui, canvas: &egui::Response, view: &CanvasView, targets: &[CornerTarget], px: &RulerUnits) {
/// let r = CornerWidgets::new("corners", targets, px).show(ui, canvas, view);
/// if let Some(edit) = r.event.as_ref().and_then(|e| e.value()) {
///     // preview the outlines with `edit`; on Commit, one history step for every target in it
/// }
/// # }
/// ```
///
/// - Look: per shown corner a circle `corner.widget` across in `overlay.handle_fill`, outlined
///   `corner.line` in `overlay.line`, with a `corner.dot` dot in `overlay.line`, at the centre of
///   the corner's arc and at least `corner.inset` screen points inside the corner. Hidden when a
///   side is shorter than `corner.min_side` screen points or the corner's angle is wider than
///   [`hide_above`](Self::hide_above).
/// - Pointer: a press within `corner.hit` screen points of a widget takes it (nearest first, of
///   equal distances the first target's; list the topmost shape first). A drag sets the radius
///   at the press plus the move projected on the corner's centre step; every target takes that
///   radius, each capped at its own maximum (lead call 23); ⌥ changes only the dragged corner of
///   `individual` targets. A click without movement changes nothing. While dragging the readout
///   "R: 25.0 px" (the canvas `Readout`, in the [ruler units](Self::new)) follows the pointer;
///   the host previews the outlines from the event values. ⌥-click reports
///   [`cycle_kind`](CornerWidgetsResponse::cycle_kind) with [`kinds`](Self::kinds); a
///   double-click (`PrimaryPress::clicks`) reports [`activate`](CornerWidgetsResponse::activate).
///   A press with ⌘ is not taken.
/// - Keyboard: one Tab stop (`PartFocus`). The host's `canvas` response must not be focusable
///   (`Sense::click_and_drag() - Sense::FOCUSABLE`): a focusable canvas takes the Tab stop
///   itself and the group never gets keyboard focus. Page Down and Page Up pick the next and previous
///   widget; the arrows change the radius of the picked widget's corner on every target by
///   [`steps`](Self::steps) (1, ⇧ 10; up and right grow it), ⌥ that corner only; Enter reports
///   `activate`; Esc leaves the group.
/// - AccessKit: a group named [`CornerWidgetsLabels::group`] with one object per widget named
///   "Corner i of n", valued "radius 12 px". While the Tab stop has focus, AccessKit reports the
///   picked widget's object as the focused node (the group's active descendant).
/// - Geometry is in document space and does not mirror right to left.
pub struct CornerWidgets<'a> {
    id_salt: IdSalt,
    targets: &'a [CornerTarget],
    units: &'a RulerUnits,
    kinds: bool,
    hide_above: f64,
    steps: (f64, f64),
    labels: Option<CornerWidgetsLabels>,
    look: Option<CornerStyle>,
    policy: EditPolicy,
}

impl<'a> CornerWidgets<'a> {
    /// The widgets of the host's `targets`, topmost first, with radii read out in `units`.
    pub fn new(id_salt: impl AsIdSalt, targets: &'a [CornerTarget], units: &'a RulerUnits) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            targets,
            units,
            kinds: false,
            hide_above: 180.0,
            steps: (nudge::STEP, nudge::BIG_STEP),
            labels: None,
            look: None,
            policy: EditPolicy::default(),
        }
    }

    /// Whether ⌥-click cycles the corner kind (VectorCraft); off by default (lead call 24).
    pub fn kinds(mut self, kinds: bool) -> Self {
        self.kinds = kinds;
        self
    }

    /// The widest inner angle, in degrees, whose corner shows a widget; 180 by default
    /// (VectorCraft's preference passes 177).
    pub fn hide_above(mut self, degrees: f64) -> Self {
        self.hide_above = degrees;
        self
    }

    /// The arrow keys' radius change in document units, plain and with ⇧ (1 and 10, lead call 25).
    pub fn steps(mut self, step: f64, big_step: f64) -> Self {
        self.steps = (step, big_step);
        self
    }

    /// The words; the installed ones by default.
    pub fn labels(mut self, labels: CornerWidgetsLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// A look of the host's in place of [`style`](Self::style)'s.
    pub fn look(mut self, style: CornerStyle) -> Self {
        self.look = Some(style);
        self
    }

    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Draws the widgets over `view` and senses through the host's `canvas` response. The host
    /// shows them before its tool input, as an overlay that claims only presses on a widget.
    pub fn show(self, ui: &mut Ui, canvas: &Response, view: &CanvasView) -> CornerWidgetsResponse {
        // pc: crates/ui-egui/src/tools/corner_radius.rs:229-297 `pointer` (⌘ not taken :233; drag
        // :246-270; one step on release :271-295), :299-330 `draw` (readout "R:" with `{r:.1} px`
        // :313), :225-227 `hovers`. vectorcraft: crates/tools/src/corners.rs:233 (⌥-click cycles).
        let ctx = ui.ctx().clone();
        let id = ui.make_persistent_id(self.id_salt);
        let tokens = Tokens::of(ui);
        let style = self.look.unwrap_or_else(|| CornerWidgets::style(&tokens));
        let labels = self.labels.clone().unwrap_or_else(|| CornerWidgetsLabels::of(ui));
        let editing = ui.is_enabled();
        let widgets = geometry::widgets(self.targets, view, style.inset, style.min_side, self.hide_above);
        let mut out = CornerWidgetsResponse { id, event: None, cycle_kind: None, activate: None, hovered: false, claimed: false };

        // The host applies the values it receives, so the gesture's own host value stays empty
        // (the echo rule reads that as the host's write-back).
        let mut gesture = Gesture::load(ui, id, &CornerEdit::default(), self.policy).enabled(editing);

        let mut grab: Option<Grab> = ctx
            .data(|d| d.get_temp(grab_key(id)))
            .filter(|g: &Grab| g.press.holds(&ctx) || (g.waits.is_some() && Claim::is_owner(&ctx, canvas.id, id)));
        if let Some(g) = &mut grab
            && g.moved
            && !gesture.is_active()
        {
            // Esc (or another cancel) ended the drag: the rest of the press does nothing.
            g.dead = true;
        }
        let waited = grab.as_ref().is_some_and(|g| g.waits.is_some());
        if let Some(g) = &grab
            && g.waits.is_some()
        {
            grab = match Claim::turn(&ctx, canvas.id, id) {
                Turn::Waits => grab,
                Turn::Runs => self.start(&widgets, g.press, g.corner),
                Turn::Now => None,
            };
        }
        if grab.is_none() && !Claim::keeps_waiting(&ctx, canvas.id, id, waited) && editing {
            grab = self.press(&ctx, canvas, view, &widgets, &style, id);
        }
        if let Some(g) = grab.take() {
            grab = if g.waits.is_some() { Some(g) } else { self.track(&ctx, view, style.click_slop, g, &mut gesture, &mut out) };
        }
        if let Some(g) = &grab {
            ctx.data_mut(|d| d.insert_temp(picked_key(id), Some(g.corner)));
        }

        let picked = ctx.data(|d| d.get_temp::<Option<CornerRef>>(picked_key(id))).flatten();
        let picked_at = picked.and_then(|c| widgets.iter().position(|w| w.corner == c));
        let parts = self.parts(&widgets, &style, &labels);
        let focus = PartFocus::new(id, &labels.group).parts(&parts).picked(picked_at.map(|i| i as u64)).show(ui, canvas);
        let picked_at = match focus.pick {
            Some(Some(key)) => usize::try_from(key).ok().filter(|i| *i < widgets.len()),
            Some(None) => None,
            None => picked_at,
        };
        let picked = picked_at.map(|i| widgets[i].corner);
        ctx.data_mut(|d| d.insert_temp(picked_key(id), picked));
        if editing && focus.focused && grab.is_none() {
            self.keys(ui, &widgets, picked_at, &mut gesture, &mut out);
        }

        let pointer = ui.input(|i| i.pointer.latest_pos());
        let over = pointer.is_some_and(|p| view.rect.contains(p) && canvas.contains_pointer() && geometry::hit(&widgets, p, style.hit).is_some());
        out.hovered = over && editing || grab.as_ref().is_some_and(|g| !g.dead);
        self.paint(ui, view, &style, &widgets, focus.keyboard.then_some(picked_at).flatten());
        if let (Some(g), Some(pointer)) = (&grab, pointer)
            && g.moved
            && !g.dead
        {
            let radius = gesture
                .shown()
                .targets
                .iter()
                .find(|t| t.key == g.corner.target)
                .and_then(|t| t.radii.get(g.corner.corner).copied())
                .unwrap_or(g.from);
            Readout::new(("craft_draw::ui::corner::CornerWidgets::readout", id))
                .row(labels.readout.clone(), self.units.format_value(radius, 0.0))
                .show(&ctx, pointer);
        }

        out.event = gesture.finish();
        out.claimed = Claim::is_owner(&ctx, canvas.id, id);
        ctx.data_mut(|d| match &grab {
            Some(grab) => {
                d.insert_temp(grab_key(id), grab.clone());
            }
            None => d.remove::<Grab>(grab_key(id)),
        });
        out
    }

    /// A press on the canvas within reach of a widget, taken through the claim.
    fn press(&self, ctx: &Context, canvas: &Response, view: &CanvasView, widgets: &[Widget], style: &CornerStyle, id: Id) -> Option<Grab> {
        if !ctx.input(|i| i.pointer.primary_pressed()) {
            return None;
        }
        let press = PrimaryPress::current(ctx)?;
        if !(view.rect.contains(press.origin) && canvas.contains_pointer()) || ctx.input(|i| i.modifiers.command) {
            return None;
        }
        let corner = widgets[geometry::hit(widgets, press.origin, style.hit)?].corner;
        if !Claim::take(ctx, canvas.id, id) {
            return None;
        }
        let mods = ctx.input(|i| i.modifiers);
        if Claim::turn(ctx, canvas.id, id) == Turn::Waits {
            return Some(Grab { press, corner, from: 0.0, targets: Vec::new(), moved: false, dead: false, waits: Some(mods) });
        }
        self.start(widgets, press, corner)
    }

    /// The press `press` on the widget of `corner`, with the state of its first pass.
    fn start(&self, widgets: &[Widget], press: PrimaryPress, corner: CornerRef) -> Option<Grab> {
        let from = widgets.iter().find(|w| w.corner == corner)?.radius;
        Some(Grab { press, corner, from, targets: self.targets.to_vec(), moved: false, dead: false, waits: None })
    }

    /// One pass of a press the widgets own: past the click slop it drags, on the release it
    /// commits the drag or, without movement, reports the click.
    fn track(
        &self,
        ctx: &Context,
        view: &CanvasView,
        slop: f32,
        mut g: Grab,
        gesture: &mut Gesture<CornerEdit>,
        out: &mut CornerWidgetsResponse,
    ) -> Option<Grab> {
        let (pointer, down, mods) = ctx.input(|i| (i.pointer.latest_pos(), i.pointer.primary_down(), i.modifiers));
        if g.dead {
            return down.then_some(g);
        }
        if let Some(pointer) = pointer {
            g.moved |= pointer.distance(g.press.origin) > slop;
            if g.moved {
                // The step is the corner's at the press: the snapshot's.
                let step = g
                    .targets
                    .iter()
                    .find(|t| t.key == g.corner.target)
                    .and_then(|t| t.corners.get(g.corner.corner))
                    .and_then(|c| fillet(c.prev, c.vertex, c.next, 1.0))
                    .map(|f| f.step());
                if let Some(step) = step {
                    let radius = geometry::dragged_radius(g.from, step, view.to_doc(g.press.origin), view.to_doc(pointer));
                    let edit = geometry::drag_edit(&g.targets, g.corner, radius, mods.alt);
                    gesture.begin();
                    gesture.update(edit, mods);
                }
            }
        }
        if down {
            return Some(g);
        }
        if g.moved {
            gesture.commit();
        } else if mods.alt && self.kinds {
            let kind = self.targets.iter().find(|t| t.key == g.corner.target).and_then(|t| t.corners.get(g.corner.corner)).map(|c| c.kind);
            out.cycle_kind = kind.map(|k| (g.corner, k.next()));
        } else if g.press.clicks(ctx) >= 2 {
            out.activate = Some(g.corner);
        }
        None
    }

    /// Page Down and Page Up are the group's; here Enter, Esc and the arrows.
    fn keys(&self, ui: &mut Ui, widgets: &[Widget], picked: Option<usize>, gesture: &mut Gesture<CornerEdit>, out: &mut CornerWidgetsResponse) {
        let Some(w) = picked.and_then(|i| widgets.get(i)) else { return };
        if !nudge::reads(ui) {
            return;
        }
        let (enter, escape) = ui.input_mut(|i| (i.consume_key(Modifiers::NONE, Key::Enter), i.consume_key(Modifiers::NONE, Key::Escape)));
        if enter {
            out.activate = Some(w.corner);
        }
        if escape {
            let id = out.id;
            ui.memory_mut(|m| m.surrender_focus(id));
        }
        if let Some((by, alt)) = nudge::read_with_alt(ui, self.steps.0, self.steps.1) {
            // Up and right grow the radius.
            let radius = (w.radius + by.x - by.y).max(0.0);
            gesture.set(geometry::drag_edit(self.targets, w.corner, radius, alt));
        }
    }

    /// The AccessKit parts: one per widget, named "Corner i of n", valued with the radius.
    fn parts(&self, widgets: &[Widget], style: &CornerStyle, labels: &CornerWidgetsLabels) -> Vec<Part> {
        let n = widgets.len().to_string();
        widgets
            .iter()
            .enumerate()
            .map(|(i, w)| Part {
                key: i as u64,
                name: craft_i18n::fmt(&labels.corner, &[("i", &(i + 1).to_string()), ("n", &n)]),
                value: Some(craft_i18n::fmt(&labels.value, &[("radius", &self.units.format(w.radius, 0.0, 0..=self.units.decimals.max(1)))])),
                rect: Rect::from_center_size(w.at, Vec2::splat(2.0 * style.hit)),
                selected: false,
            })
            .collect()
    }

    fn paint(&self, ui: &Ui, view: &CanvasView, style: &CornerStyle, widgets: &[Widget], ring: Option<usize>) {
        let painter = ui.painter_at(view.rect);
        for w in widgets {
            painter.circle(w.at, style.widget / 2.0, style.fill, Stroke::new(style.line, style.color));
            painter.circle_filled(w.at, style.dot / 2.0, style.color);
        }
        if let Some(w) = ring.and_then(|i| widgets.get(i)) {
            PartFocus::ring(&painter, Rect::from_center_size(w.at, Vec2::splat(style.widget)));
        }
    }
}

/// A press the widgets own: the widget it began on, the radius there and the targets as they
/// were, so a host that applies the values live still gets every changed target's radii from
/// the press.
#[derive(Clone, Debug)]
struct Grab {
    press: PrimaryPress,
    corner: CornerRef,
    from: f64,
    targets: Vec<CornerTarget>,
    /// Past the click slop.
    moved: bool,
    /// The drag was cancelled: the rest of the press does nothing.
    dead: bool,
    /// The press waits for another control's typing run to end ([`Claim::turn`]), with the
    /// modifiers at the press.
    waits: Option<Modifiers>,
}

fn grab_key(id: Id) -> Id {
    id.with("craft_draw::ui::corner::CornerWidgets::grab")
}

fn picked_key(id: Id) -> Id {
    id.with("craft_draw::ui::corner::CornerWidgets::picked")
}
