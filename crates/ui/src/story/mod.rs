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

use crate::{composites, controls, primitives};

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
    all.extend(crate::canvas::story::stories());
    all.extend(crate::canvas::guides::stories());
    all.extend(crate::canvas::marquee::stories());
    all.extend(crate::canvas::rulers::stories());
    all.extend(crate::canvas::transform_box::stories());
    all.extend(crate::canvas::ref_point::stories());
    all.extend(crate::canvas::bezier::story::stories());
    all.extend(crate::canvas::warp::story::stories());
    all.extend(crate::canvas::pen::story::stories());
    all.extend(crate::canvas::gradient_overlay::story::stories());
    all.extend(crate::edit::stories());
    all.extend(controls::button::stories());
    all.extend(controls::icon_button::stories());
    all.extend(controls::toggle_button::stories());
    all.extend(controls::segmented::stories());
    all.extend(controls::select::stories());
    all.extend(controls::combo_box::stories());
    all.extend(controls::link::stories());
    all.extend(controls::checkbox::stories());
    all.extend(controls::radio::stories());
    all.extend(controls::switch::stories());
    all.extend(controls::slider::stories());
    all.extend(controls::range_slider::stories());
    all.extend(controls::swatch::stories());
    all.extend(controls::badge::stories());
    all.extend(controls::kbd::stories());
    all.extend(controls::spinner::stories());
    all.extend(controls::progress::stories());
    all.extend(controls::tooltip::stories());
    all.extend(controls::text_field::stories());
    all.extend(controls::number_field::stories());
    all.extend(controls::angle_dial::stories());
    all.extend(composites::field::stories());
    all.extend(composites::property_row::stories());
    all.extend(composites::section_header::stories());
    all.extend(composites::panel_header::stories());
    all.extend(composites::toast::stories());
    all.extend(composites::notice::stories());
    all.extend(composites::empty_state::stories());
    all.extend(composites::curve_editor::stories());
    all.extend(composites::gradient::stories());
    all.extend(crate::canvas::transform_fields::stories());
    all.extend(composites::histogram::stories());
    all.extend(composites::levels_editor::stories());
    all.extend(crate::tab_strip::stories());
    all.extend(crate::list::stories());
    all.extend(crate::menu::stories());
    all.extend(crate::menu::bar::stories());
    all.extend(composites::color::stories());
    all.extend(crate::docking::stories());
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
    crate::canvas::story::COMPONENTS,
    crate::canvas::guides::COMPONENTS,
    crate::canvas::marquee::COMPONENTS,
    crate::canvas::ref_point::COMPONENTS,
    crate::canvas::transform_fields::COMPONENTS,
    crate::canvas::rulers::COMPONENTS,
    crate::canvas::transform_box::COMPONENTS,
    crate::canvas::bezier::story::COMPONENTS,
    crate::canvas::warp::story::COMPONENTS,
    crate::canvas::pen::story::COMPONENTS,
    crate::canvas::gradient_overlay::story::COMPONENTS,
    crate::edit::COMPONENTS,
    controls::button::COMPONENTS,
    controls::icon_button::COMPONENTS,
    controls::toggle_button::COMPONENTS,
    controls::segmented::COMPONENTS,
    controls::select::COMPONENTS,
    controls::combo_box::COMPONENTS,
    controls::link::COMPONENTS,
    controls::checkbox::COMPONENTS,
    controls::radio::COMPONENTS,
    controls::switch::COMPONENTS,
    controls::slider::COMPONENTS,
    controls::range_slider::COMPONENTS,
    controls::swatch::COMPONENTS,
    controls::badge::COMPONENTS,
    controls::kbd::COMPONENTS,
    controls::spinner::COMPONENTS,
    controls::progress::COMPONENTS,
    controls::tooltip::COMPONENTS,
    controls::text_field::COMPONENTS,
    controls::number_field::COMPONENTS,
    controls::angle_dial::COMPONENTS,
    composites::field::COMPONENTS,
    composites::property_row::COMPONENTS,
    composites::section_header::COMPONENTS,
    composites::panel_header::COMPONENTS,
    composites::toast::COMPONENTS,
    composites::notice::COMPONENTS,
    composites::empty_state::COMPONENTS,
    composites::curve_editor::COMPONENTS,
    composites::gradient::COMPONENTS,
    composites::histogram::COMPONENTS,
    composites::levels_editor::COMPONENTS,
    crate::tab_strip::COMPONENTS,
    crate::list::COMPONENTS,
    crate::menu::COMPONENTS,
    crate::menu::bar::COMPONENTS,
    composites::color::COMPONENTS,
    crate::docking::COMPONENTS,
];

/// Every component the crate's modules declare, in [`registry`] order.
pub fn components() -> impl Iterator<Item = &'static str> {
    DECLARED.iter().flat_map(|list| list.iter().copied())
}

fn pixels_per_point_key() -> egui::Id {
    egui::Id::unique("craft_ui::story::pixels_per_point")
}

/// Asks the gallery to draw at `pixels_per_point` while the story calling this shows, so canvas
/// stories can show device-pixel drawing at a scale other than the screen's. The scale is egui's
/// zoom, so it rescales the whole gallery, chrome included. The request holds for the pass it is
/// made in; the gallery reads it with [`take_pixels_per_point_request`] at the end of the pass,
/// applies it when it changes and returns to its own scale once no story makes it.
pub fn request_pixels_per_point(ctx: &egui::Context, pixels_per_point: f32) {
    ctx.data_mut(|d| d.insert_temp(pixels_per_point_key(), pixels_per_point));
}

/// The scale a story requested in this pass, if one did; clears the request.
pub fn take_pixels_per_point_request(ctx: &egui::Context) -> Option<f32> {
    ctx.data_mut(|d| d.remove_temp::<f32>(pixels_per_point_key()))
}
