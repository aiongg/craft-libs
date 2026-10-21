//! Font providers: embedded faces asked after the system fonts, and the regular-weight wrapper for
//! the system font provider.

use std::collections::BTreeMap;
use std::sync::{Arc, PoisonError, RwLock};

use egui::epaint::text::Tag;
use egui::{FallbackRequest, FontData, FontFamily, FontInsert, FontPriority, FontProvider, InsertFontFamily};
use skrifa::MetadataProvider as _;

/// Set a variable face's weight axis to `weight`, within the axis's range.
///
/// Leaves a face with no weight axis, or one whose weight is already set, as it is. Returns
/// whether it changed the face. Variable system fonts often default to their lightest instance,
/// e.g. Thin (100), so a face installed without this draws as hairlines.
pub fn set_weight(data: &mut FontData, weight: u16) -> bool {
    let wght = Tag::new(b"wght");
    if data.tweak.coords.as_ref().iter().any(|(tag, _)| *tag == wght) {
        return false;
    }
    let Some(axis) = data.variation_axes().into_iter().find(|axis| axis.tag == wght) else { return false };
    let value = f32::from(weight).clamp(axis.range.min, axis.range.max);
    if value == axis.default {
        return false;
    }
    data.tweak.coords.push(*b"wght", value);
    true
}

/// Wraps a font provider, usually egui's system font provider, so variable faces it finds draw
/// at each family's weight (regular, 400, unless set with [`Self::with_weight`]).
pub struct RegularWeight {
    inner: Arc<dyn FontProvider>,
    weights: Vec<(FontFamily, u16)>,
}

impl RegularWeight {
    pub fn new(inner: Arc<dyn FontProvider>) -> Self {
        Self { inner, weights: Vec::new() }
    }

    /// Draw faces found for `family` at `weight`, e.g. 600 for a semibold family.
    pub fn with_weight(mut self, family: FontFamily, weight: u16) -> Self {
        self.weights.retain(|(f, _)| *f != family);
        self.weights.push((family, weight));
        self
    }

    fn weight(&self, family: &FontFamily) -> u16 {
        self.weights.iter().find(|(f, _)| f == family).map_or(400, |(_, w)| *w)
    }
}

impl FontProvider for RegularWeight {
    fn fonts_for_family(&self, family: &FontFamily) -> Vec<FontInsert> {
        let weight = self.weight(family);
        self.inner.fonts_for_family(family).into_iter().map(|insert| weighted(insert, weight)).collect()
    }

    fn font_for(&self, request: &FallbackRequest<'_>) -> Option<FontInsert> {
        let insert = self.inner.font_for(request)?;
        Some(weighted(insert, self.weight(request.family)))
    }
}

fn weighted(mut insert: FontInsert, weight: u16) -> FontInsert {
    // egui's face store keys faces by name, so a face set to another weight needs its own.
    if set_weight(&mut insert.data, weight) && weight != 400 {
        insert.name = format!("{} @{weight}", insert.name);
    }
    insert
}

/// Embedded faces asked in order, per family, for a character no earlier face or provider has.
#[derive(Clone, Default)]
pub struct FaceList {
    faces: BTreeMap<String, FontData>,
    chains: BTreeMap<FontFamily, Vec<String>>,
}

impl FaceList {
    /// The faces asked for `family`, in order. A family without its own list uses the
    /// proportional one.
    pub fn chain(&self, family: &FontFamily) -> &[String] {
        self.chains.get(family).or_else(|| self.chains.get(&FontFamily::Proportional)).map_or(&[], Vec::as_slice)
    }

    /// Replace `family`'s list. Names without data from [`Self::add`] are skipped when asked.
    pub fn set_chain(&mut self, family: FontFamily, names: Vec<String>) {
        self.chains.insert(family, names);
    }

    pub(super) fn add(&mut self, face: super::Face) -> String {
        self.faces.entry(face.name.clone()).or_insert(face.data);
        face.name
    }

    /// Add or replace a face's data.
    pub fn insert(&mut self, name: &str, data: FontData) {
        self.faces.insert(name.to_owned(), data);
    }
}

impl FontProvider for FaceList {
    fn font_for(&self, request: &FallbackRequest<'_>) -> Option<FontInsert> {
        let c = request.cluster.chars().next()?;
        let (name, data) = self.chain(request.family).iter().find_map(|name| {
            let data = self.faces.get(name)?;
            has_char(data, c).then_some((name, data))
        })?;
        Some(FontInsert::new(name, data.clone(), vec![InsertFontFamily { family: request.family.clone(), priority: FontPriority::Lowest }]))
    }
}

/// A context's installed [`FaceList`], swapped in place when the stack is installed again, so the
/// providers added before it keep their place.
#[derive(Default)]
pub(super) struct Installed(RwLock<FaceList>);

impl Installed {
    pub(super) fn set(&self, faces: FaceList) {
        *self.0.write().unwrap_or_else(PoisonError::into_inner) = faces;
    }
}

impl FontProvider for Installed {
    fn font_for(&self, request: &FallbackRequest<'_>) -> Option<FontInsert> {
        self.0.read().unwrap_or_else(PoisonError::into_inner).font_for(request)
    }
}

/// Whether the face's character map has `c`.
pub(super) fn has_char(data: &FontData, c: char) -> bool {
    skrifa::FontRef::from_index(data.bytes(), data.index).is_ok_and(|font| font.charmap().map(c).is_some())
}
