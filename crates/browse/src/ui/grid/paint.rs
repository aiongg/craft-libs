//! One tile: thumbnail, caption, badges, bar, status, checkbox and veil.

use std::sync::Arc;

use craft_theme::TypeRole;
use craft_ui::canvas::textures::{TextureKey, TextureState, ThumbPaint, Thumbnails};
use craft_ui::controls::{Badge, BadgeStyle, BadgeTone, Checkbox, ControlSize, Rating, paint_rating};
use craft_ui::icons::{self, Icon, IconSize};
use craft_ui::primitives::FocusRing;
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, UiTokens, role_format};
use egui::text::LayoutJob;
use egui::{Color32, CornerRadius, Galley, Painter, Rect, Shape, Stroke, StrokeKind, Ui, pos2, vec2};

use super::{BadgeKind, BadgeSlot, BadgeVisibility, TileBadge, TileContent, TileFlag, TileLabels, TileStatus, TileStyle, fit_name};

/// The opacity of a disabled tile.
const DISABLED_OPACITY: f32 = 0.4;
/// The check box's backdrop: `tile.scrim` at this share.
const CHECKBOX_BACKDROP: f32 = 0.6;
/// Stars a rating has.
const STARS: f32 = 5.0;
/// The size of a glyph in a corner badge.
const BADGE_GLYPH: f32 = 12.0;
/// The size of the offline glyph in the status band.
const STATUS_GLYPH: f32 = 13.0;

/// How one tile is drawn this pass.
pub(super) struct TileLook<'a> {
    pub content: &'a TileContent,
    pub picture: Rect,
    pub caption: Option<Rect>,
    pub selected: bool,
    pub hovered: bool,
    pub lead_focused: bool,
    pub checked: Option<bool>,
    pub badges: BadgeVisibility,
    pub scrub: Option<f32>,
    pub rtl: bool,
}

/// A corner badge ready to draw: the host's, or one the grid adds for a stack or a file status.
struct Pill {
    slot: BadgeSlot,
    kind: BadgeKind,
    tone: BadgeTone,
}

impl Pill {
    fn of(badge: &TileBadge) -> Self {
        Pill { slot: badge.slot, kind: badge.kind.clone(), tone: badge.tone }
    }
}

fn layout(painter: &Painter, tokens: &UiTokens, color: Color32, text: &str) -> Arc<Galley> {
    painter.layout_job(LayoutJob::single_section(text.to_owned(), role_format(tokens, TypeRole::Caption, color)))
}

/// The corner badges of `content`: the host's, the stack badge and the file status badge.
fn pills(content: &TileContent, labels: &TileLabels) -> Vec<Pill> {
    let mut all: Vec<Pill> = content.badges.iter().map(Pill::of).collect();
    if let Some(n) = content.stack {
        all.push(Pill { slot: BadgeSlot::EndTop, kind: BadgeKind::Text(labels.stack.replace("{n}", &n.to_string())), tone: BadgeTone::Neutral });
    }
    match &content.status {
        TileStatus::Unreadable => {
            all.push(Pill { slot: BadgeSlot::StartBottom, kind: BadgeKind::Text(labels.unreadable.clone()), tone: BadgeTone::Bad })
        }
        TileStatus::Failed => all.push(Pill { slot: BadgeSlot::StartBottom, kind: BadgeKind::Text(labels.failed.clone()), tone: BadgeTone::Bad }),
        TileStatus::Normal | TileStatus::Offline(_) => {}
    }
    all
}

