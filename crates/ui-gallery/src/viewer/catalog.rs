//! What the viewer lists: every registered story, grouped by the component it shows.

use craft_ui::contract::{self, Level, Probe, Registration};
use craft_ui::testing::{StoryRegistration, stories};
use std::collections::HashMap;
use std::sync::OnceLock;

/// One registered story with the facts the viewer lists.
#[derive(Clone, Copy)]
#[non_exhaustive]
pub struct Entry {
    /// The registration.
    pub registration: &'static StoryRegistration,
    /// The registered name of the type the story shows.
    pub type_name: &'static str,
}

/// The facts of one component the tree lists and the filter reads.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ComponentInfo {
    /// The registered type name.
    pub type_name: &'static str,
    /// The level the tree lists the component under.
    pub level: Level,
    /// The component's plain name.
    pub name: String,
    /// The names of the component's stories.
    pub stories: Vec<&'static str>,
    /// The display titles of the stories, in the order of `stories`.
    pub titles: Vec<String>,
    /// The name and the titles in lower case, for the filter.
    folded: Vec<String>,
    /// The registry's level of the type and its display name in lower case; `None` when the registry does not list it.
    listed: Option<(Level, String)>,
}

impl ComponentInfo {
    /// A component with its stories, each a name and a display title.
    pub fn new(type_name: &'static str, level: Level, name: impl Into<String>, stories: &[(&'static str, &str)]) -> Self {
        let name = name.into();
        let listed = linked_catalog().entry(type_name).map(|entry| (entry.level, entry.name.to_lowercase()));
        let mut component =
            ComponentInfo { type_name, level, folded: vec![name.to_lowercase()], name, stories: Vec::new(), titles: Vec::new(), listed };
        for (story, title) in stories {
            component.add_story(story, (*title).to_owned());
        }
        component
    }

    /// Adds the story `story` with its display title.
    pub fn add_story(&mut self, story: &'static str, title: String) {
        self.stories.push(story);
        self.folded.push(title.to_lowercase());
        self.titles.push(title);
    }

    /// Whether the name or one of the story titles contains the filter, ignoring case; `folded` is the filter as
    /// [`fold_filter`] gives it. An empty filter matches.
    pub fn matches(&self, folded: &str) -> bool {
        folded.is_empty() || self.folded.iter().any(|text| text.contains(folded))
    }

    /// The registry's level of the type and its display name in lower case, the Overview's order within the level.
    pub(crate) fn listed(&self) -> Option<(Level, &str)> {
        self.listed.as_ref().map(|(level, name)| (*level, name.as_str()))
    }
}

/// The filter as [`ComponentInfo::matches`] compares it: trimmed and in lower case.
pub fn fold_filter(filter: &str) -> String {
    filter.trim().to_lowercase()
}

/// The registry's catalog of the types linked into the running binary: each one's level and plain name.
pub(crate) fn linked_catalog() -> &'static contract::Catalog {
    static CATALOG: OnceLock<contract::Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| contract::catalog(&contract::registrations()))
}

/// The plain name of the component registered as `type_name`; the registered name when the registry lists no such type.
pub fn display_name(type_name: &str) -> String {
    linked_catalog().entry(type_name).map_or_else(|| type_name.to_owned(), |entry| entry.name.clone())
}

/// Every story linked into the viewer, ordered by type then story name.
pub fn entries() -> Vec<Entry> {
    stories().into_iter().map(|registration| Entry { registration, type_name: (registration.type_name)() }).collect()
}

/// What a registration says about a type: where it is declared and what it is for.
#[derive(Clone, Copy)]
#[non_exhaustive]
pub struct Declared {
    /// The module path of the declaration.
    pub module: &'static str,
    /// The first sentence of the type's doc comment.
    pub purpose: &'static str,
    /// The component probe; `None` for a pattern.
    pub probe: Option<&'static Probe>,
}

/// The registration of each registered component and pattern type, by the type's registered name.
pub fn declared() -> HashMap<&'static str, Declared> {
    let mut out = HashMap::new();
    for registration in contract::registrations() {
        match registration {
            Registration::Component(probe) => {
                out.insert(probe.type_name(), Declared { module: probe.module, purpose: probe.purpose, probe: Some(probe) });
            }
            Registration::Pattern(probe) => {
                out.insert(probe.type_name(), Declared { module: probe.module, purpose: probe.purpose, probe: None });
            }
            _ => {}
        }
    }
    out
}

/// The components of `entries` that the registry lists, in the order of their first story.
pub fn components(entries: &[Entry]) -> Vec<ComponentInfo> {
    let mut out: Vec<ComponentInfo> = Vec::new();
    for entry in entries {
        let Some(listed) = linked_catalog().entry(entry.type_name) else { continue };
        let story = entry.registration.story.name;
        let title = contract::story_title(&listed.name, story);
        match out.iter_mut().find(|component| component.type_name == entry.type_name) {
            Some(component) => component.add_story(story, title),
            None => out.push(ComponentInfo::new(entry.type_name, listed.level, listed.name.clone(), &[(story, &title)])),
        }
    }
    out
}

/// Every registered story, the components they show and what the registrations say about each type.
#[non_exhaustive]
pub struct Catalog {
    /// The stories, ordered by type then story name.
    pub entries: Vec<Entry>,
    /// The components of the stories.
    pub components: Vec<ComponentInfo>,
    /// The registration of each type.
    pub declared: HashMap<&'static str, Declared>,
}

impl Catalog {
    /// The catalog of the stories linked into the running binary.
    pub fn linked() -> Self {
        let entries = entries();
        let declared = declared();
        let components = components(&entries);
        Catalog { entries, components, declared }
    }

    /// A catalog of `components` with no stories to show.
    pub fn listing(components: Vec<ComponentInfo>) -> Self {
        Catalog { entries: Vec::new(), components, declared: HashMap::new() }
    }

    /// The component registered as `type_name`.
    pub fn component(&self, type_name: &str) -> Option<&ComponentInfo> {
        self.components.iter().find(|component| component.type_name == type_name)
    }
}
