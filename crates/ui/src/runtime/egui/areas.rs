//! `Area` with its left-top pivot, `Order`, sublayers, transform layers and `set_layer_order`.

#[cfg(feature = "testing")]
use egui::Ui;
use egui::emath::TSTransform;
use egui::{Color32, Context, Id, LayerId, Order, Painter, Pos2, Rect, Sense, Vec2};

/// Shows an `egui::Area` named `name` and runs `body` on its `Ui`.
#[cfg(feature = "testing")]
pub(crate) fn show_area<R>(ctx: &Context, name: &'static str, body: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Area::new(crate::runtime::egui::memory::unique(name)).show(ctx, body).inner
}

/// Shows an interactable `egui::Area` at the screen's top left and runs `body` on its `Ui`.
#[cfg(feature = "testing")]
pub(crate) fn show_cover<R>(ctx: &Context, name: &'static str, body: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Area::new(crate::runtime::egui::memory::unique(name)).fixed_pos(Pos2::ZERO).interactable(true).show(ctx, body).inner
}

/// An egui layer: an `Area`, a panel or a transformed body.
pub(crate) type Layer = LayerId;

/// A layer's mapping from its own space to the screen's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LayerMap(TSTransform);

impl LayerMap {
    /// The position in the layer of a screen position.
    pub(crate) fn to_layer(self, point: Pos2) -> Pos2 {
        self.0.inverse() * point
    }

    /// The screen position of a position in the layer.
    pub(crate) fn to_screen(self, point: Pos2) -> Pos2 {
        self.0 * point
    }

    /// A screen-space movement as the layer sees it.
    pub(crate) fn delta_to_layer(self, delta: Vec2) -> Vec2 {
        delta / self.0.scaling
    }
}

/// The mapping of `layer` to the screen, `None` for a layer with no transform.
pub(crate) fn layer_map(ctx: &Context, layer: LayerId) -> Option<LayerMap> {
    ctx.layer_transform_to_global(layer).map(LayerMap)
}

/// How high `layer` stands in egui's layer order, which the overlay host sets each pass: a larger value lies over a
/// smaller one. A layer egui does not list stands lowest.
pub(crate) fn layer_position(ctx: &Context, layer: LayerId) -> usize {
    ctx.memory(|memory| memory.layer_ids().position(|listed| listed == layer).map_or(0, |at| at + 1))
}

/// The topmost layer under a screen position.
pub(crate) fn layer_at(ctx: &Context, point: Pos2) -> Option<LayerId> {
    ctx.layer_id_at(point)
}

/// The layer of a transformed body: a sublayer of `parent` in the parent's order, named by `root`.
pub(crate) fn body_layer(parent: LayerId, root: Id) -> LayerId {
    LayerId::new(parent.order, crate::runtime::memory::sub(root, "body_layer"))
}

/// What a transformed body's forwarding widget carries: the root's sense and interact rect, in screen space.
pub(crate) struct Forward {
    pub id: Id,
    pub sense: egui::Sense,
    pub rect: Rect,
}

/// A transformed body's layer: `layer` is shown through `transform`, a sublayer of `parent`, with its top left at
/// `at` in its own space and clipped to `clip` in screen space.
pub(crate) struct BodyLayer {
    pub parent: LayerId,
    pub layer: LayerId,
    pub transform: TSTransform,
    pub at: Pos2,
    pub clip: Rect,
    /// Registered on the layer first, so an allocating `Ui` on the layer cannot hide the root under egui's
    /// layer-cover rule.
    pub forward: Option<Forward>,
}

