//! `CompareCaption`: label, name, stars, flag and colour dot.

use craft_theme::{Rgba, TypeRole};
use craft_ui::controls::{ControlSize, Rating, paint_rating};
use craft_ui::direction;
use craft_ui::icons::{self, Icon, IconSize};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, role_format};
use egui::text::LayoutJob;
use egui::{Pos2, Rect, Ui, pos2, vec2};

use super::{CompareLabels, CompareStyle};
use crate::ui::grid::{TileFlag, TileStyle};
/// The strip under a compare tile's picture (`tile.compare_caption` 26 high): the label
/// (`BodyStrong`), the file name (`Caption`; dropped when less than `tile.compare_name_room`,
/// 110, remain), the stars (display only: `paint_rating`, `rating.cell` steps), the flag (14)
/// and the colour dot (10). Right under the picture, at the strip's bottom when zoomed in. Runs
/// from the right in a right-to-left interface.
#[derive(Clone, Debug, PartialEq)]
pub struct CompareCaption {
    /// The pane's label ("Select", "Candidate"), translated.
    pub label: String,
    /// The file name.
    pub name: String,
    /// Stars 0–5.
    pub rating: Option<u8>,
    /// Pick or reject.
    pub flag: Option<TileFlag>,
    /// The colour label.
    pub label_color: Option<Rgba>,
}

impl CompareCaption {
    /// A caption with a label and a name.
    pub fn new(label: impl Into<String>, name: impl Into<String>) -> Self {
        // lightcraft: crates/ui-egui/src/panels/compare.rs:345 (the caption strip; the name needs 110).
        Self { label: label.into(), name: name.into(), rating: None, flag: None, label_color: None }
    }

    /// The stars.
    #[must_use]
    pub fn rating(self, rating: Option<u8>) -> Self {
        // new: builder.
        Self { rating, ..self }
    }

    /// The flag.
    #[must_use]
    pub fn flag(self, flag: Option<TileFlag>) -> Self {
        // new: builder.
        Self { flag, ..self }
    }

    /// The colour label.
    #[must_use]
    pub fn label_color(self, color: Option<Rgba>) -> Self {
        // new: builder.
        Self { label_color: color, ..self }
    }

    /// The entries after the name in the accessible name: the rating, then the flag, each led by
    /// ", ".
    fn details(&self, labels: &CompareLabels) -> String {
        let mut details = String::new();
        if let Some(n) = self.rating.filter(|n| *n > 0) {
            details.push_str(", ");
            details.push_str(&craft_i18n::fmt(&labels.stars, &[("n", &n.min(5).to_string())]));
        }
        match self.flag {
            Some(TileFlag::Picked) => details.extend([", ", labels.picked.as_str()]),
            Some(TileFlag::Rejected) => details.extend([", ", labels.rejected.as_str()]),
            None => {}
        }
        details
    }

    /// Draws the strip in `rect`: items run from the reading start, so from the right in a
    /// right-to-left interface. Returns what it drew and the text an accessible name reads
    /// (`CompareLabels::tile_name`).
    pub(super) fn paint(&self, ui: &Ui, rect: Rect, style: &CompareStyle, labels: &CompareLabels) -> CaptionPainted {
        // lightcraft: crates/ui-egui/src/panels/compare.rs:345 (label 12 semibold, name 11.5, stars 13 apart, flag 14, dot 10).
        let tokens = Tokens::of(ui);
        let tile = TileStyle::of(&tokens);
        let rating = Rating::style(&tokens, ControlSize::Md);
        let gap = tokens.space.gap.inline;
        let rtl = direction::is_rtl(ui);
        let painter = ui.painter().with_clip_rect(rect);
        // `offset` runs from the reading start; `at` maps an item of `width` there to a screen x.
        let at = |offset: f32, width: f32| if rtl { rect.right() - offset - width } else { rect.left() + offset };
        let text = |text: &str, role: TypeRole, color| {
            let job = LayoutJob::single_section(text.to_string(), role_format(&tokens, role, color));
            painter.layout_job(job)
        };
        let mut offset = gap;
        let mut name_shown = false;
        if !self.label.is_empty() {
            let galley = text(&self.label, TypeRole::BodyStrong, tokens.color.text.normal.to_egui());
            let size = galley.size();
            painter.galley(pos2(at(offset, size.x), rect.center().y - size.y / 2.0), galley, tokens.color.text.normal.to_egui());
            offset += size.x + gap;
        }
        let galley = text(&self.name, TypeRole::Caption, tokens.color.text.muted.to_egui());
        let size = galley.size();
        if offset + size.x + style.name_room < rect.width() {
            painter.galley(pos2(at(offset, size.x), rect.center().y - size.y / 2.0), galley, tokens.color.text.muted.to_egui());
            offset += size.x + gap;
            name_shown = true;
        }
        let stars = rating.badge_step * 5.0;
        if let Some(n) = self.rating.filter(|n| *n > 0)
            && offset + stars <= rect.width()
        {
            // `paint_rating` runs rightwards; mirror the row's rect in a right-to-left interface by
            // drawing it at the mirrored start.
            let start = pos2(at(offset, stars), rect.center().y);
            paint_rating(&painter, &tokens, start, n.min(5), ControlSize::Md, rating.lit);
        }
        offset += stars + gap;
        let flag_side = tile.flag;
        if let Some(flag) = self.flag
            && offset + flag_side <= rect.width()
        {
            let slot = Rect::from_min_size(pos2(at(offset, flag_side), rect.center().y - flag_side / 2.0), vec2(flag_side, flag_side));
            let (icon, color) = match flag {
                TileFlag::Picked => (Icon::Check, tokens.color.status.good.fg.to_egui()),
                TileFlag::Rejected => (Icon::Ban, tokens.color.status.bad.fg.to_egui()),
            };
            icons::paint_on(&painter, slot, icon, icons::active(ui.ctx()), flag_side, IconSize::Sm.stroke(), color);
        }
        offset += flag_side + gap;
        if let Some(color) = self.label_color
            && offset + tile.dot <= rect.width()
        {
            let center: Pos2 = pos2(at(offset, tile.dot) + tile.dot / 2.0, rect.center().y);
            painter.circle_filled(center, tile.dot / 2.0, color.to_egui());
        }
        let name = craft_i18n::fmt(&labels.tile_name, &[("label", &self.label), ("name", &self.name), ("details", &self.details(labels))]);
        CaptionPainted { name, name_shown }
    }
}

/// What [`CompareCaption::paint`] drew.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CaptionPainted {
    /// The tile's accessible name: the caption read as text.
    pub name: String,
    /// The file name fitted and was drawn.
    pub name_shown: bool,
}
