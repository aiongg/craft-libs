//! What the drag layer shows beside the pointer besides the drag preview (8.4, 8.5): the mark of the drop target that
//! would take the payload (with the payload's name inside a region mark, for a drag that names it), and value
//! readouts beside the pointer during a gesture. Each records into a `Draw` on its layer child's canvas: the drop mark
//! in the tokens of the target's region, a readout in the env of the region that asks. The drag preview itself is the
//! ghost ui-rt keeps on its drag layer (`contract::rt::drag::ghost`); the layer takes no input.

use crate::primitives::{Canvas, Draw, RuleKind, Span, TextRun};
use crate::runtime::drag::{DragPreview, DropMark};
use crate::runtime::env::{EnvView, Label};
use crate::style::{Corners, RuleWidth, SurfaceKind, UiTokens, rule_look};
use craft_behavior::Flow;
use craft_ui_rt::{Pos2, Rect, Vec2, vec2};

/// Records rows of a label and a value beside the pointer at `at` into `draw`, in `env`'s tokens, words and direction:
/// the env of the region that asks. It sits toward the lower end side of the pointer (the lower right, mirrored under
/// right-to-left), flips to the other side where `window` (the window's rect) has no room and stays inside it. Labels
/// start at the reading start and values end at the reading end, the widest label and the widest value a section gap
/// apart. Labels are in the muted ink and values in the normal one. No rows record nothing.
pub(crate) fn record_readout(draw: &Draw<'_>, env: &EnvView<'_>, at: Pos2, window: Rect, rows: &[(Label, String)]) {
    if rows.is_empty() {
        return;
    }
    let tokens = env.shared_tokens();
    let tokens = &*tokens;
    let direction = env.direction();
    let runs: Vec<(String, String)> = rows.iter().map(|(label, value)| (env.resolve(label).as_str().to_owned(), value.clone())).collect();
    let layouts: Vec<_> = runs
        .iter()
        .map(|(label, value)| {
            (draw.layout(&TextRun::spans(vec![Span::new(label.as_str()).muted()]), f32::INFINITY), draw.layout(&TextRun::new(value), f32::INFINITY))
        })
        .collect();
    let space = &tokens.space;
    let pad = vec2(space.gap.stack, space.gap.row);
    let label_width = layouts.iter().fold(0.0f32, |widest, (label, _)| widest.max(label.size().x));
    let value_width = layouts.iter().fold(0.0f32, |widest, (_, value)| widest.max(value.size().x));
    let width = 2.0 * pad.x + label_width + space.gap.section + value_width;
    let heights: Vec<f32> = layouts.iter().map(|(label, value)| label.size().y.max(value.size().y)).collect();
    let height = 2.0 * pad.y + heights.iter().sum::<f32>() + tokens.stroke.thin * (heights.len() - 1) as f32;
    let tip = tokens.size.pointer_tip;
    let rtl = direction.is_rtl();
    let outer = readout_place(at, vec2(tip.x, tip.y), vec2(width, height), rtl, window);
    draw.surface(outer, SurfaceKind::Popup, Corners::All);
    let mut y = outer.min.y + pad.y;
    for ((label, value), row_height) in layouts.iter().zip(&heights) {
        let (label_x, value_x) = if rtl {
            (outer.max.x - pad.x - label.size().x, outer.min.x + pad.x)
        } else {
            (outer.min.x + pad.x, outer.max.x - pad.x - value.size().x)
        };
        draw.galley(Pos2::new(label_x, y), label);
        draw.galley(Pos2::new(value_x, y), value);
        y += row_height + tokens.stroke.thin;
    }
}

