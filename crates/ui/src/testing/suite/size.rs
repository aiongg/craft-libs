//! Size cases: text size, density and scale grow the layout without cutting text; themes keep the layout; a room
//! of one point does not break it; a narrow room elides with the full text as the accessible name.

use super::{ANY, Case, Subject};
use crate::runtime::egui::widgets::{self, ScrollRegion};
use crate::runtime::env::Scale;
use crate::testing::fixture::Fixture;
use crate::testing::relations::{grows_with, inside};
use crate::testing::story::Story;
use craft_theme::{Contrast, Mode, Size, TextSize};
use egui::{Rect, Vec2};

pub(super) const CASES: &[Case] = &[
    Case::new("text-size", ANY, stories_grow_with_text_density_and_scale),
    Case::new("themes", ANY, themes_keep_the_layout),
    Case::new("tiny-room", ANY, a_tiny_room_breaks_nothing),
    Case::new("elision", ANY, elided_text_keeps_its_full_name),
];

/// A settled fixture of the story after `change`: its shown rect, painted text runs and scroll regions.
struct Shot {
    shown: Rect,
    runs: Vec<crate::runtime::egui::fixture::PaintedText>,
    scrolls: Vec<ScrollRegion>,
}

fn shot(story: &Story, change: &dyn Fn(&mut Fixture)) -> Shot {
    let mut fixture = Fixture::story(story);
    change(&mut fixture);
    fixture.settle();
    Shot { shown: fixture.shown(), runs: fixture.texts().to_vec(), scrolls: widgets::scroll_regions(fixture.ctx()) }
}

impl Shot {
    /// The text run at `run` stays in its part: it lies in the shown rect, or a scroll region's view cuts it. A run
    /// in a scroll region's content shows through the region's view, so it stays in its part when the whole run lies
    /// in the content, which scrolls, and what the view shows of it in the shown rect.
    fn keeps(&self, painted: &crate::runtime::egui::fixture::PaintedText) -> bool {
        let run = painted.rect;
        // A run cut by a part's paint clip (a clip inside the shown rect) shows only what lies in the clip.
        let cut = inside(painted.clip, self.shown) && !inside(run, painted.clip);
        inside(run, self.shown)
            || cut && inside(run.intersect(painted.clip), self.shown)
            || self
                .scrolls
                .iter()
                .filter(|region| region.content.intersects(run))
                .min_by(|a, b| a.content.area().total_cmp(&b.content.area()))
                .is_some_and(|region| {
                    let seen = run.intersect(region.view);
                    inside(run, region.content) && (!seen.is_positive() || inside(seen, self.shown))
                })
    }
}

/// Text layout at scale 2 may shave a point off a wrapped measure (68 becomes 67); real growth failures are larger.
const SCALE_LAYOUT_SLACK: f32 = 1.0;

fn stories_grow_with_text_density_and_scale(subject: &Subject) -> Result<(), String> {
    subject.stories.iter().try_for_each(text_size_case)
}

/// The suite's text-size case for one story: under a larger text size, either density and scale 2 the story grows
/// and no text run that lay in its part leaves it.
pub fn text_size_case(story: &Story) -> Result<(), String> {
    let base = shot(story, &|_| {});
    let variants: [(&str, Shot); 4] = [
        (
            "text size",
            shot(story, &|f| {
                f.text_size(TextSize::Large);
            }),
        ),
        (
            "compact density",
            shot(story, &|f| {
                f.density(Size::Compact);
            }),
        ),
        (
            "comfortable density",
            shot(story, &|f| {
                f.density(Size::Comfortable);
            }),
        ),
        (
            "scale 2",
            shot(story, &|f| {
                f.scale(Scale(2.0));
            }),
        ),
    ];
    let (before, after) = (base.shown.size(), |shot: &Shot| shot.shown.size());
    for (name, grown) in &variants {
        let size = after(grown);
        let slack = if *name == "scale 2" { SCALE_LAYOUT_SLACK } else { 0.0 };
        if !(grows_with(before.x - slack, size.x) && grows_with(before.y - slack, size.y)) {
            return Err(format!("{}: under {name} the size went from {before:?} to {size:?}", story.name));
        }
        for run in base.runs.iter().filter(|run| !run.elided && !run.text.trim().is_empty() && inside(run.rect, base.shown)) {
            for same in grown.runs.iter().filter(|other| other.text == run.text) {
                // A run the component painted elided is shortened by design and keeps its full text in the access name
                // (the elision rule checks that), so it is not cut; it must still stay inside its part.
                if !grown.keeps(same) {
                    return Err(format!("{}: under {name} the text `{}` leaves its part", story.name, run.text));
                }
            }
        }
    }
    Ok(())
}

