//! Stories of the places tree: `patterns/places-tree/places`. Roots (standard folders, volumes, favourites, recents, the
//! session root), the reveal of the current folder, the hidden-places row and a drop.
//!
//! `FakeDirectories` is an in-memory tree with a removable and a network volume. Its listings run
//! on a `ManualJobs`: unless "slow listings" is on, the story finishes the waiting jobs at the
//! start of each pass; with it on, "Finish next" runs one, so the Loading row can be seen. The
//! tree's events go to the story's `PlacesSettings` and current folder. A draggable photo
//! shows the drop: only folders under Photos take it.

use std::path::PathBuf;
use std::sync::Arc;

use craft_host::{ManualJobs, Wake};
use craft_ui::behavior::{DragBadge, ListDrag};
use craft_ui::controls::Button;
use craft_ui::localized::Labels as _;
use craft_ui::localized::fmt;
use craft_ui::menu::{MenuEntry, MenuItem};
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, Story, StoryArgs, StoryLabels, lock, shared};
use craft_ui::typed_story;
use egui::{DragAndDrop, Id, Sense, Ui, vec2};

use super::{PlacesEvent, PlacesTree};
use crate::places::{FakeDirectories, FolderCache, PlaceGroupKind, Places, PlacesInput, PlacesSettings, StandardFolder, Volume};

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["PlacesTree"];

craft_ui::words! {
    /// The sample text of the places story, English by default.
    #[non_exhaustive]
    pub struct PlacesStoryLabels {
        rename: "placesStory.rename" = "Rename Folder…";
        waiting: "placesStory.waiting" = "{n} listings waiting";
        drag_photo: "placesStory.dragPhoto" = "Drag this photo onto a folder under Photos";
        browsing: "placesStory.browsing" = "Browsing: {path}";
        camera_card: "placesStory.cameraCard" = "Camera Card";
        archive: "placesStory.archive" = "Archive";
    }
}

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    vec![typed_story!(
        "patterns/places-tree/places",
        Level::Patterns,
        "PlacesTree",
        "Places",
        PlacesArgs { slow_listings: false, favorites: true, volumes: true, recent: true, height: 380.0 },
        show_places
    )]
}

#[derive(StoryArgs, Debug, PartialEq)]
struct PlacesArgs {
    /// Hold folder listings until "Finish next" (shows the Loading row).
    #[arg(name = "slow listings")]
    slow_listings: bool,
    /// The Favorites group.
    favorites: bool,
    /// The Volumes and Network groups.
    volumes: bool,
    /// The Recent Folders group.
    recent: bool,
    #[arg(range = 200.0..=600.0)]
    height: f32,
}

fn path(text: &str) -> PathBuf {
    PathBuf::from(text)
}

/// `/home/u` holds Documents, Pictures and Photos; Photos holds 2024 and 2025; 2024 holds Trip
/// and Studio; the card holds DCIM and the archive Old.
fn directories(card: &str, archive: &str) -> FakeDirectories {
    let mut fake = FakeDirectories::new()
        .standard_folder(StandardFolder::Home, "/home/u")
        .standard_folder(StandardFolder::Documents, "/home/u/Documents")
        .standard_folder(StandardFolder::Pictures, "/home/u/Pictures")
        .standard_folder(StandardFolder::Music, "/home/u/Music")
        .volume(Volume { path: path("/media/card"), name: card.to_owned(), network: false, removable: true })
        .volume(Volume { path: path("/mnt/archive"), name: archive.to_owned(), network: true, removable: false });
    for folder in [
        "/home/u/Documents",
        "/home/u/Music",
        "/home/u/Photos/2024/Studio",
        "/home/u/Photos/2024/Trip",
        "/home/u/Photos/2025",
        "/home/u/Pictures/Screenshots",
        "/media/card/DCIM",
        "/mnt/archive/Old",
    ] {
        fake = fake.folder(folder);
    }
    fake
}

/// What the story keeps between passes.
struct Demo {
    cache: FolderCache,
    jobs: Arc<ManualJobs>,
    settings: PlacesSettings,
    current: Option<PathBuf>,
    last: Option<String>,
}

