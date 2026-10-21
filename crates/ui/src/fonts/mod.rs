//! The UI font stack every Craft app installs.
//!
//! egui draws a character only if one of its installed faces has it. [`UiFonts`] builds the same
//! fallback order for every app, so a label or file name in any shipped script draws in every app:
//!
//! 1. the app's own faces (UI face, semibold, mono);
//! 2. craft-fonts faces for the UI language;
//! 3. craft-fonts faces for every other script the app ships, so a Japanese interface still draws
//!    Arabic file names;
//! 4. installed system fonts, through egui's font provider, at regular weight ([`RegularWeight`]);
//! 5. emoji faces ([`Emoji`]) and egui's own default faces.
//!
//! When the app's faces don't cover the UI language's main script (Arabic, Japanese, Thai), that
//! script's face goes first instead of the app's face. egui takes a row's height from the first
//! face of a family, so the UI language's own face sets the row height and its marks aren't
//! clipped.
//!
//! Apps keep their font files: their `build.rs` embeds craft-fonts' faces as [`CraftFace`]s.
//! [`UiFonts::stack`] returns a [`Stack`] the app can still edit (add, replace and reorder faces)
//! before [`Stack::install`].

mod coverage;
mod providers;
#[cfg(test)]
mod tests;

use std::sync::Arc;

use egui::epaint::text::{Fonts, TextOptions};
use egui::{FontData, FontDefinitions, FontFamily, FontProvider, Style, TextStyle};

pub use coverage::{Missing, Sample, assert_covers, missing_glyphs};
pub use providers::{FaceList, RegularWeight, set_weight};

/// A font file from craft-fonts that the app embeds, one per line of `fonts/manifest.txt`.
#[derive(Clone, Copy, Debug)]
pub struct CraftFace {
    /// Family name, e.g. `"Noto Sans Arabic"`.
    pub family: &'static str,
    /// `"Regular"` or `"Bold"`; faces with other styles are not used for UI text.
    pub style: &'static str,
    /// The manifest's script tags: ISO 15924 codes such as `Arab`, `Thai`, `Jpan`, `Kore`, `Hans`
    /// and `Hant`. Hong Kong forms, which have no ISO 15924 code, are tagged `Hant-HK`.
    pub scripts: &'static [&'static str],
    pub bytes: &'static [u8],
    /// Face index in a font collection (`.ttc`); `0` for a single font.
    pub index: u32,
}

impl CraftFace {
    /// The egui font name, e.g. `"Noto Sans Arabic Regular"`.
    pub fn name(&self) -> String {
        format!("{} {}", self.family, self.style)
    }

    fn data(&self) -> FontData {
        FontData { index: self.index, ..FontData::from_static(self.bytes) }
    }

    fn region(&self) -> Option<Han> {
        self.scripts.iter().find_map(|tag| match *tag {
            "Hans" => Some(Han::Simplified),
            "Hant" => Some(Han::Traditional),
            "Hant-HK" => Some(Han::HongKong),
            "Jpan" => Some(Han::Japanese),
            "Kore" => Some(Han::Korean),
            _ => None,
        })
    }

    fn covers(&self, script: &str) -> bool {
        self.scripts.iter().any(|tag| {
            *tag == script
                || match *tag {
                    "Jpan" => matches!(script, "Hani" | "Hira" | "Kana"),
                    "Kore" => matches!(script, "Hang" | "Hani"),
                    "Hans" | "Hant" | "Hant-HK" => script == "Hani",
                    _ => false,
                }
        })
    }

    /// `0` for the style a family of `weight` wants, `1` for an acceptable stand-in.
    fn style_rank(&self, weight: u16) -> Option<u8> {
        let regular = self.style.eq_ignore_ascii_case("regular");
        let bold = ["bold", "semibold"].iter().any(|s| self.style.eq_ignore_ascii_case(s));
        match (weight >= 600, regular, bold) {
            (false, true, _) | (true, _, true) => Some(0),
            (true, true, _) => Some(1),
            _ => None,
        }
    }
}

