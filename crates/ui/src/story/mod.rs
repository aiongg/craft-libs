//! Stories: named examples of a component, listed by the `craft-ui-gallery` app.
//!
//! A [`Story`] has a stable id (`level/component/story`), default [`Args`] and a `show` function.
//! [`registry`] lists every story of the crate. A component module with stories exposes
//! `pub fn stories() -> Vec<Story>` and `pub const COMPONENTS: &[&str]` (the
//! [`Story::component`] names it covers), and is added to [`registry`] and [`components`] by
//! hand. A test fails when a declared component has no story.
//!
//! ```
//! # egui::__run_test_ui(|ui| {
//! let story = &craft_ui::story::registry()[0];
//! let mut args = story.defaults();
//! (story.show)(ui, &mut args);
//! # });
//! ```

mod args;
mod colours;
mod stock_egui;
#[cfg(test)]
mod tests;

pub use args::{Arg, ArgValue, Args};

use crate::{controls, primitives};

/// Where a story's component sits in the design system, in the order the gallery lists them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Level {
    Foundations,
    Primitives,
    Controls,
    Composites,
    Patterns,
    Shells,
    Specimens,
}

impl Level {
    /// Every level, in listing order.
    pub const ALL: [Level; 7] =
        [Level::Foundations, Level::Primitives, Level::Controls, Level::Composites, Level::Patterns, Level::Shells, Level::Specimens];

    /// The first segment of story ids of this level.
    pub const fn slug(self) -> &'static str {
        match self {
            Level::Foundations => "foundations",
            Level::Primitives => "primitives",
            Level::Controls => "controls",
            Level::Composites => "composites",
            Level::Patterns => "patterns",
            Level::Shells => "shells",
            Level::Specimens => "specimens",
        }
    }

    /// The level's name for display.
    pub const fn title(self) -> &'static str {
        match self {
            Level::Foundations => "Foundations",
            Level::Primitives => "Primitives",
            Level::Controls => "Controls",
            Level::Composites => "Composites",
            Level::Patterns => "Patterns",
            Level::Shells => "Shells",
            Level::Specimens => "Specimens",
        }
    }
}

/// One example of a component.
#[derive(Clone, Copy)]
pub struct Story {
    /// `level/component/story` in lower-case slugs, e.g. `controls/button/primary`; unique.
    pub id: &'static str,
    pub level: Level,
    /// The component's display name, e.g. `Button`.
    pub component: &'static str,
    /// The story's display name, e.g. `Primary`.
    pub name: &'static str,
    /// The defaults of the args panel.
    pub args: fn() -> Args,
    /// Draws the story. Runs inside the gallery's theme scope; read colours and fonts with
    /// [`Tokens::of`](crate::theme::TokensOf::of).
    pub show: fn(&mut egui::Ui, &mut Args),
}

impl Story {
    /// The default args, labelled with this story's id for panics on a misspelt name.
    pub fn defaults(&self) -> Args {
        (self.args)().for_story(self.id)
    }
}

/// Every story of the crate. Each module's `stories()` is listed by hand: no linker registration,
/// so the order is fixed and the list works on wasm.
pub fn registry() -> Vec<Story> {
    let mut all = Vec::new();
    all.extend(colours::stories());
    all.extend(stock_egui::stories());
    all.extend(crate::icons::stories());
    all.extend(primitives::surface::stories());
    all.extend(primitives::state_layer::stories());
    all.extend(primitives::focus_ring::stories());
    all.extend(primitives::divider::stories());
    all.extend(primitives::stack::stories());
    all.extend(primitives::interactive::stories());
    all.extend(primitives::text::stories());
    all.extend(primitives::icon::stories());
    all.extend(primitives::handle::stories());
    all.extend(crate::edit::stories());
    all.extend(controls::button::stories());
    all.extend(controls::icon_button::stories());
    all.extend(controls::toggle_button::stories());
    all.extend(controls::segmented::stories());
    all.extend(controls::link::stories());
    all.extend(controls::checkbox::stories());
    all.extend(controls::radio::stories());
    all.extend(controls::switch::stories());
    all.extend(controls::slider::stories());
    all.extend(controls::range_slider::stories());
    all
}

/// The `COMPONENTS` list of every module with stories, in [`registry`] order.
const DECLARED: &[&[&str]] = &[
    colours::COMPONENTS,
    stock_egui::COMPONENTS,
    crate::icons::COMPONENTS,
    primitives::surface::COMPONENTS,
    primitives::state_layer::COMPONENTS,
    primitives::focus_ring::COMPONENTS,
    primitives::divider::COMPONENTS,
    primitives::stack::COMPONENTS,
    primitives::interactive::COMPONENTS,
    primitives::text::COMPONENTS,
    primitives::icon::COMPONENTS,
    primitives::handle::COMPONENTS,
    crate::edit::COMPONENTS,
    controls::button::COMPONENTS,
    controls::icon_button::COMPONENTS,
    controls::toggle_button::COMPONENTS,
    controls::segmented::COMPONENTS,
    controls::link::COMPONENTS,
    controls::checkbox::COMPONENTS,
    controls::radio::COMPONENTS,
    controls::switch::COMPONENTS,
    controls::slider::COMPONENTS,
    controls::range_slider::COMPONENTS,
];

/// Every component the crate's modules declare, in [`registry`] order.
pub fn components() -> impl Iterator<Item = &'static str> {
    DECLARED.iter().flat_map(|list| list.iter().copied())
}
