//! Stories of the signature pad.

use super::{DrawPad, SignaturePadDialog};
use crate::signature::SignatureDraft;
use craft_ui::contract::{Bind, Show as _};
use craft_ui::dialog::Dialog;
use craft_ui::dialog::StackDialog as _;
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

#[story(DrawPad<'static>, covers = [Default], drive = Drive::Never)]
fn draw_pad_with_strokes(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let strokes = data.value("strokes", vec![vec![[0.1, 0.5], [0.3, 0.2], [0.5, 0.7], [0.8, 0.4]]]);
    DrawPad::new("Draw", Bind::bound(strokes)).show(ui).into()
}

/// A dialog kept across passes.
#[derive(Clone)]
struct Held(std::sync::Arc<std::sync::Mutex<SignaturePadDialog>>);

impl PartialEq for Held {
    fn eq(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.0, &other.0)
    }
}

impl std::fmt::Debug for Held {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Held(..)")
    }
}

/// The dialog on the Type tab.
fn dialog(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let held =
        data.value("pad", Held(std::sync::Arc::new(std::sync::Mutex::new(SignaturePadDialog::new(SignatureDraft::new(false, "Ana Reyes")))))).clone();
    let mut stack = craft_ui::dialog::DialogStack::default();
    let mut pad = held.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    stack.push(pad.id(), None);
    let _ = pad.show(ui, &stack);
    StoryOutput::default()
}

#[story(Dialog<'static>, covers = [Default], drive = Drive::Never)]
fn signature_pad_type(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    dialog(ui, data)
}