/// Draws the picture (`Thumbnails::paint` with the tile's state, or the placeholder alone), then
/// the veil (rejected or dimmed: `tile.veil`; disabled: the whole tile at 40 %), the In and Out
/// bar (`tile.range_bar` high, `tile.range`), the scrub line (`tile.scrub`, 1 pt), the offline
/// band (`tile.status_band`), the badge bar (`tile.bar` high on `tile.scrim` per
/// `BadgeVisibility`: `paint_rating(.., ControlSize::Sm)` in `tile.badge.text`, flag glyph
/// `tile.flag`, colour dot `tile.dot` with a 1 pt `tile.ring.halo` outline), the corner badges
/// (`tile.badge_height` pills; a badge in the check box's corner moves below it), the check box
/// (the library `Checkbox` look at `tile.checkbox`), the rings (selected, hover, focus) and the
/// caption (name via `fit_name` in `Caption` `tile.caption` or `tile.caption_selected`, detail in
/// `tile.detail`). Returns where the stars were drawn, for the rating click: the width of five
/// stars from the first one's edge, so a click on star `n` sets `n`.
///
/// `thumbnails` paints the picture; without it (or without a state) the placeholder shows.
pub(super) fn paint_tile<K: TextureKey>(
    ui: &Ui,
    painter: &Painter,
    style: &TileStyle,
    labels: &TileLabels,
    look: &TileLook<'_>,
    thumbnails: Option<&Thumbnails<K>>,
    state: Option<&TextureState>,
) -> Option<Rect> {
    // lightcraft: crates/ui-egui/src/panels/grid.rs:766 (`cell`, `paint_cell_thumbnail` :816, `paint_cell_caption` :848, `paint_cell_badges` :878).
    let tokens = Tokens::of(ui);
    let content = look.content;
    let mut faded;
    let painter = if content.enabled {
        painter
    } else {
        faded = painter.clone();
        faded.multiply_opacity(DISABLED_OPACITY);
        &faded
    };
    let pen = Pen { painter, tokens: &tokens, style, look, set: icons::active(painter.ctx()) };
    let p = look.picture;

    let drawn = match (thumbnails, state) {
        (Some(thumbnails), Some(state)) => thumbnails.paint(painter, p, state, &ThumbPaint::default().checker(false)),
        _ => false,
    };
    if !drawn && !matches!(state, Some(TextureState::Empty)) {
        painter.rect_filled(p, 0.0, Thumbnails::<K>::style(&tokens).placeholder);
    }
    if content.rejected || content.dimmed {
        painter.rect_filled(p, 0.0, style.veil);
    }
    pen.range_and_scrub();
    let floor = pen.status_band(labels);
    let bar = Rect::from_min_max(pos2(p.left(), floor - style.bar), pos2(p.right(), floor));
    let show_bar = pen.bar_shown();
    let stars = if show_bar { pen.badge_bar(bar) } else { None };

    let checkbox = look.checked.map(|_| {
        let x = if look.rtl { p.right() - style.inset - 2.0 - style.checkbox } else { p.left() + style.inset + 2.0 };
        Rect::from_min_size(pos2(x, p.top() + style.inset + 2.0), vec2(style.checkbox, style.checkbox))
    });
    pen.corner_badges(labels, Rect::from_min_max(p.min, pos2(p.right(), if show_bar { bar.top() } else { floor })), checkbox);
    if let (Some(rect), Some(checked)) = (checkbox, look.checked) {
        paint_checkbox(painter, &tokens, style, rect, checked);
    }
    pen.rings(ui);
    pen.caption();
    stars
}

/// What one tile's parts are drawn with.
struct Pen<'a> {
    painter: &'a Painter,
    tokens: &'a UiTokens,
    style: &'a TileStyle,
    look: &'a TileLook<'a>,
    set: icons::IconSet,
}

