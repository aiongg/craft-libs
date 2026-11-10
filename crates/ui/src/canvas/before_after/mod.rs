//! `BeforeAfter`: the before and after views of an edit, side by side, stacked, or one picture wiped by a divider the
//! user drags.
//!
//! The layout and the divider are the library's; the pictures are the app's. The app gives the picture's aspect ratio
//! ([`BeforeAfter::aspect`]) and a painter ([`BeforeAfter::pictures`]) that the component calls once per side with
//! the rect that side may draw in and the rect the picture goes in; the divider, its knob and the captions are drawn
//! over them.

#[cfg(feature = "testing")]
mod stories;

use crate::contract::{
    Access, Bind, BindMut, Common, Component, Cx, Edit as _, EditResponse, Focus as _, MeasureCx, Name, PaintUi, PartsCx, Pointer as _, Props as _,
    Remember as _, Role, Styled as _, WordSetCx as _,
};
use crate::docking::divider::{self, Divider};
use crate::primitives::{ContentPainterCx, IconSpec, PaintShape};
use crate::runtime::access::Numeric;
use crate::runtime::env::Label;
use crate::runtime::focus::KeyUse;
use crate::style::{ComponentStyle, StyleColor, StyleCx, TextLook, TypeRole};
use craft_behavior::{Axis, CancelReason, CursorIcon, Flow, HitLevel, HitPriority, IconId, Metrics, PartKey, PressPhase, Sense};
use craft_theme::Stateful;
use egui::{Pos2, Rect, Vec2, pos2, vec2};

const DIVIDER: PartKey = PartKey::new("divider");
const PICTURE: PartKey = PartKey::new("picture");
/// The accessible image nodes of the two sides: sub-nodes with no part of their own.
const BEFORE: PartKey = PartKey::new("before");
const AFTER: PartKey = PartKey::new("after");

/// The keyboard path of the picture's press: the knob's keys set the same value.
const POINTER_ONLY: &str = "the divider knob takes the arrow keys, Home and End and sets the same fraction";

/// One key press moves the divider by this share of the picture, Shift or Page by `JUMP`.
const STEP: f32 = 0.01;
const JUMP: f32 = 0.1;

/// How [`BeforeAfter`] lays out the two pictures.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BeforeAfterMode {
    /// Only the after picture.
    #[default]
    After,
    /// Only the before picture.
    Original,
    /// Two panes in a row, before at the reading start.
    SideBySide,
    /// Two panes, before above.
    Stacked,
    /// One picture wiped by a vertical divider, before on the reading-start side.
    Split,
    /// One picture wiped by a horizontal divider, before above.
    SplitStacked,
}

impl BeforeAfterMode {
    /// Every mode, the single views first.
    pub const ALL: [BeforeAfterMode; 6] = [
        BeforeAfterMode::After,
        BeforeAfterMode::Original,
        BeforeAfterMode::SideBySide,
        BeforeAfterMode::Stacked,
        BeforeAfterMode::Split,
        BeforeAfterMode::SplitStacked,
    ];

    /// Whether the mode is a wipe with a divider.
    pub fn has_divider(self) -> bool {
        matches!(self, BeforeAfterMode::Split | BeforeAfterMode::SplitStacked)
    }
}

/// Which picture a [`BeforeAfterPane`] holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BeforeAfterSide {
    /// The original.
    Before,
    /// The edited picture.
    After,
}

/// Where one side is painted, in screen points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct BeforeAfterPane {
    /// The side.
    pub side: BeforeAfterSide,
    /// The pane: half of the area (two panes) or the whole area, less the caption strip.
    pub area: Rect,
    /// Where the side may draw: the pane, or its side of the divider in a wipe.
    pub clip: Rect,
    /// Where the picture goes: fitted in the pane and centred in it; in a wipe, the same rect for both sides.
    pub image: Rect,
}

type PicturesFn<'a> = Box<dyn FnMut(&ContentPainterCx<'_>, &BeforeAfterPane) + 'a>;

/// The app's painter of the two pictures.
pub struct BeforeAfterPictures<'a>(PicturesFn<'a>);