/// The readout's rect for a pointer at `at`: `offset` toward the reading end and down, on the other side of the pointer
/// where it would leave `window` across, and shifted to stay inside `window`.
fn readout_place(at: Pos2, offset: Vec2, size: Vec2, rtl: bool, window: Rect) -> Rect {
    let beside = |toward_end: bool| {
        let left = if toward_end != rtl { at.x + offset.x } else { at.x - offset.x - size.x };
        Rect::from_min_size(Pos2::new(left, at.y + offset.y), size)
    };
    let fits = |rect: Rect| rect.min.x >= window.min.x && rect.max.x <= window.max.x;
    let rect = if fits(beside(true)) || !fits(beside(false)) { beside(true) } else { beside(false) };
    let dx = (window.min.x - rect.min.x).max(0.0) - (rect.max.x - window.max.x).max(0.0);
    let dy = (window.min.y - rect.min.y).max(0.0) - (rect.max.y - window.max.y).max(0.0);
    rect.translate(vec2(dx, dy))
}

/// Records the drop mark `mark` into `draw` in the target region's `tokens`, with the payload's name inside a region
/// mark when the drag's `preview` names it (in the words and direction of `env`, the source's).
pub(crate) fn record_drop_mark(draw: &Draw<'_>, canvas: &Canvas<'_>, tokens: &UiTokens, env: &EnvView<'_>, mark: DropMark, preview: &DragPreview) {
    paint_mark(canvas, tokens, mark);
    if let (DropMark::Region { area, .. }, DragPreview::Named(name)) = (mark, preview) {
        paint_region_name(draw, tokens, env, area, name);
    }
}

/// The payload's name inside the region `area` a drop would take: at the reading start of its top, inside the mark's
/// band, on one line elided to the area's width. A region too small for the line shows no name.
fn paint_region_name(draw: &Draw<'_>, tokens: &UiTokens, env: &EnvView<'_>, area: Rect, name: &Label) {
    let name = env.resolve(name);
    let inner = area.shrink(tokens.space.pad.control);
    if inner.width() <= 0.0 {
        return;
    }
    let layout = draw.layout(&TextRun::new(name.as_str()).elide(), inner.width());
    if layout.size().y > inner.height() {
        return;
    }
    let (at, _) = Flow::new(inner, env.direction().is_rtl()).take_start(layout.size().x.min(inner.width()));
    draw.galley(at.min, &layout);
}

/// The drop mark: bands of the drop rule on a halo of the panel surface one thin stroke wide, so the mark reads over
/// any content. An area mark is the same band along the inside of its rect; a region is also washed with the rule's
/// colour, so the area the payload takes reads as one piece, and carries its line band.
fn paint_mark(canvas: &Canvas<'_>, tokens: &UiTokens, mark: DropMark) {
    let look = rule_look(tokens, RuleKind::DropMark);
    let ppp = canvas.pixels_per_point();
    let width = look.width.points(ppp);
    let halo = RuleWidth::Points(tokens.stroke.thin).points(ppp);
    let outline = |rect: Rect| {
        let inside = rect.shrink(width / 2.0 + halo);
        let corners = [inside.left_top(), inside.right_top(), inside.right_bottom(), inside.left_bottom()];
        (0..corners.len()).map(move |at| (corners[at], corners[(at + 1) % corners.len()]))
    };
    let segments: Vec<(Pos2, Pos2)> = match mark {
        DropMark::Line { from, to } => vec![(from, to)],
        DropMark::Into(rect) => outline(rect).collect(),
        DropMark::Region { area, line } => {
            canvas.rect_filled(area, 0u8, look.color.gamma_multiply(craft_theme::Opacity::WASH));
            outline(area).chain(line.map(|[from, to]| (from, to))).collect()
        }
    };
    let band = |(from, to): (Pos2, Pos2), half: f32| Rect::from_two_pos(from, to).expand(half);
    // Every halo before any band, so the halo of one side never covers the band of the side before it.
    let panel = crate::style::rgba_color(tokens.color.surface.panel);
    for &segment in &segments {
        canvas.rect_filled(band(segment, width / 2.0 + halo), 0u8, panel);
    }
    for &segment in &segments {
        canvas.rect_filled(band(segment, width / 2.0), 0u8, look.color);
    }
}
