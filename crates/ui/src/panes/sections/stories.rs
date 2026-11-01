//! Stories of the reorderable sections: `composites/reorderable-sections/sidebar`
//! (the spec's `panes/sections`: the gallery requires a story id to start with its level). Five
//! sections, drag, pinned header, header button, hidden, folded counts and the menu.
//!
//! The story keeps a `SectionLayout` and applies every event to it, so folding, dragging, hiding
//! and the menu change what it shows. Local starts folded and Keywords hidden. The Albums
//! button adds an album, which shows in the Albums count and rows.

use egui::{IdSalt, ScrollArea, Ui};

use super::{ReorderableSections, SectionButton, SectionEvent, SectionInfo, SectionKey, SectionLayout, SectionState, SectionsLabels};
use crate::icons::Icon;
use crate::localized::Labels as _;
use crate::menu::PressMenu;
use crate::primitives::{Content, Text};
use crate::story::{Level, Story, StoryArgs, StoryLabels, lock, shared};
use crate::typed_story;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Reorderable sections"];

crate::words! {
    /// The sample text of the sections story, English by default.
    #[non_exhaustive]
    pub struct SectionsStoryLabels {
        albums: "sectionsStory.albums" = "Albums";
        local: "sectionsStory.local" = "Local";
        by_date: "sectionsStory.byDate" = "By Date";
        folders: "sectionsStory.folders" = "Folders";
        keywords: "sectionsStory.keywords" = "Keywords";
        sidebar: "sectionsStory.sidebar" = "Sidebar";
        create_album: "sectionsStory.createAlbum" = "Create Album";
        no_albums: "sectionsStory.noAlbums" = "No albums yet";
        no_keywords: "sectionsStory.noKeywords" = "No keywords yet";
        album_n: "sectionsStory.albumN" = "Album {n}";
        row_n: "sectionsStory.rowN" = "{section} row {n}";
        menu: "sectionsStory.menu" = "Sections Menu";
    }
}

/// The story above.
pub(crate) fn stories() -> Vec<Story> {
    vec![typed_story!(
        "composites/reorderable-sections/sidebar",
        Level::Composites,
        "Reorderable sections",
        "Sidebar",
        SidebarArgs { show_counts: true, rows: 4, albums_button: true, local_available: true, height: 260.0 },
        show_sidebar
    )]
}

#[derive(StoryArgs, Debug, PartialEq)]
struct SidebarArgs {
    /// Show the row count of folded sections.
    #[arg(name = "show counts")]
    show_counts: bool,
    /// Rows in each section.
    #[arg(range = 0..=30)]
    rows: usize,
    /// The Albums header button.
    #[arg(name = "albums button")]
    albums_button: bool,
    /// Whether the build has Local (an unavailable section keeps its place and is not drawn).
    #[arg(name = "local available")]
    local_available: bool,
    /// The scroll area's height; short enough for the pinned header to show.
    #[arg(range = 120.0..=480.0)]
    height: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Side {
    Albums,
    Local,
    ByDate,
    Folders,
    Keywords,
}

impl SectionKey for Side {
    const ALL: &'static [Self] = &[Side::Albums, Side::Local, Side::ByDate, Side::Folders, Side::Keywords];

    fn id(self) -> &'static str {
        match self {
            Side::Albums => "albums",
            Side::Local => "local",
            Side::ByDate => "byDate",
            Side::Folders => "folders",
            Side::Keywords => "keywords",
        }
    }

    fn from_id(id: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|side| side.id() == id)
    }
}

/// What the story keeps between passes.
struct Demo {
    layout: SectionLayout<Side>,
    albums: usize,
    last: Option<SectionEvent<Side>>,
}

impl Demo {
    fn new() -> Self {
        let state = |section, hidden, folded| SectionState { section, hidden, folded };
        let layout = SectionLayout::from_states([state(Side::Local, false, true), state(Side::Keywords, true, false)]);
        Self { layout, albums: 2, last: None }
    }
}

fn show_sidebar(ui: &mut Ui, args: &mut SidebarArgs) {
    let words = SectionsStoryLabels::of(ui);
    let labels = SectionsLabels::of(ui);
    let story = StoryLabels::of(ui);
    let state = shared(ui, "sidebar", Demo::new);
    let mut demo = lock(&state);

    let rows = args.rows;
    let title = |side: Side| match side {
        Side::Albums => words.albums.clone(),
        Side::Local => words.local.clone(),
        Side::ByDate => words.by_date.clone(),
        Side::Folders => words.folders.clone(),
        Side::Keywords => words.keywords.clone(),
    };
    let count = |side: Side| if side == Side::Albums { demo.albums } else { rows };
    let button = SectionButton::new("albumNew", Icon::Plus, words.create_album.clone());
    let sections: Vec<SectionInfo<Side>> = Side::ALL
        .iter()
        .map(|&side| {
            let hint = if side == Side::Keywords { &words.no_keywords } else { &words.no_albums };
            let mut info = SectionInfo::new(side, title(side)).count(count(side)).empty_hint(hint.clone());
            if side == Side::Albums && args.albums_button {
                info = info.button(button.clone());
            }
            info.available(side != Side::Local || args.local_available)
        })
        .collect();

    let mut events = Vec::new();
    let entries = ReorderableSections::menu(&demo.layout, &sections, None, &labels);
    let menu_button = ui.button(words.menu.clone());
    events.extend(PressMenu::new(&entries).show(ui, &menu_button).chosen);
    let out = ScrollArea::vertical().id_salt(IdSalt::new("sections-story-scroll")).max_height(args.height).show(ui, |ui| {
        ReorderableSections::new("sections-story-sections", &demo.layout)
            .show_counts(args.show_counts)
            .label(words.sidebar.clone())
            .labels(labels.clone())
            .show(ui, &sections, |ui, side| {
                let rows = count(side);
                for n in 1..=rows {
                    let text = if side == Side::Albums {
                        craft_i18n::fmt(&words.album_n, &[("n", &n.to_string())])
                    } else {
                        craft_i18n::fmt(&words.row_n, &[("section", &title(side)), ("n", &n.to_string())])
                    };
                    ui.label(text);
                }
            })
    });
    events.extend(out.inner.events);
    for event in events {
        if let SectionEvent::Button { .. } = event {
            demo.albums += 1;
        }
        demo.layout.apply(&event);
        demo.last = Some(event);
    }
    let text = match &demo.last {
        Some(event) => craft_i18n::fmt(&story.last_event, &[("event", &format!("{event:?}"))]),
        None => story.no_event.clone(),
    };
    ui.add(Text::new(text).content(Content::Muted));
}
