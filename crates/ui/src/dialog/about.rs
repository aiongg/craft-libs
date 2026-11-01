#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! Help › About (F17.02): one window for every app, with the app's own data. Tabs About,
//! Contributors and Models (craftrules standards/contributors.md); Photoshop has no equivalent,
//! so PhotoCraft's is the model.
//!
//! The credits are plain data ([`Credits`]): each app's build script bakes them from
//! `contributors/contributors.json` into static tables, as today, and passes them in. Nothing is
//! read at run time.

use std::borrow::Cow;

use egui::Context;

use super::dialog_host::{DialogResult, StackDialog};
use super::stack::{DialogId, DialogStack, dialog_id};

crate::words! {
    /// The About window's words, English by default. `{app}` is the app's name.
    #[non_exhaustive]
    pub struct AboutLabels {
        title: "about.title" = "About {app}";
        about_tab: "about.about" = "About";
        contributors_tab: "about.contributors" = "Contributors";
        models_tab: "about.models" = "Models";
        version: "about.version" = "Version {version}";
        show: "about.show" = "Show";
        username: "about.username" = "Username";
        display_name: "about.displayName" = "Display name";
        real_name: "about.realName" = "Real name";
        sort: "about.sort" = "Sort";
        reverse: "about.reverse" = "Reverse the order";
        grab_bag: "about.grabBag" = "Grab bag";
        table: "about.table" = "Table";
        /// `{contributors}` and `{commits}`, both grouped numbers.
        counts: "about.counts" = "Contributors: {contributors} · Commits: {commits}";
        no_contributors: "about.noContributors" = "No contributor data was built into this copy.";
        no_models: "about.noModels" = "No model credits were built into this copy.";
        sort_name: "about.sortName" = "Name (A–Z)";
        sort_prs: "about.sortPrs" = "Merged PRs";
        sort_commits: "about.sortCommits" = "Commits";
        sort_lines_added: "about.sortLinesAdded" = "Lines added";
        sort_lines_deleted: "about.sortLinesDeleted" = "Lines deleted";
        sort_line_delta: "about.sortLineDelta" = "Line delta";
        sort_binary_added: "about.sortBinaryAdded" = "Binary assets added";
        sort_binary_deleted: "about.sortBinaryDeleted" = "Binary assets removed";
        sort_first: "about.sortFirst" = "First commit";
        sort_last: "about.sortLast" = "Last commit";
        head_name: "about.headName" = "Name";
        head_first: "about.headFirst" = "First";
        head_last: "about.headLast" = "Last";
        company: "about.company" = "Company";
        model: "about.model" = "Model";
        model_version: "about.modelVersion" = "Version";
        commits: "about.commits" = "Commits";
        share: "about.share" = "% of all commits";
        lines: "about.lines" = "Lines +/−";
        /// A contributor's tooltip; `{prs}` and `{commits}` are the two counted words below.
        summary: "about.summary" = "@{login}: {prs}, {commits}, +{added} / −{deleted} lines (Δ {delta}), +{binAdded} / −{binDeleted} binary assets, {first} – {last}";
        /// Counted (`about.prs` count word): "1 PR", "3 PRs".
        prs: "about.prs" = "{n} PRs";
        /// Counted (`about.commitCount` count word): "1 commit", "12 commits".
        commit_count: "about.commitCount" = "{n} commits";
    }
}

/// A link on the About tab.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AboutLink {
    pub label: String,
    pub url: String,
}

/// The About tab's contents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AboutInfo {
    /// "PhotoCraft".
    pub app_name: String,
    /// One sentence: "PhotoCraft — an open-source, native image editor written in Rust."
    pub blurb: String,
    /// The long version ("0.42.1 (a1b2c3d, 2026-10-30)").
    pub version: String,
    /// The prominent button ("Join us on Discord"), if any.
    pub community: Option<AboutLink>,
    /// The centred link row ("PhotoCraft website · GitHub · ArtCraft").
    pub links: Vec<AboutLink>,
    /// The faint last line ("egui · wgpu · photocraft-engine").
    pub built_with: String,
}

