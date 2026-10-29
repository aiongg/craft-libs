use std::collections::HashSet;

use egui::{Context, RawInput};

use super::colours::GROUPS;
use super::*;

#[test]
fn ids_are_unique_and_follow_the_level_component_story_form() {
    let stories = registry();
    assert!(!stories.is_empty());
    let mut seen = HashSet::new();
    for story in &stories {
        assert!(seen.insert(story.id), "duplicate id {}", story.id);
        let parts: Vec<&str> = story.id.split('/').collect();
        assert_eq!(parts.len(), 3, "{} is not level/component/story", story.id);
        assert_eq!(parts[0], story.level.slug(), "{} does not start with its level", story.id);
        for part in parts {
            assert!(
                !part.is_empty() && part.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "bad segment in {}",
                story.id
            );
        }
    }
}

#[test]
fn every_story_shows_with_its_default_args() {
    let ctx = Context::default();
    for story in registry() {
        let mut args = story.defaults();
        ctx.run_ui(RawInput::default(), |ui| (story.show)(ui, &mut args)).drop_without_applying_deltas();
    }
}

#[test]
fn colour_groups_cover_every_token() {
    for token in craft_theme::ColorToken::ALL {
        let section = token.path().split('.').next().unwrap_or_default();
        assert!(GROUPS.contains(&section), "no group for {}", token.path());
    }
}

#[test]
#[should_panic(expected = "foundations/colours/semantic")]
fn a_misspelt_arg_panics_with_the_story_id() {
    let story = registry().into_iter().find(|s| s.id == "foundations/colours/semantic").expect("story");
    story.defaults().bool("grup");
}

#[test]
fn every_colour_group_renders() {
    let story = registry().into_iter().find(|s| s.id == "foundations/colours/semantic").expect("story");
    let ctx = Context::default();
    for index in 0..GROUPS.len() {
        let mut args = Args::new().with_choice("group", GROUPS, index).for_story(story.id);
        ctx.run_ui(RawInput::default(), |ui| (story.show)(ui, &mut args)).drop_without_applying_deltas();
    }
}

#[test]
fn every_declared_component_has_a_story() {
    let stories = registry();
    let missing: Vec<&str> = components().filter(|c| !stories.iter().any(|s| s.component == *c)).collect();
    assert!(missing.is_empty(), "components without a story in registry(): {missing:?}");

    // Every module declaring `COMPONENTS` must be in `DECLARED`, or its list is never checked.
    fn count(dir: &std::path::Path) -> usize {
        std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .map(|path| match path.is_dir() {
                // Compiled only with the feature, like its entry in `DECLARED`.
                true if path.ends_with("title_bar") && cfg!(not(feature = "window-frame")) => 0,
                true => count(&path),
                false if path.extension().is_some_and(|e| e == "rs") => {
                    std::fs::read_to_string(&path).unwrap().lines().filter(|l| l.starts_with("pub const COMPONENTS:")).count()
                }
                false => 0,
            })
            .sum()
    }
    let files = count(std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src")));
    assert_eq!(files, DECLARED.len(), "a module declares COMPONENTS but is missing from story::DECLARED");
}

#[test]
fn a_story_sits_on_a_panel_unless_set() {
    let story = Story::new("controls/x/y", Level::Controls, "X", "Y", Args::new, |_, _| {});
    assert_eq!(story.surface, Surface::Panel);
    assert_eq!(story.on(Surface::Chrome).surface, Surface::Chrome);
    let surface_of = |component: &str| registry().into_iter().find(|s| s.component == component).map(|s| s.surface);
    assert_eq!(surface_of("Button"), Some(Surface::Panel));
    assert_eq!(surface_of("Rulers"), Some(Surface::Canvas));
    assert_eq!(surface_of("OptionsBar"), Some(Surface::Chrome));
}
