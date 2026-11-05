//! Stories of the Links panel and Link Media dialog:
//! `patterns/link-list/list` and `patterns/relink-dialog/dialog`.
//!
//! `FakeLinkFiles` is an in-memory file set with a same file, a different file and a folder
//! tree. The dialog's search and checks run on a `ManualJobs`: unless "slow search" is on the
//! story finishes the waiting jobs at the start of each pass, otherwise "Finish next" runs one,
//! so the progress line and Stop can be tried. Browse… and Locate… answer from a scripted
//! `Files` on a story `Host`. The list's events change the story's fake statuses, as an app's
//! commands would.

use std::collections::HashMap;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, SystemTime};

use craft_docpath::Refused;
use craft_docpath::links::{FakeLinkFiles, Link, LinkIdentity, LinkStatus, identity_of};
use craft_host::{FileError, Files, Jobs, ManualJobs, PickAnswer, PickId, PickKind, PickRequest, Picked};
use craft_ui::behavior::Selection;
use craft_ui::canvas::textures::{FakeThumbSource, ThumbnailOptions, Thumbnails};
use craft_ui::controls::Button;
use craft_ui::list::ThumbSize;
use craft_ui::localized::Labels as _;
use craft_ui::localized::fmt;
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, Story, StoryArgs, StoryChoice, StoryLabels, drive, ensure_files, lock, shared};
use craft_ui::typed_story;
use egui::{Ui, vec2};

use super::{LinkFilter, LinkList, LinkListEvent, LinkListSettings, LinkListState, LinkSort, RelinkDialog, RelinkOptions, RelinkOutcome, RelinkRow};

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["LinkList", "RelinkDialog"];

craft_ui::words! {
    /// The sample text of the link stories, English by default.
    #[non_exhaustive]
    pub struct LinkStoryLabels {
        media_missing: "linkStory.mediaMissing" = "Media missing";
        clip_n: "linkStory.clipN" = "Clip {n}";
        waiting: "linkStory.waiting" = "{n} jobs waiting";
        outcome: "linkStory.outcome" = "Linked {linked}, left offline {offline}";
        info_path: "linkStory.infoPath" = "Path: {path}";
        info_none: "linkStory.infoNone" = "No link selected";
    }
}

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "patterns/link-list/list",
            Level::Patterns,
            "LinkList",
            "Link list",
            ListArgs { filter: FilterArg::All, sort: SortArg::Name, thumbnails: SizeArg::Small, info_open: true, actions: true, height: 380.0 },
            show_list
        ),
        typed_story!(
            "patterns/relink-dialog/dialog",
            Level::Patterns,
            "RelinkDialog",
            "Relink dialog",
            DialogArgs { open: true, relink_others: true, exact_names: false, slow_search: false },
            show_dialog
        ),
    ]
}

fn path(text: &str) -> PathBuf {
    PathBuf::from(text)
}

const STAMP: Duration = Duration::from_secs(1);

/// `STAMP` in whole seconds, as `FakeLinkFiles::with` takes it.
const STAMP_SECS: u64 = 1;

/// What the dialog's files hold: three bytes that the stored links were recorded with.
const SAME: &[u8] = b"abc";

