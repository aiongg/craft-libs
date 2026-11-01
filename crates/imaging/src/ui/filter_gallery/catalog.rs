//! Filter Gallery's catalog: the app's filters, their categories and parameters (task WS-FG1).

use craft_ui::dialog::{DialogFields, Param};

/// One filter of the gallery: its key in the app's command (`"filter.artistic.cutout"`), its
/// label in English (translated through the app's `I18n` like `Param` labels), its category key,
/// its parameters in the step 2 notation (`Param::parse_list` of PhotoCraft's `params_doc`), and
/// whether it uses the foreground and background colours (Photoshop shows them in the settings).
#[derive(Clone, Debug, PartialEq)]
pub struct GalleryFilter {
    pub key: String,
    pub label: String,
    pub category: String,
    pub params: Vec<Param>,
    pub uses_colors: bool,
}

impl GalleryFilter {
    /// The filter's defaults as fields: each Range's default, each Choice's default (or its first
    /// option), each Bool, and colour parameters (Neon Glow's) at their default; other kinds are
    /// left out.
    #[expect(clippy::todo, reason = "skeleton: task WS-FG1")]
    pub fn defaults(&self) -> DialogFields {
        // pc: crates/ui-egui/src/dialogs/gallery_ui.rs:51 `default_params`.
        todo!()
    }
}

/// A category folder: its key, its English label and its filters' keys in menu order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilterCategory {
    pub key: String,
    pub label: String,
    pub filters: Vec<String>,
}

/// The app's filters (lead call 37). PhotoCraft implements it over `photocraft_algo::artistic`'s
/// `GalleryFilter` (`key`, `category`, `params_doc`).
pub trait FilterCatalog {
    /// The folders in Photoshop's order (Artistic, Brush Strokes, Distort, Sketch, Stylize,
    /// Texture).
    fn categories(&self) -> Vec<FilterCategory>;

    /// The filter of `key`, or `None` for a key the app no longer has.
    fn filter(&self, key: &str) -> Option<GalleryFilter>;
}

/// A filter's thumbnail key for the `Thumbnails` cache: the filter applied to a small crop of the
/// layer (PhotoCraft renders 6 a pass, 80 × 56). The app's `ThumbSource<GalleryThumb>` renders it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GalleryThumb {
    pub filter: String,
}
