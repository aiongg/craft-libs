//! Stories of `AltTextField`.

use super::{AltText, AltTextField};
use crate::contract::{Bind, Show as _, Validity};
use crate::runtime::env::Message;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use craft_behavior::Mixed;
use egui::Ui;

const PROMPT: &str = "How would you describe this object and its context to someone who is blind or has low vision?";
const LONG: &str = "A red kite with a long white tail flying over a grey beach on a windy afternoon, with two children holding its string and a dog running behind them";

#[story(AltTextField<'static>, covers = [Default], drive = Drive::Type("A kite"))]
fn alt_text_field(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    AltTextField::new(PROMPT, Bind::bound(data.value("alt", AltText::new("A red kite")))).show(ui).into()
}

#[story(AltTextField<'static>, covers = [LongText], drive = Drive::Never)]
fn alt_text_field_long(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    AltTextField::new(PROMPT, Bind::bound(data.value("long", AltText::new(LONG)))).show(ui).into()
}

#[story(AltTextField<'static>, covers = [Mixed], drive = Drive::Never)]
fn alt_text_field_mixed(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    AltTextField::new(PROMPT, Bind::bound_mixed(data.value("mixed", Mixed::<AltText>::Mixed))).show(ui).into()
}

#[story(AltTextField<'static>, covers = [Invalid], drive = Drive::Never)]
fn alt_text_field_invalid(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let refuse_empty = |alt: &AltText| {
        if alt.decorative || !alt.text.is_empty() { Validity::Ok } else { Validity::Refuse(Message::new("A description is required")) }
    };
    AltTextField::new(PROMPT, Bind::bound(data.value("empty", AltText::default())).check(refuse_empty)).show(ui).into()
}
