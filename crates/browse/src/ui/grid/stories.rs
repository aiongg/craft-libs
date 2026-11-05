//! Stories of the tile grid. The spec's ids `browse/grid-*` are `<level>/<component>/<story>`
//! here, as the gallery requires:
//!
//! | Story id | Shows |
//! |---|---|
//! | `patterns/tile-grid/justified` | 50 000 tiles in justified rows, date-like groups with a pinned header, badge bar Auto, selection, wheel |
//! | `patterns/tile-grid/cards` | square cards with captions, rating row, check mode, the zoom slider |
//! | `patterns/tile-grid/states` | pending, failed, offline, unreadable, rejected, disabled, dimmed, stack, badges in every slot and tone; RTL |
//! | `patterns/tile-grid/reorder` | cards with the insert line, Alt copy, marquee |
//!
//! The tiles are generated from their key, the pictures are gradients of the right
//! aspect, and the grid's events change the story's fake model (ratings, removal, reorder, zoom).
//! Pending pictures wait on a `ManualJobs`, one per "Finish next".

use std::sync::Arc;

use craft_host::ManualJobs;
use craft_theme::Rgba;
use craft_ui::behavior::{Axis, DropPosition, Move, Reorder, Selection};
use craft_ui::canvas::textures::{
    LoadFailure, LoadedPicture, Revision, ThumbRender, ThumbRequest, ThumbSource, ThumbnailOptions, Thumbnails, thumb_image,
};
use craft_ui::controls::{BadgeTone, Button};
use craft_ui::direction::{self, Direction, HasDirection as _};
use craft_ui::edit::EditEvent;
use craft_ui::icons::Icon;
use craft_ui::localized::Labels as _;
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, Story, StoryArgs, StoryChoice, StoryLabels, lock, shared};
use craft_ui::typed_story;
use egui::Ui;
use egui::ecolor::Hsva;

use super::{
    BadgeKind, BadgeSlot, BadgeVisibility, RatingKeys, ThumbnailSlider, TileBadge, TileContent, TileEvent, TileFlag, TileGrid, TileGroup, TileLayout,
    TileStatus, TileZoom,
};
use craft_ui::list::FakeTiles;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["TileGrid"];

craft_ui::words! {
    /// The sample text of the tile grid stories, English by default.
    #[non_exhaustive]
    pub struct GridStoryLabels {
        group_title: "gridStory.groupTitle" = "Group {n}";
        group_detail: "gridStory.groupDetail" = "{n} photos";
        duplicate_tip: "gridStory.duplicateTip" = "Duplicate of another photo";
        slot_tip: "gridStory.slotTip" = "Badge in this corner";
        offline_band: "gridStory.offlineBand" = "Offline";
        waiting: "gridStory.waiting" = "{n} pictures waiting";
        photos: "gridStory.photos" = "Photos";
    }
}

/// Which tiles the model makes.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Ratings, flags, colour labels and the odd duplicate badge.
    Photos,
    /// One state per tile, repeating every twelve.
    States,
}

/// A hash of `id` that spreads small ids.
fn mix(id: usize) -> usize {
    (id.wrapping_mul(2_654_435_761) >> 7) & 0xff_ffff
}

/// A tile's picture width over height.
fn aspect_of(id: usize) -> f32 {
    [1.5, 0.75, 1.0, 1.78, 0.67, 1.33][mix(id) % 6]
}

/// Moves or copies `mv.items` next to `mv.target` in `tiles`. A copy gets fresh ids.
fn apply_move(tiles: &mut FakeTiles, mv: &Move<usize>) {
    let mut items = mv.items.clone();
    if mv.copy {
        let first = tiles.keys.iter().max().map_or(0, |max| max + 1);
        items = (first..first + items.len()).collect();
    } else {
        tiles.keys.retain(|key| !mv.items.contains(key));
    }
    let at = tiles.keys.iter().position(|key| *key == mv.target).unwrap_or(tiles.keys.len());
    let at = if mv.position == DropPosition::After { at + 1 } else { at };
    let at = at.min(tiles.keys.len());
    tiles.keys.splice(at..at, items);
    tiles.changed();
}

