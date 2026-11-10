//! The catalog of registered types: the level of each, its plain display name, the surface its stories sit on, and
//! the display title of a story. A viewer lists "Controls › Button › Loading" from these without string surgery.
//!
//! The level comes from one place. A type declared in craft-ui takes the first row of [`LIBRARY`] whose module
//! prefix holds its module; a type of a family crate takes its level from what it registers as: a pattern is a
//! Pattern, a leaf component a Control, a component with a body or items a Composite. A craft-ui type no row
//! matches has no level, and the `catalog_has_no_unleveled_type` trial fails.

use super::registry::{PatternKind, Registration};
use super::{Probe, ShapeKind};
use std::collections::BTreeMap;

/// Where a type sits in the design system, in the order a viewer lists them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Level {
    /// Colours, the edit lifecycle, stock egui.
    Foundations,
    /// Surface, divider, stack, text, icon, canvas overlays.
    Primitives,
    /// Buttons, fields, sliders, selects, checkboxes.
    Controls,
    /// Field rows, lists, tabs, menus, toasts.
    Composites,
    /// Panels, dialogs, rails, bars, dock areas.
    Patterns,
    /// App shells and start screens.
    Shells,
    /// Icons and type faces.
    Specimens,
}

impl Level {
    /// Every level, in listing order.
    pub const ALL: [Level; 7] =
        [Level::Foundations, Level::Primitives, Level::Controls, Level::Composites, Level::Patterns, Level::Shells, Level::Specimens];

    /// The level's name for display.
    pub const fn title(self) -> &'static str {
        match self {
            Level::Foundations => "Foundations",
            Level::Primitives => "Primitives",
            Level::Controls => "Controls",
            Level::Composites => "Composites",
            Level::Patterns => "Patterns",
            Level::Shells => "Shells",
            Level::Specimens => "Specimens",
        }
    }
}

/// The surface a type's stories sit on in an app; a viewer draws a story on it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Surface {
    /// A dock panel (`surface.panel`): most types.
    #[default]
    Panel,
    /// The document area (`surface.canvas`): types drawn over the document.
    Canvas,
    /// The window chrome (`surface.chrome`): option bars, rails, title bars.
    Chrome,
    /// The type paints its own panel, dock, dialog or shell, so the viewer adds no surface round it.
    Own,
}

use Level::{Composites, Controls, Patterns, Primitives, Shells};
use Surface::{Canvas, Chrome, Own, Panel};

/// Where the craft-ui modules sit: a module prefix, its level and surface. The first row that holds a module wins,
/// so a row for a module inside a prefix comes before the prefix.
const LIBRARY: &[(&str, Level, Surface)] = &[
    ("bars::breadcrumb", Controls, Panel),
    ("bars", Patterns, Chrome),
    ("buttons", Controls, Panel),
    ("canvas", Primitives, Canvas),
    ("canvas_controls", Primitives, Canvas),
    ("choice", Controls, Panel),
    ("contract", Primitives, Panel),
    ("controls", Controls, Panel),
    ("dialog", Patterns, Own),
    ("display::badge", Controls, Panel),
    ("display::kbd", Controls, Panel),
    ("display::progress", Controls, Panel),
    ("display::spinner", Controls, Panel),
    ("display", Primitives, Panel),
    ("docking", Patterns, Own),
    ("form", Composites, Panel),
    ("graph", Composites, Panel),
    ("layout_components", Primitives, Panel),
    ("list", Composites, Panel),
    ("menu", Composites, Panel),
    ("notices", Composites, Panel),
    ("patterns::dock", Patterns, Own),
    ("patterns::form_dialogs", Patterns, Own),
    ("patterns::presets", Composites, Panel),
    ("patterns::preferences", Patterns, Own),
    ("patterns", Patterns, Panel),
    ("pickers", Controls, Panel),
    ("popover", Patterns, Panel),
    ("rails", Patterns, Chrome),
    ("sections::panel", Patterns, Own),
    ("sections", Composites, Panel),
    ("shell::title_bar", Patterns, Chrome),
    ("shell", Shells, Own),
    ("table", Composites, Panel),
    ("tabs", Composites, Panel),
    ("text", Controls, Panel),
    ("tile_grid", Composites, Panel),
    ("value_drag", Controls, Panel),
];

/// Whether `module` is `prefix` or inside it.
fn within(module: &str, prefix: &str) -> bool {
    module.strip_prefix(prefix).is_some_and(|rest| rest.is_empty() || rest.starts_with("::"))
}