/// The shown rect and the interactive parts' bounds do not depend on the theme: light and dark, standard and high
/// contrast lay out alike.
fn themes_keep_the_layout(subject: &Subject) -> Result<(), String> {
    use crate::testing::access::is_interactive;
    for story in &subject.stories {
        let layout = |mode: Mode, contrast: Contrast| {
            let mut fixture = Fixture::story(story);
            fixture.theme(mode, contrast);
            fixture.settle();
            let tree = fixture.tree();
            let parts: Vec<Rect> = tree.keys().into_iter().filter(|key| is_interactive(tree, *key)).filter_map(|key| tree.bounds_of(key)).collect();
            (fixture.shown(), parts)
        };
        let base = layout(Mode::Light, Contrast::Standard);
        for (mode, contrast) in [(Mode::Dark, Contrast::Standard), (Mode::Light, Contrast::High), (Mode::Dark, Contrast::High)] {
            let other = layout(mode, contrast);
            let same = |a: &Rect, b: &Rect| (a.min - b.min).abs().max_elem() < 0.5 && (a.size() - b.size()).abs().max_elem() < 0.5;
            if !same(&base.0, &other.0) || base.1.len() != other.1.len() || !base.1.iter().zip(&other.1).all(|(a, b)| same(a, b)) {
                return Err(format!("{}: {mode:?} {contrast:?} lays out differently from light standard", story.name));
            }
        }
    }
    Ok(())
}

/// A room of one point and a room of half the settled width show without a panic, and everything the access tree
/// reports has finite bounds.
fn a_tiny_room_breaks_nothing(subject: &Subject) -> Result<(), String> {
    for story in &subject.stories {
        let width = shot(story, &|_| {}).shown.width();
        for room in [Vec2::splat(1.0), Vec2::new((width / 2.0).max(1.0), 300.0)] {
            let mut fixture = Fixture::story(story);
            fixture.room(room);
            fixture.settle();
            let tree = fixture.tree();
            if tree.keys().into_iter().filter_map(|key| tree.bounds_of(key)).any(|bounds| !(bounds.min.is_finite() && bounds.max.is_finite())) {
                return Err(format!("{}: a node has non-finite bounds in a room of {room:?}", story.name));
            }
        }
    }
    Ok(())
}

/// In a room narrower than the story, every text run that the layout cut has its full text in an accessible name or
/// description: a cut label is the node's name, a cut description its description.
fn elided_text_keeps_its_full_name(subject: &Subject) -> Result<(), String> {
    for story in &subject.stories {
        let width = shot(story, &|_| {}).shown.width();
        if width < 40.0 {
            continue;
        }
        let mut fixture = Fixture::story(story);
        fixture.room(Vec2::new(width / 3.0, 300.0));
        fixture.settle();
        let tree = fixture.tree();
        let read: Vec<&str> = tree.keys().into_iter().flat_map(|key| [tree.name_of(key), tree.description_of(key)]).flatten().collect();
        for run in fixture.texts().iter().filter(|run| run.elided) {
            if !read.iter().any(|text| text.contains(run.text.trim())) {
                return Err(format!("{}: the elided text `{}` is in no accessible name or description", story.name, run.text));
            }
        }
    }
    Ok(())
}
