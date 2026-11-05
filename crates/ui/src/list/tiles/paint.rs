//! One tile: thumbnail, caption, badges, bar, status, checkbox and veil.

use std::sync::Arc;

use crate::canvas::textures::{TextureKey, TextureState, ThumbPaint, Thumbnails};
use crate::controls::{Badge, BadgeStyle, BadgeTone, CheckState, Checkbox, ControlSize, Rating, paint_box, paint_rating};
use crate::icons::{self, Icon, IconSize};
use crate::localized::words::counts::STARS as STARS_WORD;
use crate::localized::{LocaleNumbers, Words, fmt};
use crate::primitives::FocusRing;
use crate::theme::{ToEgui as _, Tokens, TokensOf as _, UiTokens, role_format};
use craft_theme::TypeRole;
use egui::text::LayoutJob;
use egui::{Color32, CornerRadius, Galley, Painter, Rect, Response, Stroke, StrokeKind, Ui, UiBuilder, pos2, vec2};

use super::{BadgeKind, BadgeSlot, BadgeVisibility, CaptionPlace, TileBadge, TileContent, TileFlag, TileLabels, TileStatus, TileStyle, fit_name};

/// The opacity of a disabled tile.
const DISABLED_OPACITY: f32 = 0.4;
/// The check box's backdrop: `tile.scrim` at this share.
const CHECKBOX_BACKDROP: f32 = 0.6;
/// Stars a rating has.
const STARS: f32 = 5.0;

/// What a tile's picture area shows when the host's thumbnail does not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileFace {
    /// The thumbnail from `Thumbnails` (or the placeholder while it loads).
    Picture,
    /// A folder: the placeholder fill with the folder glyph in `glyph`; no thumbnail is asked for.
    Folder {
        /// The glyph's colour.
        glyph: Color32,
    },
}

/// How one tile is drawn this pass.
pub struct TileLook<'a> {
    /// What the tile shows.
    pub content: &'a TileContent,
    /// The picture's rect.
    pub picture: Rect,
    /// Where the caption goes: under the picture, or beside it, top aligned, in the column at its
    /// reading start (`CaptionPlace::Start`: the name is then centred in the column, a hidden
    /// tile's name is slashed and its marks are drawn under it); none for a tile without one.
    pub caption: Option<Rect>,
    /// Which kind of caption `caption` is: under the picture, or the column at its reading start.
    pub caption_place: CaptionPlace,
    /// Whether the tile is selected.
    pub selected: bool,
    /// Whether the pointer is over it.
    pub hovered: bool,
    /// Whether it is the lead of a focused grid (draws the focus ring).
    pub lead_focused: bool,
    /// The check box in check mode: its state, or none outside check mode.
    pub checked: Option<bool>,
    /// When the badge bar shows.
    pub badges: BadgeVisibility,
    /// The scrub line's fraction across the picture from the reading start.
    pub scrub: Option<f32>,
    /// Right-to-left reading order.
    pub rtl: bool,
    /// The tile's response, for the state colours of its check box.
    pub response: &'a Response,
    /// The picture area.
    pub face: TileFace,
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

/// The stack pill's text: `labels.stack` with the count written by the locale's numbers.
pub fn stack_text(ctx: &egui::Context, labels: &TileLabels, n: usize) -> String {
    fmt(&labels.stack, &[("n", &LocaleNumbers::of_ctx(ctx).format(n as f64, 0..=0))])
}

fn layout(painter: &Painter, tokens: &UiTokens, color: Color32, text: &str) -> Arc<Galley> {
    painter.layout_job(LayoutJob::single_section(text.to_owned(), role_format(tokens, TypeRole::Caption, color)))
}