impl<'a> BeforeAfterPictures<'a> {
    /// A painter that draws one side with `paint`, given the content painter (clipped to the side's
    /// [`clip`](BeforeAfterPane::clip)) and the pane.
    pub fn new(paint: impl FnMut(&ContentPainterCx<'_>, &BeforeAfterPane) + 'a) -> Self {
        BeforeAfterPictures(Box::new(paint))
    }
}

/// The divider of a wipe, in screen points.
#[derive(Clone, Copy, Debug)]
struct Wipe {
    /// The shown part of the picture: the picture cut to its pane.
    shown: Rect,
    stacked: bool,
    /// The divider's coordinate across the picture: x, or y when stacked.
    at: f32,
}

impl Wipe {
    fn axis(&self) -> Axis {
        if self.stacked { Axis::Y } else { Axis::X }
    }

    /// The divider line, `width` thick, over the full length of the shown picture.
    fn line(&self, width: f32) -> Rect {
        let (r, half) = (self.shown, width / 2.0);
        if self.stacked {
            Rect::from_min_max(pos2(r.min.x, self.at - half), pos2(r.max.x, self.at + half))
        } else {
            Rect::from_min_max(pos2(self.at - half, r.min.y), pos2(self.at + half, r.max.y))
        }
    }

    fn knob_centre(&self) -> Pos2 {
        if self.stacked { pos2(self.shown.center().x, self.at) } else { pos2(self.at, self.shown.center().y) }
    }

    /// The least and most coordinate of the divider.
    fn span(&self) -> (f32, f32) {
        if self.stacked { (self.shown.min.y, self.shown.max.y) } else { (self.shown.min.x, self.shown.max.x) }
    }
}

/// The two sides of a pass and the wipe, if the mode is one.
struct Sides {
    before: Option<BeforeAfterPane>,
    after: Option<BeforeAfterPane>,
    wipe: Option<Wipe>,
}

/// A picture of aspect ratio `aspect` (width over height) fitted inside `pane` and centred; the pane itself without one.
fn fitted(pane: Rect, aspect: Option<f32>, inset: f32) -> Rect {
    let Some(aspect) = aspect.filter(|aspect| aspect.is_finite() && *aspect > 0.0) else { return pane };
    let room = pane.shrink2(Vec2::splat(inset).min(pane.size() / 2.0));
    let width = room.width().min(room.height() * aspect);
    Rect::from_center_size(room.center(), vec2(width, width / aspect))
}

impl Sides {
    /// The sides in `mode` over `area` with the divider at `fraction` from the reading start.
    fn of(mode: BeforeAfterMode, panes: [Rect; 2], picture: Option<f32>, inset: f32, fraction: f32, rtl: bool) -> Self {
        let pane = |side, area: Rect, clip: Rect, image| BeforeAfterPane { side, area, clip, image };
        let [before, after] = panes;
        match mode {
            BeforeAfterMode::After => {
                Sides { before: None, after: Some(pane(BeforeAfterSide::After, after, after, fitted(after, picture, inset))), wipe: None }
            }
            BeforeAfterMode::Original => {
                Sides { before: Some(pane(BeforeAfterSide::Before, before, before, fitted(before, picture, inset))), after: None, wipe: None }
            }
            BeforeAfterMode::SideBySide | BeforeAfterMode::Stacked => Sides {
                before: Some(pane(BeforeAfterSide::Before, before, before, fitted(before, picture, inset))),
                after: Some(pane(BeforeAfterSide::After, after, after, fitted(after, picture, inset))),
                wipe: None,
            },
            BeforeAfterMode::Split | BeforeAfterMode::SplitStacked => {
                let area = before;
                let stacked = mode == BeforeAfterMode::SplitStacked;
                let image = fitted(area, picture, inset);
                let cut = image.intersect(area);
                let shown = if cut.width() > 0.0 && cut.height() > 0.0 { cut } else { area };
                let (at, before_clip, after_clip) = if stacked {
                    let at = shown.min.y + fraction * shown.height();
                    (at, Rect::from_min_max(area.min, pos2(area.max.x, at)), Rect::from_min_max(pos2(area.min.x, at), area.max))
                } else if rtl {
                    let at = shown.max.x - fraction * shown.width();
                    (at, Rect::from_min_max(pos2(at, area.min.y), area.max), Rect::from_min_max(area.min, pos2(at, area.max.y)))
                } else {
                    let at = shown.min.x + fraction * shown.width();
                    (at, Rect::from_min_max(area.min, pos2(at, area.max.y)), Rect::from_min_max(pos2(at, area.min.y), area.max))
                };
                Sides {
                    before: Some(pane(BeforeAfterSide::Before, area, before_clip, image)),
                    after: Some(pane(BeforeAfterSide::After, area, after_clip, image)),
                    wipe: Some(Wipe { shown, stacked, at }),
                }
            }
        }
    }
}

/// The style of the before and after view: what it draws besides the app's pictures.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct BeforeAfterStyle {
    #[style(metrics)]
    metrics: Metrics,
    /// The divider line's colour: the overlay's light mark.
    line: StyleColor,
    /// The halo under the line.
    halo: StyleColor,
    /// The line's width.
    line_width: f32,
    /// How far the halo reaches past each side of the line.
    halo_width: f32,
    /// The knob's diameter and its fill.
    knob: f32,
    knob_fill: StyleColor,
    /// The glyph's side and colour on the knob.
    glyph: f32,
    glyph_color: StyleColor,
    /// The focus ring on the knob.
    focus: StyleColor,
    /// Between two side by side panes, and between two stacked ones.
    gap_side: f32,
    gap_stacked: f32,
    /// Below a caption.
    caption_gap: f32,
    /// The space between a pane's edge and its picture.
    picture_inset: f32,
    /// The captions.
    caption: TextLook,
    /// The height of a caption's line.
    caption_line: f32,
}

