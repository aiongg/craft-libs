//! What the user sees while dragging: the [`DragBadge`] at the pointer saying what is dragged
//! or what a release would do, the per-frame [drop hint](set_drop_hint) a target gives it, and
//! the [`DropVeil`] over the window while files from outside hover over it.
//!
//! A [`List`](crate::list::List)'s own row drag draws its ghost box
//! ([`paint_ghost`](crate::list::paint_ghost)); the badge is for drags that leave their list
//! (photos from a grid onto an album, a layer onto another document) and for drags no list
//! owns.

use std::borrow::Cow;
use std::path::PathBuf;

use craft_theme::TypeRole;
use egui::accesskit::Role;
use egui::{Area, AsIdSalt, Context, CursorIcon, Frame, Id, IdSalt, LayerId, Margin, Order, Sense, Stroke, StrokeKind, WidgetInfo, pos2, vec2};

use crate::composites::live::{Politeness, region};
use crate::direction;
use crate::localized::words::Counted;
use crate::localized::{Labels as _, Words};
use crate::theme::{ToEgui as _, corner_radius};

/// The badge's text for a number of items. S5C-F9 moves it into the kernel's `counts` and
/// [`COUNTS`](crate::localized::words::COUNTS).
const COUNT: Counted = Counted::new("drag.count", "{n} item", "{n} items");

fn hint_id() -> Id {
    Id::NULL.with("craft_ui::behavior::drag_feedback::hint")
}

/// The id of the badge's or the veil's layer.
pub(super) fn layer_id(kind: &'static str, salt: IdSalt) -> Id {
    Id::NULL.with(("craft_ui::behavior::drag_feedback", kind)).with_salt(salt)
}

crate::words! {
    /// The words of drag feedback, English by default.
    #[non_exhaustive]
    pub struct DragFeedbackLabels {
        /// The badge's default text: `{n}` the number of items (LightCraft's "{n} photos"). A
        /// [`Counted`](crate::localized::words::Counted) word ("{n} item", "{n} items"), written
        /// with [`Words::counted`](crate::localized::Words::counted); S5C-F9 adds it to the
        /// kernel's `counts` and [`COUNTS`](crate::localized::words::COUNTS).
        count: "drag.count" = "{n} items";
    }
}

/// The badge that follows the pointer during a drag, after LightCraft's photo and album drags.
///
/// ```ignore
/// // While photos are dragged (every frame, after the drop targets drew):
/// DragBadge::count("photos", ids.len()).show(ctx);
/// // In an album row under the pointer:
/// set_drop_hint(ctx, format!("Add to “{}”", album.name));
/// ```
///
/// - **Look.** A pill on the tooltip layer, 14 points right of and 10 below the pointer
///   (LightCraft's `pos + vec2(14, 10)`), filled with `accent.solid` (`shape.pill` radius), 9 by 3
///   points of padding, its text in the `BodyStrong` role and `text.on_accent`; not interactable. The
///   window shows the grabbing cursor.
/// - **Text.** This pass's [drop hint](drop_hint) when a target under the pointer gave one,
///   else the badge's own text ([`count`](Self::count): [`DragFeedbackLabels::count`] in the
///   locale's plural form).
/// - **Order.** The drop targets draw first and set their hint, then the host shows the badge,
///   in the same pass; a hint set after the badge drew is dropped at the end of the pass.
/// - **End.** The host stops showing it when the drag ends (the button up, or Esc); drop targets
///   act on the release pass before it.
/// - It asks for a repaint each frame it shows, so it follows the pointer.
pub struct DragBadge<'a> {
    id_salt: IdSalt,
    text: Option<Cow<'a, str>>,
    count: Option<usize>,
    labels: Option<DragFeedbackLabels>,
}

impl<'a> DragBadge<'a> {
    /// A badge saying `text` ("Paris.jpg", an album's name); `id_salt` names its layer.
    pub fn new(id_salt: impl AsIdSalt, text: impl Into<Cow<'a, str>>) -> Self {
        Self { id_salt: IdSalt::new(id_salt), text: Some(text.into()), count: None, labels: None }
    }

