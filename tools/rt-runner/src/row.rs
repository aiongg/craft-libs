//! `ListRow`: one layer row (look sheet 3.1-3.9, 3.11, 6). The row paints its fill, rules,
//! thumbnail, name and blend label; the toggle cells, the lock indicator and the rename field
//! are its children; its access node carries the cells and switches as sub-nodes.

use std::sync::Arc;

use crate::fixture::{Blend, Kind, Layer, LayerId, Op, SelectMode};
use accesskit::{Action, Node, Role, Toggled};
use craft_theme::FontSlot;
use craft_ui_rt::{
    AccessCx, BezPath, BuildCx, Constraints, Element, Event, EventCx, FontSpec, Handled, ImageKey, ImageStore, LayoutCx, NodeId, Overflow, PaintCx,
    Pos2, Rect, Rgba, RgbaImage, Sense, SizeU, Stroke, StrokeAlign, TextLayout, TextSpec, Vec2, hash_key,
};

use crate::controls::{ButtonKind, IconButton, ToggleCell, ToggleFace};
use crate::doc::{DocCell, edit};
use crate::field::rename_style;
use crate::icons::{self, Icon};
use crate::look::{Edge, KIND_GLYPH_SHARE, Look, TYPE_INK, TYPE_PAPER, icon_square, snap, thumb_texture_side};
use crate::text_field::TextField;

/// The row's hit parts.
pub const PART_ROW: u64 = 0;
pub const PART_THUMB: u64 = 1;
/// The name band: a double click here starts rename.
pub const PART_NAME: u64 = 2;

const EYE: ToggleFace = ToggleFace { on: Icon::Eye, off_hover: None };
const LOCK: ToggleFace = ToggleFace { on: Icon::Lock, off_hover: Some(Icon::LockOpen) };

/// Sub-node keys of the row's access node.
const SUB_LABEL_CELL: u64 = 1;
const SUB_EYE_CELL: u64 = 2;
const SUB_EYE: u64 = 3;
const SUB_LOCK_CELL: u64 = 4;
const SUB_LOCK: u64 = 5;

/// What a row shows; the list builds it per row in view.
#[derive(Clone, Debug)]
pub struct RowProps {
    pub layer: Layer,
    pub index: usize,
    pub count: usize,
    pub selected: bool,
    /// The document's active layer: its thumbnail carries the brackets.
    pub active: bool,
    /// The list's lead row: the list's focus ring and active access item.
    pub lead: bool,
    pub renaming: bool,
    /// The list node, whose focus decides the lead's ring.
    pub list: NodeId,
    /// The rows Tab moves a rename to.
    pub prev_rename: Option<LayerId>,
    pub next_rename: Option<LayerId>,
}

pub struct ListRow {
    p: RowProps,
    doc: DocCell,
    rename: craft_ui_rt::State<Option<LayerId>>,
}

impl ListRow {
    pub fn new(p: RowProps, doc: DocCell, rename: craft_ui_rt::State<Option<LayerId>>) -> Self {
        ListRow { p, doc, rename }
    }

    fn ink(&self, look: &Look) -> Rgba {
        if self.p.selected {
            look.text_on_selected
        } else if !self.p.layer.visible {
            look.text_faint
        } else {
            look.text
        }
    }

    /// The row's description (`row_description`, look sheet section 6).
    fn description(&self) -> String {
        let l = &self.p.layer;
        let mut parts = vec![match l.kind {
            Kind::Pixel => "pixel layer",
            Kind::Type => "type layer",
            Kind::Fill => "fill layer",
        }];
        if l.locked {
            parts.push("locked");
        }
        if !l.visible {
            parts.push("hidden");
        }
        if self.p.active {
            parts.push("pixels targeted");
        }
        parts.join(", ")
    }

    /// `room` is the width the line may take; a longer text is cut with "…".
    /// A one-line run at the font's own height: centred in the row, it lands where craft's paint
    /// puts it (the galley centred in the line box, look sheet 3.5).
    fn spec(&self, text: &str, size: f32, color: Rgba, room: Option<f32>) -> TextSpec {
        TextSpec {
            text: text.into(),
            font: FontSpec { slot: FontSlot::Ui, size },
            line_height: None,
            color,
            max_width: room.map(|w| w.max(0.0) as f32),
            overflow: if room.is_some() { Overflow::Ellipsis } else { Overflow::Wrap },
        }
    }

    fn blend_text(&self) -> &'static str {
        match self.p.layer.blend {
            Blend::Normal => "",
            blend => blend.label(),
        }
    }
}

