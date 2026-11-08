//! Stories of the Artboard tool: the boards, the bar and the Properties section.

use super::{ArtboardBackground, ArtboardBar, ArtboardItem, ArtboardOverlay, ArtboardPreset, ArtboardProperties};
use craft_behavior::kurbo::{Rect, Size};
use craft_theme::Srgba;
use craft_ui::bars::{Bar, BarKind};
use craft_ui::canvas::Canvas;
use craft_ui::contract::{Show as _, ShowContainer as _};
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

fn presets() -> Vec<ArtboardPreset> {
    vec![ArtboardPreset::new("Letter", Size::new(612.0, 792.0)), ArtboardPreset::new("Phone", Size::new(390.0, 844.0))]
}

fn boards() -> Vec<ArtboardItem> {
    vec![ArtboardItem::new("Artboard 1", Rect::new(0.0, 0.0, 180.0, 240.0)), ArtboardItem::new("Artboard 2", Rect::new(240.0, 0.0, 420.0, 240.0))]
}

#[story(ArtboardOverlay<'static>, covers = [Default])]
fn two_artboards(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let (items, selected) = (boards(), [0_usize]);
    Canvas::new("Document").show(ui, |ui, body| ArtboardOverlay::new("Artboards", &items, &selected).show_at(ui, body.rect));
    StoryOutput::default()
}

#[story(ArtboardBar<'static>, covers = [Default], drive = Drive::Never)]
fn bar_for_a_landscape_board(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let board = ArtboardItem::new("Artboard 1", Rect::new(0.0, 0.0, 480.0, 270.0));
    let presets = presets();
    Bar::new("Options", BarKind::Options).show(ui, |ui, _| ArtboardBar::new(Some(&board), &presets).show(ui)).output.into()
}

#[story(ArtboardProperties<'static>, covers = [Default], drive = Drive::Never)]
fn properties_with_a_custom_background(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let mut board = ArtboardItem::new("Artboard 1", Rect::new(0.0, 0.0, 390.0, 844.0));
    board.background = ArtboardBackground::Custom(Srgba::rgb(0.9, 0.4, 0.2));
    let presets = presets();
    ArtboardProperties::new(&board, &presets).show(ui);
    StoryOutput::default()
}
