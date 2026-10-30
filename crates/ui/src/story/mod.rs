//! Stories: named examples of a component, listed by the `craft-ui-gallery` app.
//!
//! A [`Story`] has a stable id (`level/component/story`), default [`Args`] and a `show` function.
//! [`registry`] lists every story of the crate.
//!
//! # Writing a story
//!
//! A story's args are a struct deriving [`StoryArgs`]; its fields are the arg rows of the gallery's
//! Properties panel, in field order. Choice args are enums deriving [`StoryChoice`]. The story's
//! function takes the struct, so a misspelt or mistyped arg does not compile.
//! [`typed_story!`](crate::typed_story) makes the [`Story`] from the id, the default values and
//! the function.
//!
//! ```
//! use craft_ui::story::{Level, Story, StoryArgs, StoryChoice};
//! use craft_ui::typed_story;
//!
//! #[derive(StoryChoice, Clone, Copy, PartialEq, Debug)]
//! enum Tone {
//!     Quiet,
//!     #[choice(name = "very loud")]
//!     Loud,
//! }
//!
//! #[derive(StoryArgs, PartialEq, Debug)]
//! struct NoteArgs {
//!     tone: Tone,
//!     label: String,
//!     #[arg(range = 1.0..=10.0)]
//!     lines: f64,
//!     #[arg(name = "show border")]
//!     bordered: bool,
//! }
//!
//! fn show_note(ui: &mut egui::Ui, args: &mut NoteArgs) {
//!     ui.label(format!("{:?}: {} ({} lines)", args.tone, args.label, args.lines));
//! }
//!
//! let story = typed_story!(
//!     "controls/note/playground",
//!     Level::Controls,
//!     "Note",
//!     "Playground",
//!     NoteArgs { tone: Tone::Loud, label: "Hello".into(), lines: 2.0, bordered: true },
//!     show_note
//! );
//! let args = story.defaults();
//! assert_eq!(NoteArgs::from_args(&args).tone, Tone::Loud);
//! assert_eq!(args.choice("tone"), "very loud");
//! ```
//!
//! A component module with stories exposes `pub fn stories() -> Vec<Story>` and
//! `pub const COMPONENTS: &[&str]` (the [`Story::component`] names it covers), and is added to
//! the one list in `register!` (in this file), which defines [`registry`] and [`components`]. A
//! test fails when a declared component has no story.
//!
//! ```
//! # egui::__run_test_ui(|ui| {
//! let story = &craft_ui::story::registry()[0];
//! let mut args = story.defaults();
//! (story.show)(ui, &mut args);
//! # });
//! ```

mod args;
mod choices;
mod colours;
mod stock_egui;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod typed_tests;

pub use args::{Arg, ArgValue, Args, StoryArgs, StoryChoice, show_typed};
pub use choices::SizeArg;
/// `#[derive(StoryArgs)]` and `#[derive(StoryChoice)]`; see the traits.
pub use craft_ui_macros::{StoryArgs, StoryChoice};

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

/// The surface a component sits on in an app; the gallery draws a story on it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Surface {
    /// A dock panel (`surface.panel`): most components.
    #[default]
    Panel,
    /// The document area (`surface.canvas`): components drawn over the document.
    Canvas,
    /// The window chrome (`surface.chrome`): option bars, title bars and other bars.
    Chrome,
    /// The story draws its own panel or dock, so the gallery adds no surface round it.
    Own,
}

impl Surface {
    /// Every surface, in the order the capture tool lists them.
    pub const ALL: [Surface; 4] = [Surface::Panel, Surface::Canvas, Surface::Chrome, Surface::Own];

    /// The surface's lower-case name, as the capture spec spells it.
    pub const fn slug(self) -> &'static str {
        match self {
            Surface::Panel => "panel",
            Surface::Canvas => "canvas",
            Surface::Chrome => "chrome",
            Surface::Own => "own",
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
    /// The surface the component sits on in an app; [`Surface::Panel`] unless set with [`Story::on`].
    pub surface: Surface,
}

impl Story {
    /// A story on the panel surface.
    pub const fn new(
        id: &'static str,
        level: Level,
        component: &'static str,
        name: &'static str,
        args: fn() -> Args,
        show: fn(&mut egui::Ui, &mut Args),
    ) -> Self {
        Self { id, level, component, name, args, show, surface: Surface::Panel }
    }

    /// The same story on another surface.
    pub const fn on(mut self, surface: Surface) -> Self {
        self.surface = surface;
        self
    }