/// What build registered and layout measured; paint draws from it.
#[derive(Default)]
pub struct RowLocal {
    picture: Option<ImageKey>,
    glyph: Option<ImageKey>,
    name: Option<TextLayout>,
    blend: Option<(TextLayout, f32)>,
    label_end: f32,
    /// The width of an eye or lock cell.
    cell_w: f32,
    /// The text typed into the rename field so far.
    draft: Option<String>,
}

fn kind_code(kind: Kind) -> u8 {
    match kind {
        Kind::Pixel => 0,
        Kind::Type => 1,
        Kind::Fill => 2,
    }
}

/// The thumbnail's picture, registered once per (layer look, side in pixels).
fn picture(images: &mut ImageStore, layer: &Layer, side: u32) -> ImageKey {
    let key = ImageKey(hash_key(&("thumb", layer.seed, kind_code(layer.kind), side)));
    if images.get(key).is_none() {
        let image = RgbaImage { size: SizeU::new(side, side), pixels: layer.thumbnail(side) };
        images.insert(key, Arc::new(image));
    }
    key
}

impl Element for ListRow {
    type Output = ();
    type Local = RowLocal;

    fn props_key(&self) -> u64 {
        let p = &self.p;
        let l = &p.layer;
        hash_key(&(
            (l.id, l.name.as_str(), kind_code(l.kind), l.visible, l.locked, l.blend == Blend::Multiply, l.seed),
            (p.index, p.count, p.selected, p.active, p.lead, p.renaming),
            (p.list.0, p.prev_rename, p.next_rename),
        ))
    }

    fn init(&self, _: &mut BuildCx<'_>) -> RowLocal {
        RowLocal::default()
    }

    fn build(&self, local: &mut RowLocal, cx: &mut BuildCx<'_>) {
        let look = Look::of(&cx.tokens());
        let scale = cx.env().scale.get(cx);
        let layer = &self.p.layer;
        let inner = look.thumb - 2.0 * look.thin;
        local.picture = None;
        local.glyph = None;
        match layer.kind {
            // A solid fill's picture is its colour, as PhotoCraft renders it.
            Kind::Pixel | Kind::Fill => {
                local.picture = Some(picture(cx.images(), layer, thumb_texture_side(inner * scale)));
            }
            Kind::Type => {
                let (px, stroke) = icons::pixels(inner * KIND_GLYPH_SHARE, scale, look.icon_stroke);
                local.glyph = Some(icons::mask(cx, Icon::Text, px, stroke));
            }
        }

        let (doc, id, selected) = (self.doc, layer.id, self.p.selected);
        cx.keyed("eye", |cx| {
            cx.child(ToggleCell::new(EYE, layer.visible, selected).on_toggle(move |cx| edit(doc, cx, Op::ToggleVisible(id))));
        });
        cx.keyed("lock", |cx| {
            cx.child(ToggleCell::new(LOCK, layer.locked, selected).on_toggle(move |cx| edit(doc, cx, Op::ToggleLock(id))));
        });
        if layer.locked {
            cx.keyed("indicator", |cx| {
                let button = IconButton::new(ButtonKind::Indicator, Icon::Lock, format!("Unlock {}", layer.name))
                    .on_press(move |cx| edit(doc, cx, Op::ToggleLock(id)));
                cx.child(button);
            });
        }
        if self.p.renaming {
            let field = TextField::new(layer.name.as_str(), rename_style(&look)).select_all(true).autofocus(true).label(layer.name.as_str());
            let out = cx.keyed("rename", |cx| cx.child(field));
            self.apply_rename(local, cx, out);
        } else {
            local.draft = None;
        }
    }

    fn layout(&self, local: &mut RowLocal, cx: &mut LayoutCx<'_>, c: Constraints) -> Vec2 {
        let look = Look::of(&cx.tokens());
        let size = c.constrain(Vec2::new(c.max.x, look.row_height));
        let (w, h) = (size.x, size.y);
        let layer = &self.p.layer;
        let cell = Constraints::tight(Vec2::new(look.toggle_width, h));
        for (i, x) in [(0, 0.0), (1, look.toggle_width)] {
            let child = cx.child_at(i);
            cx.layout_child(child, cell);
            cx.place_child(child, Vec2::new(x, 0.0));
        }
        let mut next = 2;
        let mut end = w - look.trailing_pad;
        if layer.locked {
            let child = cx.child_at(next);
            next += 1;
            cx.layout_child(child, Constraints::tight(Vec2::new(look.icon, look.icon)));
            cx.place_child(child, Vec2::new(end - look.icon, (h - look.icon) / 2.0));
            end -= look.icon + look.gap;
        }
        local.blend = None;
        let blend = self.blend_text();
        if !blend.is_empty() {
            let text = cx.text_layout(&self.spec(blend, look.caption_size, look.text_muted, None));
            let width = text.size.x;
            if end - (width + look.gap) - look.label_x() >= look.min_label {
                local.blend = Some((text, end - width));
                end -= width + look.gap;
            }
        }
        local.label_end = end;
        local.cell_w = look.toggle_width;
        local.name = Some(cx.text_layout(&self.spec(&layer.name, look.body_size, look.text, Some(end - look.label_x()))));
        if self.p.renaming && next < cx.child_count() {
            let child = cx.child_at(next);
            let width = (end - look.label_x()).max(0.0);
            // craft shows the field in the label's rect, row-high (`show_at(label_rect)`).
            cx.layout_child(child, Constraints::tight(Vec2::new(width, h)));
            cx.place_child(child, Vec2::new(look.label_x(), 0.0));
        }
        size
    }

