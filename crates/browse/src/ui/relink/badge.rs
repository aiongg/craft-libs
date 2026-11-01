//! `StatusBadge`: a link's status as a glyph, never colour alone.

use craft_docpath::Refused;
use craft_docpath::links::LinkStatus;
use craft_theme::TypeRole;
use craft_ui::icons::{self, Icon, IconSize};
use craft_ui::localized::fmt;
use craft_ui::theme::{current, role_format};
use egui::{Align2, Color32, Painter, Pos2, Rect, Shape, Stroke, vec2};

use super::{LinkListLabels, RelinkStyle};

/// The disc's radius as a share of the badge box (VectorCraft's 6.5 in 16).
const DISC: f32 = 0.40625;
/// The triangle's half width and apex height as a share of the box (7 in 16).
const TRIANGLE_REACH: f32 = 0.4375;
/// The triangle's base below the centre as a share of the box (6 in 16).
const TRIANGLE_BASE: f32 = 0.375;
/// The `!` below the centre as a share of the box (1.5 in 16).
const BANG_DROP: f32 = 0.09375;

/// A link's status badge in a 16 box (`link.badge`): Missing a `status.bad.solid` disc with `?`
/// in `text.on_status`; Modified a `status.warn.solid` triangle with `!`; Unreadable a
/// `status.bad.solid` disc with `×`; Refused a `status.bad.solid` disc with `Icon::Ban`; Embedded
/// `Icon::Image` in `text.muted`; Ok nothing. Has no AccessKit node of its own: the row's name
/// carries [`StatusBadge::words`], and the tooltip repeats it.
pub struct StatusBadge;

impl StatusBadge {
    /// Paints the badge for `status` centred in `rect`; `false` for `Ok` (nothing drawn).
    pub fn paint(painter: &Painter, rect: Rect, status: LinkStatus, style: &RelinkStyle) -> bool {
        // vectorcraft: crates/ui-egui/src/panels/links.rs:240 (`badge`).
        let center = rect.center();
        let box_side = style.badge;
        match status {
            LinkStatus::Missing => {
                painter.circle_filled(center, box_side * DISC, style.bad);
                glyph(painter, center, "?", style.on_status);
            }
            LinkStatus::Unreadable => {
                painter.circle_filled(center, box_side * DISC, style.bad);
                glyph(painter, center, "×", style.on_status);
            }
            LinkStatus::Refused(_) => {
                painter.circle_filled(center, box_side * DISC, style.bad);
                icon(painter, rect, Icon::Ban, style.on_status);
            }
            LinkStatus::Modified => {
                let reach = box_side * TRIANGLE_REACH;
                let base = box_side * TRIANGLE_BASE;
                let points: Vec<Pos2> = vec![center + vec2(0.0, -reach), center + vec2(reach, base), center + vec2(-reach, base)];
                painter.add(Shape::convex_polygon(points, style.warn, Stroke::NONE));
                glyph(painter, center + vec2(0.0, box_side * BANG_DROP), "!", style.on_status);
            }
            LinkStatus::Embedded => icon(painter, rect, Icon::Image, style.embedded),
            _ => return false,
        }
        true
    }

    /// The status in words, for the row's name and the tooltip ("Missing", "Modified",
    /// "Unreadable", "Not followed: the path is on another computer", "Embedded"); empty for Ok.
    pub fn words(status: LinkStatus, labels: &LinkListLabels) -> String {
        // vectorcraft: crates/ui-egui/src/panels/links.rs:260 (`status_tip`).
        match status {
            LinkStatus::Missing => labels.missing.clone(),
            LinkStatus::Modified => labels.modified.clone(),
            LinkStatus::Unreadable => labels.unreadable.clone(),
            LinkStatus::Embedded => labels.embedded.clone(),
            LinkStatus::Refused(why) => {
                let reason = match why {
                    Refused::Network => &labels.refused_network,
                    Refused::Device => &labels.refused_device,
                    Refused::Url => &labels.refused_url,
                    Refused::Invalid => &labels.refused_invalid,
                };
                fmt(&labels.refused, &[("reason", reason)])
            }
            _ => String::new(),
        }
    }
}

/// `text` centred at `at` in the caption role.
fn glyph(painter: &Painter, at: Pos2, text: &str, color: Color32) {
    let tokens = current(painter.ctx());
    let format = role_format(&tokens, TypeRole::Caption, color);
    painter.text(at, Align2::CENTER_CENTER, text, format.font_id, color);
}

/// `icon` in the small icon size, centred in `rect`.
fn icon(painter: &Painter, rect: Rect, icon: Icon, color: Color32) {
    let ctx = painter.ctx();
    let tokens = current(ctx);
    icons::paint_on(painter, rect, icon, icons::active(ctx), IconSize::Sm.points(&tokens), IconSize::Sm.stroke(), color);
}