/// Shows a transformed body in its layer.
pub(crate) fn show_body_layer<R>(ctx: &Context, body_layer: BodyLayer, body: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let BodyLayer { parent, layer, transform, at, clip, forward } = body_layer;
    ctx.set_transform_layer(layer, transform);
    ctx.set_sublayer(parent, layer);
    egui::Area::new(layer.id)
        .order(layer.order)
        .pivot(egui::Align2::LEFT_TOP)
        .fixed_pos(at)
        .constrain(false)
        .sense(egui::Sense::hover())
        .show(ctx, |ui| {
            if let Some(forward) = forward {
                ui.interact(transform.inverse() * forward.rect, forward.id, forward.sense);
                super::access::hide(ctx, forward.id);
            }
            ui.set_clip_rect(transform.inverse() * clip);
            ui.expand_to_include_rect(transform.inverse() * clip);
            body(ui)
        })
        .inner
}

/// The transform of a body zoomed by `scale` about its top left `at` and moved by `offset`.
pub(crate) fn body_transform(at: Pos2, scale: f32, offset: Vec2) -> TSTransform {
    TSTransform::new(at.to_vec2() * (1.0 - scale) + offset, scale)
}

/// The mapping between the screen and a body zoomed by `scale` about `at` and moved by `offset`.
pub(crate) fn body_map(at: Pos2, scale: f32, offset: Vec2) -> LayerMap {
    LayerMap(body_transform(at, scale, offset))
}

/// An egui layer, as the runtime's shape records name it.
pub type LayerKey = egui::LayerId;

/// How an overlay layer ranks among egui's orders: the one place the runtime chooses an `Order`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tier {
    /// Floating windows and panels.
    Window,
    /// Toasts, popups, modals and their chrome.
    Foreground,
    /// Tooltips and the ghost layer.
    Tooltip,
}

impl Tier {
    fn order(self) -> Order {
        match self {
            Tier::Window => Order::Middle,
            Tier::Foreground => Order::Foreground,
            Tier::Tooltip => Order::Tooltip,
        }
    }
}

/// What the overlay host asks of an `Area`.
pub(crate) struct LayerSpec {
    pub id: Id,
    pub tier: Tier,
    /// The physical left-top of the layer.
    pub at: Pos2,
    /// Whether the layer takes input; a tooltip and the ghost layer never do.
    pub interactable: bool,
}

/// What `show_layer` reports.
pub(crate) struct LayerShown {
    /// The layer's rect in screen space.
    pub rect: Rect,
    /// The layer's egui id, as `Context::layer_id_at` names it.
    pub layer: LayerId,
}

/// Shows an overlay layer: an `Area` with its left-top pivot at `spec.at`, never egui's default pivot.
pub(crate) fn show_layer(ctx: &Context, spec: &LayerSpec, body: impl FnOnce(&mut egui::Ui)) -> LayerShown {
    let shown = egui::Area::new(spec.id)
        .order(spec.tier.order())
        .pivot(egui::Align2::LEFT_TOP)
        .fixed_pos(spec.at)
        .constrain(false)
        .interactable(spec.interactable)
        .show(ctx, body);
    LayerShown { rect: shown.response.rect, layer: shown.response.layer_id }
}

/// Shows a layer that covers `screen` and paints `scrim` on it, around `hole` when there is one. While `absorbs` it
/// takes every pointer hit on it, so nothing below it is hit.
pub(crate) fn show_cover_layer(ctx: &Context, id: Id, tier: Tier, screen: Rect, scrim: Color32, hole: Option<Rect>, absorbs: bool) {
    let scrim = Some(scrim);
    egui::Area::new(id).order(tier.order()).pivot(egui::Align2::LEFT_TOP).fixed_pos(screen.min).constrain(false).interactable(absorbs).show(
        ctx,
        |ui| {
            if let Some(scrim) = scrim {
                let painter = ui.painter();
                match hole.map(|hole| hole.intersect(screen)).filter(|hole| hole.is_positive()) {
                    Some(hole) => {
                        let bands = [
                            Rect::from_min_max(screen.min, Pos2::new(screen.right(), hole.top())),
                            Rect::from_min_max(Pos2::new(screen.left(), hole.bottom()), screen.max),
                            Rect::from_min_max(Pos2::new(screen.left(), hole.top()), Pos2::new(hole.left(), hole.bottom())),
                            Rect::from_min_max(Pos2::new(hole.right(), hole.top()), Pos2::new(screen.right(), hole.bottom())),
                        ];
                        for band in bands {
                            painter.rect_filled(band, 0.0, scrim);
                        }
                    }
                    None => {
                        painter.rect_filled(screen, 0.0, scrim);
                    }
                }
            }
            let absorber = ui.allocate_rect(screen, Sense::click_and_drag());
            crate::runtime::egui::widgets::note_widget(ui.ctx(), absorber.id);
            crate::runtime::egui::access::hide(ui.ctx(), absorber.id);
        },
    );
}

