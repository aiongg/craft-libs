//! Cursor requests as plain data and the rule that picks the winner (8.5).

use emath::{Pos2, Vec2};
use std::borrow::Cow;

/// A system cursor: craft-behavior's copy of egui's set, converted by `runtime::egui::output`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[expect(missing_docs, reason = "the names are the system cursor names")]
pub enum CursorIcon {
    #[default]
    Default,
    None,
    ContextMenu,
    Help,
    PointingHand,
    Progress,
    Wait,
    Cell,
    Crosshair,
    Text,
    VerticalText,
    Alias,
    Copy,
    Move,
    NoDrop,
    NotAllowed,
    Grab,
    Grabbing,
    AllScroll,
    ResizeHorizontal,
    ResizeNeSw,
    ResizeNwSe,
    ResizeVertical,
    ResizeEast,
    ResizeSouthEast,
    ResizeSouth,
    ResizeSouthWest,
    ResizeWest,
    ResizeNorthWest,
    ResizeNorth,
    ResizeNorthEast,
    ZoomIn,
    ZoomOut,
}

/// An icon's egui-free key (5.2), so plain data such as [`PaintedCursor::Tool`] names icons without egui.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct IconId(Cow<'static, str>);

impl IconId {
    /// The icon named `name`.
    pub const fn new(name: &'static str) -> Self {
        IconId(Cow::Borrowed(name))
    }

    /// The name.
    pub fn name(&self) -> &str {
        &self.0
    }
}

/// A mark beside a painted cursor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CursorBadge {
    /// A plus.
    Add,
    /// A minus.
    Subtract,
    /// An intersection mark.
    Intersect,
    /// A copy mark.
    Copy,
    /// A not-allowed mark.
    NotAllowed,
}

/// A cursor the overlay host paints. Geometry is in screen points around the pointer (a tool scales its brush
/// radius by the zoom it set); colours come from semantic tokens the overlay host picks.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum PaintedCursor {
    /// A crosshair.
    Crosshair,
    /// A circle of `radius` points.
    #[non_exhaustive]
    Circle {
        /// The radius.
        radius: f32,
    },
    /// App shapes, as closed rings around the pointer.
    #[non_exhaustive]
    Outline {
        /// The rings.
        rings: Vec<Vec<Pos2>>,
    },
    /// An open hand.
    Hand,
    /// A magnifier.
    #[non_exhaustive]
    Zoom {
        /// Zooming out.
        out: bool,
    },
    /// A tool's icon.
    #[non_exhaustive]
    Tool {
        /// The icon.
        icon: IconId,
        /// The icon point that sits on the pointer.
        hotspot: Vec2,
        /// A mark beside it.
        badge: Option<CursorBadge>,
    },
    /// A system cursor with a mark.
    #[non_exhaustive]
    Badged {
        /// The system cursor.
        base: CursorIcon,
        /// The mark.
        badge: CursorBadge,
    },
    /// No cursor at all.
    Hidden,
}

impl PaintedCursor {
    /// A circle of `radius` points.
    pub const fn circle(radius: f32) -> Self {
        PaintedCursor::Circle { radius }
    }

    /// App shapes as closed rings.
    pub fn outline(rings: Vec<Vec<Pos2>>) -> Self {
        PaintedCursor::Outline { rings }
    }

    /// A magnifier, zooming out when `out`.
    pub const fn zoom(out: bool) -> Self {
        PaintedCursor::Zoom { out }
    }

    /// A tool's icon with its hotspot and an optional badge.
    pub const fn tool(icon: IconId, hotspot: Vec2, badge: Option<CursorBadge>) -> Self {
        PaintedCursor::Tool { icon, hotspot, badge }
    }

    /// A system cursor with a mark.
    pub const fn badged(base: CursorIcon, badge: CursorBadge) -> Self {
        PaintedCursor::Badged { base, badge }
    }
}

/// What a cursor request names.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Cursor {
    /// A system cursor.
    System(CursorIcon),
    /// A cursor the overlay host paints.
    Painted(PaintedCursor),
}

impl From<CursorIcon> for Cursor {
    fn from(icon: CursorIcon) -> Self {
        Cursor::System(icon)
    }
}

impl From<PaintedCursor> for Cursor {
    fn from(cursor: PaintedCursor) -> Self {
        Cursor::Painted(cursor)
    }
}

/// Who asks, in ascending order of precedence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum CursorPriority {
    /// The default.
    Default,
    /// The active tool.
    Tool,
    /// A hovered component.
    Hover,
    /// A hovered part's declared cursor.
    Part,
    /// A drop target during a drag.
    DropTarget,
    /// A drag in progress.
    Drag,
}

/// The winning request: the highest priority, the later request on a tie.
pub fn pick(requests: &[(Cursor, CursorPriority)]) -> Option<&Cursor> {
    requests.iter().enumerate().max_by_key(|(order, (_, priority))| (*priority, *order)).map(|(_, (cursor, _))| cursor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_highest_priority_wins_and_the_later_request_wins_a_tie() {
        let hand: Cursor = PaintedCursor::Hand.into();
        let requests = [
            (CursorIcon::Grab.into(), CursorPriority::Part),
            (CursorIcon::Crosshair.into(), CursorPriority::Tool),
            (hand.clone(), CursorPriority::Part),
            (CursorIcon::Text.into(), CursorPriority::Hover),
        ];
        assert_eq!(pick(&requests), Some(&hand));
        assert_eq!(pick(&[]), None);
    }
}
