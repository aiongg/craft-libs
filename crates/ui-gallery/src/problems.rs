//! The Theme workspace's Problems panel: the contrast pairs the bar's theme falls short on.
//!
//! The sweep ([`craft_theme::report::contrast_report`]) resolves the theme for every mode,
//! contrast and size, so it is computed again only when the theme changes. Standard contrast
//! reports pairs "below minimum"; High contrast must meet them, so its pairs "fail".

use std::sync::Arc;

use craft_theme::report::{ContrastReport, contrast_report};
use craft_theme::{Contrast, ContrastCheck, Explained, Mode, Size, ThemeDef, TokenRef, Variants};
use craft_ui::behavior::{RowInfo, Selection, SelectionChange, SelectionMode};
use craft_ui::composites::EmptyState;
use craft_ui::list::{List, ListEvent, ListRow, ThumbSize};
use craft_ui::theme::{Base, ToEgui};
use egui::{Align2, Ui};

use crate::browser::value_of;

/// The width of the ratio at the end of a row.
const RATIO_WIDTH: f32 = 90.0;

/// The cached contrast report of the shown theme.
#[derive(Default)]
pub struct Problems {
    report: Option<(ThemeDef, Arc<ContrastReport>)>,
}

impl Problems {
    /// The report of `theme`, computed again only when the theme differs from the last one.
    pub fn report(&mut self, theme: &ThemeDef) -> Arc<ContrastReport> {
        match &self.report {
            Some((t, r)) if t == theme => Arc::clone(r),
            _ => {
                let r = Arc::new(contrast_report(theme));
                self.report = Some((theme.clone(), Arc::clone(&r)));
                r
            }
        }
    }

    /// Draws a summary line per mode of the theme, then the warnings of what `base` shows in
    /// `mode`. Returns the token of the row clicked, its foreground.
    pub fn show(&mut self, ui: &mut Ui, base: &Base, mode: Mode, explained: &Explained) -> Option<TokenRef> {
        let theme = base.inputs.theme(mode);
        let report = self.report(theme);
        let appearance = base.shown(mode).appearance;
        let (contrast, size) = (appearance.contrast, appearance.size);
        for &m in modes(theme.variants) {
            ui.label(summary(m, contrast, count(&report, m, contrast, size)));
        }
        let warnings: &[ContrastCheck] = report.for_appearance(None, mode, contrast, size).map_or(&[], |r| r.warnings.as_slice());
        if warnings.is_empty() {
            EmptyState::new("Every checked pair meets its minimum.").show(ui);
            return None;
        }
        let rows: Vec<RowInfo<usize>> = (0..warnings.len()).map(RowInfo::leaf).collect();
        let mut selection = Selection::default();
        let sample = egui::TextStyle::Body.resolve(ui.style());
        let out = List::new("gallery.problems.list", "Contrast problems", rows.as_slice())
            .selection(&mut selection)
            .mode(SelectionMode::Single)
            .max_height(ui.available_height())
            .show(ui, |_, &i| {
                let w = &warnings[i];
                let (fg, bg) = (value_of(explained, w.fg), value_of(explained, w.bg));
                let font = sample.clone();
                ListRow::new(i, format!("{} on {}", w.fg.path(), w.bg.path()))
                    .thumb(ThumbSize::Small, move |painter, rect| {
                        painter.rect_filled(rect, 0.0, bg.to_egui());
                        painter.text(rect.center(), Align2::CENTER_CENTER, "Aa", font, fg.to_egui());
                    })
                    .trailing_item(RATIO_WIDTH, move |ui| {
                        ui.monospace(format!("{:.1} of {:.1}", w.ratio, w.minimum));
                    })
            });
        out.events.into_iter().find_map(|event| match event {
            ListEvent::Selection(SelectionChange { row: Some(i), .. }) | ListEvent::Activate(i) => warnings.get(i).map(|w| w.fg),
            _ => None,
        })
    }
}

fn modes(variants: Variants) -> &'static [Mode] {
    match variants {
        Variants::Both => &[Mode::Light, Mode::Dark],
        Variants::LightOnly => &[Mode::Light],
        Variants::DarkOnly => &[Mode::Dark],
    }
}

fn count(report: &ContrastReport, mode: Mode, contrast: Contrast, size: Size) -> usize {
    report.for_appearance(None, mode, contrast, size).map_or(0, |r| r.warnings.len())
}

/// "Dark: 3 pairs below minimum" in Standard contrast, "Dark: 3 pairs fail" in High.
pub(crate) fn summary(mode: Mode, contrast: Contrast, count: usize) -> String {
    let name = if mode == Mode::Light { "Light" } else { "Dark" };
    let pairs = if count == 1 { "1 pair" } else { &format!("{count} pairs") };
    match (count, contrast) {
        (0, _) => format!("{name}: every pair meets its minimum"),
        (1, Contrast::High) => format!("{name}: {pairs} fails"),
        (_, Contrast::High) => format!("{name}: {pairs} fail"),
        _ => format!("{name}: {pairs} below minimum"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_reports_and_high_contrast_fails() {
        assert_eq!(summary(Mode::Dark, Contrast::Standard, 3), "Dark: 3 pairs below minimum");
        assert_eq!(summary(Mode::Light, Contrast::Standard, 1), "Light: 1 pair below minimum");
        assert_eq!(summary(Mode::Dark, Contrast::High, 3), "Dark: 3 pairs fail");
        assert_eq!(summary(Mode::Dark, Contrast::High, 1), "Dark: 1 pair fails");
        assert_eq!(summary(Mode::Light, Contrast::High, 0), "Light: every pair meets its minimum");
    }
}
