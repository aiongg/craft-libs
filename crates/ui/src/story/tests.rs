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