/// The regional forms a language's Han characters take.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Han {
    /// Mainland China and Singapore (SC).
    Simplified,
    /// Taiwan (TC).
    Traditional,
    /// Hong Kong and Macau (HK).
    HongKong,
    /// Japanese kanji (JP).
    Japanese,
    /// Korean hanja (KR).
    Korean,
}

/// The scripts a language is written in, by language tag.
///
/// craft-i18n's `scripts` module answers both questions; an app adapts it in a few lines (see the
/// crate README).
pub trait LanguageScripts {
    /// ISO 15924 codes of the language's interface text, main script first: `ja` → `Hani`, `Hira`,
    /// `Kana`; `ar` → `Arab`. The unions `Jpan`, `Kore`, `Hans` and `Hant` are accepted too.
    fn scripts(&self, language: &str) -> Vec<&'static str>;

    /// The regional Han forms the language uses, `None` for a language not written with Han.
    fn han(&self, language: &str) -> Option<Han>;
}

/// A font family the app uses, beyond what its own faces imply.
#[derive(Clone, Debug)]
pub struct Family {
    family: FontFamily,
    weight: u16,
    lead: Option<String>,
}

impl Family {
    /// A named family, e.g. `Family::named("semibold").weight(600)`.
    pub fn named(name: &str) -> Self {
        Self { family: FontFamily::Name(name.into()), weight: 400, lead: None }
    }

    /// egui's proportional family.
    pub fn proportional() -> Self {
        Self { family: FontFamily::Proportional, weight: 400, lead: None }
    }

    /// The weight (`wght`) this family's faces draw at: bold craft faces are used from 600 up, and
    /// variable faces are set to this weight.
    pub fn weight(mut self, weight: u16) -> Self {
        self.weight = weight;
        self
    }

    /// Put `script`'s faces first, e.g. `Family::named("arabic").script("Arab")` for the
    /// right-to-left runs an app shapes itself.
    pub fn script(mut self, script: &str) -> Self {
        self.lead = Some(script.to_owned());
        self
    }
}

/// Emoji faces the app embeds, asked in this order after the system fonts.
#[derive(Clone, Default)]
pub struct Emoji {
    /// A colour emoji font, e.g. Noto Color Emoji (COLRv1) on desktop.
    pub color: Option<FontData>,
    /// A lighter colour emoji font for web builds, e.g. Twemoji.
    pub web: Option<FontData>,
    /// A monochrome emoji font, e.g. Noto Emoji; the last resort.
    pub monochrome: Option<FontData>,
}

/// Where installed system fonts come in the stack.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SystemFonts {
    /// After every craft face.
    #[default]
    AfterCraft,
    /// After the UI language's craft faces, before the faces for other scripts.
    BeforeCraft,
    /// Not used: text draws the same on every machine.
    Off,
}

/// Scripts the app's own UI faces cover unless [`UiFonts::app_scripts`] says otherwise.
const APP_SCRIPTS: &[&str] = &["Latn", "Cyrl", "Grek"];

/// craft-fonts' UI face for Japanese; its manifest also lists serif Japanese faces.
const DEFAULT_PREFER: &[(&str, &str)] = &[("Jpan", "BIZ UDPGothic")];

struct AppFace {
    name: String,
    data: FontData,
    families: Vec<FontFamily>,
}

/// The configuration of an app's UI fonts. Build it, then [`Self::install`] it, or take its
/// [`Self::stack`] to edit first.
pub struct UiFonts<'a> {
    scripts: &'a dyn LanguageScripts,
    app_faces: Vec<AppFace>,
    app_scripts: Vec<String>,
    craft_faces: Vec<CraftFace>,
    languages: Vec<String>,
    ui_language: String,
    prefer: Vec<(String, String)>,
    families: Vec<Family>,
    emoji: Emoji,
    system: SystemFonts,
    system_provider: Option<Arc<dyn FontProvider>>,
}

impl<'a> UiFonts<'a> {
    pub fn new(scripts: &'a dyn LanguageScripts) -> Self {
        Self {
            scripts,
            app_faces: Vec::new(),
            app_scripts: APP_SCRIPTS.iter().map(|s| (*s).to_owned()).collect(),
            craft_faces: Vec::new(),
            languages: Vec::new(),
            ui_language: "en".to_owned(),
            prefer: Vec::new(),
            families: Vec::new(),
            emoji: Emoji::default(),
            system: SystemFonts::default(),
            system_provider: None,
        }
    }