    /// The default args, labelled with this story's id for panics on a misspelt name.
    pub fn defaults(&self) -> Args {
        (self.args)().for_story(self.id)
    }
}

/// A [`Story`] whose args are a [`StoryArgs`] struct: `typed_story!(id, level, component, name,
/// defaults, show)`, where `defaults` is an expression of the args type (the story's default
/// values) and `show` a function `fn(&mut Ui, &mut A)` of that type `A`, written as a path or a
/// closure that captures nothing. See the [module](crate::story) docs.
///
/// The defaults and the function must use the same args type:
///
/// ```compile_fail
/// use craft_ui::story::{Level, StoryArgs};
/// use craft_ui::typed_story;
///
/// #[derive(StoryArgs)]
/// struct One {
///     on: bool,
/// }
/// #[derive(StoryArgs)]
/// struct Two {
///     off: bool,
/// }
/// fn show(_: &mut egui::Ui, _: &mut Two) {}
///
/// let _ = typed_story!("controls/a/b", Level::Controls, "A", "B", One { on: true }, show);
/// ```
#[macro_export]
macro_rules! typed_story {
    ($id:expr, $level:expr, $component:expr, $name:expr, $defaults:expr, $show:expr $(,)?) => {
        $crate::story::Story::new(
            $id,
            $level,
            $component,
            $name,
            || $crate::story::StoryArgs::to_args(&$defaults),
            |ui, args| $crate::story::show_typed(ui, args, || $defaults, $show),
        )
    };
}

/// Lists every module with stories once and defines [`registry`] and [`components`] from the list,
/// in the order written. Each path names a module that has `stories()` and `COMPONENTS`. The
/// list is written out, with no linker registration, so the order is fixed and it works on wasm.
macro_rules! register {
    ($($(#[$cfg:meta])* $($module:ident)::+),* $(,)?) => {
        /// Every story of the crate, in listing order.
        pub fn registry() -> Vec<Story> {
            let mut all = Vec::new();
            $($(#[$cfg])* all.extend($($module)::+::stories());)*
            all
        }

        /// The `COMPONENTS` list of every module with stories, in [`registry`] order.
        const DECLARED: &[&[&str]] = &[$($(#[$cfg])* $($module)::+::COMPONENTS),*];
    };
}

register! {
    colours,
    stock_egui,
    crate::icons,
    primitives::surface,
    primitives::state_layer,
    primitives::focus_ring,
    primitives::divider,
    primitives::stack,
    primitives::interactive,
    primitives::text,
    primitives::icon,
    primitives::handle,
    crate::canvas::story,
    crate::canvas::guides,
    crate::canvas::marquee,
    crate::canvas::rulers,
    crate::canvas::transform_box,
    crate::canvas::ref_point,
    crate::canvas::bezier::story,
    crate::canvas::warp::story,
    crate::canvas::pen::story,
    crate::canvas::gradient_overlay::story,
    crate::edit,
    controls::button,
    controls::icon_button,
    controls::toggle_button,
    controls::segmented,
    controls::select,
    controls::combo_box,
    controls::link,
    controls::checkbox,
    controls::radio,
    controls::switch,
    controls::slider,
    controls::range_slider,
    controls::thumb_row,
    controls::swatch,
    controls::badge,
    controls::kbd,
    controls::spinner,
    controls::progress,
    controls::tooltip,
    controls::text_field,
    controls::number_field,
    controls::angle_dial,
    composites::field,
    composites::form_grid,
    composites::property_row,
    composites::section_header,
    composites::section_label,
    composites::panel_header,
    composites::panel_footer,
    composites::slider_row,
    composites::toast,
    composites::notice,
    composites::empty_state,
    composites::curve_editor,
    composites::gradient,
    crate::canvas::transform_fields,
    composites::histogram,
    composites::levels_editor,
    crate::tab_strip,
    crate::list,
    crate::menu,
    crate::menu::bar,
    #[cfg(feature = "window-frame")]
    crate::title_bar::stories,
    composites::color,
    crate::docking,
    crate::dialog::stories,
    crate::dialog::full_window_stories,
    crate::dialog::preview_stories,
    crate::viewport::stories,
    crate::patterns::options_bar,
    crate::patterns::status_bar,
    crate::patterns::panel,
    crate::patterns::rail::panel_rail,
    crate::patterns::rail::flyout,
    crate::patterns::rail::tool,
    crate::popover,
}

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

/// A row of a story that lists labelled variants: `label` in the label column of a
/// [`FormGrid`](composites::FormGrid), the control beside it at its own width.
pub(crate) fn variant_row<C: composites::FieldControl>(form: &mut composites::Form<'_>, label: &str, control: impl FnOnce(&mut egui::Ui) -> C) {
    form.row(label, |ui, _| control(ui));
}

/// As [`variant_row`] for a row of several controls side by side.
pub(crate) fn variant_cluster(form: &mut composites::Form<'_>, label: &str, controls: impl FnOnce(&mut egui::Ui)) {
    variant_row(form, label, |ui| {
        use crate::primitives::{Gap, Row};
        Row::new(Gap::Inline).show(ui, controls).response
    });
}

/// A list of [`variant_row`]s: a form grid whose labels read at the start and whose label column
/// is as wide as the widest label.
pub(crate) fn variant_list<R>(ui: &mut egui::Ui, id_salt: &str, rows: impl FnOnce(&mut composites::Form<'_>) -> R) -> R {
    composites::FormGrid::new(id_salt).label_align(composites::LabelAlign::Start).show(ui, rows).inner
}