impl Demo {
    fn new(ctx: &egui::Context, words: &PlacesStoryLabels) -> Self {
        let jobs = Arc::new(ManualJobs::new());
        let directories = Arc::new(directories(&words.camera_card, &words.archive));
        let repaint = ctx.clone();
        let cache = FolderCache::new(directories, jobs.clone(), Wake::new(move || repaint.request_repaint()));
        let settings = PlacesSettings {
            favorites: vec![path("/home/u/Photos/2024/Studio")],
            hidden: vec![path("/home/u/Music")],
            recent: vec![path("/home/u/Photos/2025"), path("/media/card/DCIM")],
            session_root: None,
        };
        Self { cache, jobs, settings, current: Some(path("/home/u/Photos/2024/Trip")), last: None }
    }
}

fn show_places(ui: &mut Ui, args: &mut PlacesArgs) {
    let words = PlacesStoryLabels::of(ui);
    let shared_words = StoryLabels::of(ui);
    let demo = shared(ui, "places", || Demo::new(ui.ctx(), &words));
    let mut demo = lock(&demo);

    if args.slow_listings {
        ui.horizontal_wrapped(|ui| {
            if ui.add(Button::new(shared_words.finish_next.clone()).enabled(demo.jobs.waiting() > 0)).clicked() {
                demo.jobs.run_next();
            }
            ui.add(Text::new(fmt(&words.waiting, &[("n", &demo.jobs.waiting().to_string())])).content(Content::Muted));
        });
    } else if demo.jobs.run_all() > 0 {
        ui.ctx().request_repaint();
    }

    let mut groups = vec![PlaceGroupKind::Places];
    if args.favorites {
        groups.insert(0, PlaceGroupKind::Favorites);
    }
    if args.volumes {
        groups.extend([PlaceGroupKind::Volumes, PlaceGroupKind::Network]);
    }
    if args.recent {
        groups.push(PlaceGroupKind::Recent);
    }
    let current = demo.current.clone();
    let places = {
        let Demo { cache, settings, .. } = &mut *demo;
        let unlisted = current.as_deref().and_then(|current| cache.unlisted(current)).map(<[PathBuf]>::to_vec).unwrap_or_default();
        let roots = cache.roots().cloned();
        let (standard, volumes) = roots.map_or((Vec::new(), Vec::new()), |roots| (roots.standard, roots.volumes));
        let places = Places::resolve(&PlacesInput {
            standard: &standard,
            volumes: &volumes,
            settings,
            browsing: current.as_deref(),
            unlisted: &unlisted,
            groups: &groups,
        });
        settings.session_root = places.session_root.clone();
        places
    };

    let photo_name = fmt(&shared_words.photo_n, &[("n", "7")]);
    let photo = ListDrag { list: Id::NULL.with("places-story"), items: vec![7u32], grabbed: 7, label: photo_name.clone() };
    ui.horizontal_wrapped(|ui| {
        let response = ui.add(egui::Label::new(photo_name.clone()).sense(Sense::drag()));
        if response.drag_started() {
            DragAndDrop::set_payload(ui.ctx(), photo.clone());
        }
        ui.add(Text::new(words.drag_photo.clone()).content(Content::Muted));
    });
    if DragAndDrop::has_payload_of_type::<ListDrag<u32>>(ui.ctx()) {
        DragBadge::new("places-story-drag", photo_name.clone()).show(ui.ctx());
    }

    let rename = words.rename.clone();
    let out = {
        let Demo { cache, .. } = &mut *demo;
        let tree = PlacesTree::<u32>::new("places-story-tree", &places, cache)
            .current(current.as_deref())
            .row_menu(|_| vec![MenuEntry::from(MenuItem::new(rename.clone(), String::from("folder.rename")))])
            .accepts(|folder, _| folder.starts_with("/home/u/Photos"));
        ui.allocate_ui(vec2(ui.available_width(), args.height), |ui| {
            ui.set_min_height(args.height);
            tree.show(ui)
        })
        .inner
    };
    for event in out.events {
        demo.settings.apply(&event);
        if let PlacesEvent::Open(folder) = &event {
            demo.current = Some(folder.clone());
        }
        demo.last = Some(format!("{event:?}"));
    }
    if let Some(current) = &demo.current {
        ui.add(Text::new(fmt(&words.browsing, &[("path", &current.display().to_string())])).content(Content::Muted));
    }
    let last = match &demo.last {
        Some(event) => fmt(&shared_words.last_event, &[("event", event.as_str())]),
        None => shared_words.no_event.clone(),
    };
    ui.add(Text::new(last).content(Content::Muted));
}