/// The fake photos: keys are ids in display order, content comes from the id.
fn photos(count: usize, mode: Mode, labels: &StoryLabels, words: GridStoryLabels) -> FakeTiles {
    let mut tiles = FakeTiles::new(count);
    tiles.aspect = Box::new(aspect_of);
    let labels = labels.clone();
    tiles.describe = Box::new(move |id, content| {
        content.name = craft_i18n::fmt(&labels.photo_n, &[("n", &(id + 1).to_string())]);
        describe(mode, &words, &labels, id, content);
    });
    tiles
}

fn badge(words: &GridStoryLabels, slot: BadgeSlot, tone: BadgeTone, kind: BadgeKind) -> TileBadge {
    TileBadge { slot, kind, tone, tip: words.slot_tip.clone() }
}

fn describe(mode: Mode, words: &GridStoryLabels, labels: &StoryLabels, id: usize, content: &mut TileContent) {
    match mode {
        Mode::Photos => {
            content.rating = Some((mix(id) % 6) as u8);
            if id.is_multiple_of(11) {
                content.flag = Some(TileFlag::Picked);
            }
            if id.is_multiple_of(17) {
                content.flag = Some(TileFlag::Rejected);
                content.rejected = true;
            }
            if id.is_multiple_of(5) {
                content.label_color =
                    Some([Rgba::rgb(220, 70, 60), Rgba::rgb(230, 170, 40), Rgba::rgb(70, 160, 90), Rgba::rgb(70, 120, 220)][id / 5 % 4]);
            }
            if id.is_multiple_of(13) {
                let kind = BadgeKind::Text(labels.duplicate.clone());
                let mut duplicate = badge(words, BadgeSlot::EndTop, BadgeTone::Neutral, kind);
                duplicate.tip = words.duplicate_tip.clone();
                content.badges.push(duplicate);
            }
        }
        Mode::States => {
            content.rating = Some(3);
            match id % 12 {
                2 => content.status = TileStatus::Failed,
                3 => content.status = TileStatus::Offline(words.offline_band.clone()),
                4 => content.status = TileStatus::Unreadable,
                5 => {
                    content.flag = Some(TileFlag::Rejected);
                    content.rejected = true;
                }
                6 => content.enabled = false,
                7 => content.dimmed = true,
                8 => content.stack = Some(4),
                9 => {
                    let tones = [BadgeTone::Neutral, BadgeTone::Accent, BadgeTone::Warn, BadgeTone::Bad];
                    let slots = [BadgeSlot::StartTop, BadgeSlot::EndTop, BadgeSlot::StartBottom, BadgeSlot::EndBottom];
                    for (slot, tone) in slots.into_iter().zip(tones) {
                        content.badges.push(badge(words, slot, tone, BadgeKind::Glyph(Icon::Star)));
                    }
                }
                10 => {
                    content.badges.push(badge(words, BadgeSlot::EndTop, BadgeTone::Accent, BadgeKind::Text("1/3".to_owned())));
                    content.badges.push(badge(words, BadgeSlot::EndTop, BadgeTone::Warn, BadgeKind::Glyph(Icon::Warning)));
                    content.badges.push(badge(words, BadgeSlot::EndTop, BadgeTone::Bad, BadgeKind::Glyph(Icon::Warning)));
                }
                11 => {
                    content.flag = Some(TileFlag::Picked);
                    content.label_color = Some(Rgba::rgb(70, 120, 220));
                }
                _ => {}
            }
        }
    }
}

/// Gradient pictures of each tile's aspect. In the States story tile 1 of 12 waits on a job and
/// tile 2 fails in one.
struct Pictures {
    mode: Mode,
}

impl ThumbSource<usize> for Pictures {
    fn revision(&self, key: &usize, _: &ThumbRequest) -> Revision {
        Revision::of(key)
    }

