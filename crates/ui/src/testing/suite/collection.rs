//! The collection rows of the 22.4 table: navigation keys, selection, type-ahead, rename, reorder, the empty state,
//! virtualisation and scrolling. Each case runs over the stories of a type of the `Items` shape and reads the rows
//! from the AccessKit tree and the events from `StoryOutput`.
//!
//! The collection's own settings (selection mode, `rename`, reorder rule) are not in the probe or the output, so a
//! case that applies only to a collection with a setting starts the gesture and checks the rest when the collection
//! answered it: a rename case runs on the stories whose F2 began a rename.

use super::Subject;
use super::nodes::{focused_row, position_in_set, root, rows};
use crate::contract::ShapeKind;
use crate::runtime::access::Role;
use crate::testing::fixture::{Fixture, Target};
use crate::testing::story::{CollectionKind, EditKind, Story};
use craft_behavior::MotionMode;
use craft_host::target::Os;
use egui::{Key, Modifiers};

fn is_items(subject: &Subject) -> bool {
    matches!(subject.probe.shape, ShapeKind::Items)
}

/// A settled fixture of `story`, and the rows it shows when there are at least `least` of them.
fn with_rows(story: &Story, least: usize) -> Option<(Fixture, Target, Vec<Target>)> {
    let mut fixture = Fixture::story(story);
    fixture.motion(MotionMode::Off);
    fixture.os(Os::Linux);
    fixture.settle();
    let container = root(&fixture)?;
    let shown = rows(&fixture, container.node);
    (shown.len() >= least).then_some((fixture, container, shown))
}

/// The collection events of every pass since the last call.
fn events(fixture: &mut Fixture) -> Vec<CollectionKind> {
    fixture.take_outputs().into_iter().flat_map(|output| output.collection).collect()
}

/// The place of the row holding the focus, in its set.
fn lead(fixture: &Fixture, container: &Target) -> Option<usize> {
    let shown = rows(fixture, container.node);
    let (_, key) = focused_row(fixture, &shown)?;
    position_in_set(fixture, key).map(|(at, _)| at)
}

/// A click on the first row puts the lead there; the keys then move it: Down and Up by one row, End to the last,
/// Home to the first, Page Down forward.
pub(super) fn keys_move_the_lead(subject: &Subject) -> Result<(), String> {
    if !is_items(subject) {
        return Ok(());
    }
    for story in &subject.stories {
        let Some((mut fixture, container, shown)) = with_rows(story, 3) else { continue };
        let Some((_, size)) = position_in_set(&fixture, shown[0].node) else { continue };
        fixture.click(&shown[0]);
        let name = story.name;
        let expect = |fixture: &mut Fixture, key: Key, at: usize, what: &str| -> Result<(), String> {
            fixture.key(key, Modifiers::NONE);
            fixture.idle(2);
            match lead(fixture, &container) {
                Some(found) if found == at => Ok(()),
                found => Err(format!("{name}: after {what} the lead is {found:?}, expected {at}")),
            }
        };
        let first = lead(&fixture, &container).ok_or_else(|| format!("{name}: a click on the first row left no lead"))?;
        expect(&mut fixture, Key::ArrowDown, first + 1, "Down")?;
        expect(&mut fixture, Key::ArrowUp, first, "Up")?;
        expect(&mut fixture, Key::End, size - 1, "End")?;
        expect(&mut fixture, Key::Home, 0, "Home")?;
        fixture.key(Key::PageDown, Modifiers::NONE);
        fixture.idle(2);
        if lead(&fixture, &container).is_none_or(|found| found == 0) {
            return Err(format!("{name}: Page Down left the lead on the first row"));
        }
    }
    Ok(())
}

