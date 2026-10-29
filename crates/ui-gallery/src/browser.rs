//! The token browser: how each colour of the bar's theme and appearance got its value
//! ([`craft_theme::explain`]), and every contrast pair it takes part in.
//!
//! The page draws with the gallery's own tokens; the swatches show the explained values.

#![allow(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "the gallery chrome is a developer tool built on stock egui widgets, not product UI"
)]

use std::sync::Arc;

use craft_theme::{
    AppId, Appearance, ColorOrigin, ColorSource, ColorToken, Explained, FieldOrigin, Mode, PinLayer, Rgba, StatusKind, ThemeDef, TokenRef, explain,
};
use craft_ui::direction;
use craft_ui::theme::{Base, ToEgui, Tokens, TokensOf};
use egui::{CentralPanel, Grid, RichText, ScrollArea, Sense, Stroke, StrokeKind, Ui, vec2};

// design: allow(a colour chip is a specimen of the colour, not a control; no size token fits)
const SWATCH: egui::Vec2 = vec2(28.0, 16.0);
// design: allow(a colour chip is a specimen of the colour, not a control; no size token fits)
const RAMP_SWATCH: egui::Vec2 = vec2(22.0, 22.0);

/// The browser's filters, selection and last explanation.
#[derive(Default)]
pub struct Browser {
    filter: String,
    pinned_only: bool,
    moved_only: bool,
    failing_only: bool,
    selected: Option<TokenRef>,
    explained: Option<(ThemeDef, Appearance, Option<AppId>, Arc<Explained>)>,
}

impl Browser {
    /// Selects the row of `token`.
    pub fn select(&mut self, token: TokenRef) {
        self.selected = Some(token);
    }

    /// Draws the page for the theme and appearance `base` shows in `mode`: the detail of the
    /// selected row at the end side, the tables in the centre.
    pub fn show(&mut self, ui: &mut Ui, base: &Base, mode: Mode) {
        let explained = self.explained(base, mode);
        let ctx = ui.ctx().clone();
        direction::end_panel(&ctx, "gallery.tokens.detail").default_size(360.0).show(ui, |ui| {
            ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| self.detail(ui, &explained));
        });
        CentralPanel::default().show(ui, |ui| {
            self.filters(ui, base, mode, &explained);
            ui.separator();
            ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                self.color_table(ui, &explained);
                ui.add_space(Tokens::of(ui).space.gap.section);
                self.component_table(ui, &explained);
            });
        });
    }

    /// `explain` for what `base` shows in `mode`, computed again only when that changes.
    fn explained(&mut self, base: &Base, mode: Mode) -> Arc<Explained> {
        let theme = base.inputs.theme(mode);
        let appearance = base.shown(mode).appearance;
        let app = base.inputs.app.as_ref();
        match &self.explained {
            Some((t, a, p, e)) if t == theme && *a == appearance && p.as_ref() == app => Arc::clone(e),
            _ => {
                let e = Arc::new(explain(theme, appearance, app));
                self.explained = Some((theme.clone(), appearance, app.cloned(), Arc::clone(&e)));
                e
            }
        }
    }

    fn filters(&mut self, ui: &mut Ui, base: &Base, mode: Mode, e: &Explained) {
        let shown = base.shown(mode);
        let variant = if e.resolved.variant_used == Mode::Dark { "dark" } else { "light" };
        let failing = e.pairs.iter().filter(|p| !p.pair.passes()).count();
        ui.label(format!(
            "{} ({variant} variant, {:?} contrast): {} pairs measured, {failing} below their minimum, {} reported",
            base.inputs.theme(mode).name,
            shown.appearance.contrast,
            e.pairs.len(),
            e.resolved.warnings.len()
        ));
        ui.horizontal_wrapped(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.filter).hint_text("Filter by path"));
            ui.checkbox(&mut self.pinned_only, "Pinned");
            ui.checkbox(&mut self.moved_only, "Moved by contrast");
            ui.checkbox(&mut self.failing_only, "Failing pairs only");
        });
    }

    /// Whether a row passes the filters.
    fn shows(&self, e: &Explained, token: TokenRef, pinned: bool, moved: bool) -> bool {
        let filter = self.filter.to_lowercase();
        (filter.is_empty() || token.path().contains(&filter))
            && (!self.pinned_only || pinned)
            && (!self.moved_only || moved)
            && (!self.failing_only || e.pairs_of(token).any(|p| !p.pair.passes()))
    }

    fn color_table(&mut self, ui: &mut Ui, e: &Explained) {
        ui.heading("Colour tokens");
        let muted = Tokens::of(ui).color.text.muted.to_egui();
        Grid::new("gallery.tokens.colors").num_columns(6).striped(true).show(ui, |ui| {
            let mut group = "";
            for o in &e.colors {
                let token = TokenRef::Color(o.token);
                if !self.shows(e, token, o.pin.is_some(), o.contrast_move.is_some()) {
                    continue;
                }
                let g = group_of(o.token);
                if g != group {
                    group = g;
                    ui.label(RichText::new(g).strong());
                    ui.end_row();
                }
                swatch(ui, o.value, SWATCH);
                self.path_cell(ui, token);
                ui.monospace(o.value.to_hex());
                ui.label(o.source_words());
                ui.label(RichText::new(o.pin.map_or("", pin_words)).color(muted));
                ui.label(moved_words(o.contrast_move));
                ui.end_row();
            }
        });
    }

    fn component_table(&mut self, ui: &mut Ui, e: &Explained) {
        ui.heading("Component colours");
        let muted = Tokens::of(ui).color.text.muted.to_egui();
        Grid::new("gallery.tokens.components").num_columns(5).striped(true).show(ui, |ui| {
            for o in &e.components {
                let field = TokenRef::Component(o.field);
                if !self.shows(e, field, o.pin.is_some(), o.contrast_move.is_some()) {
                    continue;
                }
                swatch(ui, o.value, SWATCH);
                self.path_cell(ui, field);
                ui.label(reads_words(o));
                ui.label(RichText::new(o.pin.map_or(String::new(), |l| format!("override ({})", pin_words(l)))).color(muted));
                ui.label(moved_words(o.contrast_move));
                ui.end_row();
            }
        });
    }

    fn path_cell(&mut self, ui: &mut Ui, token: TokenRef) {
        if ui.selectable_label(self.selected == Some(token), RichText::new(token.path()).monospace()).clicked() {
            self.selected = Some(token);
        }
    }

    fn detail(&mut self, ui: &mut Ui, e: &Explained) {
        let Some(selected) = self.selected else {
            ui.label("Select a row to see its contrast pairs and ramp.");
            return;
        };
        ui.heading(selected.path());
        let origin = match selected {
            TokenRef::Color(t) => e.color(t).inspect(|o| color_facts(ui, o)),
            TokenRef::Component(f) => e.field(f).and_then(|o| {
                field_facts(ui, o);
                o.reads.and_then(|t| e.color(t))
            }),
        };
        ui.separator();
        ui.strong("Contrast pairs");
        pairs(ui, e, selected);
        let Some(o) = origin else { return };
        if let Some((ramp, step, toward)) = e.ramps.steps_of(o.source) {
            ui.separator();
            // An anchored token's steps are the ramp moved to its anchor.
            match o.anchoring {
                Some(a) => {
                    ui.strong(format!("Ramp (neutral, anchored on {})", a.on.path()));
                    ramp_row(ui, &a.steps.0, step, toward);
                }
                None => {
                    ui.strong(format!("Ramp ({})", ramp_name(Some(o.source))));
                    ramp_row(ui, &ramp.0, step, toward);
                }
            }
        }
    }
}

