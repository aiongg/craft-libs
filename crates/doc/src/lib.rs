#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]
//! Read-only access to the Craft apps' native documents, without the apps: PhotoCraft
//! `.pcraft`, VectorCraft `.vectorcraft` (and `.drawcraft`) and EffectCraft `.ecproj`.
//!
//! - [`sniff`] tells which app wrote a file, from its first bytes.
//! - [`info`] reads the format version and the writer the file names.
//! - [`renditions`] returns the previews and the PDF a file carries.
//! - [`outline`] returns the canvas, the pages (artboards or compositions) and the layers.
//! - `pcraft_layer_rgba`, with the feature `pcraft-pixels`, decodes a PhotoCraft layer's pixels
//!   and its layer mask.
//!
//! Bytes go in and plain data comes out: no file system access, no rendering and no app crate.
//! Each format's envelope is read through its craft-libs crate: `craft-pcraft-format`,
//! `craft-vectorcraft-format` and `craft-ecproj-format`.
//! A file whose format version is newer than this crate reads gives [`Info::supported`] `false`
//! from [`info`] and [`Error::TooNew`] from the other calls.
//!
//! Nothing panics on input. Every length a file states is checked against [`Limits`] before
//! anything is allocated for it, and every decompressed byte is counted against them.
//!
//! ```
//! let file = br#"{"format":"vectorcraft","version":4,"generator":"VectorCraft 0.1.0","document":{"artboards":[],"layers":[]}}"#;
//! assert_eq!(craft_doc::sniff(file), Some(craft_doc::Kind::VectorCraft));
//! let info = craft_doc::info(file, &craft_doc::Limits::default()).unwrap();
//! assert_eq!(info.version, 4);
//! assert!(info.supported);
//! ```

mod budget;
mod ecproj;
mod lenient;
mod pcraft;
#[cfg(feature = "pcraft-pixels")]
mod pcraft_pixels;
mod vectorcraft;
mod zip;

use budget::{Budget, check_depth, counted, counted_map, counted_option, counting, parse, parse_checked};
use lenient::{Lenient, Scalar, Shaped, lenient, parse_versioned, too_new};

#[cfg(feature = "pcraft-pixels")]
pub use pcraft_pixels::{LayerMask, LayerPixels, Samples, pcraft_layer_rgba};

/// The app that wrote a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Kind {
    /// A PhotoCraft `.pcraft` bundle (a ZIP archive).
    PhotoCraft,
    /// A VectorCraft `.vectorcraft` document (JSON, plain or gzip), or one saved under the
    /// earlier name `.drawcraft`.
    VectorCraft,
    /// An EffectCraft `.ecproj` project (JSON).
    EffectCraft,
}

impl Kind {
    /// The file extension the app saves with, without the dot.
    pub fn extension(self) -> &'static str {
        match self {
            Kind::PhotoCraft => "pcraft",
            Kind::VectorCraft => "vectorcraft",
            Kind::EffectCraft => "ecproj",
        }
    }

    /// The newest format version this crate reads: PhotoCraft's `format_version`, VectorCraft's
    /// `version` or EffectCraft's `schema`.
    pub fn supported_version(self) -> u32 {
        match self {
            Kind::PhotoCraft => craft_pcraft_format::FORMAT_VERSION,
            Kind::VectorCraft => craft_vectorcraft_format::VERSION,
            Kind::EffectCraft => craft_ecproj_format::SCHEMA_VERSION,
        }
    }
}

/// What a document says about itself.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Info {
    #[allow(missing_docs)]
    pub kind: Kind,
    /// The format version the file declares.
    pub version: u32,
    /// The writer, as the file names it: VectorCraft's `generator` (`"VectorCraft 0.13.1"`),
    /// PhotoCraft's `generator` (`"photocraft-format 0.1.0"`) or EffectCraft's `savedBy` (the
    /// app version, `"0.12.0"`). `None` when the file has none.
    pub generator: Option<String>,
    /// `false` when [`Info::version`] is newer than [`Kind::supported_version`]; the other calls
    /// then return [`Error::TooNew`].
    pub supported: bool,
}

