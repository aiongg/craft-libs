//! Stories of the font picker, Glyphs panel, Missing Fonts dialog and axis sliders. They draw
//! from the two faces egui ships (Ubuntu Light and Hack) in an in-memory catalog.

use std::sync::{Arc, Mutex, PoisonError};

use craft_font_catalog::{Limits, Os};
use craft_ui::controls::Button;
use craft_ui::dialog::{DialogHost, DialogResult, StackDialog};
use craft_ui::story::{Level, NoArgs, Story};
use craft_ui::typed_story;
use egui::{Color32, ColorImage, Ui};
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::{FontRef, MetadataProvider as _};

use crate::axes::{AxisSliders, FontAxis, NamedInstance};
use crate::glyphs::{GlyphSource, GlyphsPanel, GlyphsPanelSettings};
use crate::missing::{DocumentFont, MissingFontsDialog, document_fonts};
use crate::{Catalog, FontEnv, Source};

/// `patterns/font-picker/field`, `patterns/font-picker/filters` (with `font-picker`),
/// `patterns/glyphs-panel/panel`, `patterns/missing-fonts/dialog`, `composites/font-axes/sliders`.
pub fn stories() -> Vec<Story> {
    let mut stories = Vec::new();
    #[cfg(feature = "font-picker")]
    {
        stories.push(typed_story!("patterns/font-picker/field", Level::Patterns, "Font picker", "Field", NoArgs {}, picker::field));
        stories.push(typed_story!("patterns/font-picker/filters", Level::Patterns, "Font picker", "Filters", NoArgs {}, picker::filters));
    }
    stories.push(typed_story!("patterns/glyphs-panel/panel", Level::Patterns, "Glyphs panel", "Panel", NoArgs {}, glyphs_panel));
    stories.push(typed_story!("patterns/missing-fonts/dialog", Level::Patterns, "Missing fonts", "Dialog", NoArgs {}, missing_fonts));
    stories.push(typed_story!("composites/font-axes/sliders", Level::Composites, "Font axes", "Sliders", NoArgs {}, font_axes));
    stories
}

const UBUNTU: &[u8] = epaint_default_fonts::UBUNTU_LIGHT;
const HACK: &[u8] = epaint_default_fonts::HACK_REGULAR;

/// The two shipped faces, in memory, with no folder scanned.
fn catalog() -> Arc<Catalog> {
    let catalog = Arc::new(Catalog::new(FontEnv { os: Os::None, ..FontEnv::default() }).with_font_service(false));
    for bytes in [UBUNTU, HACK] {
        let _ = catalog.add_bytes(bytes.to_vec(), Source::Bundled, &Limits::default());
    }
    catalog
}

/// A story's state, kept in egui memory under the story's own id.
fn state<T: Send + 'static>(ui: &Ui, name: &str, make: impl FnOnce() -> T) -> Arc<Mutex<T>> {
    let key = ui.scope_id().with(("font-manager-text-story", name));
    ui.data_mut(|d| d.get_temp_mut_or_insert_with(key, || Arc::new(Mutex::new(make()))).clone())
}

#[cfg(feature = "font-picker")]
mod picker {
    use super::{Arc, Catalog, NoArgs, PoisonError, Ui, catalog, state};
    use crate::picker::{FontFilters, FontPicker, FontPickerSettings};

    struct Picker {
        catalog: Arc<Catalog>,
        family: String,
        settings: FontPickerSettings,
        filters: FontFilters,
        shown: bool,
    }

    fn show(ui: &mut Ui, name: &str, make: impl FnOnce() -> Picker) {
        let shared = state(ui, name, make);
        let mut guard = shared.lock().unwrap_or_else(PoisonError::into_inner);
        let Picker { catalog, family, settings, filters, shown } = &mut *guard;
        ui.set_min_height(200.0);
        FontPicker::new(name, catalog, family)
            .settings(settings)
            .filters(filters)
            .width(220.0)
            .list_height(150.0)
            .start_open(!*shown && !craft_ui::story::in_overview(ui.ctx()))
            .show(ui);
        *shown = true;
    }

