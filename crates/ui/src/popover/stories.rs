//! Stories of `Popover` and `PopupList`.

use super::{Popover, PopupList};
use crate::buttons::Button;
use crate::contract::{ActivateOn, Bind, LeadOnOpen, SelectMode, Show as _, ShowCollection as _, ShowContainer as _};
use crate::layout_components::Stack;
use crate::list::{List, ListRow};
use crate::runtime::focus::Initial;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use craft_behavior::{DisplayOrder, ItemSource};
use egui::Ui;

/// What the story popovers hold: two buttons; whether one was clicked.
fn body(ui: &mut Ui) -> bool {
    Stack::new()
        .show(ui, |ui, _| {
            let snap = Button::new("Snap to grid").show(ui);
            let rulers = Button::new("Show rulers").show(ui);
            crate::contract::clicked(&snap) || crate::contract::clicked(&rulers)
        })
        .inner
        .unwrap_or(false)
}

/// Shows `story` in a row as wide as the room: the opener sits at the start edge and the popup hangs from it, so the
/// popup's place mirrors with the direction.
fn in_row(ui: &mut Ui, mut story: impl FnMut(&mut Ui) -> StoryOutput) -> StoryOutput {
    let shown = Stack::new().fill().show(ui, |ui, _| story(ui));
    match shown.inner {
        Some(output) => output,
        None => shown.output.into(),
    }
}

fn popover_with(ui: &mut Ui, data: &mut StoryData, build: impl Fn(Popover<'_>) -> Popover<'_>) -> StoryOutput {
    in_row(ui, |ui| {
        let opener = Button::new("Grid").show(ui);
        let mut clicked = false;
        let shown = build(Popover::new("Grid options", Bind::bound(data.value("open", true))).opened_by(&opener)).show(ui, |ui, _| {
            clicked = body(ui);
        });
        let mut output = StoryOutput::from(shown.output);
        output.clicked |= clicked;
        output
    })
}

#[story(Popover<'static>, covers = [Default], drive = Drive::Never)]
fn popover(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    popover_with(ui, data, |popover| popover)
}

#[story(Popover<'static>, covers = [], drive = Drive::Never)]
fn popover_resizable_closing_on_enter(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    popover_with(ui, data, |popover| popover.resizable(true).close_on_enter(true).initial_focus(Initial::First))
}

/// A list of names keyed by position.
struct Names<'a>(&'a [&'a str]);

impl DisplayOrder<usize> for Names<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn key(&self, at: usize) -> usize {
        at
    }

    fn index_of(&self, key: &usize) -> Option<usize> {
        (*key < self.0.len()).then_some(*key)
    }

    fn text(&self, at: usize) -> Option<std::borrow::Cow<'_, str>> {
        self.0.get(at).map(|name| std::borrow::Cow::Borrowed(*name))
    }
}

impl ItemSource<usize> for Names<'_> {}

#[story(PopupList<'static>, covers = [Default], drive = Drive::Never)]
fn popup_list(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let names = ["Round", "Square", "Chisel", "Fan"];
    in_row(ui, |ui| {
        let opener = Button::new("Brush tip").show(ui);
        PopupList::new("Brush tips", Bind::bound(data.value("open", true)))
            .opened_by(&opener)
            .show(ui, |ui, _| {
                List::<usize>::new("Brush tips")
                    .select(SelectMode::Single)
                    .activate(ActivateOn::Click)
                    .lead_on_open(LeadOnOpen::First)
                    .lead_follows_hover(true)
                    .focus_on_press(false)
                    .show_items(ui, &Names(&names), &mut |key, _| ListRow::new(names[*key]));
            })
            .output
            .response
            .into()
    })
}