    /// One of the app's own faces, heading `families` in the order the app adds them.
    pub fn app_face(mut self, name: &str, data: FontData, families: &[FontFamily]) -> Self {
        self.app_faces.push(AppFace { name: name.to_owned(), data, families: families.to_vec() });
        self
    }

    /// The scripts the app's own faces cover (default `Latn`, `Cyrl`, `Grek`). A UI language whose
    /// main script isn't among them gets its craft face ahead of the app's faces.
    pub fn app_scripts(mut self, scripts: &[&str]) -> Self {
        self.app_scripts = scripts.iter().map(|s| (*s).to_owned()).collect();
        self
    }

    /// The craft-fonts faces this build embedded; may be empty.
    pub fn craft_faces(mut self, faces: &[CraftFace]) -> Self {
        self.craft_faces.extend_from_slice(faces);
        self
    }

    /// Every language the app offers, in its menu order.
    pub fn languages<S: AsRef<str>>(mut self, languages: &[S]) -> Self {
        self.languages = languages.iter().map(|l| l.as_ref().to_owned()).collect();
        self
    }

    /// The current UI language tag.
    pub fn ui_language(mut self, language: &str) -> Self {
        language.clone_into(&mut self.ui_language);
        self
    }

    /// Prefer the craft face of `family` for `script` (a face tag such as `Jpan`), ahead of other
    /// faces for the same script and region. Earlier calls win over later ones.
    pub fn prefer(mut self, script: &str, family: &str) -> Self {
        self.prefer.push((script.to_owned(), family.to_owned()));
        self
    }

    /// A family to build beyond those the app faces name, or a different weight or lead script for
    /// one of them.
    pub fn family(mut self, family: Family) -> Self {
        self.families.push(family);
        self
    }

    pub fn emoji(mut self, emoji: Emoji) -> Self {
        self.emoji = emoji;
        self
    }

    pub fn system_fonts(mut self, system: SystemFonts) -> Self {
        self.system = system;
        self
    }

    /// The provider to use for installed fonts instead of egui's system font provider (with the
    /// `system_fonts` feature, on desktop). It is wrapped in [`RegularWeight`].
    pub fn system_provider(mut self, provider: Arc<dyn FontProvider>) -> Self {
        self.system_provider = Some(provider);
        self
    }

    /// Build the stack and install it in `ctx`.
    pub fn install(&self, ctx: &egui::Context) -> Stack {
        let stack = self.stack();
        stack.install(ctx);
        stack
    }

    /// Build the stack without installing it.
    pub fn stack(&self) -> Stack {
        let mut stack = Stack {
            definitions: FontDefinitions::empty(),
            after_system: FaceList::default(),
            system: self.system,
            system_provider: self.system_provider.clone(),
            ui_language: self.ui_language.clone(),
            weights: Vec::new(),
        };
        let defaults = FontDefinitions::default();
        for family in self.family_specs() {
            let (configured, after) = self.chain(&family);
            let mut chain = Vec::new();
            for face in configured {
                chain.push(stack.add_configured(face.weighted(family.weight)));
            }
            let mut tail: Vec<String> = after.into_iter().map(|face| stack.after_system.add(face.weighted(family.weight))).collect();
            let default_chain = defaults.families.get(&family.family).or_else(|| defaults.families.get(&FontFamily::Proportional));
            // A family with no face of its own takes egui's defaults as its primary faces, so its
            // rows get real metrics.
            let has_faces = !chain.is_empty();
            for name in default_chain.into_iter().flatten() {
                let Some(data) = defaults.font_data.get(name) else { continue };
                let face = Face { name: name.clone(), data: (**data).clone() }.weighted(family.weight);
                if !has_faces {
                    chain.push(stack.add_configured(face));
                } else {
                    tail.push(stack.after_system.add(face));
                }
            }
            dedup(&mut chain);
            tail.retain(|name| !chain.contains(name));
            dedup(&mut tail);
            stack.definitions.families.insert(family.family.clone(), chain);
            stack.after_system.set_chain(family.family.clone(), tail);
            stack.weights.push((family.family, family.weight));
        }
        stack
    }