    /// The field with its list open (closed in an Overview tile, where the list would cover the others) over the two faces, the recent one on top.
    pub(super) fn field(ui: &mut Ui, _: &mut NoArgs) {
        show(ui, "field", || Picker {
            catalog: catalog(),
            family: "Ubuntu".to_owned(),
            settings: FontPickerSettings { recent: vec!["Ubuntu".to_owned()], ..FontPickerSettings::default() },
            filters: FontFilters::default(),
            shown: false,
        });
    }

    /// The list open with the favourites filter on and Hack starred.
    pub(super) fn filters(ui: &mut Ui, _: &mut NoArgs) {
        show(ui, "filters", || Picker {
            catalog: catalog(),
            family: "Hack".to_owned(),
            settings: FontPickerSettings { favorites: vec!["Hack".to_owned()], ..FontPickerSettings::default() },
            filters: FontFilters { favorites: true, ..FontFilters::default() },
            shown: false,
        });
    }
}

/// Opens `make` on a [`DialogHost`] kept in egui memory while the story's dialog is shown
/// (`craft_ui::story::dialog_shown`), and shows its answer afterwards.
fn dialog_story<D: StackDialog>(ui: &mut Ui, name: &str, make: impl FnOnce() -> D, describe: impl FnOnce(D::Result) -> String) {
    struct Dialogs {
        host: DialogHost,
        answer: Option<String>,
        open: bool,
    }
    if !craft_ui::story::dialog_claim(ui.ctx(), name, ui.scope_id()) {
        return;
    }
    let shared = state(ui, name, || Dialogs { host: DialogHost::new(), answer: None, open: true });
    let mut guard = shared.lock().unwrap_or_else(PoisonError::into_inner);
    let Dialogs { host, answer, open } = &mut *guard;
    let ctx = ui.ctx().clone();
    if craft_ui::story::dialog_shown(ui, *open) {
        if !host.is_open::<D>() {
            host.open(make());
            *answer = None;
        }
        host.show(&ctx);
        if let Some(result) = host.take::<D>() {
            *answer = Some(match result {
                DialogResult::Ok(value) => describe(value),
                DialogResult::Cancel => "Cancelled".to_owned(),
            });
            craft_ui::story::dialog_closed(ui, open);
        }
    } else if !craft_ui::story::in_overview(&ctx) && ui.add(Button::new("Show dialog")).clicked() {
        *open = true;
    }
    if let Some(text) = answer {
        ui.label(format!("Result: {text}"));
    }
}

/// Two missing fonts and one installed.
fn missing_fonts(ui: &mut Ui, _: &mut NoArgs) {
    let font = |family: &str, style: &str| DocumentFont { family: family.to_owned(), style: style.to_owned(), postscript_name: None };
    dialog_story(
        ui,
        "missing",
        || {
            let catalog = catalog();
            let rows = document_fonts(&catalog, &[font("Ubuntu", "Light"), font("Gotham", "Bold"), font("Proxima Nova", "Regular")]);
            MissingFontsDialog::new(catalog, rows)
        },
        |result| format!("{} replacements", result.replacements.len()),
    );
}

/// Weight and Width axes with three named instances.
fn font_axes(ui: &mut Ui, _: &mut NoArgs) {
    const WGHT: [u8; 4] = *b"wght";
    const WDTH: [u8; 4] = *b"wdth";
    let axes = [
        FontAxis { tag: WGHT, name: "Weight".to_owned(), min: 100.0, default: 400.0, max: 900.0, hidden: false },
        FontAxis { tag: WDTH, name: "Width".to_owned(), min: 75.0, default: 100.0, max: 100.0, hidden: false },
    ];
    let instance = |name: &str, weight: f32, width: f32| NamedInstance { name: name.to_owned(), coordinates: vec![(WGHT, weight), (WDTH, width)] };
    let instances = [instance("Light", 300.0, 100.0), instance("Regular", 400.0, 100.0), instance("Bold Condensed", 700.0, 75.0)];
    let values = state(ui, "axes", || vec![(WGHT, 400.0), (WDTH, 100.0)]);
    let mut values = values.lock().unwrap_or_else(PoisonError::into_inner);
    let response = AxisSliders::new(&axes, &mut values).instances(&instances).show(ui);
    for (tag, value) in response.changed {
        if let Some(at) = values.iter_mut().find(|(t, _)| *t == tag) {
            at.1 = value;
        }
    }
}

