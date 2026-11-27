//! Faces and the sources that hold them: the configured definitions, weight tweaks, the face list asked per family,
//! the regular-weight wrapper for the system fonts' source, and the baseline placement of fallback faces.

use std::collections::BTreeMap;
use std::sync::{Arc, PoisonError, RwLock};

use craft_ui_rt::{FaceSource, FallbackAsk, FontFace, StackFace};
use skrifa::MetadataProvider as _;

use super::FontRef;

/// The configured faces by name and each family's chain of face names: what a stack holds before it
/// becomes a [`craft_ui_rt::FontStack`]. A chain name without a face is skipped.
#[derive(Clone, Default)]
pub(crate) struct Definitions {
    pub(crate) font_data: BTreeMap<String, StackFace>,
    pub(crate) families: Families,
}

/// Each family's chain of face names, in the order the families were added.
#[derive(Clone, Default)]
pub(crate) struct Families(Vec<(FontFace, Vec<String>)>);

impl Families {
    pub(crate) fn get(&self, family: &FontFace) -> Option<&Vec<String>> {
        self.0.iter().find(|(f, _)| f == family).map(|(_, chain)| chain)
    }

    pub(crate) fn contains_key(&self, family: &FontFace) -> bool {
        self.get(family).is_some()
    }

    /// `family`'s chain, made empty when the family is new.
    pub(crate) fn entry(&mut self, family: FontFace) -> &mut Vec<String> {
        let at = match self.0.iter().position(|(f, _)| *f == family) {
            Some(at) => at,
            None => {
                self.0.push((family, Vec::new()));
                self.0.len() - 1
            }
        };
        &mut self.0[at].1
    }

    pub(crate) fn insert(&mut self, family: FontFace, chain: Vec<String>) {
        *self.entry(family) = chain;
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&FontFace, &Vec<String>)> {
        self.0.iter().map(|(f, chain)| (f, chain))
    }

    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = (&FontFace, &mut Vec<String>)> {
        self.0.iter_mut().map(|(f, chain)| (&*f, chain))
    }

    pub(crate) fn values(&self) -> impl Iterator<Item = &Vec<String>> {
        self.0.iter().map(|(_, chain)| chain)
    }

    pub(crate) fn values_mut(&mut self) -> impl Iterator<Item = &mut Vec<String>> {
        self.0.iter_mut().map(|(_, chain)| chain)
    }
}

impl Definitions {
    /// Each family's faces in chain order, named as their chain names them; names without a face are
    /// skipped.
    pub(crate) fn stack_families(&self) -> Vec<(FontFace, Vec<StackFace>)> {
        self.families
            .iter()
            .map(|(family, chain)| {
                (family.clone(), chain.iter().filter_map(|name| Some(self.font_data.get(name)?.clone().named(name.as_str()))).collect())
            })
            .collect()
    }
}

/// The variation axes of a face: tag, minimum, default and maximum.
pub(crate) fn variation_axes(face: &StackFace) -> Vec<([u8; 4], f32, f32, f32)> {
    let Ok(font) = skrifa::FontRef::from_index(face.data(), face.index) else { return Vec::new() };
    font.axes().iter().map(|axis| (axis.tag().to_be_bytes(), axis.min_value(), axis.default_value(), axis.max_value())).collect()
}

/// Sets a variable face's weight axis to `weight`, within the axis's range.
///
/// Leaves a face with no weight axis, or one whose weight is already set, as it is. Returns
/// whether it changed the face. Variable system fonts often default to their lightest instance,
/// e.g. Thin (100), so a face installed without this draws as hairlines.
pub fn set_weight(face: &mut StackFace, weight: u16) -> bool {
    if face.coord(*b"wght").is_some() {
        return false;
    }
    let Some((_, min, default, max)) = variation_axes(face).into_iter().find(|(tag, ..)| tag == b"wght") else { return false };
    let value = f32::from(weight).clamp(min, max);
    if value == default {
        return false;
    }
    face.coords.push((*b"wght", value));
    true
}

/// Wraps a face source, usually the system fonts, so variable faces it finds draw at each family's
/// weight (regular, 400, unless set with [`Self::with_weight`]).
pub struct RegularWeight {
    inner: Arc<dyn FaceSource>,
    weights: Vec<(FontRef, u16)>,
}

