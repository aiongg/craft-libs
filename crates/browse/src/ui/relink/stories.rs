//! Stories of the Links panel and Link Media dialog (not registered; task ST8):
//! `patterns/browse/link-list` and `patterns/browse/relink-dialog` (the spec's `browse/link-list`
//! and `browse/relink-dialog` under their level).
//!
//! Registering them (task RG2) adds `stories()` and `COMPONENTS` ("LinkList", "RelinkDialog") to
//! the browse stories.
//!
//! `FakeLinkFiles` is an in-memory file set with a same file, a different file and a folder
//! tree. The dialog's search and checks run on a `ManualJobs`: unless "slow search" is on the
//! story finishes the waiting jobs at the start of each pass, otherwise "Finish next" runs one,
//! so the progress line and Stop can be tried. Browse… and Locate… answer from a scripted
//! `Files` on a story `Host`. The list's events change the story's fake statuses, as an app's
//! commands would.

#![expect(dead_code, reason = "not registered until RG2 adds `stories()` and `COMPONENTS` to the browse stories")]

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, SystemTime};

use craft_docpath::Refused;
use craft_docpath::links::{FileInfo, FolderEntry, Link, LinkFiles, LinkIdentity, LinkStatus};
use craft_host::{AppInfo, FileError, Files, Host, Jobs, ManualJobs, PickAnswer, PickId, PickKind, PickRequest, Picked};
use craft_ui::behavior::Selection;
use craft_ui::canvas::textures::{FakeThumbSource, ThumbnailOptions, Thumbnails};
use craft_ui::controls::Button;
use craft_ui::dialog::{DialogHost, DialogResult};
use craft_ui::list::ThumbSize;
use craft_ui::localized::Labels as _;
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, Story, StoryArgs, StoryChoice, dialog_claim, dialog_closed, dialog_shown};
use craft_ui::typed_story;
use egui::{Context, Ui, vec2};

use super::{LinkFilter, LinkList, LinkListEvent, LinkListState, LinkSort, RelinkDialog, RelinkOptions, RelinkOutcome, RelinkRow};

/// The components this module has stories for. RG2 makes this `pub const COMPONENTS`.
pub(crate) const COMPONENTS: &[&str] = &["LinkList", "RelinkDialog"];

craft_ui::words! {
    /// The sample text of the link stories, English by default.
    #[non_exhaustive]
    pub struct LinkStoryLabels {
        media_missing: "linkStory.mediaMissing" = "Media missing";
        clip_n: "linkStory.clipN" = "Clip {n}";
        finish_next: "linkStory.finishNext" = "Finish next";
        waiting: "linkStory.waiting" = "{n} jobs waiting";
        last_event: "linkStory.lastEvent" = "Last event: {event}";
        no_event: "linkStory.noEvent" = "No event yet";
        result: "linkStory.result" = "Linked {linked}, left offline {offline}";
        cancelled: "linkStory.cancelled" = "Cancelled";
        info_path: "linkStory.infoPath" = "Path: {path}";
        info_none: "linkStory.infoNone" = "No link selected";
    }
}

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "patterns/browse/link-list",
            Level::Patterns,
            "LinkList",
            "Link list",
            ListArgs { filter: FilterArg::All, sort: SortArg::Name, thumbnails: SizeArg::Small, info_open: true, actions: true, height: 380.0 },
            show_list
        ),
        typed_story!(
            "patterns/browse/relink-dialog",
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

/// The identity every link was recorded with: 3 bytes, fingerprint 7.
fn recorded() -> LinkIdentity {
    LinkIdentity { size: 3, fingerprint: 7, modified: Some(SystemTime::UNIX_EPOCH + STAMP) }
}

/// Files by path: size and fingerprint.
#[derive(Default)]
struct FakeLinkFiles(Mutex<BTreeMap<PathBuf, (u64, u64)>>);

impl FakeLinkFiles {
    fn put(&self, path: &str, size: u64, fingerprint: u64) {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).insert(path.into(), (size, fingerprint));
    }

    fn get(&self, path: &Path) -> Option<(u64, u64)> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).get(path).copied()
    }
}

impl LinkFiles for FakeLinkFiles {
    fn stat(&self, path: &Path) -> Option<FileInfo> {
        self.get(path).map(|(size, _)| FileInfo { size, modified: Some(SystemTime::UNIX_EPOCH + STAMP) })
    }

    fn identity(&self, path: &Path) -> io::Result<LinkIdentity> {
        let (size, fingerprint) = self.get(path).ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
        Ok(LinkIdentity { size, fingerprint, modified: Some(SystemTime::UNIX_EPOCH + STAMP) })
    }