    /// Proportional and Monospace, every family an app face names, then the declared families.
    fn family_specs(&self) -> Vec<Family> {
        let mut specs = vec![Family::proportional(), Family { family: FontFamily::Monospace, weight: 400, lead: None }];
        for family in self.app_faces.iter().flat_map(|f| &f.families) {
            if !specs.iter().any(|s| s.family == *family) {
                specs.push(Family { family: family.clone(), weight: 400, lead: None });
            }
        }
        for declared in &self.families {
            match specs.iter_mut().find(|s| s.family == declared.family) {
                Some(spec) => *spec = declared.clone(),
                None => specs.push(declared.clone()),
            }
        }
        specs
    }

    /// The configured chain and the faces asked after the system fonts, for one family.
    fn chain(&self, family: &Family) -> (Vec<Face>, Vec<Face>) {
        let weight = family.weight;
        let ui_scripts = expand(&self.scripts.scripts(&self.ui_language));
        let ui_han = self.han(&self.ui_language);

        let lead: Vec<usize> = expand(family.lead.as_slice()).iter().flat_map(|s| self.candidates(s, ui_han, weight)).collect();
        let language: Vec<usize> = ui_scripts.iter().filter(|s| !self.app_covers(s)).flat_map(|s| self.candidates(s, ui_han, weight)).collect();
        let mut others: Vec<usize> = Vec::new();
        for tag in self.languages.iter().filter(|l| **l != self.ui_language) {
            let han = self.han(tag);
            for script in expand(&self.scripts.scripts(tag)).iter().filter(|s| !self.app_covers(s)) {
                others.extend(self.candidates(script, han, weight));
            }
        }
        let mut rest: Vec<(u8, usize)> =
            self.craft_faces.iter().enumerate().filter_map(|(i, face)| face.style_rank(weight).map(|rank| (rank, i))).collect();
        rest.sort();
        others.extend(rest.into_iter().map(|(_, i)| i));

        let app = self.app_faces_for(&family.family);
        let language_first = family.family != FontFamily::Monospace && ui_scripts.first().is_some_and(|main| !self.app_covers(main));
        let craft = |indices: Vec<usize>| indices.into_iter().map(|i| self.craft_face(i)).collect::<Vec<_>>();

        let mut configured = craft(lead);
        if language_first {
            configured.extend(craft(language));
            configured.extend(app);
        } else {
            configured.extend(app);
            configured.extend(craft(language));
        }
        let mut after = Vec::new();
        match self.system {
            SystemFonts::BeforeCraft => after.extend(craft(others)),
            SystemFonts::AfterCraft | SystemFonts::Off => configured.extend(craft(others)),
        }
        let emoji = [("emoji-color", &self.emoji.color), ("emoji-web", &self.emoji.web), ("emoji-monochrome", &self.emoji.monochrome)];
        after.extend(emoji.into_iter().filter_map(|(name, data)| Some(Face { name: name.to_owned(), data: data.clone()? })));
        (configured, after)
    }

    /// The app faces heading `family`; a named family without its own takes the proportional ones.
    fn app_faces_for(&self, family: &FontFamily) -> Vec<Face> {
        let faces = |family: &FontFamily| -> Vec<Face> {
            self.app_faces
                .iter()
                .filter(|face| face.families.contains(family))
                .map(|face| Face { name: face.name.clone(), data: face.data.clone() })
                .collect()
        };
        match faces(family) {
            own if !own.is_empty() || !matches!(family, FontFamily::Name(_)) => own,
            _ => faces(&FontFamily::Proportional),
        }
    }

    /// Craft faces for `script`, best first: the right Han region, then the app's preference,
    /// then the right style, then manifest order.
    fn candidates(&self, script: &str, han: Option<Han>, weight: u16) -> Vec<usize> {
        let preference = |face: &CraftFace| {
            let prefer = self.prefer.iter().map(|(s, f)| (s.as_str(), f.as_str())).chain(DEFAULT_PREFER.iter().copied());
            prefer.enumerate().find(|(_, (tag, family))| face.scripts.contains(tag) && face.family == *family).map_or(usize::MAX, |(i, _)| i)
        };
        let mut ranked: Vec<(u8, usize, u8, usize)> = self
            .craft_faces
            .iter()
            .enumerate()
            .filter(|(_, face)| face.covers(script))
            .filter_map(|(i, face)| {
                let style = face.style_rank(weight)?;
                let region = if script == "Hani" { region_rank(face.region(), han) } else { 0 };
                Some((region, preference(face), style, i))
            })
            .collect();
        ranked.sort();
        ranked.into_iter().map(|(.., i)| i).collect()
    }

