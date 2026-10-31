//! A chart drawn from a primitive list (N6.05).

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{Painter, Pos2};

/// Horizontal text anchor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum HAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// Vertical text anchor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum VAlign {
    #[default]
    Top,
    Middle,
    Bottom,
}

/// A drawing primitive in chart coordinates (origin top-left, y down, points). Colours are
/// straight RGBA. The app's chart engine (GridCraft's `gridcraft-chart`) produces the list; the
/// library paints it.
#[derive(Clone, Debug, PartialEq)]
pub enum ChartPrimitive {
    Rect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        fill: Option<[u8; 4]>,
        stroke: Option<([u8; 4], f32)>,
        radius: f32,
    },
    Line {
        points: Vec<[f32; 2]>,
        color: [u8; 4],
        width: f32,
        dash: bool,
    },
    Polygon {
        points: Vec<[f32; 2]>,
        fill: [u8; 4],
        stroke: Option<([u8; 4], f32)>,
    },
    /// A pie or doughnut sector: centre, radii, angles in radians (0 at 12 o'clock, clockwise).
    Wedge {
        cx: f32,
        cy: f32,
        r_outer: f32,
        r_inner: f32,
        a0: f32,
        a1: f32,
        fill: [u8; 4],
        stroke: Option<([u8; 4], f32)>,
    },
    Circle {
        cx: f32,
        cy: f32,
        r: f32,
        fill: Option<[u8; 4]>,
        stroke: Option<([u8; 4], f32)>,
    },
    /// Text anchored at `(x, y)` by `align` / `valign`, rotated by `rotation` radians about the
    /// anchor; `size` in points.
    Text {
        x: f32,
        y: f32,
        text: String,
        size: f32,
        color: [u8; 4],
        bold: bool,
        align: HAlign,
        valign: VAlign,
        rotation: f32,
    },
}

/// Paints `primitives` with their origin at `origin`, scaled by `zoom`: rects (stroke inside),
/// lines (stroke width × max(zoom, 0.5); dashed 4 / 3 points × zoom), polygons (a convex fill,
/// else a mesh of the closed path), wedges as meshes with one step per 0.05 rad (2…400 steps)
/// and their outline, circles, text at `size × zoom` in the UI face (bold when asked), rotated
/// about its anchor.
// gridcraft: crates/ui-egui/src/chartview.rs:45 `paint_prims`, `:152` `is_convex`, `:173`
// `area_mesh`; crates/chart/src/lib.rs:53 `Prim`.
pub fn paint_chart(painter: &Painter, origin: Pos2, primitives: &[ChartPrimitive], zoom: f32) {
    todo!()
}