    fn list(&self, dir: &Path) -> io::Result<Vec<FolderEntry>> {
        let files = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let mut folders = BTreeSet::new();
        let mut entries = Vec::new();
        for file in files.keys() {
            let Ok(rest) = file.strip_prefix(dir) else { continue };
            let mut parts = rest.components();
            let Some(first) = parts.next() else { continue };
            let child = dir.join(first);
            let name = first.as_os_str().to_string_lossy().into_owned();
            if parts.next().is_some() {
                if folders.insert(child.clone()) {
                    entries.push(FolderEntry { path: child, name, is_dir: true, is_link: false });
                }
            } else {
                entries.push(FolderEntry { path: child, name, is_dir: false, is_link: false });
            }
        }
        Ok(entries)
    }
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
    fn state(&self) -> LinkListState {
        LinkListState {
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
    Link { key, name: name.into(), stored: format!("/shoot/{name}"), identity: Some(recorded()), page, embedded }
}

/// What the list story keeps between passes.
struct ListDemo {
    links: Vec<Link<u32>>,
    statuses: HashMap<u32, LinkStatus>,
    selection: Selection<u32>,
    state: LinkListState,
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
        Self { links, statuses, selection: Selection::only(2), state: LinkListState::default(), shown_args: None, last: None }
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
    let id = ui.make_persistent_id("craft-browse-link-list-story");
    let state = ui.data_mut(|data| data.get_temp_mut_or_insert_with(id, || Arc::new(Mutex::new(ListDemo::new()))).clone());
    let mut demo = state.lock().unwrap_or_else(PoisonError::into_inner);
    let current = ListArgs { ..*args };
    if demo.shown_args.as_ref() != Some(&current) {
        demo.state = current.state();
        demo.shown_args = Some(current);
    }

    let thumbs = Thumbnails::<u32>::in_context(ui.ctx(), "craft-browse-link-list-story", ThumbnailOptions::default());
    let pictures = FakeThumbSource::<u32>::new().aspect(4, 3);
    let lead_path = demo.selection.lead.and_then(|lead| demo.links.iter().find(|link| link.key == lead)).map(|link| link.stored.clone());
    let info = match lead_path {
        Some(stored) => words.info_path.replace("{path}", &stored),
        None => words.info_none.clone(),
    };
    let ListDemo { links, statuses, selection, state, .. } = &mut *demo;
    let list = LinkList::new("link-list-story", links, statuses)
        .selection(selection)
        .state(state)
        .thumbnails(&thumbs, &pictures)
        .actions(args.actions)
        .info(move |ui| {
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
        if !matches!(event, LinkListEvent::Selected | LinkListEvent::State) {
            demo.last = Some(format!("{event:?}"));
        }
    }
    let text = match &demo.last {
        Some(event) => words.last_event.replace("{event}", event),
        None => words.no_event.clone(),
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

/// Installs a Host with the scripted files when the story runs without one.
fn ensure_host(ctx: &Context) {
    if Host::try_of_ctx(ctx).is_none() {
        Host::builder(AppInfo { id: "craft-browse-stories", name: "Craft Stories", version: "0" })
            .files(Arc::new(ScriptedFiles::default()))
            .install(ctx);
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
    host: DialogHost,
    jobs: Arc<ManualJobs>,
    files: Arc<FakeLinkFiles>,
    signature: Option<(bool, bool)>,
    result: Option<String>,
}

impl DialogDemo {
    fn new() -> Self {
        let files = Arc::new(FakeLinkFiles::default());
        files.put("/new/shoot/a.mov", 3, 7);
        files.put("/new/shoot/b.mov", 3, 7);
        files.put("/new/shoot/c.mov", 9, 1);
        files.put("/new/other/d.mov", 3, 7);
        files.put("/new/readme.txt", 5, 5);
        Self { host: DialogHost::new(), jobs: Arc::new(ManualJobs::new()), files, signature: None, result: None }
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
                        identity: Some(recorded()),
                        page: None,
                        embedded: false,
                    },
                    owner: words.clip_n.replace("{n}", &(n + 1).to_string()),
                    status: words.media_missing.clone(),
                }
            })
            .collect()
    }
}

fn describe(outcome: &RelinkOutcome<u32>, words: &LinkStoryLabels) -> String {
    words.result.replace("{linked}", &outcome.linked.len().to_string()).replace("{offline}", &outcome.offline.len().to_string())
}

fn show_dialog(ui: &mut Ui, args: &mut DialogArgs) {
    let ctx = ui.ctx().clone();
    ensure_host(&ctx);
    let words = LinkStoryLabels::of(ui);
    let id = ui.make_persistent_id("craft-browse-relink-story");
    let state = ui.data_mut(|data| data.get_temp_mut_or_insert_with(id, || Arc::new(Mutex::new(DialogDemo::new()))).clone());
    let mut demo = state.lock().unwrap_or_else(PoisonError::into_inner);

    if args.slow_search {
        ui.horizontal(|ui| {
            if ui.add(Button::new(words.finish_next.clone()).enabled(demo.jobs.waiting() > 0)).clicked() {
                demo.jobs.run_next();
            }
            ui.add(Text::new(words.waiting.replace("{n}", &demo.jobs.waiting().to_string())).content(Content::Muted));
        });
    } else if demo.jobs.run_all() > 0 {
        ctx.request_repaint();
    }

    if dialog_shown(ui, args.open) && dialog_claim(&ctx, "relink", ui.scope_id()) {
        let signature = (args.relink_others, args.exact_names);
        if demo.signature.is_some_and(|previous| previous != signature) {
            demo.host.cancel::<RelinkDialog<u32>>(&ctx);
            demo.host.take::<RelinkDialog<u32>>();
        }
        demo.signature = Some(signature);
        if !demo.host.is_open::<RelinkDialog<u32>>() {
            let options = RelinkOptions {
                relink_others: args.relink_others,
                exact_names: args.exact_names,
                folder: Some(path("/new")),
                jobs: Some(demo.jobs.clone() as Arc<dyn Jobs>),
                ..RelinkOptions::default()
            };
            let dialog = RelinkDialog::new(DialogDemo::rows(&words), demo.files.clone(), options);
            demo.host.open(dialog);
            demo.result = None;
        }
        demo.host.show(&ctx);
        if let Some(result) = demo.host.take::<RelinkDialog<u32>>() {
            demo.result = Some(match result {
                DialogResult::Ok(outcome) => describe(&outcome, &words),
                DialogResult::Cancel => words.cancelled.clone(),
            });
            dialog_closed(ui, &mut args.open);
        }
    } else {
        demo.host.cancel::<RelinkDialog<u32>>(&ctx);
        demo.host.take::<RelinkDialog<u32>>();
        demo.signature = None;
    }
    if let Some(result) = &demo.result {
        ui.label(result.clone());
    }
}