    fn craft_face(&self, index: usize) -> Face {
        let face = self.craft_faces.get(index);
        Face { name: face.map(CraftFace::name).unwrap_or_default(), data: face.map_or_else(|| FontData::from_static(&[]), CraftFace::data) }
    }

    fn app_covers(&self, script: &str) -> bool {
        self.app_scripts.iter().any(|s| s == script)
    }

    fn han(&self, language: &str) -> Option<Han> {
        self.scripts.han(language).or_else(|| {
            self.scripts.scripts(language).iter().find_map(|s| match *s {
                "Hans" => Some(Han::Simplified),
                "Hant" => Some(Han::Traditional),
                "Jpan" => Some(Han::Japanese),
                "Kore" => Some(Han::Korean),
                _ => None,
            })
        })
    }
}

/// How well a face's Han region suits the one wanted: lower is better.
fn region_rank(face: Option<Han>, wanted: Option<Han>) -> u8 {
    match (face, wanted) {
        (_, None) => 0,
        (Some(f), Some(w)) if f == w => 0,
        (Some(Han::Traditional), Some(Han::HongKong)) | (Some(Han::HongKong), Some(Han::Traditional)) => 1,
        (None, Some(_)) => 2,
        _ => 3,
    }
}

/// Script codes with the CLDR unions spelled out.
fn expand<S: AsRef<str>>(scripts: &[S]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for script in scripts {
        let parts: Vec<&str> = match script.as_ref() {
            "Jpan" => vec!["Hani", "Hira", "Kana"],
            "Kore" => vec!["Hang", "Hani"],
            "Hans" | "Hant" => vec!["Hani"],
            other => vec![other],
        };
        for part in parts {
            if !out.iter().any(|s| s == part) {
                out.push(part.to_owned());
            }
        }
    }
    out
}

fn dedup(names: &mut Vec<String>) {
    let mut seen = std::collections::BTreeSet::new();
    names.retain(|n| !n.is_empty() && seen.insert(n.clone()));
}

/// A face with its egui name.
#[derive(Clone)]
struct Face {
    name: String,
    data: FontData,
}

impl Face {
    /// Set a variable face to `weight`. A face that isn't at regular weight gets its own name, so
    /// the regular and bold uses of one file don't collide in egui's face store.
    fn weighted(mut self, weight: u16) -> Self {
        if set_weight(&mut self.data, weight) && weight != 400 {
            self.name = format!("{} @{weight}", self.name);
        }
        self
    }
}

/// A built font stack: the configured faces and chains, and the providers asked after them.
pub struct Stack {
    /// The configured faces and each family's chain. Edit it to add, replace or reorder faces
    /// before [`Self::install`]; a chain entry without font data is skipped.
    pub definitions: FontDefinitions,
    /// Faces asked after the system fonts: emoji, egui's defaults and, with
    /// [`SystemFonts::BeforeCraft`], the craft faces for other scripts.
    pub after_system: FaceList,
    system: SystemFonts,
    system_provider: Option<Arc<dyn FontProvider>>,
    ui_language: String,
    weights: Vec<(FontFamily, u16)>,
}

impl Stack {
    /// Install the faces and providers in `ctx`, replacing any font providers it had (such as the
    /// one eframe adds), and raise the minimum widget height to fit a row of text.
    pub fn install(&self, ctx: &egui::Context) {
        ctx.set_fonts(self.checked_definitions());
        ctx.set_font_providers(self.providers(true));
        ctx.all_styles_mut(|style| self.fit_interact_size(style));
    }

    /// Each family's weight, for wrapping another provider in [`RegularWeight`].
    pub fn weights(&self) -> &[(FontFamily, u16)] {
        &self.weights
    }

