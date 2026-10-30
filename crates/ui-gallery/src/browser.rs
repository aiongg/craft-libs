//! The token browser: how each colour of the bar's theme and appearance got its value
//! ([`craft_theme::explain`]), and every contrast pair it takes part in.
//!
//! The page draws with the gallery's own tokens; the swatches show the explained values.

use std::sync::Arc;

use craft_theme::{
    AppId, AppSizes, Appearance, ColorOrigin, ColorSource, ColorToken, ContrastIntent, ControlSize, EdgeSeed, Explained, FieldOrigin, Mode, PinLayer,
    Rgba, SizeOrigin, StatusKind, ThemeDef, TokenRef, TypeRole, Variants, explain_sized,
};
use craft_ui::behavior::{RowInfo, Selection, SelectionChange, SelectionMode};
use craft_ui::composites::{FormGrid, SectionHeader};
use craft_ui::controls::{Checkbox, Link, SearchField, Swatch};
use craft_ui::direction;
use craft_ui::layout::{ResizeHandle, ResizeHandleStyle, SplitAxis};
use craft_ui::list::{List, ListEvent, ListRow, ThumbSize};
use craft_ui::primitives::{Content, Divider, Text};
use craft_ui::theme::{Base, StrokeExt, ToEgui, Tokens, TokensOf, scale};
use egui::{Grid, RichText, ScrollArea, Sense, StrokeKind, Ui, vec2};

use crate::app_layer::{AppLayer, layer_words, value_words};
use crate::fields;
use crate::usage::Scan;

// design: allow[literal-size/vec2](a colour chip is a specimen of the colour, not a control; no size token fits)
const SWATCH: egui::Vec2 = vec2(28.0, 16.0);
// design: allow[literal-size/vec2](a colour chip is a specimen of the colour, not a control; no size token fits)
const RAMP_SWATCH: egui::Vec2 = vec2(22.0, 22.0);

const DEFAULT_SHARE: f32 = 0.55;
/// Neither the list nor the detail gets less than this share.
const MIN_SHARE: f32 = 0.15;
/// The height of the divider between list and detail.
const DIVIDER: f32 = 8.0;
/// The width of the hex value at the end of a list row.
const HEX_WIDTH: f32 = 80.0;
/// The width of the value and layer at the end of a size row.
const SIZE_WIDTH: f32 = 130.0;

/// The browser's filters, selection and last explanation.
pub struct Browser {
    filter: String,
    pinned_only: bool,
    moved_only: bool,
    failing_only: bool,
    selected: Option<TokenRef>,
    /// The path of the selected size row; at most one of this and `selected` is set.
    selected_size: Option<String>,
    /// The app's size overrides and the file they come from.
    layer: AppLayer,
    /// The share of the height under the filters that the list takes.
    share: f32,
    explained: Option<Cached>,
}

/// The inputs of the last explanation and the explanation.
type Cached = (ThemeDef, Appearance, Option<AppId>, AppSizes, Arc<Explained>);

/// A row of the token list: a group's heading, or a token.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Row {
    Group(&'static str),
    Token(TokenRef),
    /// An index into [`Explained::sizes`].
    Size(usize),
}

impl Default for Browser {
    fn default() -> Self {
        Self::new(None)
    }
}

impl Browser {
    /// A browser with the list taking `share` of its height, or the default share.
    pub fn new(share: Option<f32>) -> Self {
        let share = share.filter(|s| s.is_finite()).map_or(DEFAULT_SHARE, |s| s.clamp(MIN_SHARE, 1.0 - MIN_SHARE));
        Self {
            filter: String::new(),
            pinned_only: false,
            moved_only: false,
            failing_only: false,
            selected: None,
            selected_size: None,
            layer: AppLayer::default(),
            share,
            explained: None,
        }
    }

    /// The share of the height the list takes, to be saved.
    pub fn share(&self) -> f32 {
        self.share
    }

    /// The selected row.
    pub fn selected(&self) -> Option<TokenRef> {
        self.selected
    }

    /// Selects the row of `token`.
    pub fn select(&mut self, token: TokenRef) {
        self.selected = Some(token);
        self.selected_size = None;
    }

