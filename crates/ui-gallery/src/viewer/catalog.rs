//! What the viewer lists: every registered story, grouped by the component it shows.

use craft_ui::contract::{Probe, Registration};
use craft_ui::testing::{StoryRegistration, registered, stories};
use std::collections::HashMap;

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
    /// The module that declares the type; empty when the type has no probe.
    pub module: &'static str,
    /// The names of the component's stories.
    pub stories: Vec<&'static str>,
}

impl ComponentInfo {
    /// A component with its stories.
    pub fn new(type_name: &'static str, module: &'static str, stories: Vec<&'static str>) -> Self {
        ComponentInfo { type_name, module, stories }
    }

    /// The type's name without its module path, type arguments kept.
    pub fn short_name(&self) -> String {
        short_type(self.type_name)
    }

    /// Whether the type, its module or one of its stories contains `filter`, ignoring case. An empty filter matches.
    pub fn matches(&self, filter: &str) -> bool {
        let filter = filter.trim().to_lowercase();
        filter.is_empty() || [self.type_name, self.module].iter().chain(&self.stories).any(|text| text.to_lowercase().contains(&filter))
    }
}

/// The type's name without its module path, type arguments kept.
pub fn short_type(type_name: &str) -> String {
    let (path, arguments) = type_name.split_once('<').map_or((type_name, ""), |(path, rest)| (path, rest));
    let name = path.rsplit("::").next().unwrap_or(path);
    if arguments.is_empty() { name.to_owned() } else { format!("{name}<{arguments}") }
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
    for registration in registered() {
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

/// The components of `entries`, in the order of their first story.
pub fn components(entries: &[Entry], declared: &HashMap<&'static str, Declared>) -> Vec<ComponentInfo> {
    let mut out: Vec<ComponentInfo> = Vec::new();
    for entry in entries {
        let story = entry.registration.story.name;
        match out.iter_mut().find(|component| component.type_name == entry.type_name) {
            Some(component) => component.stories.push(story),
            None => {
                let module = declared.get(entry.type_name).map_or("", |declared| declared.module);
                out.push(ComponentInfo::new(entry.type_name, module, vec![story]));
            }
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
        let components = components(&entries, &declared);
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