/// The rect an overlay layer had when it was last shown, in screen space.
pub(crate) fn layer_rect(ctx: &Context, id: Id) -> Option<Rect> {
    ctx.memory(|memory| memory.area_rect(id))
}

/// The viewport's safe rect: the room overlays must stay inside.
pub(crate) fn safe_rect(ctx: &Context) -> Rect {
    ctx.content_rect()
}

/// Hands egui the whole layer order of the running pass, bottom first. egui consumes it by the end of the pass.
pub(crate) fn set_layer_order(ctx: &Context, order: &[LayerId]) {
    ctx.set_layer_order(order);
}

/// Makes `layer` the modal layer of the running pass: layers below it take no input.
pub(crate) fn set_modal_layer(ctx: &Context, layer: LayerId) {
    ctx.memory_mut(|memory| memory.set_modal_layer(layer));
}

/// The layer id an overlay layer named `id` has in `tier`.
pub(crate) fn layer_of(tier: Tier, id: Id) -> LayerId {
    LayerId::new(tier.order(), id)
}

/// The mapping from `layer`'s space to the screen's; the identity for a layer with no transform.
pub(crate) fn layer_to_screen(ctx: &Context, layer: LayerId) -> TSTransform {
    ctx.layer_transform_to_global(layer).unwrap_or(TSTransform::IDENTITY)
}

/// Limits `ui` to `max` and at least `min_width` across: the room the placement leaves the layer's body.
pub(crate) fn bound(ui: &mut egui::Ui, max: Vec2, min_width: f32) {
    ui.set_max_size(max);
    ui.set_min_width(min_width);
}

/// Fades everything painted in `ui` to `opacity`.
pub(crate) fn fade(ui: &mut egui::Ui, opacity: f32) {
    ui.set_opacity(opacity);
}

/// How a painted cursor is drawn: from the theme's tokens, so the ring reads on any canvas.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CursorInk {
    pub color: Color32,
    pub width: f32,
    /// The half-length of a crosshair's arms.
    pub arm: f32,
}

/// The painter of the ghost layer, over the whole screen and above every popup. The layer takes no input.
pub(crate) fn ghost_painter(ctx: &Context) -> Painter {
    ctx.layer_painter(layer_of(Tier::Tooltip, crate::runtime::egui::memory::unique("craft_ui.ghost")))
}