/// One contributor, as the build script bakes them. Display and real names are shown only with
/// recorded consent; without one the `@login` stands in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Contributor {
    pub login: Cow<'static, str>,
    pub display_name: Option<Cow<'static, str>>,
    pub real_name: Option<Cow<'static, str>>,
    pub prs: u64,
    pub commits: u64,
    pub lines_added: u64,
    pub lines_deleted: u64,
    pub binary_added: u64,
    pub binary_deleted: u64,
    /// ISO 8601 UTC; empty for someone credited only through a merged PR.
    pub first_commit: Cow<'static, str>,
    pub last_commit: Cow<'static, str>,
}

impl Contributor {
    /// The name `mode` asks for, or `@login` when there is none.
    // pc: crates/ui-egui/src/dialogs/credits.rs:134 Contributor::name
    pub fn name(&self, mode: NameMode) -> Cow<'_, str> {
        todo!()
    }

    pub fn lines_delta(&self) -> i128 {
        i128::from(self.lines_added) - i128::from(self.lines_deleted)
    }

    /// The tooltip line ([`AboutLabels::summary`]) in the interface language: PRs and commits
    /// with the locale's plural forms (`Words::counted`), every number grouped as the region
    /// writes them (`LocaleNumbers`).
    // pc: crates/ui-egui/src/dialogs/credits.rs:148 Contributor::summary
    pub fn summary(&self, ctx: &Context) -> String {
        todo!()
    }
}

/// An AI model credited in Co-Authored-By trailers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelCredit {
    pub company: Cow<'static, str>,
    pub model: Cow<'static, str>,
    pub version: Cow<'static, str>,
    pub commits: u64,
    pub lines_added: u64,
    pub lines_deleted: u64,
}

/// The credits tables.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Credits {
    pub contributors: Cow<'static, [Contributor]>,
    pub models: Cow<'static, [ModelCredit]>,
    /// Every commit, for the counts line and the models' share.
    pub total_commits: u64,
}

/// Which name the Contributors tab shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum NameMode {
    #[default]
    Username,
    DisplayName,
    RealName,
}

/// What the Contributors tab sorts by. Names and dates sort A first and oldest first; counts
/// biggest first ([`default_ascending`](Self::default_ascending)).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ContributorSort {
    Name,
    Prs,
    Commits,
    LinesAdded,
    LinesDeleted,
    LinesDelta,
    BinaryAdded,
    BinaryDeleted,
    #[default]
    FirstCommit,
    LastCommit,
}

impl ContributorSort {
    pub const ALL: [ContributorSort; 10] = [
        ContributorSort::Name,
        ContributorSort::Prs,
        ContributorSort::Commits,
        ContributorSort::LinesAdded,
        ContributorSort::LinesDeleted,
        ContributorSort::LinesDelta,
        ContributorSort::BinaryAdded,
        ContributorSort::BinaryDeleted,
        ContributorSort::FirstCommit,
        ContributorSort::LastCommit,
    ];

    // pc: crates/ui-egui/src/dialogs/credits.rs:106 default_ascending
    pub fn default_ascending(self) -> bool {
        matches!(self, ContributorSort::Name | ContributorSort::FirstCommit | ContributorSort::LastCommit)
    }

    /// The sort list's label and the table's column head: "Lines added" and `+LOC`. The heads
    /// `PRs`, `+LOC`, `−LOC`, `ΔLOC`, `+Bin`, `−Bin` are not translated (PhotoCraft).
    // pc: crates/ui-egui/src/dialogs/credits.rs:90 SortKey::label
    pub fn label(self, labels: &AboutLabels) -> (&str, &str) {
        todo!()
    }
}