/// Why a document could not be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// [`sniff`] does not recognize the bytes as a Craft document, or a ZIP archive's
    /// `manifest.json` is a JSON object without `format_version`, so the archive is not a
    /// PhotoCraft bundle.
    #[error("not a PhotoCraft, VectorCraft or EffectCraft document")]
    Unrecognized,
    /// The file is damaged or does not follow its format.
    #[error("malformed document: {0}")]
    Malformed(String),
    /// The file's format version is newer than this crate reads.
    #[error("{} version {version} is newer than this reader supports ({supported})", kind.extension())]
    TooNew {
        #[allow(missing_docs)]
        kind: Kind,
        #[allow(missing_docs)]
        version: u32,
        #[allow(missing_docs)]
        supported: u32,
    },
    /// A valid container feature this reader does not implement (ZIP64, encryption, a ZIP
    /// compression method other than STORE or DEFLATE).
    #[error("{0}")]
    Unsupported(String),
    /// Reading the file would exceed the caller's [`Limits`].
    #[error("limit exceeded: {0}")]
    LimitExceeded(String),
}

/// Bounds on the work and memory one call may use. Each is checked before the bytes it covers
/// are allocated.
///
/// ```
/// let mut limits = craft_doc::Limits::default();
/// limits.max_entry = 16 << 20;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Limits {
    /// Most bytes of JSON read: the VectorCraft file's JSON, the PhotoCraft manifest or the
    /// EffectCraft project. The names and other strings returned are parts of it, so this also
    /// bounds them. Default 256 MiB.
    pub max_json: u64,
    /// Most bytes of one rendition or one ZIP entry. Default 256 MiB.
    pub max_entry: u64,
    /// Most bytes one call decompresses from gzip, reads out of ZIP entries and returns as
    /// renditions, together; a rendition decompressed from gzip is counted once, and a second
    /// rendition of the same bytes as another copy. Default 1 GiB.
    pub max_total: u64,
    /// Deepest nesting of JSON arrays and objects, at most [`MAX_DEPTH`] (a larger value counts
    /// as [`MAX_DEPTH`]). Default [`MAX_DEPTH`]. [`renditions`] and [`outline`] also refuse a
    /// PhotoCraft manifest nested deeper than PhotoCraft reads, 367 levels
    /// (`craft_pcraft_format::manifest_depth(MAX_GROUP_DEPTH)`).
    pub max_depth: u32,
    /// Most list elements one call reads, together: EffectCraft project items and composition
    /// layers, VectorCraft artboards and layers, and PhotoCraft layers at every level. Each is
    /// counted before it is read. Default 1,048,576.
    pub max_items: u64,
}

/// The deepest JSON nesting any call reads: 400 levels, which holds PhotoCraft's 100 nested
/// groups of layers at three levels each.
pub const MAX_DEPTH: u32 = 400;

impl Default for Limits {
    fn default() -> Self {
        Limits { max_json: 256 << 20, max_entry: 256 << 20, max_total: 1 << 30, max_depth: MAX_DEPTH, max_items: 1 << 20 }
    }
}

/// What a rendition shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RenditionKind {
    /// A PDF of the document (VectorCraft's `pdf`, saved with Create PDF-Compatible File).
    Pdf,
    /// A small preview image: VectorCraft's `preview` (the first artboard) or PhotoCraft's
    /// `thumbnail`.
    Thumbnail,
    /// PhotoCraft's flattened `composite` preview.
    Composite,
}

/// An image or PDF the document carries.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Rendition {
    #[allow(missing_docs)]
    pub kind: RenditionKind,
    /// The MIME type: the one a VectorCraft file gives (a member without one gives no
    /// rendition), or `image/png` for PhotoCraft's previews.
    pub mime: String,
    /// The encoded image data.
    pub bytes: Vec<u8>,
}

/// The structure of a document, without its content.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct Outline {
    /// PhotoCraft's canvas; `None` for the other formats.
    pub canvas: Option<Canvas>,
    /// VectorCraft's artboards (in points) or EffectCraft's compositions (in pixels), in file
    /// order; empty for PhotoCraft.
    pub pages: Vec<Page>,
    /// VectorCraft's top-level layers or PhotoCraft's layer tree, bottom first; empty for
    /// EffectCraft, whose layers are in each [`Page`].
    pub layers: Vec<Layer>,
}