/// The identity every link was recorded with: the bytes [`SAME`], modified at `STAMP`.
fn recorded() -> Option<LinkIdentity> {
    identity_of(&mut Cursor::new(SAME), SAME.len() as u64, Some(SystemTime::UNIX_EPOCH + STAMP)).ok()
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum FilterArg {
    All,
    Missing,
    Modified,
    Embedded,
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum SortArg {
    Name,
    Kind,
    Status,
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum SizeArg {
    None,
    Small,
    Medium,
    Large,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct ListArgs {
    /// Which links show.
    filter: FilterArg,
    /// The order.
    sort: SortArg,
    /// Row thumbnail size.
    thumbnails: SizeArg,
    /// The Link Info section.
    #[arg(name = "info open")]
    info_open: bool,
    /// The action row under the list.
    actions: bool,
    #[arg(range = 200.0..=600.0)]
    height: f32,
}

impl ListArgs {
    fn settings(&self) -> LinkListSettings {
        LinkListSettings {
            filter: match self.filter {
                FilterArg::All => LinkFilter::All,
                FilterArg::Missing => LinkFilter::Missing,
                FilterArg::Modified => LinkFilter::Modified,
                FilterArg::Embedded => LinkFilter::Embedded,
            },
            sort: match self.sort {
                SortArg::Name => LinkSort::Name,
                SortArg::Kind => LinkSort::Kind,
                SortArg::Status => LinkSort::Status,
            },
            thumbnails: match self.thumbnails {
                SizeArg::None => ThumbSize::None,
                SizeArg::Small => ThumbSize::Small,
                SizeArg::Medium => ThumbSize::Medium,
                SizeArg::Large => ThumbSize::Large,
            },
            info_open: self.info_open,
        }
    }
}

fn link(key: u32, name: &str, page: Option<u32>, embedded: bool) -> Link<u32> {
    Link { key, name: name.into(), stored: format!("/shoot/{name}"), identity: recorded(), page, embedded }
}

/// What the list story keeps between passes.
struct ListDemo {
    links: Vec<Link<u32>>,
    statuses: HashMap<u32, LinkStatus>,
    state: LinkListState<u32>,
    shown_args: Option<ListArgs>,
    last: Option<String>,
}

impl ListDemo {
    fn new() -> Self {
        let rows = [
            ("beach.jpg", None, LinkStatus::Ok),
            ("brochure.pdf", Some(2), LinkStatus::Modified),
            ("cover.psd", None, LinkStatus::Missing),
            ("logo.ai", None, LinkStatus::Ok),
            ("map.png", None, LinkStatus::Unreadable),
            ("poster.tif", None, LinkStatus::Modified),
            ("remote.jpg", None, LinkStatus::Refused(Refused::Network)),
            ("scan.raw", None, LinkStatus::Refused(Refused::Device)),
            ("stock.jpg", None, LinkStatus::Refused(Refused::Url)),
            ("stray.png", None, LinkStatus::Refused(Refused::Invalid)),
            ("sketch.png", None, LinkStatus::Embedded),
            ("texture.png", None, LinkStatus::Ok),
        ];
        let mut links = Vec::new();
        let mut statuses = HashMap::new();
        for (key, (name, page, status)) in rows.into_iter().enumerate() {
            let key = key as u32;
            links.push(link(key, name, page, status == LinkStatus::Embedded));
            statuses.insert(key, status);
        }
        Self {
            links,
            statuses,
            state: LinkListState { selection: Selection::only(2), settings: LinkListSettings::default() },
            shown_args: None,
            last: None,
        }
    }

    fn set_status(&mut self, keys: &[u32], status: LinkStatus) {
        for key in keys {
            self.statuses.insert(*key, status);
            if status == LinkStatus::Embedded
                && let Some(link) = self.links.iter_mut().find(|link| link.key == *key)
            {
                link.embedded = true;
            }
        }
    }
}

fn show_list(ui: &mut Ui, args: &mut ListArgs) {
    let words = LinkStoryLabels::of(ui);
    let shared_words = StoryLabels::of(ui);
    let state = shared(ui, "list", ListDemo::new);
    let mut demo = lock(&state);
    let current = ListArgs { ..*args };
    if demo.shown_args.as_ref() != Some(&current) {
        demo.state.settings = current.settings();
        demo.shown_args = Some(current);
    }

    let thumbs = Thumbnails::<u32>::in_context(ui.ctx(), "craft-browse-link-list-story", ThumbnailOptions::default());
    let pictures = FakeThumbSource::<u32>::new().aspect(4, 3);
    let lead_path = demo.state.selection.lead.and_then(|lead| demo.links.iter().find(|link| link.key == lead)).map(|link| link.stored.clone());
    let info = match lead_path {
        Some(stored) => fmt(&words.info_path, &[("path", &stored)]),
        None => words.info_none.clone(),
    };
    let ListDemo { links, statuses, state, .. } = &mut *demo;
    let list = LinkList::new("link-list-story", links, statuses, state).thumbnails(&thumbs, &pictures).actions(args.actions).info(move |ui| {
        ui.label(info);
    });
    let events = ui
        .allocate_ui(vec2(ui.available_width(), args.height), |ui| {
            ui.set_min_height(args.height);
            list.show(ui).events
        })
        .inner;
    for event in events {
        match &event {
            LinkListEvent::Relink(keys) | LinkListEvent::RelinkToFolder(keys) | LinkListEvent::Update(keys) => demo.set_status(keys, LinkStatus::Ok),
            LinkListEvent::BreakLink(keys) => demo.set_status(keys, LinkStatus::Embedded),
            _ => {}
        }
        if !matches!(event, LinkListEvent::Selection(_)) {
            demo.last = Some(format!("{event:?}"));
        }
    }
    let text = match &demo.last {
        Some(event) => fmt(&shared_words.last_event, &[("event", event.as_str())]),
        None => shared_words.no_event.clone(),
    };
    ui.add(Text::new(text).content(Content::Muted));
}

/// A scripted `Files`: Browse… answers with the folder the files are in, Locate… with one file.
#[derive(Default)]
struct ScriptedFiles {
    state: Mutex<(u64, Vec<PickAnswer>)>,
}

impl Files for ScriptedFiles {
    fn pick(&self, request: PickRequest) -> PickId {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        state.0 += 1;
        let id = PickId(state.0);
        let picked = match request.kind {
            PickKind::Folder => Picked::Paths(vec![path("/new")]),
            _ => Picked::Paths(vec![path("/new/shoot/c.mov")]),
        };
        state.1.push(PickAnswer { id, request, picked });
        id
    }

    fn take_answer(&self, id: PickId) -> Option<Picked> {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let at = state.1.iter().position(|answer| answer.id == id)?;
        Some(state.1.remove(at).picked)
    }

    fn forget(&self, id: PickId) {
        self.state.lock().unwrap_or_else(PoisonError::into_inner).1.retain(|answer| answer.id != id);
    }

    fn pick_pending(&self) -> bool {
        false
    }

    fn read(&self, _path: &Path) -> Result<Vec<u8>, FileError> {
        Err(FileError::Unsupported)
    }

    fn write(&self, _path: &Path, _bytes: &[u8]) -> Result<(), FileError> {
        Err(FileError::Unsupported)
    }

    fn last_folder(&self, _purpose: &str) -> Option<PathBuf> {
        None
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct DialogArgs {
    open: bool,
    /// "Relink others automatically".
    #[arg(name = "relink others")]
    relink_others: bool,
    /// "Exact name matches only".
    #[arg(name = "exact names")]
    exact_names: bool,
    /// Hold the search and checks until "Finish next".
    #[arg(name = "slow search")]
    slow_search: bool,
}

/// What the dialog story keeps between passes.
struct DialogDemo {
    jobs: Arc<ManualJobs>,
    files: Arc<FakeLinkFiles>,
    scripted: Arc<ScriptedFiles>,
}

impl DialogDemo {
    fn new() -> Self {
        let same = |files: FakeLinkFiles, path: &str| files.with(path, SAME, STAMP_SECS);
        let files = ["/new/shoot/a.mov", "/new/shoot/b.mov", "/new/other/d.mov"].into_iter().fold(FakeLinkFiles::new(), same);
        let files = files.with("/new/shoot/c.mov", b"nine byte", STAMP_SECS).with("/new/readme.txt", b"hello", STAMP_SECS);
        Self { jobs: Arc::new(ManualJobs::new()), files: Arc::new(files), scripted: Arc::new(ScriptedFiles::default()) }
    }

    fn rows(words: &LinkStoryLabels) -> Vec<RelinkRow<u32>> {
        ["a", "b", "c", "d"]
            .iter()
            .enumerate()
            .map(|(n, name)| {
                let file = format!("{name}.mov");
                RelinkRow {
                    link: Link {
                        key: n as u32,
                        name: file.clone(),
                        stored: format!("/old/shoot/{file}"),
                        identity: recorded(),
                        page: None,
                        embedded: false,
                    },
                    owner: fmt(&words.clip_n, &[("n", &(n + 1).to_string())]),
                    status: words.media_missing.clone(),
                }
            })
            .collect()
    }
}

fn describe(outcome: &RelinkOutcome<u32>, words: &LinkStoryLabels) -> String {
    fmt(&words.outcome, &[("linked", &outcome.linked.len().to_string()), ("offline", &outcome.offline.len().to_string())])
}

fn show_dialog(ui: &mut Ui, args: &mut DialogArgs) {
    let ctx = ui.ctx().clone();
    let words = LinkStoryLabels::of(ui);
    let shared_words = StoryLabels::of(ui);
    let state = shared(ui, "dialog", DialogDemo::new);
    let demo = lock(&state);
    let _files = ensure_files(&ctx, demo.scripted.clone());

    if args.slow_search {
        ui.horizontal_wrapped(|ui| {
            if ui.add(Button::new(shared_words.finish_next.clone()).enabled(demo.jobs.waiting() > 0)).clicked() {
                demo.jobs.run_next();
            }
            ui.add(Text::new(fmt(&words.waiting, &[("n", &demo.jobs.waiting().to_string())])).content(Content::Muted));
        });
    } else if demo.jobs.run_all() > 0 {
        ctx.request_repaint();
    }

    let (relink_others, exact_names) = (args.relink_others, args.exact_names);
    drive::<RelinkDialog<u32>, _>(
        ui,
        "relink",
        &mut args.open,
        (relink_others, exact_names),
        || {
            let options = RelinkOptions {
                relink_others,
                exact_names,
                folder: Some(path("/new")),
                jobs: Some(demo.jobs.clone() as Arc<dyn Jobs>),
                ..RelinkOptions::default()
            };
            RelinkDialog::new(DialogDemo::rows(&words), demo.files.clone(), options)
        },
        |_| {},
        |outcome| describe(&outcome, &words),
    );
}