crate::words! {
    /// The captions and accessible names of the before and after view.
    pub struct BeforeAfterWords {
        before: "beforeAfter.before" = "Before";
        after: "beforeAfter.after" = "After";
        divider: "beforeAfter.divider" = "Compare position";
    }
}

/// A before and after view of the app's two pictures, with a divider the user drags or keys.
///
/// - Two panes (side by side, stacked) split the area minus a gap; before is at the reading start, so it is on the
///   right in a right-to-left interface. A wipe or a single view has one pane, the whole area. With
///   [`captions`](Self::captions) each pane gives up a strip at its top for "Before" or "After".
/// - A wipe shows before on the reading-start side of the divider. A press on the picture moves the divider to the
///   pointer and drags it (only a press near the line with [`drag_anywhere`](Self::drag_anywhere) off, so the app's
///   own drag keeps the rest); Escape puts it back. The value is the fraction across the shown picture from the
///   reading start: 0 is all after, 1 all before, and in a right-to-left interface 0 is the right edge. The picture
///   never mirrors.
/// - The knob is a Tab stop. Arrow keys move it 1 %, Shift or Page 10 %, Home to 0 % and End to 100 %; a screen
///   reader reads and sets its percentage.
/// - To show the original while a key is held, the app passes [`BeforeAfterMode::Original`] while it is down.
#[derive(crate::contract::Component, crate::contract::Props)]
#[component(content_painter)]
pub struct BeforeAfter<'a> {
    #[common]
    common: Common,
    at: Bind<'a, f32>,
    /// How the two pictures are laid out.
    #[prop]
    mode: BeforeAfterMode,
    /// The picture's aspect ratio, width over height: the picture is fitted inside each pane and centred. Without it
    /// the picture fills the pane.
    #[prop(quantity = "aspect ratio")]
    aspect: Option<f32>,
    /// The painter of the two pictures.
    #[prop(func = "FnMut(&ContentPainterCx<'_>, &BeforeAfterPane)")]
    pictures: Option<BeforeAfterPictures<'a>>,
    /// Shows the "Before" and "After" captions above the pictures.
    #[prop]
    captions: bool,
    /// Whether a press anywhere on the picture moves the divider; off, only a press near the line does.
    #[prop]
    drag_anywhere: bool,
}