/// Paints a cursor shape at `at` on the ghost layer, above every other layer. The shapes that carry no caller geometry
/// (hand, magnifier, badges, rotate, distort and skew pointers) take their colours and widths from the root env's
/// tokens: marks in `overlay.stroke` over a `overlay.halo` outline, so they read on any image.
pub(crate) fn paint_cursor(ctx: &Context, at: Pos2, cursor: &craft_behavior::PaintedCursor, ink: CursorInk) {
    use craft_behavior::PaintedCursor;
    let Some(env) = crate::runtime::env::Env::root_of(ctx) else { return };
    let painter = ghost_painter(ctx);
    let overlay = &env.tokens.color.overlay;
    let thin = env.tokens.stroke.thin;
    let pen =
        pointer::Pen { painter: &painter, at, fill: crate::style::rgba_color(overlay.stroke), halo: crate::style::rgba_color(overlay.halo), thin };
    match cursor {
        PaintedCursor::Circle { radius, .. } => pen.marks(ink.width, ink.color, &[pointer::Mark::Circle(Vec2::ZERO, *radius)]),
        PaintedCursor::Crosshair => pen.marks(
            ink.width,
            ink.color,
            &[
                pointer::Mark::Line(Vec2::new(-ink.arm, 0.0), Vec2::new(ink.arm, 0.0)),
                pointer::Mark::Line(Vec2::new(0.0, -ink.arm), Vec2::new(0.0, ink.arm)),
            ],
        ),
        PaintedCursor::Outline { rings, .. } => {
            for ring in rings.iter().filter(|ring| ring.len() >= 2) {
                let points: Vec<Pos2> = ring.iter().map(|p| at + p.to_vec2()).collect();
                painter.add(egui::Shape::closed_line(points.clone(), egui::Stroke::new(thin + 2.0 * thin, pen.halo)));
                painter.add(egui::Shape::closed_line(points, egui::Stroke::new(thin, pen.fill)));
            }
        }
        PaintedCursor::Hand => pointer::hand(&pen),
        PaintedCursor::Zoom { out, .. } => pointer::zoom(&pen, *out),
        PaintedCursor::Tool { icon, hotspot, badge, .. } => {
            let side = env.tokens.size.icon.md;
            let rect = Rect::from_min_size(at - *hotspot * (side / pointer::ICON_UNITS), Vec2::splat(side));
            let record = std::cell::RefCell::new(crate::primitives::PaintRecord::default());
            let draw = crate::primitives::Draw::new(
                &painter,
                &record,
                crate::primitives::DrawEnv { tokens: &env.tokens, direction: env.direction, ink: crate::style::Ink::OnDark },
            );
            // The halo is the icon drawn in the halo colour one thin step off in each direction.
            let step = pen.thin;
            for offset in pointer::HALO_STEPS {
                let spec = crate::primitives::IconSpec::svg(icon.clone()).color(crate::style::StyleColor(pen.halo));
                draw.icon(rect.translate(offset * step), &spec);
            }
            draw.icon(rect, &crate::primitives::IconSpec::svg(icon.clone()).color(crate::style::StyleColor(pen.fill)));
            if let Some(badge) = badge {
                pointer::badge(&pen, rect.right_bottom().to_vec2() - at.to_vec2(), *badge);
            }
        }
        PaintedCursor::Badged { badge, .. } => pointer::badge(&pen, pointer::BADGE_OFFSET, *badge),
        PaintedCursor::Turn { toward, .. } => pointer::turn(&pen, *toward),
        PaintedCursor::Distort { dir, .. } => pointer::distort(&pen, *dir),
        PaintedCursor::Skew { dir, .. } => pointer::skew(&pen, *dir),
        // `Hidden` paints nothing, and so does a kind this host does not know.
        _ => {}
    }
}