impl Pen<'_> {
    /// x at `fraction` of the picture from the reading start.
    fn along(&self, fraction: f32) -> f32 {
        let p = self.look.picture;
        if self.look.rtl { p.right() - fraction * p.width() } else { p.left() + fraction * p.width() }
    }

    /// The In and Out bar along the picture's bottom and the scrub line.
    fn range_and_scrub(&self) {
        let (p, style) = (self.look.picture, self.style);
        if let Some((from, to)) = self.look.content.range {
            let (a, b) = (self.along(from.clamp(0.0, 1.0)), self.along(to.clamp(0.0, 1.0)));
            let bar = Rect::from_x_y_ranges(a.min(b)..=a.max(b), (p.bottom() - style.range_bar)..=p.bottom());
            self.painter.rect_filled(bar, 0.0, style.range);
        }
        if let Some(fraction) = self.look.scrub {
            let x = self.along(fraction.clamp(0.0, 1.0));
            self.painter.line_segment([pos2(x, p.top()), pos2(x, p.bottom())], Stroke::new(1.0, style.scrub));
        }
    }

    /// The offline band; returns the y where the picture's free bottom edge is.
    fn status_band(&self, labels: &TileLabels) -> f32 {
        let (p, style, rtl) = (self.look.picture, self.style, self.look.rtl);
        let TileStatus::Offline(text) = &self.look.content.status else { return p.bottom() };
        let on_status = self.tokens.color.text.on_status.to_egui();
        let band = Rect::from_min_max(pos2(p.left(), p.bottom() - style.status_band), p.right_bottom());
        self.painter.rect_filled(band, 0.0, self.tokens.color.status.bad.solid.to_egui());
        let glyph_at = if rtl { band.right() - style.inset - STATUS_GLYPH } else { band.left() + style.inset };
        let glyph = Rect::from_min_size(pos2(glyph_at, band.center().y - STATUS_GLYPH / 2.0), vec2(STATUS_GLYPH, STATUS_GLYPH));
        icons::paint_on(self.painter, glyph, Icon::Unlink, self.set, STATUS_GLYPH, IconSize::Sm.stroke(), on_status);
        let message = if text.is_empty() { &labels.offline } else { text };
        let room = (band.width() - STATUS_GLYPH - 3.0 * style.inset).max(0.0);
        let shown = fit_name(message, |t| layout(self.painter, self.tokens, on_status, t).size().x <= room);
        let galley = layout(self.painter, self.tokens, on_status, &shown);
        let x = if rtl { glyph.left() - style.inset - galley.size().x } else { glyph.right() + style.inset };
        self.painter.galley(pos2(x, band.center().y - galley.size().y / 2.0), galley, on_status);
        band.top()
    }

    /// Whether the badge bar shows.
    fn bar_shown(&self) -> bool {
        let (look, content) = (self.look, self.look.content);
        let has_values = content.rating.is_some_and(|r| r > 0) || content.flag.is_some() || content.label_color.is_some();
        match look.badges {
            BadgeVisibility::Always => true,
            BadgeVisibility::Never => false,
            BadgeVisibility::Auto => look.hovered || look.selected || has_values,
        }
    }

    /// The badge bar: stars, the flag and the colour dot. Returns the stars' area.
    fn badge_bar(&self, bar: Rect) -> Option<Rect> {
        let (content, style, rtl) = (self.look.content, self.style, self.look.rtl);
        self.painter.rect_filled(bar, 0.0, style.scrim);
        let pad = style.inset + 2.0;
        let forward = if rtl { -1.0 } else { 1.0 };
        let mut cursor = if rtl { bar.right() - pad } else { bar.left() + pad };
        // The slot of `width` at the cursor in reading order; moves the cursor past it.
        let mut slot = |width: f32| {
            let x = if rtl { cursor - width } else { cursor };
            cursor += forward * (width + pad);
            Rect::from_min_size(pos2(x, bar.top()), vec2(width, bar.height()))
        };
        let mut stars = None;
        if let Some(rating) = content.rating {
            let step = Rating::style(self.tokens, ControlSize::Sm).badge_step;
            let area = slot(STARS * step);
            paint_rating(self.painter, self.tokens, pos2(area.min.x, bar.center().y), rating, ControlSize::Sm, style.badge_text);
            stars = Some(area);
        }
        if let Some(flag) = content.flag {
            let area = slot(style.flag);
            let (icon, color) = match flag {
                TileFlag::Picked => (Icon::Check, self.tokens.color.status.good.fg.to_egui()),
                TileFlag::Rejected => (Icon::Ban, self.tokens.color.status.bad.fg.to_egui()),
            };
            icons::paint_on(self.painter, area, icon, self.set, style.flag, IconSize::Sm.stroke(), color);
        }
        if let Some(color) = content.label_color {
            let x = if rtl { bar.left() + pad + style.dot / 2.0 } else { bar.right() - pad - style.dot / 2.0 };
            self.painter.circle(pos2(x, bar.center().y), style.dot / 2.0, color.to_egui(), Stroke::new(1.0, style.ring_halo));
        }
        stars
    }

    /// The corner badges in `area`, around the check box.
    fn corner_badges(&self, labels: &TileLabels, area: Rect, checkbox: Option<Rect>) {
        let (style, rtl) = (self.style, self.look.rtl);
        let all = pills(self.look.content, labels);
        let pad_x = Badge::look(self.tokens, BadgeTone::Neutral, BadgeStyle::Solid).pad_x;
        for slot in [BadgeSlot::StartTop, BadgeSlot::EndTop, BadgeSlot::StartBottom, BadgeSlot::EndBottom] {
            let in_slot: Vec<&Pill> = all.iter().filter(|b| b.slot == slot).collect();
            if in_slot.is_empty() {
                continue;
            }
            let (start, top) = match slot {
                BadgeSlot::StartTop => (true, true),
                BadgeSlot::EndTop => (false, true),
                BadgeSlot::StartBottom => (true, false),
                BadgeSlot::EndBottom => (false, false),
            };
            let galleys: Vec<Option<Arc<Galley>>> = in_slot
                .iter()
                .map(|b| match &b.kind {
                    BadgeKind::Glyph(_) => None,
                    BadgeKind::Text(text) => Some(layout(self.painter, self.tokens, pill_colors(self.tokens, style, b.tone).1, text)),
                })
                .collect();
            let widths: Vec<f32> =
                galleys.iter().map(|g| g.as_ref().map_or(style.badge_height, |g| (g.size().x + 2.0 * pad_x).max(style.badge_height))).collect();
            let rects = badge_rects(area, &widths, style, checkbox, start != rtl, top);
            for ((pill, galley), rect) in in_slot.into_iter().zip(galleys).zip(rects) {
                let (fill, text) = pill_colors(self.tokens, style, pill.tone);
                self.painter.rect_filled(rect, CornerRadius::same((style.badge_height / 2.0).round() as u8), fill);
                match (&pill.kind, galley) {
                    (BadgeKind::Glyph(icon), _) => icons::paint_on(self.painter, rect, *icon, self.set, BADGE_GLYPH, IconSize::Sm.stroke(), text),
                    (BadgeKind::Text(_), Some(galley)) => self.painter.galley(rect.center() - galley.size() / 2.0, galley, text),
                    (BadgeKind::Text(_), None) => {}
                }
            }
        }
    }

    /// The selection outside the picture with the halo outside it, the pointer's, the focus.
    fn rings(&self, ui: &Ui) {
        let (p, style, look) = (self.look.picture, self.style, self.look);
        if look.selected {
            self.painter.rect_stroke(p, 0.0, Stroke::new(style.ring_width, style.ring_selected), StrokeKind::Outside);
            self.painter.rect_stroke(p.expand(style.ring_width), 0.0, Stroke::new(style.ring_halo_width, style.ring_halo), StrokeKind::Outside);
        } else if look.hovered {
            self.painter.rect_stroke(p, 0.0, Stroke::new(style.ring_hover_width, style.ring_hover), StrokeKind::Outside);
        }
        FocusRing::paint_if(ui, p.expand(style.ring_width + style.ring_halo_width), CornerRadius::ZERO, look.lead_focused);
    }

    /// The name and the detail under the picture, centred.
    fn caption(&self) {
        let (style, content, look) = (self.style, self.look.content, self.look);
        let Some(caption) = look.caption else { return };
        let name_color = if matches!(content.status, TileStatus::Offline(_)) {
            self.tokens.color.status.bad.fg.to_egui()
        } else if look.selected {
            style.caption_selected
        } else {
            style.caption
        };
        let room = caption.width();
        let line = |text: &str, color: Color32, y: f32| -> f32 {
            let shown = fit_name(text, |t| layout(self.painter, self.tokens, color, t).size().x <= room);
            let galley = layout(self.painter, self.tokens, color, &shown);
            let height = galley.size().y;
            self.painter.galley(pos2(caption.center().x - galley.size().x / 2.0, y), galley, color);
            height
        };
        let top = caption.top() + style.caption_gap;
        let height = line(&content.name, name_color, top);
        if let Some(detail) = &content.detail {
            line(detail, style.detail, top + height);
        }
    }
}

