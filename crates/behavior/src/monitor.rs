//! The monitor type of persisted layouts.

use emath::Rect;

/// A display in desktop logical points, the one monitor type of saved data. Applications on an egui with monitor
/// reports fill their monitor list with these; a native floating window stores the one it is on.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct Monitor {
    /// Display name from the OS, when it reports one.
    pub name: Option<String>,
    /// The display's area on the desktop. Saved as `[x, y, width, height]`.
    #[serde(with = "rect_serde")]
    pub rect: Rect,
    /// Physical pixels per logical point.
    pub scale: f32,
}

impl Monitor {
    /// A display at `rect` with `scale` physical pixels per logical point.
    pub fn new(name: Option<String>, rect: Rect, scale: f32) -> Self {
        Self { name, rect, scale }
    }
}

/// Saved rects are `[x, y, width, height]`: the shape older files hold. `#[serde(with = "..")]` adapters and the
/// two conversions for readers that build the array themselves.
pub(crate) mod rect_serde {
    use emath::{Pos2, Rect, Vec2};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    /// `[x, y, width, height]` of `rect`.
    pub(crate) fn to_array(rect: Rect) -> [f32; 4] {
        [rect.min.x, rect.min.y, rect.width(), rect.height()]
    }

    /// The rect at `[x, y, width, height]`.
    pub(crate) fn from_array([x, y, width, height]: [f32; 4]) -> Rect {
        Rect::from_min_size(Pos2::new(x, y), Vec2::new(width, height))
    }

    pub(crate) fn serialize<S: Serializer>(rect: &Rect, serializer: S) -> Result<S::Ok, S::Error> {
        to_array(*rect).serialize(serializer)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Rect, D::Error> {
        <[f32; 4]>::deserialize(deserializer).map(from_array)
    }

    /// The adapter for `Option<Rect>` fields. A field using it also needs `#[serde(default)]`.
    pub(crate) mod option {
        use super::{Deserialize, Deserializer, Rect, Serialize, Serializer, from_array, to_array};

        pub(crate) fn serialize<S: Serializer>(rect: &Option<Rect>, serializer: S) -> Result<S::Ok, S::Error> {
            rect.map(to_array).serialize(serializer)
        }

        pub(crate) fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<Rect>, D::Error> {
            Option::<[f32; 4]>::deserialize(deserializer).map(|rect| rect.map(from_array))
        }
    }
}