fn color_facts(ui: &mut Ui, o: &ColorOrigin) {
    Grid::new("gallery.tokens.facts").num_columns(2).show(ui, |ui| {
        fact(ui, "Value", o.value);
        ui.label("Source");
        ui.label(o.source_words());
        ui.end_row();
        fact(ui, "Derived", o.derived);
        ui.label("Pin");
        ui.label(o.pin.map_or("none", pin_words));
        ui.end_row();
        if let Some(before) = o.contrast_move {
            fact(ui, "Before the contrast pass", before);
        }
    });
}

fn field_facts(ui: &mut Ui, o: &FieldOrigin) {
    Grid::new("gallery.tokens.facts").num_columns(2).show(ui, |ui| {
        fact(ui, "Value", o.value);
        ui.label("Default");
        ui.label(reads_words(o));
        ui.end_row();
        ui.label("Override");
        ui.label(o.pin.map_or("none", pin_words));
        ui.end_row();
        if let Some(before) = o.contrast_move {
            fact(ui, "Before the contrast pass", before);
        }
    });
}

fn fact(ui: &mut Ui, name: &str, value: Rgba) {
    ui.label(name);
    ui.horizontal(|ui| {
        swatch(ui, value, SWATCH);
        ui.monospace(value.to_hex());
    });
    ui.end_row();
}

/// Every pair `token` takes part in. Results are words in text colours, not colour alone.
fn pairs(ui: &mut Ui, e: &Explained, token: TokenRef) {
    let mut any = false;
    Grid::new("gallery.tokens.pairs").num_columns(6).striped(true).show(ui, |ui| {
        for p in e.pairs_of(token) {
            if !any {
                for title in ["", "On / under", "Ratio", "Minimum", "Result", ""] {
                    ui.label(RichText::new(title).strong());
                }
                ui.end_row();
                any = true;
            }
            let pair = p.pair;
            let (other, role) = if pair.fg == token { (pair.bg, "on") } else { (pair.fg, "under") };
            swatch(ui, value_of(e, other), SWATCH);
            ui.monospace(format!("{role} {}", other.path()));
            ui.monospace(format!("{:.2}", pair.ratio));
            ui.monospace(format!("{:.1}", pair.minimum));
            if pair.passes() {
                ui.label("pass");
            } else {
                ui.label(RichText::new("FAIL").strong());
            }
            let pass_note = if p.fixed_by_pass {
                format!("fixed by pass (was {:.2})", p.ratio_before)
            } else if p.moved_by_pass {
                format!("moved (was {:.2})", p.ratio_before)
            } else {
                String::new()
            };
            ui.label(pass_note);
            ui.end_row();
        }
    });
    if !any {
        ui.label("No contrast rule measures this colour.");
    }
}