    fn paint(&self, local: &RowLocal, cx: &mut PaintCx<'_>) {
        let look = Look::of(&cx.tokens());
        let scale = cx.scale();
        let bounds = cx.bounds();
        let (w, h) = (bounds.width(), bounds.height());
        let state = cx.interaction();
        let list = cx.interaction_of(self.p.list);
        let list_focused = self.p.lead && list.focused;
        let ring = self.p.lead && list.focus_visible;

        cx.hit(bounds, Sense::HOVER | Sense::CLICK | Sense::DRAG, PART_ROW);
        let slot = Rect::from_min_size(Pos2::new(look.thumb_x(), (h - look.thumb) / 2.0), Vec2::new(look.thumb, look.thumb));
        cx.hit(slot, Sense::HOVER | Sense::CLICK | Sense::DRAG, PART_THUMB);
        let label = crate::geo::rect(look.label_x(), 0.0, local.label_end.max(look.label_x()), h);
        cx.hit(label, Sense::HOVER | Sense::CLICK | Sense::DRAG, PART_NAME);

        let fill = if self.p.selected {
            Some(look.selected)
        } else if state.pressed {
            Some(look.pressed)
        } else if state.hovered || list_focused {
            Some(look.hover)
        } else {
            None
        };
        if let Some(fill) = fill {
            cx.fill_rect(bounds, fill);
        }
        if self.p.index + 1 < self.p.count {
            look.edge_rule(cx, crate::geo::rect(0.0, 0.0, w, h), Edge::Bottom);
        }
        // Each toggle cell's rule lies just past the cell.
        for x in [look.toggle_width, look.band_x()] {
            look.edge_rule(cx, crate::geo::rect(x, 0.0, w, h), Edge::Left);
        }

        self.paint_thumb(local, cx, &look, scale, slot);

        let ink = self.ink(&look);
        if !self.p.renaming
            && let Some(name) = &local.name
        {
            let y = (h - name.size.y) / 2.0;
            cx.text(name, Pos2::new(look.label_x(), y), Some(ink));
        }
        if let Some((blend, x)) = &local.blend {
            let y = (h - blend.size.y) / 2.0;
            cx.text(blend, Pos2::new(*x, y), Some(look.text_muted));
        }
        if self.p.active {
            brackets(cx, slot, &look, ink);
        }
        if ring {
            let ring = Stroke::new(look.focus_width, look.focus);
            let rect = snap(bounds, scale);
            cx.over(|cx| cx.rect(rect, 0.0, Rgba::TRANSPARENT, Some(ring)));
        }
    }

    fn event(&self, local: &mut RowLocal, cx: &mut EventCx<'_>, ev: &Event) -> Handled {
        let Event::Access(a) = ev else { return Handled::No };
        let id = self.p.layer.id;
        match (a.action, a.sub) {
            (Action::Click, Some(SUB_EYE)) => edit(self.doc, cx, Op::ToggleVisible(id)),
            (Action::Click, Some(SUB_LOCK)) => edit(self.doc, cx, Op::ToggleLock(id)),
            (Action::Click, None) => edit(self.doc, cx, Op::Select(id, SelectMode::Replace)),
            _ => return Handled::No,
        }
        let _ = local;
        Handled::Yes
    }