/// A click on a row that selects reports one selection event and shows that row selected.
pub(super) fn click_selects_one_row(subject: &Subject) -> Result<(), String> {
    if !is_items(subject) {
        return Ok(());
    }
    for story in &subject.stories {
        let Some((mut fixture, container, shown)) = with_rows(story, 2) else { continue };
        fixture.take_outputs();
        fixture.click(&shown[1]);
        let kinds = events(&mut fixture);
        let selections = kinds.iter().filter(|kind| **kind == CollectionKind::Selection).count();
        if selections == 0 {
            continue;
        }
        if selections != 1 {
            return Err(format!("{}: one click gave {selections} selection events", story.name));
        }
        let now = rows(&fixture, container.node);
        if !now.iter().any(|row| row.node == shown[1].node && super::nodes::is_selected(&fixture, row.node)) {
            return Err(format!("{}: the clicked row does not report itself selected", story.name));
        }
    }
    Ok(())
}

/// Typing the first letter of a later row's name moves the lead to the first row that starts with it.
pub(super) fn type_ahead_moves_the_lead(subject: &Subject) -> Result<(), String> {
    if !is_items(subject) {
        return Ok(());
    }
    for story in &subject.stories {
        let Some((mut fixture, container, shown)) = with_rows(story, 3) else { continue };
        let names: Vec<Option<char>> = shown.iter().map(|row| fixture.tree().name_of(row.node).and_then(|name| name.chars().next())).collect();
        let Some(first) = names[0] else { continue };
        let Some(target) = (1..shown.len()).find(|at| names[*at].is_some_and(|letter| !letter.to_lowercase().eq(first.to_lowercase()))) else {
            continue;
        };
        let Some(letter) = names[target] else { continue };
        fixture.click(&shown[0]);
        fixture.type_text(&letter.to_string());
        fixture.idle(2);
        let Some((_, size)) = position_in_set(&fixture, shown[0].node) else { continue };
        let found = lead(&fixture, &container);
        if found != Some(target) {
            return Err(format!("{}: typing `{letter}` left the lead at {found:?} of {size}, expected row {target}", story.name));
        }
    }
    Ok(())
}

/// F2 on the lead, for a collection that renames: the editor takes focus, Enter gives one commit, and Escape gives
/// one cancel and returns focus to a row.
pub(super) fn rename_commits_and_cancels_once(subject: &Subject) -> Result<(), String> {
    if !is_items(subject) {
        return Ok(());
    }
    let rename = |kinds: &[CollectionKind], edit: EditKind| kinds.iter().filter(|kind| **kind == CollectionKind::Rename(edit)).count();
    for story in &subject.stories {
        let Some((mut fixture, container, shown)) = with_rows(story, 1) else { continue };
        fixture.click(&shown[0]);
        fixture.take_outputs();
        fixture.key(Key::F2, Modifiers::NONE);
        fixture.idle(2);
        let began = events(&mut fixture);
        if rename(&began, EditKind::Begin) == 0 {
            continue;
        }
        let name = story.name;
        if fixture.tree().role_of(fixture.tree().focused_key()) != Some(Role::TEXT_INPUT) {
            return Err(format!("{name}: F2 began a rename and the editor does not hold the focus"));
        }
        fixture.key(Key::Enter, Modifiers::NONE);
        fixture.idle(2);
        let ended = events(&mut fixture);
        if rename(&ended, EditKind::Commit) != 1 || rename(&ended, EditKind::Cancel) != 0 {
            return Err(format!("{name}: Enter gave {ended:?}, expected one rename commit"));
        }
        fixture.key(Key::F2, Modifiers::NONE);
        fixture.idle(2);
        events(&mut fixture);
        fixture.key(Key::Escape, Modifiers::NONE);
        fixture.idle(2);
        let cancelled = events(&mut fixture);
        if rename(&cancelled, EditKind::Cancel) != 1 || rename(&cancelled, EditKind::Commit) != 0 {
            return Err(format!("{name}: Escape gave {cancelled:?}, expected one rename cancel"));
        }
        if lead(&fixture, &container).is_none() {
            return Err(format!("{name}: Escape left the focus off the rows"));
        }
    }
    Ok(())
}