    /// The app's size overrides the gallery resolves with.
    pub(crate) fn app_sizes(&self) -> &AppSizes {
        self.layer.sizes()
    }

    /// Draws the panel for the theme and appearance `base` shows in `mode`: the filters, the
    /// token list, a divider the user drags, and under it the detail of the selected row. The
    /// list and the detail each scroll in their own part of the panel. `scan` supplies the
    /// detail's Used by list; the component a link in it named is returned.
    pub(crate) fn show(&mut self, ui: &mut Ui, base: &Base, mode: Mode, scan: &Scan) -> Option<&'static str> {
        self.layer.take_dropped(ui.ctx());
        let explained = self.explained(base, mode);
        self.filters(ui, base, mode, &explained);
        let total = (ui.available_height() - DIVIDER).max(1.0);
        let list_height = (total * self.share).round();
        ui.allocate_ui(vec2(ui.available_width(), list_height), |ui| self.list(ui, &explained, list_height));
        let delta = self.divider(ui);
        if delta != 0.0 {
            self.share = ((list_height + delta) / total).clamp(MIN_SHARE, 1.0 - MIN_SHARE);
        }
        ScrollArea::vertical().id_salt("gallery.tokens.detail").auto_shrink([false, false]).show(ui, |ui| self.detail(ui, &explained, scan)).inner
    }

    /// The draggable and keyboard-operable divider between list and detail; returns the
    /// requested movement in points.
    fn divider(&mut self, ui: &mut Ui) -> f32 {
        let tokens = Tokens::of(ui);
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), DIVIDER), Sense::hover());
        let style = ResizeHandleStyle {
            hovered: tokens.color.accent.solid.to_egui().gamma_multiply(0.5),
            dragged: tokens.color.accent.solid.to_egui(),
            focused: tokens.color.line.focus.to_egui(),
        };
        let hairline = scale::Stroke::Hairline.egui(ui.ctx(), tokens.color.line.divider.to_egui());
        ui.painter().hline(rect.x_range(), rect.center().y, hairline);
        ResizeHandle::new(ui.make_persistent_id("gallery.tokens.split"), SplitAxis::Vertical, rect, "Resize the token list", style).show(ui).delta
    }

    /// The tokens as a list: group headings, then rows of path, swatch and hex.
    fn list(&mut self, ui: &mut Ui, e: &Explained, height: f32) {
        let mut rows: Vec<RowInfo<Row>> = Vec::new();
        let mut group = "";
        for o in &e.colors {
            let token = TokenRef::Color(o.token);
            if !self.shows(e, token, o.pin.is_some(), o.contrast_move.is_some()) {
                continue;
            }
            let g = group_of(o.token);
            if g != group {
                group = g;
                rows.push(RowInfo::section(Row::Group(g)));
            }
            rows.push(RowInfo::leaf(Row::Token(token)));
        }
        let components: Vec<TokenRef> = e
            .components
            .iter()
            .filter(|o| self.shows(e, TokenRef::Component(o.field), o.pin.is_some(), o.contrast_move.is_some()))
            .map(|o| TokenRef::Component(o.field))
            .collect();
        if !components.is_empty() {
            rows.push(RowInfo::section(Row::Group("component colours")));
            rows.extend(components.into_iter().map(|t| RowInfo::leaf(Row::Token(t))));
        }
        let mut group = "";
        for (i, o) in e.sizes.iter().enumerate() {
            if !self.shows_size(o) {
                continue;
            }
            let g = size_group(o);
            if g != group {
                group = g;
                rows.push(RowInfo::section(Row::Group(g)));
            }
            rows.push(RowInfo::leaf(Row::Size(i)));
        }
        let selected_row = self
            .selected
            .map(Row::Token)
            .or_else(|| self.selected_size.as_ref().and_then(|p| e.sizes.iter().position(|o| o.path == *p)).map(Row::Size));
        let mut selection = selected_row.filter(|r| rows.iter().any(|i| i.key == *r)).map_or_else(Selection::default, Selection::only);
        let out = List::new("gallery.tokens.list", "Tokens", rows.as_slice())
            .selection(&mut selection)
            .mode(SelectionMode::Single)
            .type_ahead(true)
            .max_height(height)
            .show(ui, |_, row| match row {
                Row::Group(name) => ListRow::new(*row, *name),
                Row::Size(i) => {
                    let o = &e.sizes[*i];
                    let words = format!("{} · {}", value_words(o.value), layer_words(o.layer));
                    ListRow::new(*row, o.path.as_str()).trailing_item(SIZE_WIDTH, move |ui| {
                        ui.monospace(words.clone());
                    })
                }
                Row::Token(token) => {
                    let value = value_of(e, *token);
                    ListRow::new(*row, token.path())
                        .thumb(ThumbSize::Small, move |painter, rect| {
                            painter.rect_filled(rect, 0.0, value.to_egui());
                        })
                        .trailing_item(HEX_WIDTH, move |ui| {
                            ui.monospace(value.to_hex());
                        })
                }
            });
        for event in out.events {
            match event {
                ListEvent::Selection(SelectionChange { row: Some(Row::Token(t)), .. }) | ListEvent::Activate(Row::Token(t)) => self.select(t),
                ListEvent::Selection(SelectionChange { row: Some(Row::Size(i)), .. }) | ListEvent::Activate(Row::Size(i)) => {
                    self.selected = None;
                    self.selected_size = e.sizes.get(i).map(|o| o.path.clone());
                }
                _ => {}
            }
        }
    }

    /// `explain` for what `base` shows in `mode`, computed again only when that changes.
    pub(crate) fn explained(&mut self, base: &Base, mode: Mode) -> Arc<Explained> {
        let theme = base.inputs.theme(mode);
        let appearance = base.shown(mode).appearance;
        let app = base.inputs.app.as_ref();
        let sizes = &base.inputs.sizes;
        match &self.explained {
            Some((t, a, p, s, e)) if t == theme && *a == appearance && p.as_ref() == app && s == sizes => Arc::clone(e),
            _ => {
                let e = Arc::new(explain_sized(theme, appearance, app, sizes));
                self.explained = Some((theme.clone(), appearance, app.cloned(), sizes.clone(), Arc::clone(&e)));
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
            let filter = SearchField::new("gallery.tokens.filter", "Filter by path", &self.filter, "Clear filter")
                .placeholder("Filter by path")
                .policy(fields::policy())
                .show(ui);
            fields::apply(&filter, &mut self.filter);
            let _ = Checkbox::bool("Pinned", &mut self.pinned_only).show(ui);
            let _ = Checkbox::bool("Moved by contrast", &mut self.moved_only).show(ui);
            let _ = Checkbox::bool("Failing pairs only", &mut self.failing_only).show(ui);
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

    /// Whether a size row passes the filters. Contrast filters hide sizes, which have no colour.
    fn shows_size(&self, o: &SizeOrigin) -> bool {
        (self.filter.is_empty() || o.path.contains(&self.filter.to_lowercase()))
            && (!self.pinned_only || o.layer.from_app())
            && !self.moved_only
            && !self.failing_only
    }

    fn detail(&mut self, ui: &mut Ui, e: &Explained, scan: &Scan) -> Option<&'static str> {
        let size = self.selected_size.as_ref().and_then(|p| e.sizes.iter().find(|o| o.path == *p));
        if let Some(origin) = size {
            ui.heading(origin.path.as_str());
            self.layer.editor(ui, origin);
            Divider::horizontal().show(ui);
            SectionHeader::new("gallery.app_layer", "App sizes").show(ui, |ui| self.layer.file(ui));
            return None;
        }
        let Some(selected) = self.selected else {
            ui.label("Select a row to see its contrast pairs and ramp.");
            Divider::horizontal().show(ui);
            SectionHeader::new("gallery.app_layer", "App sizes").show(ui, |ui| self.layer.file(ui));
            return None;
        };
        ui.heading(selected.path());
        let origin = match selected {
            TokenRef::Color(t) => e.color(t).inspect(|o| color_facts(ui, o)),
            TokenRef::Component(f) => e.field(f).and_then(|o| {
                field_facts(ui, o);
                o.reads.and_then(|t| e.color(t))
            }),
        };
        let used = used_by(ui, e, selected, scan);
        if let Some(token) = used.select {
            self.selected = Some(token);
        }
        Divider::horizontal().show(ui);
        ui.strong("Contrast pairs");
        pairs(ui, e, selected);
        let Some(o) = origin else { return used.open };
        if let Some((ramp, step, toward)) = e.ramps.steps_of(o.source) {
            Divider::horizontal().show(ui);
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
        used.open
    }
}

/// What the Used by and Read by sections reported.
#[derive(Default)]
struct UsedBy {
    /// The component a link named.
    open: Option<&'static str>,
    /// The token a link named.
    select: Option<TokenRef>,
}

/// The components that paint with `token` (from the scan, which fills in as it runs) and the
/// tokens it is read through: the component fields that default to a semantic token, or the
/// semantic token a field reads.
fn used_by(ui: &mut Ui, e: &Explained, token: TokenRef, scan: &Scan) -> UsedBy {
    let mut out = UsedBy::default();
    SectionHeader::new("gallery.tokens.used_by", "Used by").show(ui, |ui| {
        scan.mark_shown();
        let painters = scan.painters(token);
        let headline = match scan.progress() {
            Some((done, total)) if done < total => format!("Scanning stories… {done} of {total}"),
            Some(_) if painters.is_empty() => "Not painted by any component at rest".to_owned(),
            Some(_) => format!("Painted by {} {}", painters.len(), if painters.len() == 1 { "component" } else { "components" }),
            None => String::new(),
        };
        ui.add(Text::new(headline));
        for (component, shapes) in painters {
            direction::horizontal(ui, |ui| {
                if ui.add(Link::new(component).id_salt(("gallery.tokens.used_by", component))).clicked() {
                    out.open = Some(component);
                }
                ui.add(Text::new(format!("{shapes} {}", if shapes == 1 { "shape" } else { "shapes" })).content(Content::Muted));
            });
        }
        ui.add(Text::new("Stories at rest; blended colours not counted").role(TypeRole::Caption).content(Content::Muted));
    });
    let read: Vec<TokenRef> = match token {
        TokenRef::Color(t) => e.readers(t).into_iter().map(TokenRef::Component).collect(),
        TokenRef::Component(f) => e.field(f).and_then(|o| o.reads).map(TokenRef::Color).into_iter().collect(),
    };
    let title = if matches!(token, TokenRef::Color(_)) { "Read by" } else { "Reads" };
    SectionHeader::new("gallery.tokens.read_by", title).show(ui, |ui| {
        if read.is_empty() {
            ui.add(Text::new("Nothing reads it by default").content(Content::Muted));
        }
        for other in read {
            if ui.add(Link::new(other.path()).role(TypeRole::Mono).id_salt(("gallery.tokens.read_by", other.path()))).clicked() {
                out.select = Some(other);
            }
        }
    });
    out
}

/// The theme's seeds and shape as read-only rows: a swatch and hex per colour seed, then the
/// modes, corners, edge and contrast intent. Seeds are those `mode` derives from.
pub fn basics(ui: &mut Ui, theme: &ThemeDef, mode: Mode) {
    SectionHeader::new("gallery.theme.basics", "Basics").show(ui, |ui| {
        let seeds = theme.seeds_for(mode);
        FormGrid::new("gallery.theme.basics.grid").show(ui, |form| {
            let colours = [
                ("Neutral", seeds.neutral),
                ("Accent", seeds.accent),
                ("Good", seeds.status.good),
                ("Warn", seeds.status.warn),
                ("Bad", seeds.status.bad),
                ("Info", seeds.status.info),
            ];
            for (name, color) in colours {
                form.row(format!("{name}:"), |ui, _| {
                    let swatch =
                        ui.add(Swatch::new(("gallery.theme.seed", name), format!("{name} seed"), color).size(ControlSize::Sm).enabled(false));
                    ui.monospace(color.to_hex());
                    swatch
                });
            }
            form.value("Modes:", variants_words(theme.variants));
            form.value("Corners:", corners_words(theme));
            form.value("Edge:", edge_words(theme));
            form.value("Contrast:", if theme.contrast_intent == ContrastIntent::Low { "deliberately low" } else { "standard" });
        });
    });
}

fn variants_words(variants: Variants) -> &'static str {
    match variants {
        Variants::Both => "light and dark",
        Variants::LightOnly => "light only",
        Variants::DarkOnly => "dark only",
    }
}

fn corners_words(theme: &ThemeDef) -> String {
    let Some(shape) = theme.shape else { return "from the Size preset".to_owned() };
    match (shape.radius, shape.radius_offset) {
        (Some(r), _) => format!("control {}, popup {}, window {}, panel {}, button {}", r.control, r.popup, r.window, r.panel, r.button),
        (None, Some(o)) => format!("preset, control {:+}, popup {:+}, window {:+}", o.control, o.popup, o.window),
        (None, None) => "from the Size preset".to_owned(),
    }
}

fn edge_words(theme: &ThemeDef) -> String {
    match theme.shape.and_then(|s| s.edge) {
        Some(EdgeSeed::Flat) => "flat".to_owned(),
        Some(EdgeSeed::Bevel { width }) => format!("bevel, {width} px"),
        None => "from the Size preset".to_owned(),
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

/// Every pair `token` takes part in, each as a swatch and the other colour's path with the
/// ratio, minimum, result and pass note wrapped under it. Results are words, not colour alone.
fn pairs(ui: &mut Ui, e: &Explained, token: TokenRef) {
    let mut any = false;
    for p in e.pairs_of(token) {
        any = true;
        let pair = p.pair;
        let (other, role) = if pair.fg == token { (pair.bg, "on") } else { (pair.fg, "under") };
        ui.horizontal(|ui| {
            swatch(ui, value_of(e, other), SWATCH);
            ui.add(egui::Label::new(RichText::new(format!("{role} {}", other.path())).monospace()).wrap());
        });
        ui.horizontal_wrapped(|ui| {
            ui.monospace(format!("{:.2} of {:.1}", pair.ratio, pair.minimum));
            if pair.passes() {
                ui.label("pass");
            } else {
                ui.label(RichText::new("FAIL").strong());
            }
            if p.fixed_by_pass {
                ui.label(format!("fixed by pass (was {:.2})", p.ratio_before));
            } else if p.moved_by_pass {
                ui.label(format!("moved (was {:.2})", p.ratio_before));
            }
        });
        ui.add_space(Tokens::of(ui).space.gap.section);
    }
    if !any {
        ui.label("No contrast rule measures this colour.");
    }
}

/// Twelve swatches with the used step (and the step a mix moves toward) outlined and their
/// numbers in bold.
fn ramp_row(ui: &mut Ui, ramp: &[Rgba; 12], used: usize, toward: Option<usize>) {
    let mark = Tokens::of(ui).color.text.normal.to_egui();
    let ring = scale::Stroke::W2.egui(ui.ctx(), mark);
    ui.horizontal(|ui| {
        for (i, &c) in ramp.iter().enumerate() {
            let step = i + 1;
            ui.vertical(|ui| {
                let rect = swatch(ui, c, RAMP_SWATCH);
                let label = RichText::new(step.to_string()).small();
                if step == used || toward == Some(step) {
                    ui.painter().rect_stroke(rect.expand(scale::Space::S2.px()), scale::Radius::R3.px(), ring, StrokeKind::Outside);
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
    let hairline = scale::Stroke::Hairline.egui(ui.ctx(), border);
    ui.painter().rect(rect, scale::Radius::R2.px(), color.to_egui(), hairline, StrokeKind::Inside);
    rect
}

pub(crate) fn value_of(e: &Explained, token: TokenRef) -> Rgba {
    match token {
        TokenRef::Color(t) => e.resolved.tokens.color(t),
        TokenRef::Component(f) => e.resolved.tokens.component.color(f).unwrap_or(Rgba::TRANSPARENT),
    }
}

/// The heading a size row is listed under: its base group, or the component sizes.
fn size_group(o: &SizeOrigin) -> &'static str {
    if !o.base {
        return "component sizes";
    }
    match o.path.split_once('.').map_or(o.path.as_str(), |(g, _)| g) {
        "size" => "sizes",
        "space" => "spacing",
        "radius" => "corner radii",
        _ => "type",
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

#[cfg(test)]
mod tests {
    use super::*;
    use craft_theme::explain;

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