    fn access(&self, local: &RowLocal, cx: &mut AccessCx<'_>) -> Option<Node> {
        let l = &self.p.layer;
        let size = cx.size();
        let rect = |x0: f32, x1: f32| accesskit::Rect::new(f64::from(x0), 0.0, f64::from(x1), f64::from(size.y));
        let cell = |bounds| {
            let mut n = Node::new(Role::Cell);
            n.set_label(l.name.as_str());
            n.set_selected(self.p.selected);
            n.set_bounds(bounds);
            n
        };
        let switch = |name: String, on: bool, bounds| {
            let mut n = Node::new(Role::Switch);
            n.set_label(name);
            n.set_toggled(if on { Toggled::True } else { Toggled::False });
            n.add_action(Action::Click);
            n.set_bounds(bounds);
            n
        };
        let (eye, lock, band) = (rect(0.0, local.cell_w), rect(local.cell_w, 2.0 * local.cell_w), rect(2.0 * local.cell_w, size.x));
        cx.sub(SUB_LABEL_CELL, None, cell(band));
        cx.sub(SUB_EYE_CELL, None, cell(eye));
        cx.sub(SUB_EYE, Some(SUB_EYE_CELL), switch(format!("Visibility {}", l.name), l.visible, eye));
        cx.sub(SUB_LOCK_CELL, None, cell(lock));
        cx.sub(SUB_LOCK, Some(SUB_LOCK_CELL), switch(format!("Lock {}", l.name), l.locked, lock));
        if self.p.lead {
            cx.set_active();
        }
        let mut node = Node::new(Role::TreeItem);
        node.set_label(l.name.as_str());
        node.set_description(self.description());
        node.set_selected(self.p.selected);
        node.set_level(1);
        node.set_position_in_set(self.p.index + 1);
        node.set_size_of_set(self.p.count);
        node.add_action(Action::Click);
        node.add_action(Action::Focus);
        Some(node)
    }

    fn tag(&self) -> u64 {
        self.p.layer.id + 1
    }
}

impl ListRow {
    /// What the rename field reported: commit, cancel or move on, as look sheet 5.5 says.
    fn apply_rename(&self, local: &mut RowLocal, cx: &mut BuildCx<'_>, out: crate::text_field::TextFieldOutput) {
        let id = self.p.layer.id;
        if let Some(text) = out.edited {
            local.draft = Some(text);
        }
        let commit = |cx: &mut BuildCx<'_>, text: &str| {
            if let Some(name) = crate::list_rules::rename_commit(&self.p.layer.name, text) {
                edit(self.doc, cx, Op::Rename(id, name));
            }
        };
        if let Some((text, keep)) = out.submit {
            commit(cx, &text);
            if !keep {
                self.rename.set(cx, None);
            }
        } else if out.cancel {
            self.rename.set(cx, None);
        } else if let Some(back) = out.tab {
            if let Some(text) = local.draft.take() {
                commit(cx, &text);
            }
            let to = if back { self.p.prev_rename } else { self.p.next_rename };
            self.rename.set(cx, to);
        } else if out.blur {
            if let Some(text) = local.draft.take() {
                commit(cx, &text);
            }
            self.rename.set(cx, None);
        }
    }

    fn paint_thumb(&self, local: &RowLocal, cx: &mut PaintCx<'_>, look: &Look, scale: f32, slot: Rect) {
        let picture = slot.shrink(look.thin);
        let glyph_side = picture.width().min(picture.height()) * KIND_GLYPH_SHARE;
        match self.p.layer.kind {
            Kind::Pixel | Kind::Fill => {
                // The transparency grid lies under a picture that covers it whole.
                if let Some(key) = local.picture {
                    cx.image(key, snap(picture, scale), Rgba::WHITE);
                }
            }
            Kind::Type => {
                let rect = snap(picture, scale);
                cx.fill_rect(rect, TYPE_PAPER);
                if let Some(key) = local.glyph {
                    let (square, _) = icon_square(rect.center(), glyph_side, scale);
                    cx.image(key, square, TYPE_INK);
                }
            }
        }
        cx.rect(slot, 0.0, Rgba::TRANSPARENT, Some(Stroke::new(look.thin, look.frame)));
    }
}

/// The four corner brackets of the active layer's thumbnail (look sheet 3.4).
fn brackets(cx: &mut PaintCx<'_>, slot: Rect, look: &Look, ink: Rgba) {
    let r = slot.expand(look.tight);
    let arm = look.gap;
    let stroke = Stroke { width: look.medium, color: ink, align: StrokeAlign::Middle, dash: None };
    let corners = [(r.min.x, r.min.y, 1.0, 1.0), (r.max.x, r.min.y, -1.0, 1.0), (r.max.x, r.max.y, -1.0, -1.0), (r.min.x, r.max.y, 1.0, -1.0)];
    for (x, y, sx, sy) in corners {
        let mut path = BezPath::new();
        path.move_to(kurbo::Point::new(f64::from(x + sx * arm), f64::from(y)));
        path.line_to(kurbo::Point::new(f64::from(x), f64::from(y)));
        path.line_to(kurbo::Point::new(f64::from(x), f64::from(y + sy * arm)));
        cx.path(Arc::new(path), None, Some(stroke));
    }
}