/// A PhotoCraft canvas.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Canvas {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Resolution in dots per inch.
    pub resolution_dpi: f32,
    /// The color mode as the manifest writes it: `Rgb`, `Cmyk`, `Grayscale`, `Lab`, `Indexed`,
    /// `Bitmap`, `Duotone` or `Multichannel`.
    pub mode: String,
    /// The sample type as the manifest writes it: `U8`, `U16` or `F32`.
    pub depth: String,
}

/// A VectorCraft artboard or an EffectCraft composition.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Page {
    #[allow(missing_docs)]
    pub name: String,
    /// The left edge (0 for a composition).
    pub x: f64,
    /// The top edge (0 for a composition).
    pub y: f64,
    /// Width, in the same units as `x`.
    pub width: f64,
    /// Height, in the same units as `y`.
    pub height: f64,
    /// A composition's frame rate and duration; `None` for an artboard, and for a composition
    /// that gives either one without the other.
    pub timing: Option<Timing>,
    /// A composition's layers, bottom first; empty for an artboard.
    pub layers: Vec<Layer>,
}

/// A composition's timing.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct Timing {
    /// Frames per second as `frame_rate_num / frame_rate_den`.
    pub frame_rate_num: i64,
    #[allow(missing_docs)]
    pub frame_rate_den: i64,
    #[allow(missing_docs)]
    pub duration_seconds: f64,
}

/// One layer.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Layer {
    #[allow(missing_docs)]
    pub name: String,
    /// What the layer holds, as the file names it: `layer` for VectorCraft; `raster`, `group`,
    /// `adjustment`, `fill`, `text`, `shape` or `smart` for PhotoCraft; the source type for
    /// EffectCraft (`Footage`, `Comp`, `Solid`, `Text`, `Shape`, `Null`, `Camera`, `Light`,
    /// `Model`, `Primitive`).
    pub kind: String,
    #[allow(missing_docs)]
    pub visible: bool,
    /// Opacity from 0 to 1; `None` for EffectCraft, where opacity is an animated property.
    pub opacity: Option<f32>,
    /// The blend mode as the file names it (`Normal`, `Multiply`…).
    pub blend: Option<String>,
    /// A PhotoCraft group's layers, bottom first.
    pub children: Vec<Layer>,
}

/// Which app wrote the document whose first bytes are `head`, or `None`. The first 64 KiB are
/// enough for every file the apps write. A re-zipped `.pcraft` whose `manifest.json` entry
/// starts past `head` is recognized only when `head` holds the whole file.
pub fn sniff(head: &[u8]) -> Option<Kind> {
    if pcraft::sniff(head) {
        Some(Kind::PhotoCraft)
    } else if vectorcraft::sniff(head) {
        Some(Kind::VectorCraft)
    } else if ecproj::sniff(head) {
        Some(Kind::EffectCraft)
    } else {
        None
    }
}

/// The document's kind, version and writer. A version newer than this crate reads is not an
/// error here: [`Info::supported`] is `false`.
pub fn info(bytes: &[u8], limits: &Limits) -> Result<Info, Error> {
    let mut budget = Budget::new(limits);
    let kind = sniff(bytes).ok_or(Error::Unrecognized)?;
    let (version, generator) = match kind {
        Kind::PhotoCraft => pcraft::info(bytes, &mut budget)?,
        Kind::VectorCraft => vectorcraft::info(bytes, &mut budget)?,
        Kind::EffectCraft => ecproj::info(bytes, &mut budget)?,
    };
    Ok(Info { kind, version, generator, supported: version <= kind.supported_version() })
}

/// The previews and the PDF the document carries, the thumbnail first. A
/// `.vectorcraft` file has a `preview` only when saved with one and a `pdf` only when saved with
/// Create PDF-Compatible File; a `.pcraft` bundle has the `thumbnail` and `composite` its
/// manifest names; an `.ecproj` project has none.
pub fn renditions(bytes: &[u8], limits: &Limits) -> Result<Vec<Rendition>, Error> {
    let mut budget = Budget::new(limits);
    match sniff(bytes).ok_or(Error::Unrecognized)? {
        Kind::PhotoCraft => pcraft::renditions(bytes, &mut budget),
        Kind::VectorCraft => vectorcraft::renditions(bytes, &mut budget),
        Kind::EffectCraft => ecproj::renditions(bytes, &mut budget),
    }
}

