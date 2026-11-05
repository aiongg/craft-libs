//! Filter Gallery's catalog: the app's filters, their categories and parameters.

use craft_ui::dialog::{DialogFields, Param, ParamKind};

/// One filter of the gallery: its key in the app's command (`"filter.artistic.cutout"`), its
/// label in English (translated through the app's `I18n` like `Param` labels), its category key,
/// its parameters in the step 2 notation (`Param::parse_list` of PhotoCraft's `params_doc`), and
/// whether it uses the foreground and background colours (Photoshop shows them in the settings).
#[derive(Clone, Debug, PartialEq)]
pub struct GalleryFilter {
    /// The filter's key in the app's command.
    pub key: String,
    /// The English label.
    pub label: String,
    /// The key of the filter's category.
    pub category: String,
    /// The filter's parameters.
    pub params: Vec<Param>,
    /// Whether the filter uses the foreground and background colours.
    pub uses_colors: bool,
}

impl GalleryFilter {
    /// The filter's defaults as fields: each Range's default, each Choice's default (or its first
    /// option), each Bool, and colour parameters (Neon Glow's) at their default; other kinds are
    /// left out.
    pub fn defaults(&self) -> DialogFields {
        // pc: crates/ui-egui/src/dialogs/gallery_ui.rs:51 `default_params`.
        let mut fields = DialogFields::new();
        for param in &self.params {
            if matches!(param.kind, ParamKind::Range { .. } | ParamKind::Choice { .. } | ParamKind::Bool { .. } | ParamKind::Color { .. })
                && let Some(value) = param.kind.default_value()
            {
                fields.set(param.key.clone(), value);
            }
        }
        fields
    }
}

/// A category folder: its key, its English label and its filters' keys in menu order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilterCategory {
    /// The category's key.
    pub key: String,
    /// The English label.
    pub label: String,
    /// The keys of the category's filters, in menu order.
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
    /// The key of the filtered effect.
    pub filter: String,
}
