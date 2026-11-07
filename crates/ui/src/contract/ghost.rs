//! The ghost layer's content (8.4, 8.5): the running drag's preview badge beside the pointer and the mark of the drop
//! target that would take the payload. Both come from the drag session's data; the layer takes no input.

use crate::primitives::{Draw, DrawEnv, PaintRecord, PaintShape, RuleKind, TextRun, outline};
use crate::runtime::drag::{self, DragPreview, DropMark};
use crate::runtime::egui::areas;
use crate::runtime::env::Env;
use crate::runtime::pointer;
use crate::style::{Corners, Ink, SurfaceKind};
use egui::{Context, CornerRadius, Pos2, Rect, Vec2};
use std::cell::RefCell;

/// The end hook's painter of the ghost layer: pass it to `InstallSpec::frame_layers`.
pub fn paint(ctx: &Context) {
    let Some(session) = drag::ghost(ctx) else { return };
    let env = Env::of_root(ctx);
    let painter = areas::ghost_painter(ctx);
    let record = RefCell::new(PaintRecord::default());
    let draw = Draw::new(&painter, &record, DrawEnv { tokens: &env.tokens, direction: env.direction, ink: Ink::OnPanel });
    if let Some(mark) = session.mark {
        paint_mark(&draw, mark);
    }
    let Some(at) = pointer::frame(ctx).pos else { return };
    match &session.preview {
        DragPreview::Badge(label) => {
            let label = env.view().resolve(label);
            paint_badge(&draw, &env, at, label.as_str());
        }
        DragPreview::Frame { size, label } => {
            let frame = Rect::from_center_size(at, *size);
            painter.extend(outline(&env.tokens, &PaintShape::Rect(frame, CornerRadius::ZERO)));
            if let Some(label) = label {
                let label = env.view().resolve(label);
                let run = TextRun::new(label.as_str());
                let layout = draw.layout(&run, frame.width());
                draw.galley(frame.min + Vec2::splat(env.tokens.space.pad.control), &layout);
            }
        }
        _ => {}
    }
}

fn paint_badge(draw: &Draw<'_>, env: &Env, at: Pos2, text: &str) {
    let space = &env.tokens.space;
    let run = TextRun::new(text);
    let layout = draw.layout(&run, f32::INFINITY);
    let pad = Vec2::splat(space.pad.control);
    let outer = Rect::from_min_size(at + Vec2::splat(space.gap.inline), layout.size() + pad * 2.0);
    draw.surface(outer, SurfaceKind::Popup, Corners::All);
    draw.galley(outer.min + pad, &layout);
}

fn paint_mark(draw: &Draw<'_>, mark: DropMark) {
    match mark {
        DropMark::Line { from, to } => draw.rule(RuleKind::DropMark, from, to),
        DropMark::Into(rect) | DropMark::Split(rect) => {
            let corners = [rect.left_top(), rect.right_top(), rect.right_bottom(), rect.left_bottom()];
            for (at, from) in corners.iter().enumerate() {
                draw.rule(RuleKind::DropMark, *from, corners[(at + 1) % corners.len()]);
            }
        }
    }
}