    fn render(&self, key: &usize, request: &ThumbRequest) -> ThumbRender {
        let id = *key;
        let aspect = aspect_of(id);
        let frame = [(aspect * 100.0) as u32, 100];
        let hue = (mix(id) % 360) as f32 / 360.0;
        let image = thumb_image(frame, request.side, |x, y| {
            let t = (x as f32 / frame[0] as f32 + y as f32 / frame[1] as f32) / 2.0;
            Hsva::new((hue + 0.25 * t).fract(), 0.55, 0.85, 1.0).to_srgba_unmultiplied()
        });
        if self.mode == Mode::States {
            match id % 12 {
                1 => return ThumbRender::Later(Box::new(move |_| Ok(LoadedPicture::Image(image)))),
                2 => return ThumbRender::Later(Box::new(|_| Err(LoadFailure::Message("Fake failure".to_owned())))),
                _ => {}
            }
        }
        ThumbRender::Ready(image)
    }
}

/// What a story keeps between passes.
struct Demo {
    photos: FakeTiles,
    selection: Selection<usize>,
    groups: Vec<TileGroup>,
    zoom: f32,
    zoom_before: f32,
    jobs: Arc<ManualJobs>,
    last: Option<String>,
}

/// Groups of `size` tiles over `count`.
fn groups(count: usize, size: usize, words: &GridStoryLabels) -> Vec<TileGroup> {
    (0..count.div_ceil(size))
        .map(|n| {
            let start = n * size;
            let len = size.min(count - start);
            TileGroup {
                start,
                count: len,
                title: craft_i18n::fmt(&words.group_title, &[("n", &(n + 1).to_string())]),
                detail: craft_i18n::fmt(&words.group_detail, &[("n", &len.to_string())]),
            }
        })
        .collect()
}

/// How a story configures the grid.
struct Setup<'a> {
    salt: &'a str,
    layout: TileLayout,
    badges: BadgeVisibility,
    rating_keys: RatingKeys,
    check_mode: bool,
    reorder: bool,
    use_groups: bool,
    height: f32,
    mode: Mode,
}

/// Draws the grid and applies its events to the demo.
fn show_grid(ui: &mut Ui, demo: &mut Demo, setup: &Setup) {
    let words = GridStoryLabels::of(ui);
    let options = ThumbnailOptions { lanes: 1, jobs: Some(demo.jobs.clone()), ..ThumbnailOptions::default() };
    let thumbs = Thumbnails::<usize>::in_context(ui.ctx(), ("craft-browse-grid-stories", setup.salt), options);
    let pictures = Pictures { mode: setup.mode };
    let Demo { photos, selection, groups, .. } = &mut *demo;
    let mut grid = TileGrid::new(setup.salt, &*photos)
        .layout(setup.layout)
        .selection(selection)
        .thumbnails(&thumbs, &pictures)
        .badges(setup.badges)
        .rating_keys(setup.rating_keys)
        .check_mode(setup.check_mode)
        .label(words.photos.clone());
    if setup.use_groups {
        grid = grid.groups(groups);
    }
    if setup.reorder {
        grid = grid.reorder(Reorder::new(Axis::Horizontal).copy_with_alt(true).right_to_left(ui.ctx().direction() == Direction::Rtl));
    }
    let events = ui
        .allocate_ui(egui::vec2(ui.available_width(), setup.height), |ui| {
            ui.set_min_height(setup.height);
            grid.show(ui).events
        })
        .inner;
    for event in events {
        match &event {
            TileEvent::Rate { keys, rating } => {
                for key in keys {
                    demo.photos.ratings.insert(*key, *rating);
                }
            }
            TileEvent::Remove(keys) => {
                demo.photos.keys.retain(|key| !keys.contains(key));
                demo.photos.changed();
            }
            TileEvent::Reorder(edit @ (EditEvent::Commit { .. } | EditEvent::Set { .. })) => {
                if let Some(mv) = edit.value() {
                    apply_move(&mut demo.photos, mv);
                }
            }
            TileEvent::Zoom(step) => demo.zoom = TileZoom::Ladder.step(demo.zoom, *step),
            _ => {}
        }
        if !matches!(event, TileEvent::Scrub { .. }) {
            demo.last = Some(format!("{event:?}"));
        }
    }
}

