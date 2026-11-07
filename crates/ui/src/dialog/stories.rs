//! Stories of the dialog shells.

use super::{BlockingOverlay, Dialog, DialogFrame, FullWindowDialog};
use crate::contract::{Bind, Props as _, Show as _, ShowContainer as _};
use crate::controls::check::Checkbox;
use crate::layout_components::Stack;
use crate::runtime::focus::Initial;
use crate::sections::HeaderAction;
use crate::testing::{Drive, Fixture, StoryData, StoryOutput, story};
use egui::Ui;

/// What the story dialogs hold: two check boxes of fixed values.
fn body(ui: &mut Ui) {
    Stack::new().show(ui, |ui, _| {
        Checkbox::new(Bind::given(true)).label("Snap to grid").show(ui);
        Checkbox::new(Bind::given(false)).label("Show rulers").show(ui);
    });
}

/// The drive of the shells with a close action: a click on it.
fn close_box(fixture: &mut Fixture) {
    if let Some(close) = fixture.by_label("Close") {
        fixture.click(&close);
    }
}

#[story(Dialog<'static>, covers = [Default], drive = Drive::Custom(close_box))]
fn dialog(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Dialog::new("Grid options").opened(Bind::bound(data.value("open", true))).show(ui, |ui, _| body(ui)).into()
}

#[story(Dialog<'static>, covers = [], drive = Drive::Never)]
fn dialog_with_footer(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Dialog::new("Grid options")
        .opened(Bind::bound(data.value("open", true)))
        .footer(|ui, _| {
            Checkbox::new(Bind::given(false)).label("Don't ask again").show(ui);
        })
        .show(ui, |ui, _| body(ui))
        .into()
}

#[story(Dialog<'static>, covers = [], drive = Drive::Never)]
fn dialog_resizable_with_initial_focus(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Dialog::new("Grid options")
        .opened(Bind::bound(data.value("open", true)))
        .initial_focus(Initial::First)
        .resizable(true)
        .show(ui, |ui, _| body(ui))
        .into()
}

#[story(Dialog<'static>, covers = [], drive = Drive::Never)]
fn dialog_modeless(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Dialog::new("Grid options").opened(Bind::bound(data.value("open", true))).modeless().show(ui, |ui, _| body(ui)).into()
}

#[story(DialogFrame<'static>, covers = [Default], drive = Drive::Custom(close_box))]
fn dialog_frame(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    DialogFrame::new("Grid options").closable(true).show(ui, |ui, _| body(ui)).into()
}

#[story(DialogFrame<'static>, covers = [], drive = Drive::Never)]
fn dialog_frame_with_footer(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    DialogFrame::new("Grid options")
        .roomy(true)
        .footer(|ui, _| {
            Checkbox::new(Bind::given(false)).label("Don't ask again").show(ui);
        })
        .show(ui, |ui, _| body(ui))
        .into()
}

#[story(FullWindowDialog<'static>, covers = [Default], drive = Drive::Custom(close_box))]
fn full_window_dialog(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    FullWindowDialog::new("Grid options")
        .opened(Bind::bound(data.value("open", true)))
        .actions([HeaderAction::Close])
        .show(ui, |ui, _| body(ui))
        .into()
}

#[story(BlockingOverlay<'static>, covers = [Default], drive = Drive::Never)]
fn blocking_overlay(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    BlockingOverlay::new("Exporting").opened(Bind::bound(data.value("open", true))).card(true).scrim(true).show(ui, |ui, _| body(ui)).into()
}