/// Alt+Down on the lead, for a reorderable collection: one end event and no cancel.
pub(super) fn keyboard_reorder_ends_once(subject: &Subject) -> Result<(), String> {
    if !is_items(subject) {
        return Ok(());
    }
    for story in &subject.stories {
        let Some((mut fixture, _, shown)) = with_rows(story, 2) else { continue };
        fixture.click(&shown[0]);
        fixture.take_outputs();
        fixture.key(Key::ArrowDown, Modifiers::ALT);
        fixture.idle(2);
        let kinds = events(&mut fixture);
        let reorders: Vec<EditKind> = kinds
            .iter()
            .filter_map(|kind| match kind {
                CollectionKind::Reorder(edit) => Some(*edit),
                _ => None,
            })
            .collect();
        if reorders.is_empty() {
            continue;
        }
        let ends = reorders.iter().filter(|edit| matches!(edit, EditKind::Commit | EditKind::Set)).count();
        if ends != 1 || reorders.contains(&EditKind::Cancel) {
            return Err(format!("{}: Alt+Down gave the reorder events {reorders:?}", story.name));
        }
    }
    Ok(())
}

/// An empty collection shows no rows and reports an empty visible range.
pub(super) fn empty_collection_shows_no_rows(subject: &Subject) -> Result<(), String> {
    if !is_items(subject) {
        return Ok(());
    }
    for story in subject.stories.iter().filter(|story| story.covers.contains(&crate::testing::story::Required::Empty)) {
        let mut fixture = Fixture::story(story);
        fixture.settle();
        let Some(container) = root(&fixture) else { continue };
        let shown = rows(&fixture, container.node);
        if !shown.is_empty() {
            return Err(format!("{}: an empty collection shows {} rows", story.name, shown.len()));
        }
        if !(fixture.shown().height() > 0.0) {
            return Err(format!("{}: an empty collection allocates no height", story.name));
        }
        let visible = fixture.take_outputs().into_iter().last().and_then(|output| output.visible);
        if visible.is_some_and(|range| !range.is_empty()) {
            return Err(format!("{}: an empty collection reports visible rows", story.name));
        }
    }
    Ok(())
}

/// A collection with more rows than its room holds builds fewer rows than it has.
pub(super) fn long_collection_builds_only_visible_rows(subject: &Subject) -> Result<(), String> {
    if !is_items(subject) {
        return Ok(());
    }
    for story in &subject.stories {
        let Some((fixture, container, shown)) = with_rows(story, 1) else { continue };
        let Some((_, size)) = position_in_set(&fixture, shown[0].node) else { continue };
        let row = shown[0].rect.height();
        if !(row > 0.0) || (size as f32) * row <= 2.0 * container.rect.height() {
            continue;
        }
        if shown.len() >= size {
            return Err(format!(
                "{}: all {size} rows are built though the room holds about {}",
                story.name,
                (container.rect.height() / row) as usize
            ));
        }
    }
    Ok(())
}

/// One wheel step moves the rows and the frame it happens in shows rows over the whole viewport.
pub(super) fn scroll_shows_rows_in_the_same_frame(subject: &Subject) -> Result<(), String> {
    if !is_items(subject) {
        return Ok(());
    }
    for story in &subject.stories {
        let Some((mut fixture, container, shown)) = with_rows(story, 3) else { continue };
        let Some((_, size)) = position_in_set(&fixture, shown[0].node) else { continue };
        if shown.len() >= size {
            continue;
        }
        fixture.frame(vec![crate::testing::pointer::moved(container.rect.center())]);
        fixture.frame(vec![crate::testing::pointer::wheel(-3.0 * shown[0].rect.height())]);
        let now = rows(&fixture, container.node);
        let (Some(top), Some(bottom)) = (now.first(), now.last()) else {
            return Err(format!("{}: no row is shown in the frame of the scroll", story.name));
        };
        let row = top.rect.height();
        let last_row = position_in_set(&fixture, bottom.node).is_some_and(|(at, count)| at + 1 == count);
        if top.rect.min.y > container.rect.min.y + row || (!last_row && bottom.rect.max.y < container.rect.max.y - row) {
            return Err(format!("{}: the frame of the scroll leaves part of the viewport without rows", story.name));
        }
        if now.windows(2).any(|pair| pair[1].rect.min.y - pair[0].rect.max.y > row) {
            return Err(format!("{}: the frame of the scroll shows rows with a gap between them", story.name));
        }
    }
    Ok(())
}