    /// A badge counting `n` items in the installed words.
    pub fn count(id_salt: impl AsIdSalt, n: usize) -> Self {
        Self { id_salt: IdSalt::new(id_salt), text: None, count: Some(n), labels: None }
    }

    /// Words other than the installed ones (LightCraft's "{n} photos").
    pub fn labels(mut self, labels: DragFeedbackLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // lightcraft: crates/ui-egui/src/panels/grid.rs:1120 drag_feedback; panels/left/albums.rs:366 (album drag)
    pub fn show(self, ctx: &Context) {
        let Self { id_salt, text, count, labels } = self;
        let Some(pointer) = ctx.input(|i| i.pointer.latest_pos()) else { return };
        let text = drop_hint(ctx).or_else(|| text.map(Cow::into_owned)).unwrap_or_else(|| {
            let labels = labels.unwrap_or_else(|| DragFeedbackLabels::of_ctx(ctx));
            Words::of_ctx(ctx).counted(ctx, COUNT, &labels.count, count.unwrap_or(0) as u64)
        });
        let tokens = crate::theme::current(ctx);
        let (fill, ink) = (tokens.color.accent.solid.to_egui(), tokens.color.text.on_accent.to_egui());
        let drag = &tokens.component.drag;
        let pad = vec2(drag.badge_pad_x, drag.badge_pad_y);
        let font = tokens.font(TypeRole::BodyStrong);
        let rtl = direction::is_rtl(ctx);
        let (pivot, at) = if rtl {
            (egui::Align2::RIGHT_TOP, pointer + vec2(-drag.badge_offset_x, drag.badge_offset_y))
        } else {
            (egui::Align2::LEFT_TOP, pointer + vec2(drag.badge_offset_x, drag.badge_offset_y))
        };
        ctx.set_cursor_icon(CursorIcon::Grabbing);
        Area::new(layer_id("badge", id_salt)).order(Order::Tooltip).interactable(false).pivot(pivot).fixed_pos(at).show(ctx, |ui| {
            let galley = ui.painter().layout_no_wrap(text.clone(), font, ink);
            let size = galley.size() + 2.0 * pad;
            let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
            ui.painter().rect_filled(rect, corner_radius(tokens.shape.radius.pill.min(size.y / 2.0)), fill);
            ui.painter().galley(rect.min + pad, galley, ink);
            response.widget_info(|| WidgetInfo::labeled(Role::Label, true, &text));
        });
        ctx.request_repaint();
    }
}

/// What a release on the drop target under the pointer would do ("Add to “Trips”"), for this
/// pass's [`DragBadge`]. A target calls it each pass the pointer is over it while a drag it
/// takes is under way, before the badge is shown; the hint lasts this pass only.
// lightcraft: crates/ui-egui/src/panels/grid.rs:1115 drag_hint
pub fn set_drop_hint(ctx: &Context, text: impl Into<String>) {
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|d| d.insert_temp(hint_id(), (pass, text.into())));
}

/// This pass's drop hint, if a target gave one.
// lightcraft: crates/ui-egui/src/panels/grid.rs:1133 (hint for this frame)
pub fn drop_hint(ctx: &Context) -> Option<String> {
    let pass = ctx.cumulative_pass_nr();
    ctx.data(|d| d.get_temp::<(u64, String)>(hint_id())).filter(|(set, _)| *set == pass).map(|(_, text)| text)
}

/// The files hovering over the window this pass, from egui's `hovered_files`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HoveredFiles {
    /// The paths the platform gave, non-empty ones only.
    pub paths: Vec<PathBuf>,
    /// How many came without a path (browsers, some platforms); the app counts them in its own
    /// words (LightCraft's "{n} files").
    pub unnamed: usize,
}

impl HoveredFiles {
    /// This pass's hovering files; empty when none.
    // lightcraft: crates/ui-egui/src/drop_overlay.rs:81 (count, known)
    pub fn of(ctx: &Context) -> Self {
        ctx.input(|i| {
            let paths: Vec<PathBuf> = i.raw.hovered_files.iter().filter_map(|f| f.path.clone()).filter(|p| !p.as_os_str().is_empty()).collect();
            let unnamed = i.raw.hovered_files.len() - paths.len();
            Self { paths, unnamed }
        })
    }

