//! Stories of `CompareTile`: a pair (active, rejected, pending, failed) and a survey.

use std::cell::RefCell;

use craft_theme::Srgba;
use craft_ui::contract::{Bind, ShowContainer as _};
use craft_ui::layout_components::Row;
use craft_ui::patterns::form_dialogs::PreviewView;
use craft_ui::primitives::textures::Revision;
use craft_ui::runtime::env::Env;
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use craft_ui::tile_grid::TileFlag;
use egui::{ColorImage, Rect, Ui, Vec2, pos2, vec2};

use super::{ArrangeSpace, CompareCaption, CompareImage, ComparePair, CompareTile, RenderState, survey_columns};

/// The share of the room's width the narrow tile takes: too little for the caption's file name.
const NARROW_SHARE: f32 = 0.4;

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
    CompareImage::Picture(key, Revision::of(&hue.to_bits()), PICTURE, Box::new(move || gradient(hue)))
}

fn caption(label: &str, name: &str) -> CompareCaption {
    CompareCaption::new(label, name).rating(Some(4)).flag(Some(TileFlag::Picked)).label_color(Some(Srgba::rgb(0.86, 0.31, 0.24)))
}

/// Runs `content` over the room the story fills. The room is a container's body, so the story allocates what the
/// tiles are placed in.
fn filling(ui: &mut Ui, content: impl FnOnce(&mut Ui, Rect) -> StoryOutput) -> StoryOutput {
    let content = RefCell::new(Some(content));
    let shown = Row::new().fill().show(ui, |ui, body| content.borrow_mut().take().map(|content| content(ui, body.rect)));
    shown.inner.flatten().unwrap_or_default()
}

#[story(CompareTile<'static>, covers = [Default], drive = Drive::Never)]
fn compare_pair(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let rtl = Env::of(ui).view().direction().is_rtl();
    filling(ui, |ui, area| {
        let mut active = *data.value("active", 0usize);
        let view = data.value("view", PreviewView::fitting());
        let [left, right] = ComparePair::new(ArrangeSpace::new(18.0, 16.0)).rtl(rtl).rects(area);
        let select = CompareTile::new("Select", Bind::bound(view), image(1, 0.58))
            .caption(caption("Select", "IMG_0001.jpg"))
            .active(active == 0)
            .place(ui, left);
        if select.clicked {
            active = 0;
        }
        let candidate = CompareTile::new("Candidate", Bind::bound(view), image(2, 0.08))
            .caption(caption("Candidate", "IMG_0002.jpg"))
            .active(active == 1)
            .rejected(true)
            .place(ui, right);
        if candidate.clicked {
            active = 1;
        }
        *data.value("active", 0usize) = active;
        select.into()
    })
}

#[story(CompareTile<'static>, covers = [LongText], drive = Drive::Never)]
fn compare_tile_narrow(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    filling(ui, |ui, area| {
        let view = data.value("view", PreviewView::fitting());
        let rect = Rect::from_min_size(area.min, Vec2::splat(area.width() * NARROW_SHARE));
        CompareTile::new("Select", Bind::bound(view), image(1, 0.58))
            .caption(caption("Select", "A_file_name_much_longer_than_the_tile.jpeg"))
            .place(ui, rect)
            .into()
    })
}

#[story(CompareTile<'static>, covers = [], drive = Drive::Never)]
fn compare_tile_pending(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    filling(ui, |ui, area| {
        let view = data.value("view", PreviewView::fitting());
        CompareTile::new("Candidate", Bind::bound(view), image(2, 0.08))
            .caption(caption("Candidate", "IMG_0002.jpg"))
            .state(RenderState::Pending(true))
            .place(ui, area)
            .into()
    })
}

#[story(CompareTile<'static>, covers = [], drive = Drive::Never)]
fn compare_tile_failed(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    filling(ui, |ui, area| {
        let view = data.value("view", PreviewView::fitting());
        CompareTile::new("Candidate", Bind::bound(view), CompareImage::None)
            .caption(CompareCaption::new("Candidate", "IMG_0002.jpg"))
            .state(RenderState::Failed("The renderer stopped".into()))
            .place(ui, area)
            .into()
    })
}

/// Twelve tiles at 3:2 in the survey arrangement: fitted, no zoom or pan.
#[story(CompareTile<'static>, covers = [Many], drive = Drive::Never)]
fn compare_survey(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    const COUNT: usize = 12;
    const ASPECT: f32 = 1.5;
    let rtl = Env::of(ui).view().direction().is_rtl();
    filling(ui, |ui, area| {
        let view = data.value("view", PreviewView::fitting());
        let columns = survey_columns(COUNT, area, ASPECT);
        let rows = COUNT.div_ceil(columns);
        let cell = vec2(area.width() / columns as f32, area.height() / rows as f32);
        let mut outs = Vec::new();
        for at in 0..COUNT {
            let column = (at % columns) as f32;
            let x = if rtl { area.max.x - cell.x * (column + 1.0) } else { area.min.x + cell.x * column };
            let rect = Rect::from_min_size(pos2(x, area.min.y + cell.y * (at / columns) as f32), cell);
            let hue = at as f32 / COUNT as f32;
            let name = format!("IMG_{at:04}.jpg");
            let out = CompareTile::new(name.clone(), Bind::bound(view), image(at as u64, hue))
                .caption(CompareCaption::new("", name))
                .survey(true)
                .place(ui, rect);
            outs.push(out);
        }
        outs.swap_remove(0).into()
    })
}