impl<'a> BeforeAfter<'a> {
    /// A view named `label` in `mode` with the divider at `at`, from the reading start (0 to 1).
    pub fn new(label: impl Into<Label>, mode: BeforeAfterMode, at: Bind<'a, f32>) -> Self {
        BeforeAfter { common: Common::new(), at, mode, aspect: None, pictures: None, captions: false, drag_anywhere: true }.label(label)
    }

    fn strip(&self, style: &BeforeAfterStyle) -> f32 {
        if self.captions { style.caption_line + style.caption_gap } else { 0.0 }
    }

    /// The `[before, after]` panes of the mode in `area`, each less its caption strip.
    fn panes(&self, area: Rect, style: &BeforeAfterStyle, rtl: bool) -> [Rect; 2] {
        let strip = self.strip(style);
        let cut = |r: Rect| Rect::from_min_max(pos2(r.min.x, (r.min.y + strip).min(r.max.y)), r.max);
        match self.mode {
            BeforeAfterMode::SideBySide => {
                let width = ((area.width() - style.gap_side) / 2.0).max(0.0);
                let left = Rect::from_min_size(area.min, vec2(width, area.height()));
                let right = Rect::from_min_size(pos2(area.max.x - width, area.min.y), vec2(width, area.height()));
                let [before, after] = if rtl { [right, left] } else { [left, right] };
                [cut(before), cut(after)]
            }
            BeforeAfterMode::Stacked => {
                let height = ((area.height() - style.gap_stacked) / 2.0).max(0.0);
                let top = Rect::from_min_size(area.min, vec2(area.width(), height));
                let bottom = Rect::from_min_size(pos2(area.min.x, area.max.y - height), vec2(area.width(), height));
                [cut(top), cut(bottom)]
            }
            _ => [cut(area); 2],
        }
    }

    fn sides(&self, area: Rect, style: &BeforeAfterStyle, fraction: f32, rtl: bool) -> Sides {
        Sides::of(self.mode, self.panes(area, style, rtl), self.aspect, style.picture_inset, fraction, rtl)
    }

    /// The divider in the logical space of `area`: mirrored across it for right-to-left, so the fraction counts from
    /// the left and the shared divider code mirrors the bar back.
    fn divider(wipe: &Wipe, area: Rect, style: &BeforeAfterStyle, rtl: bool) -> Divider {
        let flow = Flow::new(area, rtl);
        let shown = flow.to_physical(wipe.shown);
        let logical = Wipe { shown, at: if wipe.stacked || !rtl { wipe.at } else { area.left() + area.right() - wipe.at }, ..*wipe };
        let (lo, hi) = logical.span();
        Divider::new(logical.axis(), logical.at, shown, (lo, hi), style.knob)
    }
}

fn fraction_of(divider: &Divider, coord: f32) -> f32 {
    let extent = (divider.hi - divider.lo).max(f32::EPSILON);
    ((coord.clamp(divider.lo, divider.hi) - divider.lo) / extent).clamp(0.0, 1.0)
}

/// The press on the picture in progress.
#[derive(Clone, Copy, Default)]
struct Held(bool);