    pub fn is_empty(&self) -> bool {
        self.paths.is_empty() && self.unnamed == 0
    }
}

/// The veil over the window while files from outside hover over it, after LightCraft's drop
/// overlay: what dropping them will do, in the app's words, before the user lets go.
///
/// ```ignore
/// let files = HoveredFiles::of(ctx);
/// if !files.is_empty() && !app.blocks_input() {
///     DropVeil::new("drop", &describe_drop(&files)).show(ctx);
/// }
/// ```
///
/// - **Look.** On the tooltip layer over the whole window: `surface.scrim` (LightCraft's black at
///   150 of 255), a 2-point `accent.solid` frame 10 points in with 12-point corners, and a card in
///   the middle (`surface.raised`, 10-point corners, 1-point `accent.solid` edge) with the
///   lines centred in the `Title` role, 8 points apart, 24 points of padding at the top and 32 at
///   the sides, wrapping at the window's width less 120 (at least 200).
/// - **When.** The host shows it each frame [`HoveredFiles::of`] is not empty and its drop would
///   land (LightCraft skips it while a modal prompt holds the input, so the veil never promises
///   a drop the barrier would refuse). It takes no input and asks for a repaint.
/// - **AccessKit.** The card is a live `Alert` region with the lines as its text.
pub struct DropVeil<'a> {
    id_salt: IdSalt,
    lines: &'a [String],
}

impl<'a> DropVeil<'a> {
    /// A veil saying `lines` ("Import 3 photos", "1 preset"; the app sorts the files the way
    /// its drop does, LightCraft's `classify`); `id_salt` names its layer.
    pub fn new(id_salt: impl AsIdSalt, lines: &'a [String]) -> Self {
        Self { id_salt: IdSalt::new(id_salt), lines }
    }

    // lightcraft: crates/ui-egui/src/drop_overlay.rs:73 show
    pub fn show(self, ctx: &Context) {
        let Self { id_salt, lines } = self;
        let tokens = crate::theme::current(ctx);
        let c = &tokens.color;
        let (accent, raised, ink) = (c.accent.solid.to_egui(), c.surface.raised.to_egui(), c.text.normal.to_egui());
        let screen = ctx.content_rect();
        let id = layer_id("veil", id_salt);
        // The scrim and its frame go on the layer first, under the card the area adds.
        let painter = ctx.layer_painter(LayerId::new(Order::Tooltip, id));
        painter.rect_filled(screen, 0.0, c.surface.scrim.to_egui());
        let veil = &tokens.component.drop_veil;
        painter.rect_stroke(screen.shrink(veil.inset), veil.frame_radius, Stroke::new(veil.frame_width, accent), StrokeKind::Inside);
        let wrap = (screen.width() - veil.wrap_margin).max(veil.min_wrap);
        let font = tokens.font(TypeRole::Title);
        let galleys: Vec<_> = lines.iter().map(|line| painter.layout(line.clone(), font.clone(), ink, wrap)).collect();
        let width = galleys.iter().fold(0.0f32, |w, g| w.max(g.size().x));
        let height = galleys.iter().map(|g| g.size().y).sum::<f32>() + veil.card_gap * galleys.len().saturating_sub(1) as f32;
        let margin = Margin::symmetric(veil.card_pad_x as i8, veil.card_pad_y as i8);
        Area::new(id).order(Order::Tooltip).interactable(false).pivot(egui::Align2::CENTER_CENTER).fixed_pos(screen.center()).show(ctx, |ui| {
            let live_id = id.with("live");
            region(ui, live_id, Role::Alert, Politeness::Polite, |ui, labels| {
                for line in lines {
                    labels.add(line.clone());
                }
                Frame::NONE.fill(raised).stroke(Stroke::new(1.0, accent)).corner_radius(veil.card_radius).inner_margin(margin).show(ui, |ui| {
                    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
                    let mut y = rect.top();
                    for galley in galleys {
                        let size = galley.size();
                        ui.painter().galley(pos2(rect.center().x - size.x / 2.0, y), galley, ink);
                        y += size.y + veil.card_gap;
                    }
                });
            });
        });
        ctx.request_repaint();
    }
}