/// The fill and text of a corner badge: the scrim for `Neutral`, the tone's solid pair otherwise.
fn pill_colors(tokens: &UiTokens, style: &TileStyle, tone: BadgeTone) -> (Color32, Color32) {
    match tone {
        BadgeTone::Neutral => (style.scrim, style.badge_text),
        tone => {
            let look = Badge::look(tokens, tone, BadgeStyle::Solid);
            (look.fill, look.text)
        }
    }
}

/// The library check box's resting look in `rect`, on a backdrop that keeps it readable over a picture.
fn paint_checkbox(painter: &Painter, tokens: &UiTokens, style: &TileStyle, rect: Rect, checked: bool) {
    let look = Checkbox::style(tokens, ControlSize::Sm);
    painter.rect_filled(rect.expand(2.0), look.radius, style.scrim.gamma_multiply(CHECKBOX_BACKDROP));
    let colors = if checked { &look.on } else { &look.off };
    painter.rect(rect, look.radius, colors.fill.rest, Stroke::new(look.outline, colors.border.rest), StrokeKind::Inside);
    if let Some(mark) = colors.mark {
        let stroke = Stroke::new(look.mark, mark.rest);
        let k = rect.width() / 14.0;
        let a = rect.left_center() + vec2(3.0, 0.5) * k;
        let b = rect.center_bottom() + vec2(-1.0, -3.5) * k;
        let c = rect.right_top() + vec2(-3.0, 3.5) * k;
        painter.add(Shape::line_segment([a, b], stroke));
        painter.add(Shape::line_segment([b, c], stroke));
    }
}

