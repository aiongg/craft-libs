//! The hand-off of a colour choice from a component to the app's colour picker.

use craft_theme::Srgba;

/// A component's request for a colour: the app opens its colour picker with `title` and `color`
/// and answers with the component's `set_color(&key, Srgba)`.
///
/// Components report it as `pick_color: Option<ColorRequest>`, set for the one pass in which the
/// user asked (an edge, taken once). `key` names what is being coloured so one component can ask
/// for several colours; the picker returns it unchanged with the answer. `title` is the picker's
/// title; `None` leaves the app's default.
#[derive(Clone, Debug, PartialEq)]
pub struct ColorRequest {
    /// What the colour is for, returned with the answer.
    pub key: String,
    /// The colour picker's title, translated; `None` for the picker's default.
    pub title: Option<String>,
    /// The colour the picker opens with.
    pub color: Srgba,
}

impl ColorRequest {
    /// A request for `key`, opening the picker at `color`.
    pub fn new(key: impl Into<String>, color: Srgba) -> Self {
        Self { key: key.into(), title: None, color }
    }

    /// The picker's title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
}