/// The document's canvas, pages and layers (see [`Outline`]).
pub fn outline(bytes: &[u8], limits: &Limits) -> Result<Outline, Error> {
    let mut budget = Budget::new(limits);
    match sniff(bytes).ok_or(Error::Unrecognized)? {
        Kind::PhotoCraft => pcraft::outline(bytes, &mut budget),
        Kind::VectorCraft => vectorcraft::outline(bytes, &mut budget),
        Kind::EffectCraft => ecproj::outline(bytes, &mut budget),
    }
}

fn malformed(m: impl std::fmt::Display) -> Error {
    Error::Malformed(m.to_string())
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use serde::de::{self, Deserializer, Visitor};

    use super::*;

    #[test]
    fn depth_counts_brackets_outside_strings() {
        assert!(check_depth(br#"[[["[[[[[[[["]]]"#, 3).is_ok());
        assert!(check_depth(br#"[[[["\"[["]]]]"#, 3).is_err());
        assert!(check_depth(br#"{"a":{"b":[1]}}"#, 3).is_ok());
    }

    #[test]
    fn the_budget_refuses_past_its_limits() {
        let limits = Limits { max_json: 10, max_entry: 5, max_total: 8, max_depth: 4, max_items: 2 };
        let mut b = Budget::new(&limits);
        assert!(b.take_entry(6, "x").is_err());
        assert!(b.take_entry(5, "x").is_ok());
        assert!(matches!(b.take_entry(4, "x"), Err(Error::LimitExceeded(_))));
        assert_eq!(b.left(), 3);
        assert!(b.check_json(11, "x").is_err() && b.check_json(10, "x").is_ok());
    }

    /// A value that can be skipped but not read: reading it whole, as a buffer of the value
    /// would, fails.
    #[derive(Clone)]
    pub(crate) struct Unread;

    impl<'de> Deserializer<'de> for Unread {
        type Error = de::value::Error;

        fn deserialize_any<V: Visitor<'de>>(self, _: V) -> Result<V::Value, Self::Error> {
            Err(de::Error::custom("the value was read, not skipped"))
        }

        fn deserialize_ignored_any<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Self::Error> {
            v.visit_unit()
        }

        serde::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf option unit
            unit_struct newtype_struct seq tuple tuple_struct map struct enum identifier
        }
    }

    impl<'de> de::IntoDeserializer<'de> for Unread {
        type Deserializer = Unread;

        fn into_deserializer(self) -> Unread {
            self
        }
    }

    /// An array of `n` [`Unread`] elements.
    pub(crate) fn unread_array(n: usize) -> de::value::SeqDeserializer<std::iter::RepeatN<Unread>, de::value::Error> {
        de::value::SeqDeserializer::new(std::iter::repeat_n(Unread, n))
    }

    /// An object whose `n` members hold [`Unread`] values.
    pub(crate) fn unread_object(n: usize) -> de::value::MapDeserializer<'static, std::vec::IntoIter<(String, Unread)>, de::value::Error> {
        de::value::MapDeserializer::new((0..n).map(|i| (i.to_string(), Unread)).collect::<Vec<_>>().into_iter())
    }

    #[test]
    fn a_lenient_string_skips_an_array_or_object_without_reading_it_whole() {
        assert!(Lenient::<String>::deserialize(unread_array(1000)).unwrap().0.is_none());
        assert!(Lenient::<String>::deserialize(unread_object(1000)).unwrap().0.is_none());
        let s: Lenient<String> = Lenient::deserialize(de::value::StrDeserializer::<de::value::Error>::new("VectorCraft 0.1.0")).unwrap();
        assert_eq!(s.0.as_deref(), Some("VectorCraft 0.1.0"));
    }
}

#[cfg(test)]
extern crate self as craft_doc;

#[cfg(test)]
#[path = "../tests/main.rs"]
mod integration;