/// The Glyphs panel over Ubuntu Light and Hack, editing, with three recent glyphs.
fn glyphs_panel(ui: &mut Ui, _: &mut NoArgs) {
    struct Panel {
        family: String,
        style: String,
        settings: GlyphsPanelSettings,
    }
    let shared = state(ui, "glyphs", || Panel {
        family: "Ubuntu".to_owned(),
        style: "Light".to_owned(),
        settings: GlyphsPanelSettings { recent: ["€", "→", "ß"].map(str::to_owned).to_vec(), ..GlyphsPanelSettings::default() },
    });
    let mut guard = shared.lock().unwrap_or_else(PoisonError::into_inner);
    let Panel { family, style, settings } = &mut *guard;
    let families = ["Ubuntu".to_owned(), "Hack".to_owned()];
    let styles = [if family == "Hack" { "Regular" } else { "Light" }.to_owned()];
    *style = styles[0].clone();
    GlyphsPanel::new(&FaceGlyphs, family, style, settings).editing(true).families(&families).styles(&styles).show(ui);
}

/// Glyphs of the shipped faces, drawn by skrifa's outlines.
struct FaceGlyphs;

impl FaceGlyphs {
    fn font(family: &str) -> Option<FontRef<'static>> {
        FontRef::new(if family == "Hack" { HACK } else { UBUNTU }).ok()
    }
}

impl GlyphSource for FaceGlyphs {
    fn charmap(&self, family: &str, _style: &str) -> Vec<char> {
        let Some(font) = Self::font(family) else { return Vec::new() };
        let mut chars: Vec<char> = font.charmap().mappings().filter_map(|(code, _)| char::from_u32(code)).filter(|c| !c.is_control()).collect();
        chars.sort_unstable();
        chars.dedup();
        chars
    }

    fn glyph(&self, family: &str, _style: &str, c: char, px: u32) -> Option<ColorImage> {
        let font = Self::font(family)?;
        let id = font.charmap().map(c)?;
        let em = px as f32 * 0.8;
        let size = Size::new(em);
        let advance = font.glyph_metrics(size, LocationRef::default()).advance_width(id).unwrap_or(em);
        let baseline = px as f32 * 0.78;
        let mut pen = Outline { edges: Vec::new(), start: (0.0, 0.0), at: (0.0, 0.0), dx: (px as f32 - advance) / 2.0, baseline };
        font.outline_glyphs().get(id)?.draw(DrawSettings::unhinted(size, LocationRef::default()), &mut pen).ok()?;
        Some(rasterize(&pen.edges, px as usize))
    }

    fn in_category(&self, c: char, category: &str) -> bool {
        let code = c as u32;
        match category {
            "Punctuation" => c.is_ascii_punctuation() && !"$+<=>^`|~".contains(c) || "¡¿«»·".contains(c) || (0x2010..=0x2027).contains(&code),
            "Numbers" => c.is_numeric(),
            "Currency" => "$¢£¤¥".contains(c) || (0x20A0..=0x20CF).contains(&code),
            "Symbols" => "©®°§¶†‡•".contains(c) || (0x2100..=0x2BFF).contains(&code) && !(0x2190..=0x22FF).contains(&code),
            "Math Symbols" => "+−×÷±<=>¬".contains(c) || (0x2200..=0x22FF).contains(&code),
            "Arrows" => (0x2190..=0x21FF).contains(&code),
            "Latin" => c.is_alphabetic() && code < 0x250,
            "Greek" => (0x370..=0x3FF).contains(&code),
            "Cyrillic" => (0x400..=0x52F).contains(&code),
            "Letterlike Symbols" => (0x2100..=0x214F).contains(&code),
            "Superscripts and Subscripts" => "¹²³".contains(c) || (0x2070..=0x209F).contains(&code),
            "Ornaments" => (0x2766..=0x2767).contains(&code) || code == 0x2042,
            _ => false,
        }
    }
}

type Segment = ((f32, f32), (f32, f32));

/// An outline flattened to line segments in image pixels, y down.
struct Outline {
    edges: Vec<Segment>,
    start: (f32, f32),
    at: (f32, f32),
    dx: f32,
    baseline: f32,
}

impl Outline {
    fn point(&self, x: f32, y: f32) -> (f32, f32) {
        (x + self.dx, self.baseline - y)
    }