/// The shapes of the pointers the host paints, in points from the pointer.
mod pointer {
    use craft_behavior::CursorBadge;
    use egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, Vec2, vec2};

    /// The icon sets draw on a box this many units on a side; a tool cursor's hotspot is in these.
    pub(super) const ICON_UNITS: f32 = 24.0;
    /// Where a badge sits beside a system pointer arrow: toward its lower right.
    pub(super) const BADGE_OFFSET: Vec2 = vec2(12.0, 12.0);
    /// The unit offsets the halo of an icon is drawn at.
    pub(super) const HALO_STEPS: [Vec2; 8] =
        [vec2(-1.0, 0.0), vec2(1.0, 0.0), vec2(0.0, -1.0), vec2(0.0, 1.0), vec2(-1.0, -1.0), vec2(1.0, 1.0), vec2(-1.0, 1.0), vec2(1.0, -1.0)];
    /// Half the side of a badge glyph.
    const BADGE_HALF: f32 = 3.5;
    /// The rotate pointer: radius of its arc, half its sweep in radians, and its segment count.
    const TURN_RADIUS: f32 = 12.0;
    const HALF_SWEEP: f32 = 0.75;
    const ARC_SEGMENTS: usize = 16;
    /// A rotate arrowhead's unit of size per radius, and its proportions in those units: the tip lies `HEAD_REACH`
    /// past the arc's end, the head is `HEAD_LENGTH` long, its base lies back by `HEAD_BACK` and is
    /// `2 * HEAD_HALF_WIDTH` wide.
    const HEAD_PER_RADIUS: f32 = 1.0 / 12.0;
    const HEAD_REACH: f32 = 2.0;
    const HEAD_LENGTH: f32 = 3.5;
    const HEAD_BACK: f32 = 3.0;
    const HEAD_HALF_WIDTH: f32 = 3.5;
    /// The distort dart: length, half width, and how far back from the tip its notch lies as a fraction of the length.
    const DART_LENGTH: f32 = 16.0;
    const DART_HALF_WIDTH: f32 = 7.0;
    const DART_NOTCH: f32 = 0.65;
    /// The skew arrows: distance of each from the pointer across the edge, half length, head length, head back.
    const SKEW_GAP: f32 = 4.0;
    const SKEW_HALF_LENGTH: f32 = 9.0;
    const SKEW_HEAD: f32 = 3.5;
    const SKEW_BACK: f32 = 2.5;
    /// The open hand, in points from the pointer: the box of the palm, the fingers' x, their tip heights, and
    /// the thumb. The pointer is on the middle of the hand's box.
    const HAND_SHIFT: Vec2 = vec2(2.0, 0.5);
    const PALM_CENTER: Vec2 = vec2(0.0, 4.5);
    const PALM_HALF: Vec2 = vec2(6.8, 5.5);
    const PALM_ROUND: f32 = 3.0;
    const FINGER_X: [f32; 4] = [-5.1, -1.7, 1.7, 5.1];
    const FINGER_TIP: [f32; 4] = [-7.0, -9.5, -8.5, -5.5];
    const FINGER_BASE: f32 = 3.0;
    const FINGER_R: f32 = 1.7;
    const THUMB: ([f32; 2], [f32; 2], f32) = ([-4.5, 7.0], [-10.5, 1.5], 1.9);
    const CREASE_X: [f32; 3] = [-3.4, 0.0, 3.4];
    const CREASE_END: f32 = 3.5;
    /// The magnifier: the lens radius (its centre is the pointer), the handle, and the arms of the plus or minus.
    const LENS: f32 = 6.5;
    const HANDLE: ([f32; 2], [f32; 2], f32) = ([5.2, 5.2], [10.8, 10.8], 2.2);
    const GLYPH_ARM: f32 = 3.4;

    /// What the shapes paint with: the pointer, the mark colour, its halo and the hairline width.
    pub(super) struct Pen<'a> {
        pub painter: &'a Painter,
        pub at: Pos2,
        pub fill: Color32,
        pub halo: Color32,
        pub thin: f32,
    }

    /// A line mark, in points from the pointer.
    pub(super) enum Mark {
        Line(Vec2, Vec2),
        Circle(Vec2, f32),
        Rect(Rect),
    }

    impl Pen<'_> {
        /// `marks` as `width` lines in `color`, each over a halo line a hairline wider on each side.
        pub(super) fn marks(&self, width: f32, color: Color32, marks: &[Mark]) {
            for (width, color) in [(width + 2.0 * self.thin, self.halo), (width, color)] {
                let stroke = Stroke::new(width, color);
                for mark in marks {
                    match mark {
                        Mark::Line(a, b) => {
                            self.painter.line_segment([self.at + *a, self.at + *b], stroke);
                        }
                        Mark::Circle(center, radius) => {
                            self.painter.circle_stroke(self.at + *center, *radius, stroke);
                        }
                        Mark::Rect(rect) => {
                            self.painter.rect_stroke(rect.translate(self.at.to_vec2()), 0.0, stroke, egui::StrokeKind::Middle);
                        }
                    }
                }
            }
        }

        /// A filled disc.
        fn disc(&self, center: Vec2, radius: f32, color: Color32) {
            self.painter.circle_filled(self.at + center, radius, color);
        }

        /// A capsule: the segment `a` to `b` grown by `radius`.
        fn capsule(&self, a: Vec2, b: Vec2, radius: f32, color: Color32) {
            self.painter.line_segment([self.at + a, self.at + b], Stroke::new(2.0 * radius, color));
            self.disc(a, radius, color);
            self.disc(b, radius, color);
        }

        /// A polygon filled with the mark colour and outlined in the halo.
        fn polygon(&self, points: Vec<Pos2>) {
            self.painter.add(Shape::convex_polygon(points, self.fill, Stroke::new(self.thin, self.halo)));
        }
    }

    /// The open hand: its silhouette in the mark colour over the same grown by a hairline in the halo colour, with the
    /// creases between the fingers.
    pub(super) fn hand(pen: &Pen<'_>) {
        let shift = |p: [f32; 2]| vec2(p[0], p[1]) + HAND_SHIFT;
        for (grow, color) in [(pen.thin, pen.halo), (0.0, pen.fill)] {
            let palm = Rect::from_center_size(pen.at + PALM_CENTER + HAND_SHIFT, PALM_HALF * 2.0).expand(grow);
            pen.painter.rect_filled(palm, PALM_ROUND + grow, color);
            for (x, tip) in FINGER_X.into_iter().zip(FINGER_TIP) {
                pen.capsule(shift([x, FINGER_BASE]), shift([x, tip]), FINGER_R + grow, color);
            }
            let (a, b, r) = THUMB;
            pen.capsule(shift(a), shift(b), r + grow, color);
        }
        for (x, (a, b)) in CREASE_X.into_iter().zip(FINGER_TIP.into_iter().zip(FINGER_TIP.into_iter().skip(1))) {
            // Each crease starts just below the lower of the two fingertips it separates.
            let from = a.max(b) + 1.0;
            pen.painter.line_segment([pen.at + shift([x, from]), pen.at + shift([x, CREASE_END])], Stroke::new(pen.thin, pen.halo));
        }
    }

    /// The magnifier with a plus, or a minus when `out`.
    pub(super) fn zoom(pen: &Pen<'_>, out: bool) {
        let (a, b, r) = HANDLE;
        let (a, b) = (vec2(a[0], a[1]), vec2(b[0], b[1]));
        for (grow, color) in [(pen.thin, pen.halo), (0.0, pen.fill)] {
            pen.disc(Vec2::ZERO, LENS + grow, color);
            pen.capsule(a, b, r + grow, color);
        }
        let stroke = Stroke::new(pen.thin, pen.halo);
        pen.painter.line_segment([pen.at + vec2(-GLYPH_ARM, 0.0), pen.at + vec2(GLYPH_ARM, 0.0)], stroke);
        if !out {
            pen.painter.line_segment([pen.at + vec2(0.0, -GLYPH_ARM), pen.at + vec2(0.0, GLYPH_ARM)], stroke);
        }
    }

    /// A badge centred `center` from the pointer.
    pub(super) fn badge(pen: &Pen<'_>, center: Vec2, badge: CursorBadge) {
        let h = BADGE_HALF;
        let at = |x: f32, y: f32| center + vec2(x, y);
        let marks = match badge {
            CursorBadge::Add => vec![Mark::Line(at(-h, 0.0), at(h, 0.0)), Mark::Line(at(0.0, -h), at(0.0, h))],
            CursorBadge::Subtract => vec![Mark::Line(at(-h, 0.0), at(h, 0.0))],
            CursorBadge::Intersect => {
                let d = 0.8 * h;
                vec![Mark::Line(at(-d, -d), at(d, d)), Mark::Line(at(-d, d), at(d, -d))]
            }
            CursorBadge::Copy => {
                let side = Vec2::splat(h * 1.4);
                vec![
                    Mark::Rect(Rect::from_center_size(Pos2::ZERO + at(-h / 3.0, -h / 3.0), side)),
                    Mark::Rect(Rect::from_center_size(Pos2::ZERO + at(h / 3.0, h / 3.0), side)),
                ]
            }
            CursorBadge::NotAllowed => {
                let d = h * std::f32::consts::FRAC_1_SQRT_2;
                vec![Mark::Circle(center, h), Mark::Line(at(-d, d), at(d, -d))]
            }
            _ => Vec::new(),
        };
        pen.marks(pen.thin, pen.fill, &marks);
    }

    /// The rotate pointer: an arc with an arrowhead at each end, bent round the corner in direction `toward`.
    pub(super) fn turn(pen: &Pen<'_>, toward: Vec2) {
        let len = toward.length();
        if !len.is_finite() || len < f32::EPSILON {
            return;
        }
        let u = toward / len;
        let centre = pen.at + u * TURN_RADIUS;
        let mid = (-u.y).atan2(-u.x);
        let on_arc = |a: f32| centre + vec2(a.cos(), a.sin()) * TURN_RADIUS;
        let points: Vec<Pos2> = (0..=ARC_SEGMENTS).map(|i| on_arc(mid - HALF_SWEEP + 2.0 * HALF_SWEEP * i as f32 / ARC_SEGMENTS as f32)).collect();
        pen.painter.add(Shape::line(points.clone(), Stroke::new(pen.thin * 3.0, pen.halo)));
        pen.painter.add(Shape::line(points, Stroke::new(pen.thin, pen.fill)));
        let head = TURN_RADIUS * HEAD_PER_RADIUS;
        for (a, sign) in [(mid - HALF_SWEEP, -1.0f32), (mid + HALF_SWEEP, 1.0)] {
            let tangent = vec2(-a.sin(), a.cos()) * sign;
            let normal = vec2(a.cos(), a.sin());
            let tip = on_arc(a) + tangent * HEAD_REACH * head;
            pen.polygon(vec![
                tip + tangent * HEAD_LENGTH * head,
                tip - tangent * HEAD_BACK * head + normal * HEAD_HALF_WIDTH * head,
                tip - tangent * HEAD_BACK * head - normal * HEAD_HALF_WIDTH * head,
            ]);
        }
    }

    /// The unit vector along `dir`, or `fallback` for a zero or non-finite one.
    fn unit(dir: Vec2, fallback: Vec2) -> Vec2 {
        let len = dir.length();
        if len.is_finite() && len > f32::EPSILON { dir / len } else { fallback }
    }

    /// The free-distort pointer: a dart with its tip on the pointer, pointing along `dir` (up and left for zero).
    pub(super) fn distort(pen: &Pen<'_>, dir: Vec2) {
        let u = unit(dir, vec2(-1.0, -1.0).normalized());
        let perp = vec2(-u.y, u.x);
        let base = pen.at - u * DART_LENGTH;
        let (left, right, notch) = (base + perp * DART_HALF_WIDTH, base - perp * DART_HALF_WIDTH, pen.at - u * (DART_LENGTH * DART_NOTCH));
        // Two triangles meeting along the tip and the notch: the dart is not convex.
        for triangle in [vec![pen.at, left, notch], vec![pen.at, notch, right]] {
            pen.painter.add(Shape::convex_polygon(triangle, pen.fill, Stroke::NONE));
        }
        pen.painter.add(Shape::closed_line(vec![pen.at, left, notch, right], Stroke::new(pen.thin * 2.0, pen.halo)));
    }

    /// The skew pointer: two opposed arrows beside each other along `dir` (along x for zero).
    pub(super) fn skew(pen: &Pen<'_>, dir: Vec2) {
        let u = unit(dir, vec2(1.0, 0.0));
        let perp = vec2(-u.y, u.x);
        for (offset, sign) in [(SKEW_GAP, 1.0f32), (-SKEW_GAP, -1.0)] {
            let centre = pen.at + perp * offset;
            let end = centre + u * (sign * SKEW_HALF_LENGTH);
            let start = centre - u * (sign * SKEW_HALF_LENGTH);
            pen.painter.line_segment([start, end], Stroke::new(pen.thin * 3.0, pen.halo));
            pen.painter.line_segment([start, end], Stroke::new(pen.thin, pen.fill));
            pen.polygon(vec![
                end + u * (sign * SKEW_HEAD),
                end - u * (sign * SKEW_BACK) + perp * SKEW_HEAD,
                end - u * (sign * SKEW_BACK) - perp * SKEW_HEAD,
            ]);
        }
    }
}