/// The corner badges of `content`: the host's, the stack badge and the file status badge.
fn pills(ctx: &egui::Context, content: &TileContent, labels: &TileLabels) -> Vec<Pill> {
    let mut all: Vec<Pill> = content.badges.iter().map(Pill::of).collect();
    if let Some(n) = content.stack {
        all.push(Pill { slot: BadgeSlot::EndTop, kind: BadgeKind::Text(stack_text(ctx, labels, n)), tone: BadgeTone::Neutral });
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
/// `look.face` swaps the picture for a folder; the stack pill is `content.stack`.
pub fn paint_tile<K: TextureKey>(
    ui: &mut Ui,
    style: &TileStyle,
    labels: &TileLabels,
    look: &TileLook<'_>,
    thumbnails: Option<&Thumbnails<K>>,
    state: Option<&TextureState>,
) -> Option<Rect> {
    // lightcraft: crates/ui-egui/src/panels/grid.rs:766 (`cell`, `paint_cell_thumbnail` :816, `paint_cell_caption` :848, `paint_cell_badges` :878).
    let tokens = Tokens::of(ui);
    let content = look.content;
    let mut faded = ui.painter().clone();
    if !content.enabled {
        faded.multiply_opacity(DISABLED_OPACITY);
    }
    let painter = &faded;
    let pen = Pen { painter, tokens: &tokens, style, look, set: icons::active(painter.ctx()), glyph: IconSize::Sm.points(&tokens) };
    let p = look.picture;

    match look.face {
        TileFace::Picture => {
            let drawn = match (thumbnails, state) {
                (Some(thumbnails), Some(state)) => thumbnails.paint(painter, p, state, &ThumbPaint::default().checker(false)),
                _ => false,
            };
            if !drawn && !matches!(state, Some(TextureState::Empty)) {
                painter.rect_filled(p, 0.0, Thumbnails::<K>::style(&tokens).placeholder);
            }
        }
        TileFace::Folder { glyph } => {
            painter.rect_filled(p, 0.0, Thumbnails::<K>::style(&tokens).placeholder);
            icons::paint_on(painter, p, Icon::Folder, pen.set, IconSize::Lg.points(&tokens), IconSize::Lg.stroke(), glyph);
        }
    }
    if content.rejected || content.dimmed {
        painter.rect_filled(p, 0.0, style.veil);
    }
    super::strip::paint_hidden_and_marks(painter, &tokens, style, look, pen.set);
    pen.range_and_scrub();
    let floor = pen.status_band(labels);
    let bar = Rect::from_min_max(pos2(p.left(), floor - style.bar), pos2(p.right(), floor));
    let show_bar = pen.bar_shown();
    let stars = if show_bar { pen.badge_bar(bar) } else { None };

    let checkbox = look.checked.map(|_| {
        let margin = style.inset + style.checkbox_backdrop;
        let x = if look.rtl { p.right() - margin - style.checkbox } else { p.left() + margin };
        Rect::from_min_size(pos2(x, p.top() + margin), vec2(style.checkbox, style.checkbox))
    });
    pen.corner_badges(ui.ctx(), labels, Rect::from_min_max(p.min, pos2(p.right(), if show_bar { bar.top() } else { floor })), checkbox);
    if let (Some(rect), Some(checked)) = (checkbox, look.checked) {
        paint_checkbox(ui, &tokens, style, rect, checked, look.response, content.enabled);
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
    /// A badge's and the status band's glyph: `IconSize::Sm`.
    glyph: f32,
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
        let glyph_at = if rtl { band.right() - style.inset - self.glyph } else { band.left() + style.inset };
        let glyph = Rect::from_min_size(pos2(glyph_at, band.center().y - self.glyph / 2.0), vec2(self.glyph, self.glyph));
        icons::paint_on(self.painter, glyph, Icon::Unlink, self.set, self.glyph, IconSize::Sm.stroke(), on_status);
        let message = if text.is_empty() { &labels.offline } else { text };
        let room = (band.width() - self.glyph - 3.0 * style.inset).max(0.0);
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
                TileFlag::Picked => (Icon::Flag, self.tokens.color.status.good.fg.to_egui()),
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
    fn corner_badges(&self, ctx: &egui::Context, labels: &TileLabels, area: Rect, checkbox: Option<Rect>) {
        let (style, rtl) = (self.style, self.look.rtl);
        let all = pills(ctx, self.look.content, labels);
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
                    (BadgeKind::Glyph(icon), _) => icons::paint_on(self.painter, rect, *icon, self.set, self.glyph, IconSize::Sm.stroke(), text),
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
        if look.caption_place == CaptionPlace::Start {
            if let Some((galley, at)) = super::strip::start_name(self.painter, self.tokens, style, look) {
                self.painter.galley(at, galley, style.caption);
            }
            return;
        }
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

/// The library check box in `rect`, on a backdrop that keeps it readable over a picture. A
/// disabled tile's box fades with the tile.
fn paint_checkbox(ui: &mut Ui, tokens: &UiTokens, style: &TileStyle, rect: Rect, checked: bool, response: &Response, enabled: bool) {
    let look = Checkbox::style(tokens, ControlSize::Sm);
    let mut layer = ui.new_child(UiBuilder::new().max_rect(ui.max_rect()));
    if !enabled {
        layer.multiply_opacity(DISABLED_OPACITY);
    }
    layer.painter().rect_filled(rect.expand(style.checkbox_backdrop), look.radius, style.scrim.gamma_multiply(CHECKBOX_BACKDROP));
    paint_box(&layer, &look, rect, if checked { CheckState::Checked } else { CheckState::Unchecked }, response);
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
pub(super) fn accessible_name(ctx: &egui::Context, content: &TileContent, labels: &TileLabels, label_name: Option<&str>) -> String {
    // lightcraft: crates/ui-egui/src/panels/grid.rs:800 (`cell_description`).
    let marks = || content.marks.iter().map(|m| m.tip.clone()).filter(|tip| !tip.is_empty());
    let joined = |name: &str, details: &[String]| {
        if details.is_empty() { name.to_owned() } else { fmt(&labels.tile_name, &[("name", name), ("details", &details.join(", "))]) }
    };
    // The app's own name replaces the grid's description; the marks still join it.
    if let Some(name) = &content.accessible_name {
        return joined(name, &marks().collect::<Vec<_>>());
    }
    let mut details: Vec<String> = Vec::new();
    if let Some(n) = content.rating.filter(|n| *n > 0) {
        let words = Words::of_ctx(ctx);
        details.push(words.counted(ctx, STARS_WORD, &words.get(STARS_WORD.word), u64::from(n)));
    }
    match content.flag {
        Some(TileFlag::Picked) => details.push(labels.picked.clone()),
        Some(TileFlag::Rejected) => details.push(labels.rejected.clone()),
        None if content.rejected => details.push(labels.rejected.clone()),
        None => {}
    }
    if let Some(name) = label_name.filter(|_| content.label_color.is_some()) {
        details.push(fmt(&labels.label, &[("label", name)]));
    }
    match &content.status {
        TileStatus::Normal => {}
        TileStatus::Offline(text) if text.is_empty() => details.push(labels.offline.clone()),
        TileStatus::Offline(text) => details.push(fmt(&labels.offline_with, &[("offline", &labels.offline), ("message", text)])),
        TileStatus::Unreadable => details.push(labels.unreadable.clone()),
        TileStatus::Failed => details.push(labels.failed.clone()),
    }
    if let Some(n) = content.stack {
        details.push(stack_text(ctx, labels, n));
    }
    details.extend(content.badges.iter().map(|b| b.tip.clone()).filter(|tip| !tip.is_empty()));
    details.extend(marks());
    joined(&content.name, &details)
}