    fn line(&mut self, to: (f32, f32)) {
        self.edges.push((self.at, to));
        self.at = to;
    }
}

const FLATTEN: usize = 8;

impl OutlinePen for Outline {
    fn move_to(&mut self, x: f32, y: f32) {
        self.at = self.point(x, y);
        self.start = self.at;
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let to = self.point(x, y);
        self.line(to);
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        let (p0, p1, p2) = (self.at, self.point(cx0, cy0), self.point(x, y));
        for i in 1..=FLATTEN {
            let t = i as f32 / FLATTEN as f32;
            let u = 1.0 - t;
            self.line((u * u * p0.0 + 2.0 * u * t * p1.0 + t * t * p2.0, u * u * p0.1 + 2.0 * u * t * p1.1 + t * t * p2.1));
        }
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        let (p0, p1, p2, p3) = (self.at, self.point(cx0, cy0), self.point(cx1, cy1), self.point(x, y));
        for i in 1..=FLATTEN {
            let t = i as f32 / FLATTEN as f32;
            let u = 1.0 - t;
            let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
            self.line((a * p0.0 + b * p1.0 + c * p2.0 + d * p3.0, a * p0.1 + b * p1.1 + c * p2.1 + d * p3.1));
        }
    }

    fn close(&mut self) {
        let start = self.start;
        self.line(start);
    }
}

/// Sub-scanlines per pixel row; coverage across a row is exact.
const SUB_ROWS: usize = 4;

/// White with the nonzero-winding coverage of `edges` in the alpha channel.
fn rasterize(edges: &[Segment], side: usize) -> ColorImage {
    let mut alpha = vec![0.0f32; side * side];
    let mut crossings: Vec<(f32, i32)> = Vec::new();
    for row in 0..side {
        let line = &mut alpha[row * side..(row + 1) * side];
        for sub in 0..SUB_ROWS {
            let y = row as f32 + (sub as f32 + 0.5) / SUB_ROWS as f32;
            crossings.clear();
            for &((x0, y0), (x1, y1)) in edges {
                if (y0 <= y && y < y1) || (y1 <= y && y < y0) {
                    crossings.push((x0 + (y - y0) * (x1 - x0) / (y1 - y0), if y1 > y0 { 1 } else { -1 }));
                }
            }
            crossings.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut winding = 0;
            for pair in crossings.windows(2) {
                winding += pair[0].1;
                if winding == 0 {
                    continue;
                }
                let (from, to) = (pair[0].0.clamp(0.0, side as f32), pair[1].0.clamp(0.0, side as f32));
                let mut x = from.floor() as usize;
                while (x as f32) < to && x < side {
                    let covered = (to.min(x as f32 + 1.0) - from.max(x as f32)).max(0.0);
                    line[x] += covered / SUB_ROWS as f32;
                    x += 1;
                }
            }
        }
    }
    ColorImage::new([side, side], alpha.iter().map(|a| Color32::from_white_alpha((a.clamp(0.0, 1.0) * 255.0).round() as u8)).collect::<Vec<_>>())
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::accesskit::Role;
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable as _;

    // The rasteriser draws the glyph's ink: an 'A' has coverage, a space none.
    #[test]
    fn glyphs_have_ink() {
        let ink = |c| FaceGlyphs.glyph("Ubuntu", "Light", c, 32).map(|image| image.pixels.iter().filter(|p| p.a() > 0).count());
        assert!(ink('A').is_some_and(|n| n > 40));
        assert!(ink(' ').is_none_or(|n| n == 0));
        assert!(FaceGlyphs.charmap("Hack", "Regular").contains(&'€'));
    }

    // The picker story opens with its list showing the catalog's families.
    #[cfg(feature = "font-picker")]
    #[test]
    fn the_picker_story_starts_with_the_list_open() {
        let mut harness = Harness::builder()
            .with_size([760.0, 640.0])
            .with_context_setup(|ctx| craft_ui::theme::install(ctx, &craft_ui::theme::Base::suite_default()))
            .build_ui(|ui| picker::field(ui, &mut NoArgs {}));
        harness.run();
        let rows = harness.query_all_by_role(Role::ListBoxOption).count();
        assert!(rows >= 2, "{rows} rows");
    }
}