/// Twelve swatches with the used step (and the step a mix moves toward) outlined and their
/// numbers in bold.
fn ramp_row(ui: &mut Ui, ramp: &[Rgba; 12], used: usize, toward: Option<usize>) {
    let mark = Tokens::of(ui).color.text.normal.to_egui();
    ui.horizontal(|ui| {
        for (i, &c) in ramp.iter().enumerate() {
            let step = i + 1;
            ui.vertical(|ui| {
                let rect = swatch(ui, c, RAMP_SWATCH);
                let label = RichText::new(step.to_string()).small();
                if step == used || toward == Some(step) {
                    ui.painter().rect_stroke(rect.expand(2.0), 3.0, Stroke::new(2.0, mark), StrokeKind::Outside); // design: allow(no stroke-width token; the ring marks the used ramp step)
                    ui.label(label.strong());
                } else {
                    ui.label(label);
                }
            });
        }
    });
}

/// A filled rectangle with a border in the page's `line.border`.
fn swatch(ui: &mut Ui, color: Rgba, size: egui::Vec2) -> egui::Rect {
    let border = Tokens::of(ui).color.line.border.to_egui();
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter().rect(rect, 2.0, color.to_egui(), Stroke::new(1.0, border), StrokeKind::Inside); // design: allow(no stroke-width token; hairline around a colour chip)
    rect
}

fn value_of(e: &Explained, token: TokenRef) -> Rgba {
    match token {
        TokenRef::Color(t) => e.resolved.tokens.color(t),
        TokenRef::Component(f) => e.resolved.tokens.component.color(f).unwrap_or(Rgba::TRANSPARENT),
    }
}

/// The token's group as in `ColorTokens`: its first path segment.
fn group_of(token: ColorToken) -> &'static str {
    let path = token.path();
    path.split_once('.').map_or(path, |(g, _)| g)
}

fn status_name(kind: StatusKind) -> &'static str {
    match kind {
        StatusKind::Good => "good",
        StatusKind::Warn => "warn",
        StatusKind::Bad => "bad",
        StatusKind::Info => "info",
    }
}

fn ramp_name(source: Option<ColorSource>) -> String {
    match source {
        Some(ColorSource::Neutral(_) | ColorSource::Anchored(_) | ColorSource::StateVariant { .. } | ColorSource::NeutralAlpha(..)) => {
            "neutral".to_owned()
        }
        Some(ColorSource::Accent(_) | ColorSource::AccentMix { .. }) => "accent".to_owned(),
        Some(ColorSource::Status(kind, _) | ColorSource::StatusMix { kind, .. }) => format!("status {}", status_name(kind)),
        _ => String::new(),
    }
}

fn pin_words(layer: PinLayer) -> &'static str {
    match layer {
        PinLayer::Theme => "theme pin",
        PinLayer::App => "app pin",
        PinLayer::ThemeHighContrast => "theme high-contrast pin",
        PinLayer::AppHighContrast => "app high-contrast pin",
        _ => "pin",
    }
}

fn reads_words(o: &FieldOrigin) -> String {
    o.reads.map_or_else(|| "computed".to_owned(), |t| format!("reads {}", t.path()))
}

fn moved_words(before: Option<Rgba>) -> String {
    before.map_or_else(String::new, |c| format!("moved from {}", c.to_hex()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sources_read_as_words() {
        let theme = &craft_theme::builtins()[0];
        let appearance = Appearance {
            mode: Mode::Light,
            contrast: craft_theme::Contrast::Standard,
            size: craft_theme::Size::Compact,
            text_size: craft_theme::TextSize::Default,
            reduced_motion: false,
        };
        let e = explain(theme, appearance, None);
        let words = |t: ColorToken| e.color(t).unwrap().source_words();
        assert_eq!(words(ColorToken::TextFaint), "neutral 10");
        assert_eq!(words(ColorToken::SurfaceRaised), "neutral 1, anchored on surface.panel");
        assert_eq!(words(ColorToken::AccentPressed), "accent 10 → 11 at 0.5");
        assert_eq!(words(ColorToken::CaptionInactive), "= text.faint");
        assert_eq!(words(ColorToken::EdgeDark), "surface.panel shaded by 0.26");
        assert_eq!(words(ColorToken::TextOnAccent), "white or black on accent.solid");
        assert_eq!(words(ColorToken::TextOnStatus), "white or black on status.good.solid and 11 more solids");
    }
}