/// The registration's module path, its type name and, for a component, its probe.
fn parts(registration: &Registration) -> (&'static str, &'static str, Option<&Probe>) {
    match registration {
        Registration::Component(probe) => (probe.module, probe.type_name(), Some(probe)),
        Registration::Pattern(probe) => (probe.module, probe.type_name(), None),
    }
}

/// The level and surface of `registration`; `None` for a craft-ui type that no row of [`LIBRARY`] holds.
fn classify(registration: &Registration) -> Option<(Level, Surface)> {
    let (module, _, probe) = parts(registration);
    if let Some(inside) = module.strip_prefix("craft_ui::") {
        return LIBRARY.iter().find(|(prefix, ..)| within(inside, prefix)).map(|&(_, level, surface)| (level, surface));
    }
    let surface = match registration {
        Registration::Pattern(pattern) => match pattern.kind {
            PatternKind::Tool => Canvas,
            PatternKind::Dialog => Own,
            _ => Panel,
        },
        Registration::Component(probe) if probe.canvas_control => Canvas,
        Registration::Component(_) => Panel,
    };
    let level = match probe {
        None => Patterns,
        Some(probe) if probe.shape == ShapeKind::Leaf => Controls,
        Some(_) => Composites,
    };
    Some((level, surface))
}

/// The type's path without its type arguments.
fn base_path(type_name: &str) -> &str {
    type_name.split('<').next().unwrap_or(type_name)
}

fn pascal(segment: &str) -> String {
    segment.split('_').filter(|word| !word.is_empty()).map(|word| word[..1].to_uppercase() + &word[1..]).collect()
}

/// One registered type as a viewer lists it.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct CatalogEntry {
    /// The registration (the first of a generic type's instantiations).
    pub registration: &'static Registration,
    /// The type's level.
    pub level: Level,
    /// The surface its stories sit on.
    pub surface: Surface,
    /// The type's name with its type arguments, as the registry spells it; the key a story names its type by.
    pub type_name: &'static str,
    /// The type's display name: a PascalCase identifier, unique in the catalog.
    pub name: String,
}

/// Every registered type that has a level, in listing order (level, then name ignoring case), and the types that
/// have none.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct Catalog {
    /// The types with a level.
    pub entries: Vec<CatalogEntry>,
    /// The type names no level was found for.
    pub unleveled: Vec<&'static str>,
}

impl Catalog {
    /// The entry of the type whose registered name is `type_name`.
    pub fn entry(&self, type_name: &str) -> Option<&CatalogEntry> {
        self.entries.iter().find(|entry| base_path(entry.type_name) == base_path(type_name))
    }
}

/// Modules whose registrations are kit stand-ins, not components.
pub(crate) const KIT_MODULES: [&str; 1] = ["craft_ui::testing::layout"];