impl Component for BeforeAfter<'_> {
    const ROLE: Role = Role::GROUP;
    const KEYS: KeyUse = KeyUse::SPLITTER;
    type Style = BeforeAfterStyle;
    type Output = EditResponse<f32>;

    fn style(cx: &StyleCx<'_>, _: BeforeAfterStyleKey) -> BeforeAfterStyle {
        let (c, tokens) = (&cx.tokens.color, cx.tokens);
        BeforeAfterStyle {
            metrics: Metrics::new().min_size(Vec2::new(tokens.size.plot.width, tokens.size.plot.height)),
            line: cx.color(c.overlay.handle_fill),
            halo: cx.color(c.overlay.halo),
            line_width: tokens.stroke.thick,
            halo_width: tokens.stroke.thin,
            knob: tokens.size.control.sm,
            knob_fill: cx.color(c.accent.strong),
            glyph: tokens.size.icon.sm,
            glyph_color: cx.color(c.text.on_strong),
            focus: cx.color(c.line.focus),
            gap_side: tokens.space.pad.regular,
            gap_stacked: tokens.space.pad.roomy,
            picture_inset: tokens.space.gap.stack,
            caption_gap: tokens.space.gap.tight,
            caption: TextLook::new(TypeRole::Label, Stateful::new(cx.color(c.text.muted))),
            caption_line: tokens.typography.get(TypeRole::Label).line_height,
        }
    }

    /// The room the flow offers across, and the style's least height.
    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        Vec2::new(cx.room.filter(|room| room.is_finite()).unwrap_or(0.0), 0.0)
    }

    fn fills(&self) -> Option<Axis> {
        Some(Axis::X)
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn bind(&mut self) -> Option<BindMut<'_, f32>> {
        Some(self.at.reborrow())
    }

    /// The divider knob is an edge part that drags and takes the keyboard; the picture takes a press anywhere on it
    /// while `drag_anywhere` is on.
    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        let (Some(fraction), true) = (cx.value().copied(), self.mode.has_divider()) else { return };
        let rtl = cx.flow.is_rtl();
        let sides = self.sides(rect, cx.style, fraction.clamp(0.0, 1.0), rtl);
        let Some(wipe) = sides.wipe else { return };
        if self.drag_anywhere {
            cx.part(PICTURE, wipe.shown)
                .sense(Sense::DRAG)
                .priority(HitPriority::new(HitLevel::Body, 0))
                .cursor(cursor(wipe.stacked))
                .pointer_only(POINTER_ONLY);
        }
        divider::declare(cx, rect, DIVIDER, 0, &Self::divider(&wipe, rect, cx.style, rtl));
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let Some(fraction) = cx.value().copied() else { return };
        let (rect, rtl) = (cx.rect, cx.flow.is_rtl());
        let sides = self.sides(rect, cx.style, fraction.clamp(0.0, 1.0), rtl);
        let Some(wipe) = sides.wipe else { return };
        let line = Self::divider(&wipe, rect, cx.style, rtl);
        let flow = Flow::new(rect, rtl);
        let coord = |pos: Pos2| {
            let logical = flow.to_physical(Rect::from_min_max(pos, pos)).min;
            if wipe.stacked { logical.y } else { logical.x }
        };

        match divider::pointer(cx, DIVIDER, 0, &line) {
            Some(divider::Drag::Start) => cx.begin(),
            Some(divider::Drag::To(at)) => cx.propose(fraction_of(&line, at)),
            Some(divider::Drag::Release) => cx.commit(),
            None => {}
        }
        if self.drag_anywhere {
            let press = cx.part(PICTURE).press;
            let mut held = *cx.memory::<Held>();
            if press.went_down
                && let Some(start) = press.phase.start()
            {
                held.0 = true;
                cx.begin();
                cx.propose(fraction_of(&line, coord(start.origin)));
                cx.focus_part(DIVIDER);
            } else if held.0 {
                match press.phase {
                    PressPhase::DragStart { pos, .. } | PressPhase::Drag { pos, .. } => cx.propose(fraction_of(&line, coord(pos))),
                    PressPhase::Release { pos, .. } => {
                        cx.propose(fraction_of(&line, coord(pos)));
                        cx.commit();
                        held.0 = false;
                    }
                    PressPhase::Cancel { .. } => {
                        cx.cancel(CancelReason::Escape);
                        held.0 = false;
                    }
                    _ => {}
                }
            }
            *cx.memory::<Held>() = held;
        }

        let extent = line.hi - line.lo;
        if let Some((_, to)) = divider::keys(cx, DIVIDER, &[(0, line)], extent * STEP, extent * JUMP) {
            let next = fraction_of(&line, to);
            if (next - fraction).abs() > f32::EPSILON {
                cx.set(next);
            }
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let (rect, rtl, style) = (cx.rect, cx.flow.is_rtl(), cx.style);
        let fraction = cx.value().copied().unwrap_or(0.5).clamp(0.0, 1.0);
        let sides = self.sides(rect, style, fraction, rtl);
        if let Some(pictures) = self.pictures.as_mut() {
            for pane in [&sides.before, &sides.after].into_iter().flatten() {
                let painter = ContentPainterCx::new(ui.ui.painter().with_clip_rect(pane.clip.intersect(ui.ui.clip_rect())), ui.env, ui.ink);
                (pictures.0)(&painter, pane);
            }
        }
        let draw = ui.draw();
        if self.captions {
            let color = cx.resolve(&style.caption.color);
            let run = |text: &str| crate::primitives::TextRun::new(text.to_owned()).role(style.caption.role).color(color);
            let words = cx.words::<BeforeAfterWords>();
            let put = |text: &str, within: Rect, at_end: bool, top: f32| {
                let layout = draw.layout(&run(text), f32::INFINITY);
                let start = if rtl { within.max.x - layout.size().x } else { within.min.x };
                let end = if rtl { within.min.x } else { within.max.x - layout.size().x };
                draw.galley(pos2(if at_end { end } else { start }, top), &layout);
            };
            let strip = self.strip(style);
            if let Some(wipe) = sides.wipe {
                let top = sides.before.as_ref().map_or(wipe.shown.min.y, |pane| pane.area.min.y) - strip;
                put(words.before.as_str(), wipe.shown, false, top);
                put(words.after.as_str(), wipe.shown, true, top);
            } else {
                for (pane, text) in [(&sides.before, words.before.as_str()), (&sides.after, words.after.as_str())] {
                    if let Some(pane) = pane {
                        put(text, pane.area, false, pane.area.min.y - strip);
                    }
                }
            }
        }
        let Some(wipe) = sides.wipe.filter(|_| cx.value().is_some()) else { return };
        let line = wipe.line(style.line_width);
        let across = if wipe.stacked { vec2(0.0, style.halo_width) } else { vec2(style.halo_width, 0.0) };
        draw.fill(&PaintShape::Rect(line.expand2(across), egui::CornerRadius::ZERO), style.halo);
        draw.fill(&PaintShape::Rect(line, egui::CornerRadius::ZERO), style.line);
        let centre = wipe.knob_centre();
        draw.fill(&PaintShape::Circle(centre, style.knob / 2.0), style.knob_fill);
        let glyph = IconId::new(if wipe.stacked { "swap_vertical" } else { "swap_horizontal" });
        draw.icon(Rect::from_center_size(centre, Vec2::splat(style.glyph)), &IconSpec::svg(glyph).color(style.glyph_color));
        if cx.focus_visible() {
            draw.stroke(&PaintShape::Circle(centre, style.knob / 2.0), crate::primitives::StrokeWidth::Medium, style.focus);
        }
    }

    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        let words = cx.words::<BeforeAfterWords>();
        let fraction = cx.value().copied().map(|value| value.clamp(0.0, 1.0));
        let (rtl, style) = (cx.flow.is_rtl(), cx.style);
        let sides = self.sides(cx.rect, style, fraction.unwrap_or(0.5), rtl);
        for (key, pane, name) in [(BEFORE, &sides.before, &words.before), (AFTER, &sides.after, &words.after)] {
            if pane.is_some() {
                access.node(key, Role::IMAGE).name = Name::Text(name.as_str().to_owned());
            }
        }
        if let (Some(fraction), Some(_)) = (fraction, sides.wipe) {
            let percent = f64::from((fraction * 100.0).round());
            let node = access.node(DIVIDER.at(0), Role::SLIDER);
            node.name = Name::Text(words.divider.as_str().to_owned());
            let text = format!("{}%", cx.format().number(percent, 0));
            node.numeric = Some(Numeric::new(percent, 0.0..=100.0).step(1.0).text(text));
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<f32> {
        cx.edit_response()
    }
}

fn cursor(stacked: bool) -> CursorIcon {
    if stacked { CursorIcon::ResizeVertical } else { CursorIcon::ResizeHorizontal }
}
