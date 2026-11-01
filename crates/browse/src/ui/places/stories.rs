//! Stories of the places tree: `patterns/browse/places` (the spec's
//! `browse/places` under its level). Roots (standard folders, volumes, favourites, recents, the
//! session root), the reveal of the current folder, the hidden-places row and a drop.
//!
//! `FakeDirectories` is an in-memory tree with a removable and a network volume. Its listings run
//! on a `ManualJobs`: unless "slow listings" is on, the story finishes the waiting jobs at the
//! start of each pass; with it on, "Finish next" runs one, so the Loading row can be seen. The
//! tree's events go to the story's `PlacesSettings` and current folder. A draggable photo
//! shows the drop: only folders under Photos take it.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use craft_host::queue::Cancel;
use craft_host::{ManualJobs, Wake};
use craft_ui::behavior::{DragBadge, ListDrag};
use craft_ui::controls::Button;
use craft_ui::localized::Labels as _;
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, Story, StoryArgs};
use craft_ui::typed_story;
use egui::{DragAndDrop, Id, Sense, Ui, vec2};

use super::{PlaceExtra, PlacesEvent, PlacesTree};
use crate::places::{Directories, Folder, FolderCache, PlaceGroupKind, Places, PlacesInput, PlacesSettings, StandardFolder, Volume};

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Places tree"];

craft_ui::words! {
    /// The sample text of the places story, English by default.
    #[non_exhaustive]
    pub struct PlacesStoryLabels {
        rename: "placesStory.rename" = "Rename Folder…";
        finish_next: "placesStory.finishNext" = "Finish next";
        waiting: "placesStory.waiting" = "{n} listings waiting";
        drag_photo: "placesStory.dragPhoto" = "Drag this photo onto a folder under Photos";
        photo: "placesStory.photo" = "Photo 7";
        browsing: "placesStory.browsing" = "Browsing: {path}";
        last_event: "placesStory.lastEvent" = "Last event: {event}";
        no_event: "placesStory.noEvent" = "No event yet";
        camera_card: "placesStory.cameraCard" = "Camera Card";
        archive: "placesStory.archive" = "Archive";
        places: "placesStory.places" = "Places";
    }
}

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    vec![typed_story!(
        "patterns/browse/places",
        Level::Patterns,
        "Places tree",
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
struct FakeDirectories {
    card: String,
    archive: String,
}

impl Directories for FakeDirectories {
    fn standard(&self) -> Vec<(StandardFolder, PathBuf)> {
        vec![
            (StandardFolder::Home, path("/home/u")),
            (StandardFolder::Documents, path("/home/u/Documents")),
            (StandardFolder::Pictures, path("/home/u/Pictures")),
            (StandardFolder::Music, path("/home/u/Music")),
        ]
    }

    fn volumes(&self) -> Vec<Volume> {
        vec![
            Volume { path: path("/media/card"), name: self.card.clone(), network: false, removable: true },
            Volume { path: path("/mnt/archive"), name: self.archive.clone(), network: true, removable: false },
        ]
    }

    fn subfolders(&self, folder: &Path, _: &Cancel) -> std::io::Result<Vec<Folder>> {
        let names: &[&str] = match folder.to_str().unwrap_or_default() {
            "/home/u" => &["Documents", "Music", "Photos", "Pictures"],
            "/home/u/Photos" => &["2024", "2025"],
            "/home/u/Photos/2024" => &["Studio", "Trip"],
            "/home/u/Pictures" => &["Screenshots"],
            "/media/card" => &["DCIM"],
            "/mnt/archive" => &["Old"],
            _ => &[],
        };
        Ok(names.iter().map(|name| Folder { path: folder.join(name), name: (*name).into() }).collect())
    }

    fn unlisted_ancestors(&self, _: &Path) -> Vec<PathBuf> {
        Vec::new()
    }
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
        let directories = Arc::new(FakeDirectories { card: words.camera_card.clone(), archive: words.archive.clone() });
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
    let id = ui.make_persistent_id("craft-browse-places-story");
    let shared = ui.data_mut(|data| data.get_temp_mut_or_insert_with(id, || Arc::new(std::sync::Mutex::new(Demo::new(ui.ctx(), &words)))).clone());
    let mut demo = shared.lock().unwrap_or_else(std::sync::PoisonError::into_inner);

    if args.slow_listings {
        ui.horizontal(|ui| {
            if ui.add(Button::new(words.finish_next.clone()).enabled(demo.jobs.waiting() > 0)).clicked() {
                demo.jobs.run_next();
            }
            ui.add(Text::new(words.waiting.replace("{n}", &demo.jobs.waiting().to_string())).content(Content::Muted));
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

    let photo = ListDrag { list: Id::NULL.with("places-story"), items: vec![7u32], grabbed: 7, label: words.photo.clone() };
    ui.horizontal(|ui| {
        let response = ui.add(egui::Label::new(words.photo.clone()).sense(Sense::drag()));
        if response.drag_started() {
            DragAndDrop::set_payload(ui.ctx(), photo.clone());
        }
        ui.add(Text::new(words.drag_photo.clone()).content(Content::Muted));
    });
    if DragAndDrop::has_payload_of_type::<ListDrag<u32>>(ui.ctx()) {
        DragBadge::new("places-story-drag", words.photo.clone()).show(ui.ctx());
    }

    let extras = [PlaceExtra::new("folder.rename", words.rename.clone())];
    let out = {
        let Demo { cache, .. } = &mut *demo;
        let tree = PlacesTree::<u32>::new("places-story-tree", &places, cache)
            .current(current.as_deref())
            .extras(&extras)
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
        ui.add(Text::new(words.browsing.replace("{path}", &current.display().to_string())).content(Content::Muted));
    }
    let last = match &demo.last {
        Some(event) => words.last_event.replace("{event}", event),
        None => words.no_event.clone(),
    };
    ui.add(Text::new(last).content(Content::Muted));
}
