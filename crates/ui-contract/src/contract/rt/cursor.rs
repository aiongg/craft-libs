//! The cursor service (docs/contract-rt.md section 10, "Cursor"). A component names the cursor over
//! its own shape in `Component::cursor` and over its parts in `PartBuilder::cursor`; the node keeps
//! them and records them with its hit shapes. ui-rt picks the window's cursor from the hovered (or
//! pressed) shape and sends it to the platform only when it differs from the last one, so hovering
//! a node with another cursor runs no hook and no per-frame cursor pass.
//!
//! A painted cursor (`PaintedCursor`) is drawn by the window at the pointer (ui-rt
//! `PaintedPointer`): the run records each painted cursor the node names once, with its env, and
//! paint hands the drawing to ui-rt with the shape. A brush tip is a mark on its canvas: it shows
//! clipped to the naming node, under every layer above it.

use std::sync::Arc;

use craft_behavior::{Cursor, CursorIcon, PaintedCursor};
use craft_ui_rt::{HitShape, PaintCx, PaintedPointer, Pos2, Rect, Sense};

use super::node::Kept;
use crate::primitives::Canvas;
use crate::primitives::Services;
use crate::primitives::pointer_art::{Art, pointer_art};
use crate::runtime::env::Env;

/// Records the node's own hit shape: its sense and, when the component names one, its cursor. A node
/// that only names a cursor senses hover, so the pointer over it finds the shape.
pub(crate) fn record_own_shape(kept: &Kept, cx: &mut PaintCx<'_>) {
    let bounds = cx.bounds();
    match &kept.cursor {
        Some(cursor) => record(kept, cx, HitShape::Rect(bounds), kept.sense | Sense::HOVER, 0, cursor),
        None if !kept.sense.is_empty() => cx.hit(HitShape::Rect(bounds), kept.sense, 0),
        None => {}
    }
}

/// Records a hit shape with the cursor the pointer shows over it: a system cursor as the platform's,
/// a painted one as the drawing its run recorded.
pub(crate) fn record(kept: &Kept, cx: &mut PaintCx<'_>, shape: HitShape, sense: Sense, part: u64, cursor: &Cursor) {
    match cursor {
        Cursor::Painted(painted) => match kept.pointers.iter().find(|(c, _)| c == painted) {
            Some((_, pointer)) => cx.hit_painted(shape, sense, part, pointer.clone()),
            None => cx.hit_cursor(shape, sense, part, base(painted)),
        },
        cursor => cx.hit_cursor(shape, sense, part, platform(cursor)),
    }
}

/// The platform cursor for a system cursor.
pub(crate) fn platform(cursor: &Cursor) -> craft_ui_rt::Cursor {
    match cursor {
        Cursor::System(icon) => craft_ui_rt::Cursor::Icon(*icon),
        Cursor::Painted(painted) => base(painted),
        _ => craft_ui_rt::Cursor::Icon(CursorIcon::Default),
    }
}

/// The platform cursor under a painted one: the system arrow a badge sits beside, else none.
fn base(painted: &PaintedCursor) -> craft_ui_rt::Cursor {
    match painted {
        PaintedCursor::Badged { base, .. } => craft_ui_rt::Cursor::Icon(*base),
        _ => craft_ui_rt::Cursor::Hidden,
    }
}

/// The drawings of the painted cursors in `cursors`, each recorded once with the run's env: a hand
/// also as the fist it shows while a press runs. `Hidden` has no drawing.
pub(crate) fn painted_pointers<'c>(
    services: &dyn Services,
    env: &Env,
    cursors: impl Iterator<Item = &'c Cursor>,
) -> Vec<(PaintedCursor, PaintedPointer)> {
    let mut out: Vec<(PaintedCursor, PaintedPointer)> = Vec::new();
    for cursor in cursors {
        let Cursor::Painted(painted) = cursor else { continue };
        if out.iter().any(|(c, _)| c == painted) {
            continue;
        }
        let open = pointer_art(env, painted, false, false);
        let Some(mut bounds) = open.bounds() else { continue };
        let closed = matches!(painted, PaintedCursor::Hand).then(|| pointer_art(env, painted, true, false));
        if let Some(b) = closed.as_ref().and_then(|art| art.bounds()) {
            bounds = bounds.union(b);
        }
        let pointer = PaintedPointer {
            base: base(painted),
            art: drawing(services, &open, bounds),
            pressed: closed.map(|art| drawing(services, &art, bounds)),
            offset: bounds.min.to_vec2(),
            size: bounds.size(),
            confined: matches!(painted, PaintedCursor::Circle { .. } | PaintedCursor::Outline { .. }),
        };
        out.push((painted.clone(), pointer));
    }
    out
}

/// `art` recorded with the top left of `bounds` (around the hotspot) at the origin.
fn drawing(services: &dyn Services, art: &Art, bounds: Rect) -> Arc<craft_ui_rt::DisplayList> {
    let canvas = Canvas::new(services, Pos2::ZERO, Rect::EVERYTHING);
    art.paint(&canvas, Pos2::ZERO - bounds.min.to_vec2());
    Arc::new(canvas.finish().0)
}