impl RegularWeight {
    /// Wraps `inner`, with every family at the regular weight.
    pub fn new(inner: Arc<dyn FaceSource>) -> Self {
        Self { inner, weights: Vec::new() }
    }

    /// Draws faces found for `family` at `weight`, e.g. 600 for a semibold family.
    pub fn with_weight(mut self, family: FontRef, weight: u16) -> Self {
        self.weights.retain(|(f, _)| *f != family);
        self.weights.push((family, weight));
        self
    }

    fn weight(&self, family: &FontFace) -> u16 {
        self.weights.iter().find(|(f, _)| f.face() == family).map_or(400, |(_, w)| *w)
    }
}

impl FaceSource for RegularWeight {
    fn faces_for(&self, family: &FontFace) -> Vec<StackFace> {
        let weight = self.weight(family);
        self.inner.faces_for(family).into_iter().map(|face| weighted(face, weight)).collect()
    }

    fn face_for(&self, ask: &FallbackAsk<'_>) -> Option<StackFace> {
        let face = self.inner.face_for(ask)?;
        Some(weighted(face, self.weight(ask.family)))
    }
}

fn weighted(mut face: StackFace, weight: u16) -> StackFace {
    // The engines key faces by name, so a face set to another weight needs its own.
    if set_weight(&mut face, weight) && weight != 400 {
        face.name = format!("{} @{weight}", face.name).into();
    }
    face
}

/// A craft face, set to sit on the primary face's baseline (the engines put a fallback face's glyphs on the primary
/// face's baseline and grow the row to fit them).
pub(crate) fn baseline_fallback(mut face: StackFace) -> StackFace {
    face.tweak.baseline_fallback = true;
    face
}

/// Wraps the system fonts' source so the text faces it finds sit on the row's baseline as
/// fallbacks, as craft faces do. Emoji and symbol faces stay centred in the row.
struct OnBaseline(Arc<dyn FaceSource>);

impl FaceSource for OnBaseline {
    fn faces_for(&self, family: &FontFace) -> Vec<StackFace> {
        self.0.faces_for(family).into_iter().map(on_baseline_face).collect()
    }

    fn face_for(&self, ask: &FallbackAsk<'_>) -> Option<StackFace> {
        self.0.face_for(ask).map(on_baseline_face)
    }
}

/// `source` with its text faces on the row's baseline.
pub(crate) fn on_baseline(source: Arc<dyn FaceSource>) -> Arc<dyn FaceSource> {
    Arc::new(OnBaseline(source))
}

fn on_baseline_face(face: StackFace) -> StackFace {
    if is_text_face(&face) { baseline_fallback(face) } else { face }
}

/// Whether a face draws text rather than emoji or symbols: it has no colour glyphs, and neither
/// its name nor its family name says emoji or symbols.
pub(crate) fn is_text_face(data: &StackFace) -> bool {
    let name = data.name.as_ref();
    let pictorial = |name: &str| {
        let name = name.to_lowercase();
        ["emoji", "symbol"].iter().any(|word| name.contains(word))
    };
    if pictorial(name) {
        return false;
    }
    let Ok(font) = skrifa::FontRef::from_index(data.data(), data.index) else { return true };
    let colour = [b"COLR", b"CBDT", b"sbix", b"SVG "].iter().any(|tag| font.table_data(skrifa::Tag::new(tag)).is_some());
    let family = font.localized_strings(skrifa::string::StringId::FAMILY_NAME).english_or_first().map(|s| s.to_string());
    !colour && !family.is_some_and(|family| pictorial(&family))
}

/// Embedded faces asked in order, per family, for a character no earlier face or provider has.
#[derive(Clone, Default)]
pub struct FaceList {
    faces: BTreeMap<String, StackFace>,
    chains: Families,
}

impl FaceList {
    /// The faces asked for `family`, in order. A family without its own list uses the
    /// proportional one.
    pub fn chain(&self, family: &FontRef) -> &[String] {
        self.chain_of(family.face())
    }