/// `list` in display order. Names compare case-insensitively without the `@`; someone with no
/// commit date sorts after everyone with one; ties fall back to the name, then the login, so the
/// order is stable.
// pc: crates/ui-egui/src/dialogs/credits.rs:193 sorted, :172 compare
pub fn sorted_contributors(list: &[Contributor], mode: NameMode, sort: ContributorSort, ascending: bool) -> Vec<&Contributor> {
    todo!()
}

/// A model's share of all commits, in whole percent, written by `percent` (the region's
/// `Format::percent`); a model with any commits never reads 0 % (`<1%`).
// pc: crates/ui-egui/src/dialogs/credits.rs:493 percent
pub fn commit_share(commits: u64, total: u64) -> CommitShare {
    todo!()
}

/// [`commit_share`]'s answer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CommitShare {
    /// Above zero but under half a percent.
    UnderOne,
    /// A ratio from 0 to 1, rounded to whole percent when written.
    Ratio(f64),
}

/// Which tab is up.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AboutTab {
    #[default]
    About,
    Contributors,
    Models,
}

/// The Contributors tab's view state, kept for the window's life.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ContributorsView {
    names: NameMode,
    sort: ContributorSort,
    ascending: bool,
    table: bool,
}

/// The About window, 700 px wide on every tab so switching tabs doesn't resize it.
///
/// - **About**: the blurb, "Version …", the community button and the link row centred, the
///   built-with line faint. The three tabs are the library's tab strip.
/// - **Contributors**: Show (Username / Display name / Real name), Sort (the list) with ▲/▼ to
///   reverse, Grab bag / Table; the counts line; the names as links to `github.com/<login>`
///   flowing as a grab bag (14 × 8 spacing) or a sortable table (the controls part's
///   `list::Table` with `TableSort`, the header click sorts; `// needs: controls list::Table`).
///   The list is 340 px tall on both views.
/// - **Models**: a `list::Table` (columns not sortable) of Company, Model, Version, Commits,
///   % of all commits, Lines +/−, as tall as its rows up to 340 px; the model name takes the
///   spare width.
///
/// The tabs are a tab strip (one Tab stop, arrows switch); every link and control is reachable
/// with Tab; Esc and the close box close it. Numbers are grouped as the region writes them.
///
/// Links never open from here: a click is handed to the app, which opens the URL with the
/// platform (`craft_platform::open::open_external`; egui's `open_url` is unreliable on desktop):
/// `dialogs.get_mut::<AboutDialog>().and_then(AboutDialog::take_link)`.
pub struct AboutDialog {
    id: DialogId,
    info: AboutInfo,
    credits: Credits,
    tab: AboutTab,
    view: ContributorsView,
    clicked: Option<String>,
}

impl AboutDialog {
    pub fn new(info: AboutInfo, credits: Credits) -> Self {
        Self {
            id: dialog_id("about"),
            info,
            credits,
            tab: AboutTab::About,
            view: ContributorsView { names: NameMode::Username, sort: ContributorSort::FirstCommit, ascending: true, table: false },
            clicked: None,
        }
    }

    /// Opens on `tab` (automation's `ui.dialog.set`).
    pub fn tab(mut self, tab: AboutTab) -> Self {
        self.tab = tab;
        self
    }

    pub fn current_tab(&self) -> AboutTab {
        self.tab
    }

    pub fn set_tab(&mut self, tab: AboutTab) {
        self.tab = tab;
    }

    /// The URL of a link clicked since the last call.
    pub fn take_link(&mut self) -> Option<String> {
        self.clicked.take()
    }
}

impl StackDialog for AboutDialog {
    type Result = ();

    fn id(&self) -> DialogId {
        self.id
    }

    // pc: crates/ui-egui/src/dialogs.rs:351 DialogKind::About, :753 about_tab_body
    // pc: crates/ui-egui/src/dialogs/credits.rs:256 contributors_in, :511 models_in
    // needs: controls list::table (the sortable column table for the Contributors and Models tables)
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<()>> {
        todo!()
    }
}