/// The pointer position of the running pass, in screen space.
pub(crate) fn pointer_pos(ctx: &Context) -> Option<Pos2> {
    ctx.input(|input| input.pointer.latest_pos())
}

/// The layer order after `Context::set_layer_order`.
#[cfg(feature = "testing")]
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LayerOrder {
    /// The listed layer is the topmost of the layers shown.
    pub listed_on_top: bool,
    /// The unlisted layers keep their relative order.
    pub unlisted_keep_order: bool,
    /// A listed id that is no layer does not become one.
    pub unknown_ignored: bool,
}

/// Two frames over three areas; the second lists the first area and an unknown layer.
#[cfg(feature = "testing")]
pub fn layer_order() -> LayerOrder {
    let ctx = super::fixture::bare_context();
    let ids =
        [super::memory::unique("craft_ui.surface.la"), super::memory::unique("craft_ui.surface.lb"), super::memory::unique("craft_ui.surface.lc")];
    let layers = ids.map(|id| LayerId::new(Order::Middle, id));
    let ghost = LayerId::new(Order::Middle, super::memory::unique("craft_ui.surface.ghost"));
    let frame = |time: f64, list: &[LayerId]| {
        let mut output = ctx.run_ui(super::surface::raw(time, Vec::new()), |ui| {
            let ctx = ui.ctx().clone();
            for id in ids {
                egui::Area::new(id).show(&ctx, |ui| {
                    ui.label("layer");
                });
            }
            if !list.is_empty() {
                ctx.set_layer_order(list);
            }
        });
        output.textures_delta.clear();
        ctx.memory(|memory| memory.layer_ids().filter(|layer| layers.contains(layer) || *layer == ghost).collect::<Vec<_>>())
    };
    let before = frame(1.0, &[]);
    let after = frame(2.0, &[layers[0], ghost]);
    let rest = |order: &[LayerId]| order.iter().copied().filter(|layer| *layer != layers[0]).collect::<Vec<_>>();
    LayerOrder {
        listed_on_top: after.last() == Some(&layers[0]),
        unlisted_keep_order: rest(&before) == rest(&after),
        unknown_ignored: !after.contains(&ghost),
    }
}

/// Gives a container layer's area a size before its first pass, so egui runs no hidden sizing pass for it. That pass
/// shows no body, so nothing would be measured in it; the layer's first pass is a real one that records the sizes
/// the discard pass places by.
pub(crate) fn seed_area(ctx: &Context, layer: LayerId, at: Pos2, room: Vec2) {
    let state =
        egui::AreaState { pivot_pos: Some(at), pivot: egui::Align2::LEFT_TOP, size: Some(room), interactable: false, last_became_visible_at: None };
    ctx.memory_mut(|memory| memory.areas_mut().set_state(layer, state));
}

/// The width of the widest layer above the background, 0 when there is none.
#[cfg(feature = "testing")]
pub(crate) fn widest_layer(ctx: &Context) -> f32 {
    ctx.memory(|memory| {
        memory
            .layer_ids()
            .filter(|layer| layer.order != Order::Background)
            .filter_map(|layer| memory.area_rect(layer.id))
            .map(|rect| rect.width())
            .fold(0.0, f32::max)
    })
}
