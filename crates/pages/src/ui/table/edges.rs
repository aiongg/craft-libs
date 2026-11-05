//! [`EdgePicker`] (N3.07): the diagram of a cell range's (or a paragraph's) six edges, as
//! InDesign's Stroke proxy in the Table and Cell Styles panels and Word's Borders dialog show it:
//! top, bottom, left, right, inside horizontal and inside vertical, each line on or off; and
//! Word's border presets (All, None, Outside, Inside, one edge) as menu items.

use craft_ui::behavior::GridNav;
use craft_ui::direction;
use craft_ui::localized::Labels as _;
use craft_ui::primitives::{Interactive, Surface, SurfaceKind};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _};
use egui::accesskit::Role;
use egui::{Pos2, Rect, Response, Sense, Stroke, Ui, UiBuilder, pos2, vec2};

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
        /// The paragraph picker's name for the inside horizontal line (DesignCraft).
        between: "edges.between" = "Between";
        /// The tooltip on the picker's square (InDesign's Stroke proxy).
        hint: "edges.hint" = "Click a line to include or exclude it. Double-click selects all outer or all inner lines; triple-click selects or clears all.";
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
    #[allow(missing_docs)]
    Top,
    #[allow(missing_docs)]
    Bottom,
    #[allow(missing_docs)]
    Left,
    #[allow(missing_docs)]
    Right,
    #[allow(missing_docs)]
    InsideHorizontal,
    #[allow(missing_docs)]
    InsideVertical,
}

impl EdgeKey {
    /// The six edges in the picker's order.
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
    /// Every edge chosen.
    pub const ALL: BorderEdges = BorderEdges([true; 6]);

    /// Whether `edge` is chosen.
    pub fn get(self, edge: EdgeKey) -> bool {
        self.0[edge as usize]
    }

    /// Chooses or clears `edge`.
    pub fn set(&mut self, edge: EdgeKey, on: bool) {
        self.0[edge as usize] = on;
    }
}

/// Word's border presets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BorderPreset {
    #[allow(missing_docs)]
    All,
    #[allow(missing_docs)]
    None,
    #[allow(missing_docs)]
    Outside,
    #[allow(missing_docs)]
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
        let mut out = edges;
        match self {
            BorderPreset::All => out = BorderEdges::ALL,
            BorderPreset::None => out = BorderEdges::default(),
            BorderPreset::Outside => OUTER.iter().for_each(|&e| out.set(e, true)),
            BorderPreset::Inside => INNER.iter().for_each(|&e| out.set(e, true)),
            BorderPreset::Edge(edge) => out.set(edge, !edges.get(edge)),
        }
        out
    }
}

const OUTER: [EdgeKey; 4] = [EdgeKey::Top, EdgeKey::Bottom, EdgeKey::Left, EdgeKey::Right];
const INNER: [EdgeKey; 2] = [EdgeKey::InsideHorizontal, EdgeKey::InsideVertical];

/// What the picker did.
#[derive(Debug)]
pub struct EdgePickerResponse {
    /// Whether an edge was toggled this pass.
    pub changed: bool,
    #[allow(missing_docs)]
    pub response: Response,
}

/// The picker, editing `edges` in place.
///
/// - A square of [`EdgePickerStyle`](crate::ui::EdgePickerStyle)'s side with a field border,
///   filled with its fill inside the inset; the outer rectangle at the inset and the two inner
///   lines through its middle. Chosen lines in the accent at `line_on`, others in
///   `line_off_color` at `line_off`, the line under the pointer at `line_hover` (DesignCraft;
///   InDesign's proxy).
/// - The pointer picks the nearest line within the inset less one point (`hit_reach`).
/// - Click toggles a line. Double-click on a line picks its group alone (an outer line: the four
///   outer lines on, the inner off; an inner line the other way round). Triple-click turns all on,
///   or all off when all were on (InDesign's proxy).
/// - [`paragraph`](Self::paragraph): no inside vertical line, and the inside horizontal is
///   "Between" paragraphs (Word's paragraph borders).
/// - Keys: one Tab stop (an AccessKit Group "Edges" of six CheckBoxes); ←/→/↑/↓ move between
///   lines through craft-ui's [`GridNav`] over the lines' ragged
///   rows `[Top] [Left, InsideH, InsideV, Right] [Bottom]` (`GridNav::ragged`; the paragraph
///   picker's middle row has no InsideV), Space toggles. The edges are the panel's choice of
///   which lines a stroke applies to (view state, not the document), so the picker edits them in
///   place and reports [`EdgePickerResponse::changed`].
pub struct EdgePicker<'a> {
    id_salt: egui::IdSalt,
    edges: &'a mut BorderEdges,
    paragraph: bool,
    labels: Option<EdgePickerLabels>,
}

