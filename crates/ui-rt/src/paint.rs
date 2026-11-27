use std::sync::Arc;

use craft_behavior::Sense;
use craft_theme::{Rgba, Tokens};
use emath::{Pos2, Rect, Vec2};
use kurbo::BezPath;
use smallvec::SmallVec;

use crate::display::{Clip, DisplayList, Fill, ImageKey, Item, Mesh, Stroke, SurfaceKey, TextHighlight};
use crate::frame::Part;
use crate::geom::CornerRadii;
use crate::hit::{HitKind, HitRecord, HitShape};
use crate::id::{NodeId, WindowId};
use crate::platform::Cursor;
use crate::runtime::Runtime;
use crate::state::{Access, Phase, Reader, Reads};
use crate::text::TextLayout;
use crate::tree::Dirty;
use crate::window::Env;

/// Records one node's display lists: `content` under its children, `overlay` over them.
pub struct PaintCx<'a> {
    rt: &'a mut Runtime,
    node: NodeId,
    window: WindowId,
    size: Vec2,
    content: DisplayList,
    overlay: DisplayList,
    part: Part,
    clip: Option<Clip>,
    opacity: f32,
    hits: Vec<HitRecord>,
    cursor: Option<Cursor>,
    window_drop: bool,
    close_guard: bool,
}

impl Reads for PaintCx<'_> {
    fn access(&mut self) -> Access<'_> {
        Access { store: &mut self.rt.store, reader: Some(Reader::Node(self.node, Phase::Paint)) }
    }
}