/// The catalog of `registrations`, without the kit stand-ins of [`KIT_MODULES`].
///
/// The display name is the type's name without module path or type arguments. Two registered types that share it
/// each take the last module segments of their paths in front of it (`DockButton`, then `DockGroupButton`)
/// until the names differ.
pub fn catalog(registrations: &[&'static Registration]) -> Catalog {
    let mut types: BTreeMap<&'static str, &'static Registration> = BTreeMap::new();
    for &registration in registrations.iter().filter(|registration| !KIT_MODULES.contains(&parts(registration).0)) {
        types.entry(base_path(parts(registration).1)).or_insert(registration);
    }
    let mut by_short: BTreeMap<&str, Vec<&'static str>> = BTreeMap::new();
    for &path in types.keys() {
        by_short.entry(path.rsplit("::").next().unwrap_or(path)).or_default().push(path);
    }
    let mut out = Catalog::default();
    for (short, paths) in by_short {
        for &path in &paths {
            let registration = types[path];
            let name = qualified(path, short, &paths);
            match classify(registration) {
                Some((level, surface)) => {
                    let type_name = parts(registration).1;
                    out.entries.push(CatalogEntry { registration, level, surface, type_name, name });
                }
                None => out.unleveled.push(parts(registration).1),
            }
        }
    }
    out.entries.sort_by_key(|entry| (entry.level, entry.name.to_lowercase()));
    out
}

/// The display name of the type at `path` among the types `paths` that share its short name `short`.
fn qualified(path: &str, short: &str, paths: &[&str]) -> String {
    if paths.len() == 1 {
        return short.to_owned();
    }
    let segments: Vec<&str> = path.split("::").collect();
    let modules = &segments[..segments.len() - 1];
    let others: Vec<Vec<&str>> = paths.iter().filter(|other| **other != path).map(|other| other.split("::").collect::<Vec<_>>()).collect();
    for depth in 1..=modules.len() {
        let prefix: String = modules[modules.len() - depth..].iter().map(|segment| pascal(segment)).collect();
        let clash = others.iter().any(|other| {
            let theirs = &other[..other.len() - 1];
            theirs.len() >= depth && theirs[theirs.len() - depth..].iter().map(|segment| pascal(segment)).collect::<String>() == prefix
        });
        if !clash {
            return format!("{prefix}{short}");
        }
    }
    modules.iter().map(|segment| pascal(segment)).collect::<String>() + short
}

/// `name` in snake case, with an acronym kept as one word.
fn snake(name: &str) -> Vec<String> {
    let chars: Vec<char> = name.chars().collect();
    let mut words: Vec<String> = Vec::new();
    for (index, &c) in chars.iter().enumerate() {
        let starts = c.is_uppercase()
            && index > 0
            && (chars[index - 1].is_lowercase() || chars[index - 1].is_ascii_digit() || chars.get(index + 1).is_some_and(|next| next.is_lowercase()));
        if starts || words.is_empty() {
            words.push(String::new());
        }
        words.last_mut().expect("a word").extend(c.to_lowercase());
    }
    words
}

/// The display title of a story named `story` (a function name in snake case) of the type `type_name`, a display
/// name: the leading words the type's name ends in are dropped (`places_tree_empty` of `PlacesTree` is "Empty",
/// `bar_in_layout` of `PerspectiveBar` is "In layout"), and the rest is in sentence case. A story named for the
/// whole type is "Default".
pub fn story_title(type_name: &str, story: &str) -> String {
    let type_words = snake(type_name);
    let words: Vec<&str> = story.split('_').filter(|word| !word.is_empty()).collect();
    let dropped = (1..=type_words.len().min(words.len()))
        .rev()
        .find(|&count| type_words[type_words.len() - count..].iter().zip(&words).all(|(a, b)| a == b))
        .unwrap_or(0);
    let rest = &words[dropped..];
    if rest.is_empty() {
        return "Default".to_owned();
    }
    let text = rest.join(" ");
    text[..1].to_uppercase() + &text[1..]
}

/// Every library registration linked into the running binary: the types of the library crates, not the fixtures of
/// an integration binary.
fn linked() -> Vec<&'static Registration> {
    super::registrations().into_iter().filter(|registration| registration.crate_target() == registration.crate_name().replace('-', "_")).collect()
}

/// Guards against a registered type that no row places (it would list under no level) and against names a
/// viewer cannot show as one identifier or that two types share (the tree would list two rows alike).
#[crate::testing::trial]
fn catalog_names_are_unique_pascal_case_and_every_type_has_a_level() {
    let catalog = catalog(&linked());
    assert!(catalog.unleveled.is_empty(), "types with no level: {:?}", catalog.unleveled);
    let mut seen = std::collections::HashSet::new();
    for entry in &catalog.entries {
        let mut chars = entry.name.chars();
        assert!(
            chars.next().is_some_and(|first| first.is_ascii_uppercase()) && chars.all(|c| c.is_ascii_alphanumeric()),
            "`{}` ({}) is not a PascalCase identifier",
            entry.name,
            entry.type_name
        );
        assert!(seen.insert((entry.level, entry.name.to_lowercase())), "`{}` is listed twice in {:?}", entry.name, entry.level);
    }
}

/// Guards the story title rule: the type's words come off the front, the rest reads as a sentence.
#[crate::testing::trial]
fn story_titles_drop_the_type_prefix_and_use_sentence_case() {
    assert_eq!(story_title("Button", "button_loading"), "Loading");
    assert_eq!(story_title("PlacesTree", "places_tree_empty_group"), "Empty group");
    assert_eq!(story_title("PerspectiveBar", "bar_in_layout"), "In layout");
    assert_eq!(story_title("Progress", "progress"), "Default");
    assert_eq!(story_title("Histogram", "inks"), "Inks");
    assert_eq!(story_title("PDFExport", "pdf_export_open"), "Open");
}

/// Guards the disambiguation: two types with one short name take their module segments, and a lone type keeps it.
#[crate::testing::trial]
fn clashing_short_names_take_module_segments() {
    let paths = ["a::tools::Button", "a::bars::Button"];
    assert_eq!(qualified(paths[0], "Button", &paths), "ToolsButton");
    assert_eq!(qualified(paths[1], "Button", &paths), "BarsButton");
    assert_eq!(qualified("a::Solo", "Solo", &["a::Solo"]), "Solo");
}
