//! Stories of `CompareTile`: a pair (active, rejected, pending, failed) and a survey.

use craft_theme::Srgba;
use craft_ui::contract::Bind;
use craft_ui::patterns::form_dialogs::PreviewView;
use craft_ui::primitives::textures::Revision;
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use craft_ui::tile_grid::TileFlag;
use egui::{ColorImage, Rect, Ui, pos2, vec2};

use super::{ArrangeSpace, CompareCaption, CompareImage, ComparePair, CompareTile, RenderState, survey_columns};

/// The picture's size in pixels.
const PICTURE: [u32; 2] = [480, 320];

/// A gradient with a grid, so zoom and pan show.
fn gradient(hue: f32) -> ColorImage {
    let [w, h] = PICTURE;
    let mut pixels = Vec::with_capacity((w * h) as usize);
    for y in 0..h {
        for x in 0..w {
            let on_grid = x % 40 == 0 || y % 40 == 0;
            let level = if on_grid { 0.25 } else { 0.55 + 0.4 * (x as f32 / w as f32) };
            pixels.push(egui::ecolor::Hsva::new(hue, 0.5, level, 1.0).into());
        }
    }
    ColorImage { size: [w as usize, h as usize], source_size: vec2(w as f32, h as f32), pixels }
}

fn image(key: u64, hue: f32) -> CompareImage<'static> {
    CompareImage::Picture { key, revision: Revision::of(&hue.to_bits()), size: PICTURE, make: Box::new(move || gradient(hue)) }
}

fn caption(label: &str, name: &str) -> CompareCaption {
    CompareCaption::new(label, name).rating(Some(4)).flag(Some(TileFlag::Picked)).label_color(Some(Srgba::rgb(0.86, 0.31, 0.24)))
}

#[story(CompareTile<'static>, covers = [Default], drive = Drive::Never)]
fn compare_pair(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let view = data.value("view", PreviewView::fitting());
    let active = data.value("active", 0usize);
    let area = ui.max_rect();
    let [left, right] = ComparePair::new(ArrangeSpace::new(18.0, 16.0)).rects(area);
    let select =
        CompareTile::new("Select", Bind::bound(view), image(1, 0.58)).caption(caption("Select", "IMG_0001.jpg")).active(*active == 0).show(ui, left);
    if select.clicked {
        *active = 0;
    }
    let candidate = CompareTile::new("Candidate", Bind::bound(view), image(2, 0.08))
        .caption(caption("Candidate", "IMG_0002.jpg"))
        .active(*active == 1)
        .rejected(true)
        .show(ui, right);
    if candidate.clicked {
        *active = 1;
    }
    select.into()
}

#[story(CompareTile<'static>, covers = [LongText], drive = Drive::Never)]
fn compare_tile_narrow(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let view = data.value("view", PreviewView::fitting());
    let rect = Rect::from_min_size(ui.max_rect().min, vec2(160.0, 160.0));
    CompareTile::new("Select", Bind::bound(view), image(1, 0.58))
        .caption(caption("Select", "A_file_name_much_longer_than_the_tile.jpeg"))
        .show(ui, rect)
        .into()
}

#[story(CompareTile<'static>, covers = [], drive = Drive::Never)]
fn compare_tile_pending(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let view = data.value("view", PreviewView::fitting());
    CompareTile::new("Candidate", Bind::bound(view), image(2, 0.08))
        .caption(caption("Candidate", "IMG_0002.jpg"))
        .state(RenderState::Pending { overdue: true })
        .show(ui, ui.max_rect())
        .into()
}

#[story(CompareTile<'static>, covers = [], drive = Drive::Never)]
fn compare_tile_failed(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let view = data.value("view", PreviewView::fitting());
    CompareTile::new("Candidate", Bind::bound(view), CompareImage::None)
        .caption(CompareCaption::new("Candidate", "IMG_0002.jpg"))
        .state(RenderState::Failed("The renderer stopped".into()))
        .show(ui, ui.max_rect())
        .into()
}

/// Twelve tiles at 3:2 in the survey arrangement: fitted, no zoom or pan.
#[story(CompareTile<'static>, covers = [Many], drive = Drive::Never)]
fn compare_survey(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    const COUNT: usize = 12;
    const ASPECT: f32 = 1.5;
    let view = data.value("view", PreviewView::fitting());
    let area = ui.max_rect();
    let columns = survey_columns(COUNT, area, ASPECT);
    let rows = COUNT.div_ceil(columns);
    let cell = vec2(area.width() / columns as f32, area.height() / rows as f32);
    let mut outs = Vec::new();
    for at in 0..COUNT {
        let rect = Rect::from_min_size(pos2(area.min.x + cell.x * (at % columns) as f32, area.min.y + cell.y * (at / columns) as f32), cell);
        let hue = at as f32 / COUNT as f32;
        let name = format!("IMG_{at:04}.jpg");
        let out = CompareTile::new(name.clone(), Bind::bound(view), image(at as u64, hue))
            .caption(CompareCaption::new("", name))
            .survey(true)
            .show(ui, rect);
        outs.push(out);
    }
    outs.swap_remove(0).into()
}
