//! The stories the gallery shows and how the Components list orders them.
use craft_ui::story::{Level, Story};

use crate::faces;

/// Every story the gallery shows: the facade's (the kernel's and every enabled crate's) and the gallery's own.
pub(crate) fn stories() -> Vec<Story> {
    let mut stories = craft::story::all();
    stories.extend(faces::stories());
    stories.extend(crate::preview::stories());
    stories
}

/// The story the Overview draws and the Components panel opens for `component`: its first story,
/// which shows the component at rest.
pub(crate) fn default_story_of<'a>(stories: &'a [Story], component: &str) -> Option<&'a Story> {
    stories.iter().find(|s| s.component == component)
}

/// The navigation's levels in their order, each with the components that have a story matching
/// `filter` (lowercase), sorted by name ignoring case, whatever the registration order.
pub(crate) fn nav(stories: &[Story], filter: &str) -> Vec<(Level, Vec<&'static str>)> {
    Level::ALL
        .into_iter()
        .filter_map(|level| {
            let mut components: Vec<&'static str> = Vec::new();
            for story in stories.iter().filter(|s| s.level == level && matches(s, filter)) {
                if !components.contains(&story.component) {
                    components.push(story.component);
                }
            }
            components.sort_by_cached_key(|c| c.to_lowercase());
            (!components.is_empty()).then_some((level, components))
        })
        .collect()
}

fn matches(story: &Story, filter: &str) -> bool {
    filter.is_empty() || story.id.contains(filter) || story.component.to_lowercase().contains(filter) || story.name.to_lowercase().contains(filter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nav_lists_each_levels_components_alphabetically() {
        let mut stories = craft::story::all();
        stories.extend(faces::stories());
        let nav = nav(&stories, "");
        assert!(!nav.is_empty());
        for (level, components) in &nav {
            let keys: Vec<String> = components.iter().map(|c| c.to_lowercase()).collect();
            assert!(keys.is_sorted(), "{level:?}: {components:?}");
        }
        let levels: Vec<Level> = nav.iter().map(|(level, _)| *level).collect();
        let order: Vec<Level> = Level::ALL.into_iter().filter(|l| levels.contains(l)).collect();
        assert_eq!(levels, order);
    }

    #[test]
    fn story_ids_are_unique_across_crates_and_the_crates_follow_the_level_component_story_form() {
        let mut seen = std::collections::HashSet::new();
        for story in stories() {
            assert!(seen.insert(story.id), "duplicate id {}", story.id);
        }
        let crates = craft::story::all();
        assert!(!crates.is_empty());
        for story in &crates {
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
}