    pub(crate) fn chain_of(&self, family: &FontFace) -> &[String] {
        self.chains.get(family).or_else(|| self.chains.get(FontRef::proportional().face())).map_or(&[], Vec::as_slice)
    }

    /// Replaces `family`'s list. Names without data from `Self::insert` are skipped when asked.
    pub fn set_chain(&mut self, family: FontRef, names: Vec<String>) {
        self.chains.insert(family.into_face(), names);
    }

    /// Adds a face under `name` unless the list has one, and returns the name.
    pub(crate) fn add(&mut self, name: String, data: StackFace) -> String {
        self.faces.entry(name.clone()).or_insert_with(|| data.named(name.as_str()));
        name
    }

    /// Each family's list, for renaming entries in place.
    pub(crate) fn chains_mut(&mut self) -> impl Iterator<Item = (&FontFace, &mut Vec<String>)> {
        self.chains.iter_mut()
    }

    /// Adds or replaces a face's data, under `name`.
    pub fn insert(&mut self, name: &str, data: StackFace) {
        self.faces.insert(name.to_owned(), data.named(name));
    }

    /// Whether the list has data for the face called `name`.
    pub fn contains(&self, name: &str) -> bool {
        self.faces.contains_key(name)
    }

    /// The data of the face called `name`, with its tweak.
    pub fn get(&self, name: &str) -> Option<&StackFace> {
        self.faces.get(name)
    }

    /// Every face in the list with its name, in name order.
    pub fn faces(&self) -> impl Iterator<Item = (&str, &StackFace)> {
        self.faces.iter().map(|(name, data)| (name.as_str(), data))
    }

    /// Removes the face called `name` and drops it from every chain.
    pub fn remove(&mut self, name: &str) {
        self.faces.remove(name);
        for chain in self.chains.values_mut() {
            chain.retain(|n| n != name);
        }
    }
}

impl FaceSource for FaceList {
    /// The first face with the cluster's first character, preferring one that also has the
    /// cluster's other visible characters: the engine shapes the whole cluster with this face.
    fn face_for(&self, ask: &FallbackAsk<'_>) -> Option<StackFace> {
        let mut chars = ask.cluster.chars();
        let first = chars.next()?;
        let rest: Vec<char> = chars.filter(|&c| draws(c)).collect();
        let faces = self.chain_of(ask.family).iter().filter_map(|name| self.faces.get(name)).filter(|data| has_char(data, first));
        let face = faces.clone().find(|data| rest.iter().all(|&c| has_char(data, c))).or_else(|| faces.clone().next())?;
        Some(face.clone())
    }
}

/// A [`FaceList`] and the source asked when it misses too (on the web, the loader that fetches served faces), swapped
/// in place, so a stack offered again keeps the source in its place.
#[derive(Default)]
pub(crate) struct Installed(RwLock<(FaceList, Option<Arc<dyn FaceSource>>)>);

impl Installed {
    /// Replaces the list and the source asked when it misses.
    pub(crate) fn set(&self, faces: FaceList, miss: Option<Arc<dyn FaceSource>>) {
        *self.0.write().unwrap_or_else(PoisonError::into_inner) = (faces, miss);
    }
}

impl FaceSource for Installed {
    fn face_for(&self, ask: &FallbackAsk<'_>) -> Option<StackFace> {
        let (faces, miss) = &*self.0.read().unwrap_or_else(PoisonError::into_inner);
        faces.face_for(ask).or_else(|| miss.as_ref()?.face_for(ask))
    }
}

/// Whether the face's character map has `c`.
pub(crate) fn has_char(data: &StackFace, c: char) -> bool {
    skrifa::FontRef::from_index(data.data(), data.index).is_ok_and(|font| font.charmap().map(c).is_some())
}

/// Whether `c` shows a glyph: not whitespace, a control, a format character or another invisible
/// mark.
pub(crate) fn draws(c: char) -> bool {
    !(c.is_whitespace()
        || c.is_control()
        || matches!(
            c,
            '\u{00AD}' | '\u{034F}' | '\u{061C}' | '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2060}'..='\u{206F}' | '\u{FEFF}'
        )
        || matches!(c, '\u{FE00}'..='\u{FE0F}' | '\u{E0100}'..='\u{E01EF}'))
}
