//! [`EdgePicker`] (N3.07): the diagram of a cell range's (or a paragraph's) six edges, as
//! InDesign's Stroke proxy in the Table and Cell Styles panels and Word's Borders dialog show it:
//! top, bottom, left, right, inside horizontal and inside vertical, each line on or off; and
//! Word's border presets (All, None, Outside, Inside, one edge) as menu items.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{Response, Ui};

/// The picker's size, screen points (DesignCraft's `PROXY_SIZE`); lines sit 8 pt in.
pub const PICKER_SIZE: f32 = 64.0;

craft_ui::words! {
    /// The edges' and presets' names (Word's border menu, InDesign's proxy tooltips).
    #[non_exhaustive]
    pub struct EdgePickerLabels {
        picker: "edges.picker" = "Edges";
        top: "edges.top" = "Top Border";
        bottom: "edges.bottom" = "Bottom Border";
        left: "edges.left" = "Left Border";
        right: "edges.right" = "Right Border";
        inside_horizontal: "edges.insideH" = "Inside Horizontal Border";
        inside_vertical: "edges.insideV" = "Inside Vertical Border";
        all: "edges.all" = "All Borders";
        none: "edges.none" = "No Border";
        outside: "edges.outside" = "Outside Borders";
        inside: "edges.inside" = "Inside Borders";
    }
}

/// One of the six edges, in the picker's order (DesignCraft's `EDGE_NAMES`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EdgeKey {
    Top,
    Bottom,
    Left,
    Right,
    InsideHorizontal,
    InsideVertical,
}

impl EdgeKey {
    pub const ALL: [EdgeKey; 6] = [EdgeKey::Top, EdgeKey::Bottom, EdgeKey::Left, EdgeKey::Right, EdgeKey::InsideHorizontal, EdgeKey::InsideVertical];

    /// The name commands take (`top`, `bottom`, `left`, `right`, `innerHorizontal`,
    /// `innerVertical`).
    pub fn id(self) -> &'static str {
        match self {
            EdgeKey::Top => "top",
            EdgeKey::Bottom => "bottom",
            EdgeKey::Left => "left",
            EdgeKey::Right => "right",
            EdgeKey::InsideHorizontal => "innerHorizontal",
            EdgeKey::InsideVertical => "innerVertical",
        }
    }
}

/// Which edges are chosen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct BorderEdges(pub [bool; 6]);

impl BorderEdges {
    pub const ALL: BorderEdges = BorderEdges([true; 6]);

    pub fn get(self, edge: EdgeKey) -> bool {
        self.0[edge as usize]
    }

    pub fn set(&mut self, edge: EdgeKey, on: bool) {
        self.0[edge as usize] = on;
    }
}

/// Word's border presets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BorderPreset {
    All,
    None,
    Outside,
    Inside,
    /// One edge, toggled.
    Edge(EdgeKey),
}

impl BorderPreset {
    /// Word's menu order: All, No, Outside, Inside, Top, Bottom, Left, Right, Inside Horizontal,
    /// Inside Vertical.
    pub const MENU: [BorderPreset; 10] = [
        BorderPreset::All,
        BorderPreset::None,
        BorderPreset::Outside,
        BorderPreset::Inside,
        BorderPreset::Edge(EdgeKey::Top),
        BorderPreset::Edge(EdgeKey::Bottom),
        BorderPreset::Edge(EdgeKey::Left),
        BorderPreset::Edge(EdgeKey::Right),
        BorderPreset::Edge(EdgeKey::InsideHorizontal),
        BorderPreset::Edge(EdgeKey::InsideVertical),
    ];

    /// The edges after this preset: All, None, Outside and Inside set theirs (Outside and Inside
    /// leave the other group as it is); one edge toggles.
    // wordcraft: crates/ui-egui/src/dialogs/dialogs_table_style.rs:217 `pick_edges`.
    pub fn apply(self, edges: BorderEdges) -> BorderEdges {
        todo!()
    }
}

/// What the picker did.
#[derive(Debug)]
pub struct EdgePickerResponse {
    pub changed: bool,
    pub response: Response,
}

/// The picker, editing `edges` in place.
///
/// - A 64 pt square with a 1 pt field border, filled grey (0xF2) 8 pt in; the outer rectangle
///   8 pt in and the two inner lines through its middle. Chosen lines are 2.5 pt in the accent
///   colour, others 1 pt grey (0x9A), the line under the pointer 3 pt (DesignCraft); the grey
///   fill and line become tokens in the fill.
/// - Click toggles a line. Double-click on a line picks its group alone (an outer line: the four
///   outer lines on, the inner off; an inner line the other way round). Triple-click turns all on,
///   or all off when all were on (InDesign's proxy).
/// - [`paragraph`](Self::paragraph): no inside vertical line, and the inside horizontal is
///   "Between" paragraphs (Word's paragraph borders).
/// - Keys: one Tab stop (an AccessKit Group "Edges" of six CheckBoxes); ←/→/↑/↓ move between
///   lines, space toggles.
pub struct EdgePicker<'a> {
    id_salt: egui::IdSalt,
    edges: &'a mut BorderEdges,
    paragraph: bool,
    labels: Option<EdgePickerLabels>,
}

impl<'a> EdgePicker<'a> {
    pub fn new(id_salt: impl egui::AsIdSalt, edges: &'a mut BorderEdges) -> Self {
        EdgePicker { id_salt: egui::IdSalt::new(id_salt), edges, paragraph: false, labels: None }
    }

    pub fn paragraph(mut self, on: bool) -> Self {
        self.paragraph = on;
        self
    }

    pub fn labels(mut self, labels: EdgePickerLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // designcraft: crates/ui-egui/src/panels/cell_stroke.rs:110 `edge_proxy`, :91 `proxy_click`,
    // :52 `proxy_lines`; wordcraft: crates/ui-egui/src/dialogs/dialogs_para.rs:554 `borders` (paragraph borders).
    pub fn show(self, ui: &mut Ui) -> EdgePickerResponse {
        todo!()
    }
}