impl PaintCx<'_> {
    pub fn node(&self) -> NodeId {
        self.node
    }

    pub(crate) fn runtime_mut(&mut self) -> &mut Runtime {
        self.rt
    }

    /// The nearest value of type `T` this node or a node above it provides
    /// ([`crate::BuildCx::provide`]).
    pub fn provided<T: Clone + 'static>(&self) -> Option<T> {
        self.rt.tree.provided::<T>(self.node)
    }

    /// Records a hit shape in local coordinates, at the current level: shapes recorded inside
    /// [`PaintCx::over`] sit above the node's children. `part` tells the node's parts apart in
    /// events (0 when it has one).
    pub fn hit(&mut self, shape: impl Into<HitShape>, sense: Sense, part: u64) {
        self.hits.push(HitRecord { shape: shape.into(), sense, part, level: self.part, kind: HitKind::Input, cursor: None });
    }

    /// [`PaintCx::hit`] with the cursor the pointer shows over the shape, and keeps while a press
    /// that went down on it runs (a splitter's resize arrows).
    pub fn hit_cursor(&mut self, shape: impl Into<HitShape>, sense: Sense, part: u64, cursor: Cursor) {
        self.hits.push(HitRecord { shape: shape.into(), sense, part, level: self.part, kind: HitKind::Input, cursor: Some(cursor) });
    }

    /// The cursor the pointer shows over this node and its descendants, where no nearer node or
    /// shape names one (a canvas tool's brush image, or [`Cursor::Hidden`]). Kept with the paint:
    /// a node changes its cursor by repainting.
    pub fn cursor(&mut self, cursor: Cursor) {
        self.cursor = Some(cursor);
    }

    /// Records a shape where a drag may drop, in local coordinates. While a drag is over it the
    /// node gets [`crate::Event::Drop`] (bubbling to ancestors until one accepts).
    pub fn drop_target(&mut self, shape: impl Into<HitShape>, part: u64) {
        self.hits.push(HitRecord { shape: shape.into(), sense: Sense::empty(), part, level: self.part, kind: HitKind::Drop, cursor: None });
    }

    /// Records a place, in local coordinates, where a primary press holds the focused node's draft:
    /// the focused node gets [`crate::Event::PressKept`] with `holds`, and keeps its focus when it
    /// holds drafts, whatever shape takes the press. The shape takes no input itself.
    pub fn holds_drafts(&mut self, shape: impl Into<HitShape>) {
        self.hits.push(HitRecord { shape: shape.into(), sense: Sense::empty(), part: 0, level: self.part, kind: HitKind::Hold, cursor: None });
    }

    /// Records a place the operating system acts on in local coordinates: a press there moves the
    /// window, resizes it or is a caption button, as the host's `tag` says
    /// ([`crate::Runtime::window_zones`]). It takes no pointer input of the runtime's.
    pub fn window_zone(&mut self, shape: impl Into<HitShape>, tag: u64) {
        self.hits.push(HitRecord { shape: shape.into(), sense: Sense::empty(), part: tag, level: self.part, kind: HitKind::Zone, cursor: None });
    }

    /// The node takes drops anywhere in its window that no [`PaintCx::drop_target`] shape takes,
    /// and drops of OS files that come with no position. The node that registered last asks first.
    pub fn window_drop_target(&mut self) {
        self.window_drop = true;
    }

    /// The node answers its window's close requests: the window's close button, Alt+F4 and Quit
    /// reach it as [`crate::Event::CloseRequested`] instead of closing the window. The node closes
    /// the window itself ([`crate::EventCx::window_command`] with `Close`, now or after a prompt);
    /// a window with no guard closes at once. The node that registered last is asked.
    pub fn close_guard(&mut self) {
        self.close_guard = true;
    }

    pub fn window(&self) -> WindowId {
        self.window
    }

    pub fn size(&self) -> Vec2 {
        self.size
    }

    pub fn bounds(&self) -> Rect {
        Rect::from_min_size(Pos2::ZERO, self.size)
    }

    pub fn env(&self) -> Env {
        self.rt.env_of(self.node)
    }

    pub fn tokens(&mut self) -> Arc<Tokens> {
        let tokens = self.env().tokens;
        tokens.get(self)
    }

    pub fn scale(&mut self) -> f32 {
        let scale = self.env().scale;
        scale.get(self)
    }

    /// The frame time in seconds. Reading it records the read, so the node repaints every frame
    /// while it keeps reading, and the run loop asks the platform for frames until it stops.
    pub fn time(&mut self) -> f64 {
        let time = self.env().time;
        time.get(self)
    }

    /// The frame time, for a paint that changes only at moments it can name (a caret's blink
    /// edges): `next` gives the first such moment after the time, and the node repaints then,
    /// with no frames between them, where a read of [`PaintCx::time`] repaints every frame.
    pub fn time_until(&mut self, next: impl FnOnce(f64) -> Option<f64>) -> f64 {
        let now = self.rt.time;
        if let Some(at) = next(now) {
            self.rt.wakes.push((self.node, at));
        }
        now
    }

    /// Clips this node's content, children and overlay to `rect` (local coordinates).
    pub fn clip(&mut self, rect: Rect) {
        self.clip = Some(Clip::from(rect));
    }

    /// Clips like [`PaintCx::clip`], with rounded corners.
    pub fn clip_rounded(&mut self, rect: Rect, radii: impl Into<CornerRadii>) {
        self.clip = Some(Clip { rect, radii: radii.into() });
    }

    pub fn opacity(&mut self, opacity: f32) {
        self.opacity = opacity;
    }

    /// Records the items `f` adds over this node's children.
    pub fn over<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        let saved = std::mem::replace(&mut self.part, Part::Overlay);
        let out = f(self);
        self.part = saved;
        out
    }

    /// Appends a recorded list's items at the current level: a node replaying display lists it
    /// recorded earlier (the contract bridge's paint variants).
    pub fn extend(&mut self, list: &DisplayList) {
        match self.part {
            Part::Content => self.content.items.extend(list.items.iter().cloned()),
            Part::Overlay => self.overlay.items.extend(list.items.iter().cloned()),
        }
    }

    pub fn push(&mut self, item: Item) {
        match self.part {
            Part::Content => self.content.items.push(item),
            Part::Overlay => self.overlay.items.push(item),
        }
    }

    pub fn fill_rect(&mut self, rect: Rect, color: Rgba) {
        self.push(Item::Rect { rect, radii: CornerRadii::ZERO, fill: color, stroke: None });
    }

    pub fn rect(&mut self, rect: Rect, radii: impl Into<CornerRadii>, fill: Rgba, stroke: Option<Stroke>) {
        self.push(Item::Rect { rect, radii: radii.into(), fill, stroke });
    }

    pub fn circle(&mut self, center: Pos2, radius: f32, fill: Rgba, stroke: Option<Stroke>) {
        self.push(Item::Circle { center, radius, fill, stroke });
    }

    pub fn line(&mut self, from: Pos2, to: Pos2, stroke: Stroke) {
        self.push(Item::Line { from, to, stroke });
    }

    pub fn path(&mut self, path: Arc<BezPath>, fill: Option<Fill>, stroke: Option<Stroke>) {
        self.push(Item::Path { path, fill, stroke });
    }

    pub fn mesh(&mut self, mesh: Arc<Mesh>) {
        self.push(Item::Mesh(mesh));
    }

    pub fn shadow(&mut self, rect: Rect, radii: impl Into<CornerRadii>, offset: Vec2, blur: f32, spread: f32, color: Rgba) {
        self.push(Item::Shadow { rect, radii: radii.into(), offset, blur, spread, color });
    }

    pub fn vgradient(&mut self, rect: Rect, top: Rgba, bottom: Rgba) {
        self.push(Item::VGradient { rect, top, bottom });
    }

    pub fn image(&mut self, key: ImageKey, rect: Rect, tint: Rgba) {
        self.push(Item::Image { key, rect, uv: FULL_UV, tint });
    }

    /// `color` through the image's alpha (an icon mask).
    pub fn image_mask(&mut self, key: ImageKey, rect: Rect, color: Rgba) {
        self.push(Item::ImageMask { key, rect, uv: FULL_UV, color });
    }

    /// The app-drawn surface `key` inside `rect` ([`Runtime::invalidate_surface`] damages it).
    pub fn surface(&mut self, key: SurfaceKey, rect: Rect) {
        self.push(Item::Surface { key, rect });
    }

    pub fn text(&mut self, layout: &TextLayout, origin: Pos2, color: Option<Rgba>) {
        self.push(Item::Text { layout: layout.handle.clone(), origin, size: layout.size, color, highlight: None, angle: 0.0 });
    }

    /// A text with a range of it filled behind the glyphs (see [`TextHighlight`]).
    pub fn text_highlighted(&mut self, layout: &TextLayout, origin: Pos2, color: Option<Rgba>, highlight: TextHighlight) {
        self.push(Item::Text { layout: layout.handle.clone(), origin, size: layout.size, color, highlight: Some(highlight), angle: 0.0 });
    }

    pub fn push_clip(&mut self, clip: impl Into<Clip>) {
        self.push(Item::PushClip(clip.into()));
    }

    pub fn pop_clip(&mut self) {
        self.push(Item::PopClip);
    }

    /// Composites the items up to [`PaintCx::pop_opacity`] as one group; nothing in it paints
    /// outside `bounds` (a disabled veil over a control).
    pub fn push_opacity(&mut self, opacity: f32, bounds: Rect) {
        self.push(Item::PushOpacity { opacity, bounds });
    }

    pub fn pop_opacity(&mut self) {
        self.push(Item::PopOpacity);
    }
}

