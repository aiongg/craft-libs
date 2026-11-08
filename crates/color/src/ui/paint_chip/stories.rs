//! Stories of the paint chip and the paint list.

use super::{GradientPreview, ImagePreview, Paint, PaintChip, PaintLink, PaintLinkKind, PaintList, PaintListOptions, PaintRow};
use craft_theme::Srgba;
use craft_ui::contract::{Bind, Show as _};
use craft_ui::list::List;
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

const BLUE: Srgba = Srgba::rgb(0.2, 0.4, 0.8);

fn chip(ui: &mut Ui, data: &mut StoryData, paint: Paint) -> StoryOutput {
    let stops = vec![(0.0, BLUE), (1.0, Srgba::rgb(1.0, 1.0, 1.0))];
    let held = data.value("paint", paint);
    PaintChip::new("Fill", Bind::bound(held), BLUE).none_allowed().gradient_editor(GradientPreview::new(stops), |_: &mut Ui| None).show(ui).into()
}

#[story(PaintChip<'static>, covers = [Default], drive = Drive::Never)]
fn paint_chip(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    chip(ui, data, Paint::Solid(Srgba { r: 0.9, g: 0.2, b: 0.2, a: 0.6 }))
}

#[story(PaintChip<'static>, covers = [], drive = Drive::Never)]
fn paint_chip_no_color(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    chip(ui, data, Paint::None)
}

#[story(PaintChip<'static>, covers = [], drive = Drive::Never)]
fn paint_chip_gradient(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    chip(ui, data, Paint::Gradient(GradientPreview::new(vec![(0.0, BLUE), (0.5, Srgba::rgb(0.9, 0.8, 0.1)), (1.0, Srgba::rgb(0.9, 0.2, 0.2))])))
}

#[story(PaintChip<'static>, covers = [], drive = Drive::Never)]
fn paint_chip_pattern(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    chip(ui, data, Paint::Pattern(ImagePreview::new("Bubbles", None)))
}

fn paints() -> Vec<(String, Paint)> {
    vec![
        ("Brand blue".to_owned(), Paint::Solid(BLUE)),
        ("Sky".to_owned(), Paint::Solid(Srgba { r: 0.5, g: 0.7, b: 1.0, a: 0.5 })),
        ("Sunset".to_owned(), Paint::Gradient(GradientPreview::new(vec![(0.0, Srgba::rgb(0.9, 0.5, 0.1)), (1.0, Srgba::rgb(0.6, 0.1, 0.4))]))),
        ("Nothing".to_owned(), Paint::None),
    ]
}

#[story(List<'static, usize>, covers = [], drive = Drive::Never)]
fn paint_list(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let paints = paints();
    PaintList::new("Paints", &paints, Bind::bound(data.value("selected", None::<usize>))).show(ui).into()
}

#[story(List<'static, usize>, covers = [], drive = Drive::Never)]
fn paint_list_selection_colors(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let paints = paints();
    let rows = [PaintRow::linked(PaintLink::new("Primary / Blue", PaintLinkKind::Style))];
    PaintList::new("Selection colors", &paints, Bind::bound(data.value("selected", Some(0usize))))
        .options(PaintListOptions { select_users: true, bind: true, remove: true, add: true, ..PaintListOptions::SELECTION_COLORS })
        .rows(&rows)
        .show(ui)
        .into()
}
