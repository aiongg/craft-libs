//! Stories of the display components.

use super::{Badge, Divider, Elide, Icon, Kbd, Progress, Spinner, Text};
use crate::contract::{Props as _, Show as _};
use crate::primitives::IconSpec;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use craft_behavior::IconId;
use craft_theme::{
    Variant,
    variant::{Emphasis, Tone},
};
use egui::Ui;

#[story(Badge, covers = [Default], drive = Drive::Never)]
fn badge(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    Badge::new("Needs review").variant(Variant::look(Emphasis::Soft, Tone::Warn)).show(ui).into()
}

#[story(Badge, covers = [], drive = Drive::Never)]
fn badge_count(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    Badge::count(1234).variant(Variant::look(Emphasis::Solid, Tone::Bad)).show(ui).into()
}

#[story(Kbd, covers = [Default], drive = Drive::Never)]
fn kbd(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    Kbd::new("Ctrl+Shift+S").show(ui).into()
}

#[story(Kbd, covers = [], drive = Drive::Never)]
fn kbd_keycap(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    Kbd::new("Ctrl+Shift+S").keycap(true).show(ui).into()
}

#[story(Progress, covers = [Default], drive = Drive::Never)]
fn progress(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    Progress::new(0.4).label("Export").show(ui).into()
}

#[story(Progress, covers = [], drive = Drive::Never)]
fn progress_indeterminate(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    Progress::indeterminate().label("Working").show(ui).into()
}

#[story(Spinner, covers = [Default], drive = Drive::Never)]
fn spinner(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    Spinner::new().label("Loading").show(ui).into()
}

#[story(Text, covers = [Default], drive = Drive::Never)]
fn text(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    Text::new("Layers").show(ui).into()
}

#[story(Text, covers = [], drive = Drive::Never)]
fn text_section(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    Text::section("Blend mode").show(ui).into()
}

#[story(Text, covers = [], drive = Drive::Never)]
fn text_elided(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    Text::new("A layer name much longer than the room it is shown in").elide(Elide::End).show(ui).into()
}

#[story(Icon<'static>, covers = [Default], drive = Drive::Never)]
fn icon(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    Icon::new(IconSpec::svg(IconId::new("settings"))).label("Settings").show(ui).into()
}

#[story(Divider, covers = [Default], drive = Drive::Never)]
fn divider(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    Divider::horizontal().show(ui).into()
}