/// Puts each filled slot's items in place of its [`Item::Slot`] in `list`, and notes the slots `list` reserves. An
/// unfilled slot stays in the list; renderers paint nothing for it.
fn fill_slots(list: &mut DisplayList, kept: &[(u64, Arc<DisplayList>)], reserved: &mut SmallVec<[u64; 2]>) {
    if !list.items.iter().any(|item| matches!(item, Item::Slot(_))) {
        return;
    }
    for item in std::mem::take(&mut list.items) {
        match item {
            Item::Slot(key) => {
                reserved.push(key);
                match kept.iter().find(|(k, _)| *k == key) {
                    Some((_, filled)) => list.items.extend(filled.items.iter().cloned()),
                    None => list.items.push(Item::Slot(key)),
                }
            }
            item => list.items.push(item),
        }
    }
}

const FULL_UV: Rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));

/// Re-records the display lists of the queued nodes. Nodes that did not change are not visited.
pub(crate) fn paint_pass(rt: &mut Runtime) {
    let queue = std::mem::take(&mut rt.tree.paint_q);
    for id in queue {
        let Some(node) = rt.tree.nodes.get_mut(&id) else {
            continue;
        };
        if !node.dirty.has(Dirty::PAINT) {
            continue;
        }
        node.dirty.remove(Dirty::PAINT);
        let Some(el) = node.element.take() else {
            continue;
        };
        let (size, window) = (node.frame.size, node.window);
        rt.wakes.retain(|&(n, _)| n != id);
        let mut cx = PaintCx {
            rt: &mut *rt,
            node: id,
            window,
            size,
            content: DisplayList::default(),
            overlay: DisplayList::default(),
            part: Part::Content,
            clip: None,
            opacity: 1.0,
            hits: Vec::new(),
            cursor: None,
            window_drop: false,
            close_guard: false,
        };
        el.paint(&mut cx);
        let PaintCx { mut content, mut overlay, clip, opacity, hits, cursor, window_drop, close_guard, .. } = cx;
        if let Some(node) = rt.tree.nodes.get_mut(&id)
            && !node.slot_lists.is_empty()
        {
            let mut reserved: SmallVec<[u64; 2]> = SmallVec::new();
            fill_slots(&mut content, &node.slot_lists, &mut reserved);
            fill_slots(&mut overlay, &node.slot_lists, &mut reserved);
            node.slot_lists.retain(|(key, _)| reserved.contains(key));
        }
        rt.version_clock += 1;
        let version = rt.version_clock;
        rt.counters.painted += 1;
        let media: SmallVec<[_; 1]> = content.items.iter().chain(&overlay.items).filter_map(Item::media).collect();
        let mut restyled = false;
        let mut pointer_look = false;
        if let Some(node) = rt.tree.nodes.get_mut(&id) {
            restyled = node.frame.clip != clip || node.frame.opacity != opacity;
            if node.media != media {
                let old = std::mem::replace(&mut node.media, media);
                for (m, _) in &old {
                    if let Some(users) = rt.media.get_mut(m) {
                        users.retain(|&mut u| u != id);
                    }
                }
                for (m, _) in &node.media {
                    let users = rt.media.entry(*m).or_default();
                    if !users.contains(&id) {
                        users.push(id);
                    }
                }
            }
            node.element = Some(el);
            node.frame.content_ink = content.ink();
            node.frame.overlay_ink = overlay.ink();
            node.frame.content = Arc::new(content);
            node.frame.overlay = Arc::new(overlay);
            node.frame.clip = clip;
            node.frame.opacity = opacity;
            node.frame.version = version;
            pointer_look = node.cursor != cursor || node.hits.iter().any(|h| h.cursor.is_some()) || hits.iter().any(|h| h.cursor.is_some());
            node.hits = hits;
            node.cursor = cursor;
        }
        if let Some(w) = rt.windows.get_mut(&window) {
            if restyled {
                w.restyled.push(id);
            }
            if pointer_look {
                w.input.cursor_check = true;
            }
            let listed = w.window_drops.iter().position(|&n| n == id);
            match (window_drop, listed) {
                (true, None) => w.window_drops.push(id),
                (false, Some(i)) => {
                    w.window_drops.remove(i);
                }
                (true, Some(_)) | (false, None) => {}
            }
            let guarded = w.close_guards.iter().position(|&n| n == id);
            match (close_guard, guarded) {
                (true, None) => w.close_guards.push(id),
                (false, Some(i)) => {
                    w.close_guards.remove(i);
                }
                (true, Some(_)) | (false, None) => {}
            }
            w.changes.repainted.push(id);
        }
    }
}