fn status_line(ui: &mut Ui, demo: &Demo) {
    let labels = StoryLabels::of(ui);
    let text = match &demo.last {
        Some(event) => craft_i18n::fmt(&labels.last_event, &[("event", event.as_str())]),
        None => labels.no_event.clone(),
    };
    ui.add(Text::new(text).content(Content::Muted));
}

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "patterns/tile-grid/justified",
            Level::Patterns,
            "TileGrid",
            "Justified",
            JustifiedArgs { row_height: 110.0, groups: true, badges: BadgesArg::Auto, height: 420.0 },
            show_justified
        ),
        typed_story!(
            "patterns/tile-grid/cards",
            Level::Patterns,
            "TileGrid",
            "Cards",
            CardsArgs { caption: true, check_mode: false, keys: KeysArg::Digits, height: 420.0 },
            show_cards
        ),
        typed_story!("patterns/tile-grid/states", Level::Patterns, "TileGrid", "States", StatesArgs { rtl: false, height: 420.0 }, show_states),
        typed_story!("patterns/tile-grid/reorder", Level::Patterns, "TileGrid", "Reorder", ReorderArgs { rtl: false, height: 360.0 }, show_reorder),
    ]
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum BadgesArg {
    Auto,
    Always,
    Never,
}

impl BadgesArg {
    fn visibility(self) -> BadgeVisibility {
        match self {
            BadgesArg::Auto => BadgeVisibility::Auto,
            BadgesArg::Always => BadgeVisibility::Always,
            BadgesArg::Never => BadgeVisibility::Never,
        }
    }
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum KeysArg {
    None,
    Digits,
    #[choice(name = "Command and digits")]
    CommandDigits,
}

impl KeysArg {
    fn keys(self) -> RatingKeys {
        match self {
            KeysArg::None => RatingKeys::None,
            KeysArg::Digits => RatingKeys::Digits,
            KeysArg::CommandDigits => RatingKeys::CommandDigits,
        }
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct JustifiedArgs {
    #[arg(name = "row height", range = 60.0..=240.0)]
    row_height: f32,
    /// Group headers (the first stays pinned while its group scrolls).
    groups: bool,
    /// When the badge bar shows.
    badges: BadgesArg,
    #[arg(range = 200.0..=700.0)]
    height: f32,
}

fn show_justified(ui: &mut Ui, args: &mut JustifiedArgs) {
    let words = GridStoryLabels::of(ui);
    let labels = StoryLabels::of(ui);
    let state = shared(ui, "justified", || {
        let count = 50_000;
        Demo {
            photos: photos(count, Mode::Photos, &labels, words.clone()),
            selection: Selection::new(),
            groups: groups(count, 500, &words),
            zoom: 1.0,
            zoom_before: 1.0,
            jobs: Arc::new(ManualJobs::new()),
            last: None,
        }
    });
    let mut demo = lock(&state);
    let setup = Setup {
        salt: "grid-justified",
        layout: TileLayout::Justified { row_height: args.row_height },
        badges: args.badges.visibility(),
        rating_keys: RatingKeys::CommandDigits,
        check_mode: false,
        reorder: false,
        use_groups: args.groups,
        height: args.height,
        mode: Mode::Photos,
    };
    show_grid(ui, &mut demo, &setup);
    status_line(ui, &demo);
}

#[derive(StoryArgs, Debug, PartialEq)]
struct CardsArgs {
    /// The caption under each picture.
    caption: bool,
    #[arg(name = "check mode")]
    check_mode: bool,
    /// Which keys rate the selection.
    keys: KeysArg,
    #[arg(range = 200.0..=700.0)]
    height: f32,
}

const CARD_SIDE: f32 = 120.0;

fn show_cards(ui: &mut Ui, args: &mut CardsArgs) {
    let words = GridStoryLabels::of(ui);
    let labels = StoryLabels::of(ui);
    let state = shared(ui, "cards", || Demo {
        photos: photos(240, Mode::Photos, &labels, words.clone()),
        selection: Selection::only(2),
        groups: Vec::new(),
        zoom: 1.0,
        zoom_before: 1.0,
        jobs: Arc::new(ManualJobs::new()),
        last: None,
    });
    let mut demo = lock(&state);
    let out = ThumbnailSlider::new("grid-cards-zoom", demo.zoom, TileZoom::Ladder).show(ui);
    match out.event {
        Some(EditEvent::Begin { .. }) => demo.zoom_before = demo.zoom,
        Some(EditEvent::Cancel { .. }) => demo.zoom = demo.zoom_before,
        Some(event) => {
            if let Some(value) = event.value() {
                demo.zoom = *value as f32;
            }
        }
        None => {}
    }
    let setup = Setup {
        salt: "grid-cards",
        layout: TileLayout::Cards { side: CARD_SIDE * demo.zoom, aspect: 1.0, caption: args.caption },
        badges: BadgeVisibility::Auto,
        rating_keys: args.keys.keys(),
        check_mode: args.check_mode,
        reorder: false,
        use_groups: false,
        height: args.height,
        mode: Mode::Photos,
    };
    show_grid(ui, &mut demo, &setup);
    status_line(ui, &demo);
}

#[derive(StoryArgs, Debug, PartialEq)]
struct StatesArgs {
    /// Right-to-left: Start badges move to the right.
    rtl: bool,
    #[arg(range = 200.0..=700.0)]
    height: f32,
}

fn with_direction(ui: &mut Ui, rtl: bool, add: impl FnOnce(&mut Ui)) {
    let ctx = ui.ctx().clone();
    let outer = ctx.direction();
    direction::apply(&ctx, if rtl { Direction::Rtl } else { Direction::Ltr });
    add(ui);
    direction::apply(&ctx, outer);
}

fn show_states(ui: &mut Ui, args: &mut StatesArgs) {
    let words = GridStoryLabels::of(ui);
    let labels = StoryLabels::of(ui);
    let state = shared(ui, "states", || Demo {
        photos: photos(24, Mode::States, &labels, words.clone()),
        selection: Selection::only(0),
        groups: Vec::new(),
        zoom: 1.0,
        zoom_before: 1.0,
        jobs: Arc::new(ManualJobs::new()),
        last: None,
    });
    let mut demo = lock(&state);
    ui.horizontal(|ui| {
        if ui.add(Button::new(labels.finish_next.clone()).enabled(demo.jobs.waiting() > 0)).clicked() {
            demo.jobs.run_next();
        }
        ui.add(Text::new(craft_i18n::fmt(&words.waiting, &[("n", &demo.jobs.waiting().to_string())])).content(Content::Muted));
    });
    let setup = Setup {
        salt: "grid-states",
        layout: TileLayout::Cards { side: 150.0, aspect: 1.0, caption: true },
        badges: BadgeVisibility::Auto,
        rating_keys: RatingKeys::CommandDigits,
        check_mode: false,
        reorder: false,
        use_groups: false,
        height: args.height,
        mode: Mode::States,
    };
    with_direction(ui, args.rtl, |ui| show_grid(ui, &mut demo, &setup));
    status_line(ui, &demo);
}

#[derive(StoryArgs, Debug, PartialEq)]
struct ReorderArgs {
    /// Right-to-left: the insert line and the order mirror.
    rtl: bool,
    #[arg(range = 200.0..=700.0)]
    height: f32,
}

fn show_reorder(ui: &mut Ui, args: &mut ReorderArgs) {
    let words = GridStoryLabels::of(ui);
    let labels = StoryLabels::of(ui);
    let state = shared(ui, "reorder", || Demo {
        photos: photos(24, Mode::Photos, &labels, words.clone()),
        selection: Selection::new(),
        groups: Vec::new(),
        zoom: 1.0,
        zoom_before: 1.0,
        jobs: Arc::new(ManualJobs::new()),
        last: None,
    });
    let mut demo = lock(&state);
    let setup = Setup {
        salt: "grid-reorder",
        layout: TileLayout::Cards { side: CARD_SIDE, aspect: 1.0, caption: true },
        badges: BadgeVisibility::Auto,
        rating_keys: RatingKeys::CommandDigits,
        check_mode: false,
        reorder: true,
        use_groups: false,
        height: args.height,
        mode: Mode::Photos,
    };
    with_direction(ui, args.rtl, |ui| show_grid(ui, &mut demo, &setup));
    status_line(ui, &demo);
}
