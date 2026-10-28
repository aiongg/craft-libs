//! [`Readout`]: values beside the pointer during a canvas gesture.

use egui::accesskit::Role;
use egui::{Area, Color32, Context, CornerRadius, Id, Order, Pos2, Sense, Stroke, StrokeKind, Vec2, WidgetInfo, pos2, vec2};

use craft_theme::TypeRole;

use crate::direction;
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};

/// A readout's look, from [`Readout::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ReadoutStyle {
    /// From the pointer to the readout's corner: `readout.offset_x`, `readout.offset_y`.
    pub offset: Vec2,
    /// Around the rows: `readout.pad_x`, `readout.pad_y`.
    pub pad: Vec2,
    /// Between the widest label and the widest value: `readout.col_gap`.
    pub col_gap: f32,
    /// Between rows: `readout.row_gap`.
    pub row_gap: f32,
    /// `surface.raised`.
    pub fill: Color32,
    /// 1 point of `line.border`.
    pub border: Stroke,
    /// `shape.radius.control`.
    pub radius: CornerRadius,
    /// The type role of labels and values: `label`.
    pub role: TypeRole,
    /// Labels: `text.muted`.
    pub label: Color32,
    /// Values: `text.normal`.
    pub value: Color32,
}

/// Where a readout's rows go, relative to its top-left corner.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ReadoutLayout {
    pub size: Vec2,
    /// The top-left corner of each row's label and value.
    pub rows: Vec<(Pos2, Pos2)>,
}

impl ReadoutLayout {
    /// Lays out rows of (label size, value size): labels at the reading start, values aligned on
    /// the end edge, the widest label and the widest value `col_gap` apart.
    pub(crate) fn new(style: &ReadoutStyle, rows: &[(Vec2, Vec2)], rtl: bool) -> Self {
        let label_col = rows.iter().fold(0.0f32, |w, (l, _)| w.max(l.x));
        let value_col = rows.iter().fold(0.0f32, |w, (_, v)| w.max(v.x));
        let width = 2.0 * style.pad.x + label_col + style.col_gap + value_col;
        let mut y = style.pad.y;
        let mut out = Vec::with_capacity(rows.len());
        for (i, (label, value)) in rows.iter().enumerate() {
            if i > 0 {
                y += style.row_gap;
            }
            let height = label.y.max(value.y);
            let (label_x, value_x) = if rtl { (width - style.pad.x - label.x, style.pad.x) } else { (style.pad.x, width - style.pad.x - value.x) };
            out.push((pos2(label_x, y + (height - label.y) / 2.0), pos2(value_x, y + (height - value.y) / 2.0)));
            y += height;
        }
        ReadoutLayout { size: vec2(width, y + style.pad.y), rows: out }
    }
}

/// Rows of labelled values beside the pointer during a canvas gesture, e.g. a marquee's
/// `W:` and `H:` (PhotoCraft's transformation values):
/// `Readout::new("marquee-size").row("W:", w).row("H:", h).show(ctx, pointer)`.
///
/// It floats above everything at the pointer plus `readout.offset`, kept on screen, and takes no
/// input. Labels and values are the host's translated text; the host decides whether to show it
/// (PhotoCraft's Show Transformation Values). In AccessKit it is a tooltip node holding the rows'
/// text.
#[derive(Clone, Debug)]
pub struct Readout {
    id: Id,
    rows: Vec<(String, String)>,
}

impl Readout {
    /// An empty readout; `id_source`, unique in the app, keeps its area apart from other
    /// readouts.
    pub fn new(id_source: impl std::hash::Hash + std::fmt::Debug) -> Self {
        Self { id: Id::unique(id_source), rows: Vec::new() }
    }

    /// Adds a row: `label` at the start, `value` aligned at the end.
    pub fn row(mut self, label: impl Into<String>, value: impl Into<String>) -> Self {
        self.rows.push((label.into(), value.into()));
        self
    }

    pub fn style(tokens: &UiTokens) -> ReadoutStyle {
        let (r, c) = (&tokens.component.readout, &tokens.color);
        ReadoutStyle {
            offset: vec2(r.offset_x, r.offset_y),
            pad: vec2(r.pad_x, r.pad_y),
            col_gap: r.col_gap,
            row_gap: r.row_gap,
            fill: c.surface.raised.to_egui(),
            border: Stroke::new(1.0, c.line.border.to_egui()),
            radius: tokens.radius.control,
            role: TypeRole::Label,
            label: c.text.muted.to_egui(),
            value: c.text.normal.to_egui(),
        }
    }

    /// Shows the readout beside `pointer` for this pass.
    pub fn show(self, ctx: &Context, pointer: Pos2) {
        let offset = Readout::style(&crate::theme::current(ctx)).offset;
        Area::new(self.id).order(Order::Tooltip).fixed_pos(pointer + offset).interactable(false).constrain(true).show(ctx, |ui| {
            let tokens = Tokens::of(ui);
            let style = Readout::style(&tokens);
            let font = tokens.font(style.role);
            let painter = ui.painter();
            let galleys: Vec<_> = self
                .rows
                .iter()
                .map(|(l, v)| {
                    (painter.layout_no_wrap(l.clone(), font.clone(), style.label), painter.layout_no_wrap(v.clone(), font.clone(), style.value))
                })
                .collect();
            let sizes: Vec<_> = galleys.iter().map(|(l, v)| (l.size(), v.size())).collect();
            let layout = ReadoutLayout::new(&style, &sizes, direction::is_rtl(ui));
            let (rect, response) = ui.allocate_exact_size(layout.size, Sense::hover());
            let painter = ui.painter();
            painter.rect(rect, style.radius, style.fill, style.border, StrokeKind::Inside);
            for ((label, value), (lp, vp)) in galleys.into_iter().zip(layout.rows) {
                painter.galley(rect.min + lp.to_vec2(), label, style.label);
                painter.galley(rect.min + vp.to_vec2(), value, style.value);
            }
            let text = self.rows.iter().map(|(l, v)| format!("{l} {v}")).collect::<Vec<_>>().join("\n");
            response.widget_info(|| WidgetInfo::labeled(Role::Tooltip, true, &text));
        });
    }
}