impl<'a> EdgePicker<'a> {
    /// The picker's sizes and colours, from the tokens.
    pub fn style(tokens: &craft_ui::theme::UiTokens) -> crate::ui::EdgePickerStyle {
        use craft_ui::theme::ToEgui as _;
        let t = &tokens.component.pages;
        crate::ui::EdgePickerStyle {
            side: t.edge_picker_side,
            inset: t.edge_picker_inset,
            fill: t.edge_picker_fill.to_egui(),
            line_on: t.edge_picker_line_on,
            line_off: t.edge_picker_line_off,
            line_off_color: t.edge_picker_line_off_color.to_egui(),
            line_hover: t.edge_picker_line_hover,
        }
    }

    /// A picker editing `edges` in place.
    pub fn new(id_salt: impl egui::AsIdSalt, edges: &'a mut BorderEdges) -> Self {
        EdgePicker { id_salt: egui::IdSalt::new(id_salt), edges, paragraph: false, labels: None }
    }

    /// Switches to paragraph borders: no inside vertical line, and the inside horizontal line is the line between paragraphs.
    pub fn paragraph(mut self, on: bool) -> Self {
        self.paragraph = on;
        self
    }

    /// Sets the translated labels; the default reads them from the interface locale.
    pub fn labels(mut self, labels: EdgePickerLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // designcraft: crates/ui-egui/src/panels/cell_stroke.rs:110 `edge_proxy`, :91 `proxy_click`,
    // :52 `proxy_lines`; wordcraft: crates/ui-egui/src/dialogs/dialogs_para.rs:554 `borders` (paragraph borders).
    /// Paints the picker and applies clicks and keys to `edges`.
    pub fn show(self, ui: &mut Ui) -> EdgePickerResponse {
        let tokens = Tokens::of(ui);
        let style = Self::style(&tokens);
        let labels = self.labels.clone().unwrap_or_else(|| EdgePickerLabels::of(ui));
        let keys = visible_keys(self.paragraph);
        let (_, rect) = ui.allocate_space(vec2(style.side, style.side));
        let group = ui.new_child(
            UiBuilder::new().id_salt(self.id_salt).max_rect(rect).accessibility_role(Role::Group).accessibility_label(labels.picker.as_str()),
        );
        let lines = edge_lines(rect, style.inset);

        // Lines in reading order: the top, the middle row from the left, the bottom.
        let order = reading_order(self.paragraph);
        let ids: Vec<egui::Id> = order.iter().map(|&k| Interactive::id_in(&group, k.id())).collect();
        let nav = GridNav::ragged(order_rows(&order)).rtl(direction::is_rtl(&group));
        let (stop, _) = GridNav::roving_move_in(&group, &ids, &nav, None, true);

        let mut changed = false;
        for (cell, &key) in order.iter().enumerate() {
            let (a, b) = lines[key as usize];
            let sense = if cell == stop { Sense::click() } else { Sense::CLICK };
            let name = edge_name(&labels, key, self.paragraph);
            let sensed = Interactive::with_id(ids[cell], Role::CheckBox, name)
                .checked(self.edges.get(key))
                .sense(sense)
                .interact_at(&group, Rect::from_two_pos(a, b).expand(style.inset / 2.0));
            if sensed.response().clicked() {
                self.edges.set(key, !self.edges.get(key));
                changed = true;
            }
            sensed.finish(&group);
        }

        // The pointer is read last, so a press on the square goes to the nearest line whichever
        // line's focus rectangle it falls in.
        let square = group.interact(rect, group.scope_id().with("pointer"), Sense::CLICK);
        let hit = square.interact_pointer_pos().or_else(|| square.hover_pos()).and_then(|p| line_at(&lines, &keys, p, hit_reach(style.inset)));
        if square.clicked() {
            let last_key = square.id.with("last");
            let start_key = square.id.with("start");
            let last: Option<EdgeKey> = group.data(|d| d.get_temp(last_key)).flatten();
            let double = square.double_clicked();
            let triple = square.triple_clicked();
            if !double && !triple {
                group.data_mut(|d| d.insert_temp(start_key, *self.edges));
            }
            let start: BorderEdges = group.data(|d| d.get_temp(start_key)).unwrap_or(*self.edges);
            changed |= pointer_click(self.edges, &keys, PointerClick { start, hit, last, double, triple });
            group.data_mut(|d| d.insert_temp(last_key, hit));
        }

        if group.is_rect_visible(rect) {
            let painter = group.painter();
            Surface::paint(painter, rect, SurfaceKind::Sunken, &tokens);
            painter.rect_filled(rect.shrink(style.inset), 0.0, style.fill);
            let accent = tokens.color.accent.solid.to_egui();
            for &key in &keys {
                let (a, b) = lines[key as usize];
                let on = self.edges.get(key);
                let width = if hit == Some(key) {
                    style.line_hover
                } else if on {
                    style.line_on
                } else {
                    style.line_off
                };
                painter.line_segment([a, b], Stroke::new(width, if on { accent } else { style.line_off_color }));
            }
        }
        let mut response = square.on_hover_text(labels.hint.as_str());
        if changed {
            response.mark_changed();
        }
        EdgePickerResponse { changed, response }
    }
}

/// The lines drawn: all six, without the inside vertical in a paragraph.
fn visible_keys(paragraph: bool) -> Vec<EdgeKey> {
    EdgeKey::ALL.into_iter().filter(|&k| !(paragraph && k == EdgeKey::InsideVertical)).collect()
}

/// The lines in the order the arrow keys walk them: the top, the left, inside and right lines of
/// the middle row, the bottom.
fn reading_order(paragraph: bool) -> Vec<EdgeKey> {
    [EdgeKey::Top, EdgeKey::Left, EdgeKey::InsideHorizontal, EdgeKey::InsideVertical, EdgeKey::Right, EdgeKey::Bottom]
        .into_iter()
        .filter(|&k| !(paragraph && k == EdgeKey::InsideVertical))
        .collect()
}

/// The row lengths of [`reading_order`]: the top alone, the middle lines, the bottom alone.
fn order_rows(order: &[EdgeKey]) -> [usize; 3] {
    [1, order.len() - 2, 1]
}

fn edge_name(labels: &EdgePickerLabels, key: EdgeKey, paragraph: bool) -> String {
    match key {
        EdgeKey::Top => labels.top.clone(),
        EdgeKey::Bottom => labels.bottom.clone(),
        EdgeKey::Left => labels.left.clone(),
        EdgeKey::Right => labels.right.clone(),
        EdgeKey::InsideHorizontal if paragraph => labels.between.clone(),
        EdgeKey::InsideHorizontal => labels.inside_horizontal.clone(),
        EdgeKey::InsideVertical => labels.inside_vertical.clone(),
    }
}

/// The six lines (start, end) in [`EdgeKey::ALL`] order: the outer rectangle at `inset` and the
/// two inner lines through its middle.
pub(super) fn edge_lines(rect: Rect, inset: f32) -> [(Pos2, Pos2); 6] {
    let i = rect.shrink(inset);
    let c = i.center();
    [
        (i.left_top(), i.right_top()),
        (i.left_bottom(), i.right_bottom()),
        (i.left_top(), i.left_bottom()),
        (i.right_top(), i.right_bottom()),
        (pos2(i.left(), c.y), pos2(i.right(), c.y)),
        (pos2(c.x, i.top()), pos2(c.x, i.bottom())),
    ]
}

/// How much nearer than the inset the pointer must be to a line to pick it: DesignCraft's
/// `d <= PROXY_INSET - 1.0` (`cell_stroke.rs:91`), so the lines' hit zones stay short of each
/// other and of the square's edge.
const REACH_SHORTFALL: f32 = 1.0;

/// The distance from a line within which the pointer picks it.
pub(super) fn hit_reach(inset: f32) -> f32 {
    inset - REACH_SHORTFALL
}

/// The line nearest `p` among `keys`, if one is within `reach`.
pub(super) fn line_at(lines: &[(Pos2, Pos2); 6], keys: &[EdgeKey], p: Pos2, reach: f32) -> Option<EdgeKey> {
    keys.iter()
        .map(|&key| {
            let (a, b) = lines[key as usize];
            let t = ((p - a).dot(b - a) / (b - a).length_sq()).clamp(0.0, 1.0);
            (key, p.distance(a + (b - a) * t))
        })
        .filter(|&(_, distance)| distance <= reach)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(key, _)| key)
}

/// A click of the pointer on the square.
struct PointerClick {
    /// The choice before the first click of this click sequence.
    start: BorderEdges,
    /// The line clicked, and the line of the click before.
    hit: Option<EdgeKey>,
    last: Option<EdgeKey>,
    double: bool,
    triple: bool,
}

/// Applies a click to `edges` over the lines in `keys`; whether the choice changed. A double
/// click counts only on the line of the click before it; a triple-click decides from the choice
/// before the sequence's first click.
fn pointer_click(edges: &mut BorderEdges, keys: &[EdgeKey], click: PointerClick) -> bool {
    if click.triple {
        let all = keys.iter().all(|&k| click.start.get(k));
        keys.iter().for_each(|&k| edges.set(k, !all));
        return true;
    }
    let Some(hit) = click.hit else { return false };
    if click.double && click.last == Some(hit) {
        let outer = OUTER.contains(&hit);
        for &k in keys {
            edges.set(k, OUTER.contains(&k) == outer);
        }
    } else {
        edges.set(hit, !edges.get(hit));
    }
    true
}