    /// The chain of `family`, if the stack has it.
    pub fn chain(&self, family: &FontFamily) -> &[String] {
        self.definitions.families.get(family).map_or(&[], Vec::as_slice)
    }

    /// Insert a face into `family`'s chain at `index` (clamped), adding its data.
    pub fn insert(&mut self, family: &FontFamily, index: usize, name: &str, data: FontData) {
        self.definitions.font_data.insert(name.to_owned(), Arc::new(data));
        let chain = self.definitions.families.entry(family.clone()).or_default();
        chain.retain(|n| n != name);
        chain.insert(index.min(chain.len()), name.to_owned());
    }

    /// Replace the data of the face called `name`, wherever it is used.
    pub fn replace(&mut self, name: &str, data: FontData) {
        self.definitions.font_data.insert(name.to_owned(), Arc::new(data));
    }

    /// Remove the face called `name` from every chain.
    pub fn remove(&mut self, name: &str) {
        self.definitions.font_data.remove(name);
        for chain in self.definitions.families.values_mut() {
            chain.retain(|n| n != name);
        }
    }

    /// Raise `egui`'s minimum interactive height in `style` to fit a row of this stack's body and
    /// button text. [`Self::install`] does this for the context's styles; call it again for a
    /// style set later.
    pub fn fit_interact_size(&self, style: &mut Style) {
        let rows = self.row_heights(style);
        fit_interact_size(style, &rows);
    }

    /// An epaint [`Fonts`] with this stack, for tests and measurement. `with_system` adds the
    /// system font provider.
    pub fn fonts(&self, with_system: bool) -> Fonts {
        Fonts::new(TextOptions::default(), self.checked_definitions()).with_font_providers(self.providers(with_system))
    }

    fn row_heights(&self, style: &Style) -> Vec<f32> {
        let mut fonts = self.fonts(false);
        let mut view = fonts.with_pixels_per_point(1.0);
        [TextStyle::Body, TextStyle::Button].iter().filter_map(|s| style.text_styles.get(s)).map(|font_id| view.row_height(font_id)).collect()
    }

    fn checked_definitions(&self) -> FontDefinitions {
        let mut definitions = self.definitions.clone();
        for chain in definitions.families.values_mut() {
            chain.retain(|name| self.definitions.font_data.contains_key(name));
        }
        for family in [FontFamily::Proportional, FontFamily::Monospace] {
            definitions.families.entry(family).or_default();
        }
        definitions
    }

    fn providers(&self, with_system: bool) -> Vec<Arc<dyn FontProvider>> {
        let mut providers: Vec<Arc<dyn FontProvider>> = Vec::new();
        let system = (with_system && self.system != SystemFonts::Off)
            .then(|| self.system_provider.clone().or_else(|| default_system_provider(&self.ui_language)))
            .flatten();
        if let Some(system) = system {
            providers.push(Arc::new(self.weights.iter().fold(RegularWeight::new(system), |p, (f, w)| p.with_weight(f.clone(), *w))));
        }
        providers.push(Arc::new(self.after_system.clone()));
        providers
    }

    fn add_configured(&mut self, face: Face) -> String {
        self.definitions.font_data.entry(face.name.clone()).or_insert_with(|| Arc::new(face.data));
        face.name
    }
}

fn fit_interact_size(style: &mut Style, rows: &[f32]) {
    let row = rows.iter().copied().fold(0.0_f32, f32::max);
    let minimum = row + 2.0 * style.spacing.button_padding.y;
    if minimum.is_finite() && style.spacing.interact_size.y < minimum {
        style.spacing.interact_size.y = minimum;
    }
}

#[cfg(all(feature = "system_fonts", not(target_arch = "wasm32")))]
fn default_system_provider(ui_language: &str) -> Option<Arc<dyn FontProvider>> {
    Some(Arc::new(egui_system_font_provider::SystemFontProvider::new().with_locale(Some(ui_language))))
}

#[cfg(not(all(feature = "system_fonts", not(target_arch = "wasm32"))))]
fn default_system_provider(_ui_language: &str) -> Option<Arc<dyn FontProvider>> {
    None
}
