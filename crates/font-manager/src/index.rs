//! The Google Fonts index (aiongg/google-fonts-index), as craft-font-store reads it, and the
//! categories the catalogue filters by.

pub use craft_font_store::{Family, FontFile, Index, IndexError, Language, Script};

/// The pinned index compiled into craft-font-store, read without a request.
pub fn bundled() -> Result<Index, IndexError> {
    Index::bundled()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Category {
    SansSerif,
    Serif,
    Display,
    Handwriting,
    Monospace,
}

impl Category {
    /// The categories the filter offers, in Google Fonts' order.
    pub const ALL: [Category; 5] = [Category::SansSerif, Category::Serif, Category::Display, Category::Handwriting, Category::Monospace];

    /// The index's name for it (`SANS_SERIF`).
    pub fn of(name: &str) -> Option<Category> {
        match name {
            "SANS_SERIF" => Some(Category::SansSerif),
            "SERIF" => Some(Category::Serif),
            "DISPLAY" => Some(Category::Display),
            "HANDWRITING" => Some(Category::Handwriting),
            "MONOSPACE" => Some(Category::Monospace),
            _ => None,
        }
    }
}

/// A family's categories that this version knows.
pub fn categories(family: &Family) -> impl Iterator<Item = Category> + '_ {
    family.categories.iter().filter_map(|c| Category::of(c))
}

pub fn script_name<'a>(index: &'a Index, id: &str) -> Option<&'a str> {
    index.scripts().iter().find(|s| s.id == id).map(|s| s.name.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_index_reads() {
        let index = bundled().unwrap();
        assert!(index.families().len() > 1000);
        let roboto = index.family("roboto").unwrap();
        assert_eq!(categories(roboto).collect::<Vec<_>>(), [Category::SansSerif]);
        assert_eq!(script_name(&index, "Arab"), Some("Arabic"));
    }
}
