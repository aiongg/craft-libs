//! `TaskBar`: the contextual task bar over a canvas, hung below its anchor until the user moves it.

use crate::contract::{Bind, Inner};
use crate::runtime::env::Label;
use craft_behavior::kurbo::Point;
use egui::{Rect, Response, Ui};
use std::marker::PhantomData;

/// Where the task bar sits: pinned or following its anchor, and where the user moved it. The one owner of the bar's
/// place; the serde shape is PhotoCraft's, so saved preferences load unchanged, and a non-finite `at` counts as unset.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TaskBarPlace {
    /// The bar keeps its place instead of following the anchor.
    #[serde(default)]
    pub pinned: bool,
    /// The bar's top left corner relative to the area's, once moved; `None` while it follows the anchor.
    #[serde(default)]
    pub at: Option<[f32; 2]>,
}

/// The contextual task bar: a movable modeless layer hung below the screen bounds of `anchor` while its place is
/// unset, with a body of members and a More menu (Hide, Reset, Pin). One place, pin and Hide setting for Crop and
/// Frame.
#[must_use = "show the task bar with `show`"]
#[derive(crate::contract::Pattern, crate::contract::Props)]
pub struct TaskBar<'a> {
    #[allow(dead_code)] // P3(tools-w16): tl.T49
    label: Label,
    #[allow(dead_code)] // P3(tools-w16): tl.T49
    anchor: &'a [Point],
    #[allow(dead_code)] // P3(tools-w16): tl.T49
    place: Bind<'a, TaskBarPlace>,
}

/// What the task bar reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct TaskBarOutput {
    /// The bar's response.
    pub response: Response,
    /// Hide bar was picked from the More menu.
    pub hide: bool,
    /// The bar's rect on screen.
    pub rect: Rect,
}

/// The context the task bar's body gets.
pub struct TaskBarCx<'a> {
    marker: PhantomData<&'a mut ()>,
}

impl TaskBarCx<'_> {
    /// Places the More menu button here, before the members that follow it (the Crop bar puts Cancel and Done after
    /// it); without the call the button comes after the body.
    pub fn more_menu(&mut self, ui: &mut Ui) {
        let _ = ui;
        todo!("tl.T49")
    }
}

impl<'a> TaskBar<'a> {
    /// A bar named `label` hung below the screen bounds of `anchor` (document points) until `place` moves it.
    pub fn new(label: impl Into<Label>, anchor: &'a [Point], place: Bind<'a, TaskBarPlace>) -> Self {
        TaskBar { label: label.into(), anchor, place }
    }

    /// Shows the bar inside `area`; `body` fills its members.
    pub fn show<R>(self, ui: &mut Ui, area: Rect, body: impl FnOnce(&mut Ui, &mut TaskBarCx<'_>) -> R) -> Inner<TaskBarOutput, R> {
        let _ = (ui, area, body);
        todo!("tl.T49")
    }
}