/// The badge rects for one corner, inward from it, skipping the check box's rect (LightCraft's
/// `a_badge_never_covers_the_checkbox`). `left` says the corner is the picture's left one; the
/// caller turns Start and End into left and right by the reading direction. Badges stack toward
/// the middle of the picture, `tile.caption_gap` apart; one that would touch the check box moves
/// past it, away from the corner.
pub(super) fn badge_rects(picture: Rect, widths: &[f32], style: &TileStyle, checkbox: Option<Rect>, left: bool, top: bool) -> Vec<Rect> {
    // lightcraft: crates/ui-egui/src/dialogs/import/body.rs:460 (`badge_rect`: below the box when too wide).
    let (inset, height, gap) = (style.inset, style.badge_height, style.caption_gap);
    let mut y = if top { picture.top() + inset } else { picture.bottom() - inset - height };
    widths
        .iter()
        .map(|&width| {
            let x = if left { picture.left() + inset } else { picture.right() - inset - width };
            let mut rect = Rect::from_min_size(pos2(x, y), vec2(width, height));
            if let Some(cb) = checkbox.filter(|cb| rect.intersects(*cb)) {
                let moved = if top { cb.bottom() + gap } else { cb.top() - gap - height };
                rect = rect.translate(vec2(0.0, moved - rect.top()));
            }
            y = if top { rect.bottom() + gap } else { rect.top() - gap - height };
            rect
        })
        .collect()
}

/// The accessible name built from `TileLabels`: name, stars, flag, label, status, badge tips.
pub(super) fn accessible_name(content: &TileContent, labels: &TileLabels, label_name: Option<&str>) -> String {
    // lightcraft: crates/ui-egui/src/panels/grid.rs:800 (`cell_description`).
    if let Some(name) = &content.accessible_name {
        return name.clone();
    }
    let mut details: Vec<String> = Vec::new();
    match content.rating {
        Some(1) => details.push(labels.one_star.clone()),
        Some(n) if n > 1 => details.push(labels.stars.replace("{n}", &n.to_string())),
        _ => {}
    }
    match content.flag {
        Some(TileFlag::Picked) => details.push(labels.picked.clone()),
        Some(TileFlag::Rejected) => details.push(labels.rejected.clone()),
        None if content.rejected => details.push(labels.rejected.clone()),
        None => {}
    }
    if let Some(name) = label_name.filter(|_| content.label_color.is_some()) {
        details.push(labels.label.replace("{label}", name));
    }
    match &content.status {
        TileStatus::Normal => {}
        TileStatus::Offline(text) if text.is_empty() => details.push(labels.offline.clone()),
        TileStatus::Offline(text) => details.push(format!("{}: {text}", labels.offline)),
        TileStatus::Unreadable => details.push(labels.unreadable.clone()),
        TileStatus::Failed => details.push(labels.failed.clone()),
    }
    if let Some(n) = content.stack {
        details.push(labels.stack.replace("{n}", &n.to_string()));
    }
    details.extend(content.badges.iter().map(|b| b.tip.clone()).filter(|tip| !tip.is_empty()));
    if details.is_empty() {
        content.name.clone()
    } else {
        labels.tile_name.replace("{name}", &content.name).replace("{details}", &details.join(", "))
    }
}
