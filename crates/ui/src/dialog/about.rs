//! Help › About (F17.02): one window for every app, with the app's own data. Tabs About,
//! Contributors and Models (craftrules standards/contributors.md); Photoshop has no equivalent,
//! so PhotoCraft's is the model.
//!
//! The credits are plain data ([`Credits`]): each app's build script bakes them from
//! `contributors/contributors.json` into static tables, as today, and passes them in. Nothing is
//! read at run time.

use std::borrow::Cow;
use std::cmp::Ordering;

use craft_i18n::{CalendarDate, DateFormat, DateStyle, DateTime};
use egui::{Context, Ui};

use super::dialog::Dialog;
use super::dialog_host::{DialogResult, StackDialog};
use super::footer::Footer;
use super::size::Width;
use super::stack::{DialogId, DialogStack, dialog_id};
use super::state::DialogState;
use crate::behavior::DisplayOrder;
use crate::controls::{Button, ButtonVariant, IconButton, Link, Segmented, Select};
use crate::icons::Icon;
use crate::list::{SortOrder, Table, TableColumn, TableEvent, TableLayout, TableSort};
use crate::localized::words::counts;
use crate::localized::{Labels as _, LocaleNumbers, Words};
use crate::primitives::{Content, Text};
use crate::tab_strip::{Tab, TabEvent, TabKind, TabStrip};
use crate::theme::{Tokens, TokensOf as _, role_format};
use craft_theme::TypeRole;

/// Rows of the Contributors and Models lists.
const LIST_ROWS: f32 = 12.0;

fn list_height_of(ui: &Ui) -> f32 {
    LIST_ROWS * Tokens::of(ui).size.control.md
}
const FLEX_MIN: f32 = 110.0;

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
        view: "about.view" = "View";
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
        /// A signed count in the Contributors table: `{count}` a grouped number.
        plus: "about.plus" = "+{count}";
        minus: "about.minus" = "−{count}";
        /// A model's lines in the Models table: `{added}` and `{deleted}`, grouped numbers.
        lines_range: "about.linesRange" = "+{added} / −{deleted}";
        /// A share under half a percent: `{percent}` the written 1 %.
        under_one: "about.underOne" = "<{percent}";
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
    /// Link text.
    pub label: String,
    /// Target URL, opened in the system browser.
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
    /// GitHub login, shown as `@login` when no name is credited.
    pub login: Cow<'static, str>,
    /// Name the contributor consented to show; used in [`NameMode::DisplayName`].
    pub display_name: Option<Cow<'static, str>>,
    /// Real name the contributor consented to show; used in [`NameMode::RealName`].
    pub real_name: Option<Cow<'static, str>>,
    #[allow(missing_docs)]
    pub prs: u64,
    #[allow(missing_docs)]
    pub commits: u64,
    #[allow(missing_docs)]
    pub lines_added: u64,
    #[allow(missing_docs)]
    pub lines_deleted: u64,
    /// Binary assets added.
    pub binary_added: u64,
    /// Binary assets deleted.
    pub binary_deleted: u64,
    /// ISO 8601 UTC; empty for someone credited only through a merged PR.
    pub first_commit: Cow<'static, str>,
    /// ISO 8601 UTC time of the latest commit; empty for someone credited only through a merged PR.
    pub last_commit: Cow<'static, str>,
}

impl Contributor {
    /// The name `mode` asks for, or `@login` when there is none.
    // pc: crates/ui-egui/src/dialogs/credits.rs:134 Contributor::name
    pub fn name(&self, mode: NameMode) -> Cow<'_, str> {
        let chosen = match mode {
            NameMode::Username => None,
            NameMode::DisplayName => self.display_name.as_ref(),
            NameMode::RealName => self.real_name.as_ref(),
        };
        chosen.map_or_else(|| Cow::Owned(format!("@{}", self.login)), |name| Cow::Borrowed(name.as_ref()))
    }

    /// Lines added minus lines deleted; negative when the contributor removed more than they added.
    pub fn lines_delta(&self) -> i128 {
        i128::from(self.lines_added) - i128::from(self.lines_deleted)
    }

    /// The tooltip line ([`AboutLabels::summary`]) in the interface language: PRs and commits
    /// with the locale's plural forms (`Words::counted`), every number grouped as the region
    /// writes them (`LocaleNumbers`).
    // pc: crates/ui-egui/src/dialogs/credits.rs:148 Contributor::summary
    pub fn summary(&self, ctx: &Context) -> String {
        let labels = AboutLabels::of_ctx(ctx);
        let words = Words::of_ctx(ctx);
        let numbers = LocaleNumbers::of_ctx(ctx);
        let dates = crate::localized::dates(ctx);
        let prs = words.counted(ctx, counts::ABOUT_PRS, &labels.prs, self.prs);
        let commits = words.counted(ctx, counts::ABOUT_COMMITS, &labels.commit_count, self.commits);
        craft_i18n::fmt(
            &labels.summary,
            &[
                ("login", &self.login),
                ("prs", &prs),
                ("commits", &commits),
                ("added", &numbers.count(self.lines_added)),
                ("deleted", &numbers.count(self.lines_deleted)),
                ("delta", &signed(&labels, &numbers, self.lines_delta())),
                ("binAdded", &numbers.count(self.binary_added)),
                ("binDeleted", &numbers.count(self.binary_deleted)),
                ("first", &day(&dates, &self.first_commit)),
                ("last", &day(&dates, &self.last_commit)),
            ],
        )
    }
}

/// An AI model credited in Co-Authored-By trailers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelCredit {
    #[allow(missing_docs)]
    pub company: Cow<'static, str>,
    #[allow(missing_docs)]
    pub model: Cow<'static, str>,
    #[allow(missing_docs)]
    pub version: Cow<'static, str>,
    /// Commits carrying the model's Co-Authored-By trailer.
    pub commits: u64,
    /// Lines added in those commits.
    pub lines_added: u64,
    /// Lines deleted in those commits.
    pub lines_deleted: u64,
}

/// The credits tables.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Credits {
    /// Rows of the Contributors tab.
    pub contributors: Cow<'static, [Contributor]>,
    /// Rows of the Models tab.
    pub models: Cow<'static, [ModelCredit]>,
    /// Every commit, for the counts line and the models' share.
    pub total_commits: u64,
}

/// Which name the Contributors tab shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum NameMode {
    #[default]
    /// Show `@login`.
    Username,
    /// Show the consented display name, else `@login`.
    DisplayName,
    /// Show the consented real name, else `@login`.
    RealName,
}

/// What the Contributors tab sorts by. Names and dates sort A first and oldest first; counts
/// biggest first ([`default_ascending`](Self::default_ascending)).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ContributorSort {
    /// Sort by the name [`NameMode`] selects.
    Name,
    /// Sort by merged PR count.
    Prs,
    /// Sort by commit count.
    Commits,
    /// Sort by lines added.
    LinesAdded,
    /// Sort by lines deleted.
    LinesDeleted,
    /// Sort by lines added minus lines deleted.
    LinesDelta,
    /// Sort by binary assets added.
    BinaryAdded,
    /// Sort by binary assets deleted.
    BinaryDeleted,
    #[default]
    /// Sort by the date of the first commit.
    FirstCommit,
    /// Sort by the date of the latest commit.
    LastCommit,
}

impl ContributorSort {
    /// Every sort key in the order the sort list and the table columns first show them.
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
    /// Whether this key sorts ascending when first chosen: true for names and dates, false for counts.
    pub fn default_ascending(self) -> bool {
        matches!(self, ContributorSort::Name | ContributorSort::FirstCommit | ContributorSort::LastCommit)
    }

    /// The sort list's label and the table's column head: "Lines added" and `+LOC`. The heads
    /// `PRs`, `+LOC`, `−LOC`, `ΔLOC`, `+Bin`, `−Bin` are not translated (PhotoCraft).
    // pc: crates/ui-egui/src/dialogs/credits.rs:90 SortKey::label
    pub fn label(self, labels: &AboutLabels) -> (&str, &str) {
        match self {
            ContributorSort::Name => (&labels.sort_name, &labels.head_name),
            ContributorSort::Prs => (&labels.sort_prs, "PRs"),
            ContributorSort::Commits => (&labels.sort_commits, &labels.commits),
            ContributorSort::LinesAdded => (&labels.sort_lines_added, "+LOC"),
            ContributorSort::LinesDeleted => (&labels.sort_lines_deleted, "−LOC"),
            ContributorSort::LinesDelta => (&labels.sort_line_delta, "ΔLOC"),
            ContributorSort::BinaryAdded => (&labels.sort_binary_added, "+Bin"),
            ContributorSort::BinaryDeleted => (&labels.sort_binary_deleted, "−Bin"),
            ContributorSort::FirstCommit => (&labels.sort_first, &labels.head_first),
            ContributorSort::LastCommit => (&labels.sort_last, &labels.head_last),
        }
    }

    /// Counts line up at the reading end of their column; names and dates start it.
    fn numeric(self) -> bool {
        !self.default_ascending()
    }

    /// The table cell for `c`; the name column shows the name as a link instead.
    fn cell(self, c: &Contributor, numbers: &LocaleNumbers, dates: &DateFormat, labels: &AboutLabels) -> String {
        match self {
            ContributorSort::Name => c.login.to_string(),
            ContributorSort::Prs => numbers.count(c.prs),
            ContributorSort::Commits => numbers.count(c.commits),
            ContributorSort::LinesAdded => numbers.count(c.lines_added),
            ContributorSort::LinesDeleted => numbers.count(c.lines_deleted),
            ContributorSort::LinesDelta => signed(labels, numbers, c.lines_delta()),
            ContributorSort::BinaryAdded => numbers.count(c.binary_added),
            ContributorSort::BinaryDeleted => numbers.count(c.binary_deleted),
            ContributorSort::FirstCommit => day(dates, &c.first_commit),
            ContributorSort::LastCommit => day(dates, &c.last_commit),
        }
    }
}

/// `list` in display order. Names compare case-insensitively without the `@`; someone with no
/// commit date sorts after everyone with one; ties fall back to the name, then the login, so the
/// order is stable.
// pc: crates/ui-egui/src/dialogs/credits.rs:193 sorted, :172 compare
pub fn sorted_contributors(list: &[Contributor], mode: NameMode, sort: ContributorSort, ascending: bool) -> Vec<&Contributor> {
    sorted_indices(list, mode, sort, ascending).into_iter().map(|at| &list[at]).collect()
}

/// [`sorted_contributors`] as positions in `list`, which the table keys its rows by.
fn sorted_indices(list: &[Contributor], mode: NameMode, sort: ContributorSort, ascending: bool) -> Vec<usize> {
    let mut order: Vec<usize> = (0..list.len()).collect();
    order.sort_by(|&a, &b| {
        let (a, b) = (&list[a], &list[b]);
        let ordering = compare(a, b, mode, sort);
        let ordering = if ascending { ordering } else { ordering.reverse() };
        ordering.then_with(|| compare(a, b, mode, ContributorSort::Name)).then_with(|| a.login.cmp(&b.login))
    });
    order
}

/// Alphabetical key: case-insensitive, ignoring the `@` of usernames.
fn name_key(name: &str) -> String {
    name.trim_start_matches('@').to_lowercase()
}

fn compare(a: &Contributor, b: &Contributor, mode: NameMode, sort: ContributorSort) -> Ordering {
    // Someone credited only through a merged PR has no date and sorts after everyone with one.
    let dated = |date: &str| (date.is_empty(), date.to_owned());
    match sort {
        ContributorSort::Name => name_key(&a.name(mode)).cmp(&name_key(&b.name(mode))),
        ContributorSort::Prs => a.prs.cmp(&b.prs),
        ContributorSort::Commits => a.commits.cmp(&b.commits),
        ContributorSort::LinesAdded => a.lines_added.cmp(&b.lines_added),
        ContributorSort::LinesDeleted => a.lines_deleted.cmp(&b.lines_deleted),
        ContributorSort::LinesDelta => a.lines_delta().cmp(&b.lines_delta()),
        ContributorSort::BinaryAdded => a.binary_added.cmp(&b.binary_added),
        ContributorSort::BinaryDeleted => a.binary_deleted.cmp(&b.binary_deleted),
        ContributorSort::FirstCommit => dated(&a.first_commit).cmp(&dated(&b.first_commit)),
        ContributorSort::LastCommit => dated(&a.last_commit).cmp(&dated(&b.last_commit)),
    }
}

/// `+1,234` or `−1,234`.
pub(super) fn signed(labels: &AboutLabels, numbers: &LocaleNumbers, delta: i128) -> String {
    let magnitude = numbers.count(u64::try_from(delta.unsigned_abs()).unwrap_or(u64::MAX));
    craft_i18n::fmt(if delta < 0 { &labels.minus } else { &labels.plus }, &[("count", &magnitude)])
}

/// The date of an ISO 8601 timestamp in the region's date style, "—" for none; a timestamp that
/// doesn't parse shows its first ten characters.
pub(super) fn day(dates: &DateFormat, iso: &str) -> String {
    if iso.is_empty() {
        return "—".to_owned();
    }
    match iso_date(iso) {
        Some(date) => dates.format(DateTime { date, hour: 0, minute: 0, second: 0 }, DateStyle::DATE),
        None => iso.get(..10).unwrap_or(iso).to_owned(),
    }
}

/// The calendar date of the `YYYY-MM-DD` that starts an ISO 8601 timestamp.
fn iso_date(iso: &str) -> Option<CalendarDate> {
    let mut parts = iso.get(..10)?.split('-');
    let (year, month, day) = (parts.next()?.parse().ok()?, parts.next()?.parse().ok()?, parts.next()?.parse().ok()?);
    CalendarDate::new(year, month, day)
}

/// A model's share of all commits, in whole percent, written by `percent` (the region's
/// `Format::percent`); a model with any commits never reads 0 % (`<1%`).
// pc: crates/ui-egui/src/dialogs/credits.rs:493 percent
pub fn commit_share(commits: u64, total: u64) -> CommitShare {
    let ratio = commits as f64 / total.max(1) as f64;
    if commits > 0 && ratio < 0.005 { CommitShare::UnderOne } else { CommitShare::Ratio(ratio.min(1.0)) }
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
    /// The About tab (app name, blurb, links).
    About,
    /// The Contributors tab.
    Contributors,
    /// The AI models tab.
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

/// The Models table's columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ModelColumn {
    Company,
    Model,
    Version,
    Commits,
    Share,
    Lines,
}

impl ModelColumn {
    const ALL: [ModelColumn; 6] =
        [ModelColumn::Company, ModelColumn::Model, ModelColumn::Version, ModelColumn::Commits, ModelColumn::Share, ModelColumn::Lines];

    fn head(self, labels: &AboutLabels) -> &str {
        match self {
            ModelColumn::Company => &labels.company,
            ModelColumn::Model => &labels.model,
            ModelColumn::Version => &labels.model_version,
            ModelColumn::Commits => &labels.commits,
            ModelColumn::Share => &labels.share,
            ModelColumn::Lines => &labels.lines,
        }
    }

    fn numeric(self) -> bool {
        matches!(self, ModelColumn::Commits | ModelColumn::Share | ModelColumn::Lines)
    }

    /// The cell for `m`; `total` counts every commit.
    pub(super) fn cell(self, m: &ModelCredit, total: u64, numbers: &LocaleNumbers, labels: &AboutLabels) -> String {
        match self {
            ModelColumn::Company => m.company.to_string(),
            ModelColumn::Model => m.model.to_string(),
            ModelColumn::Version => m.version.to_string(),
            ModelColumn::Commits => numbers.count(m.commits),
            ModelColumn::Share => match commit_share(m.commits, total) {
                CommitShare::UnderOne => craft_i18n::fmt(&labels.under_one, &[("percent", &numbers.percent(0.01))]),
                CommitShare::Ratio(ratio) => numbers.percent(ratio),
            },
            ModelColumn::Lines => {
                craft_i18n::fmt(&labels.lines_range, &[("added", &numbers.count(m.lines_added)), ("deleted", &numbers.count(m.lines_deleted))])
            }
        }
    }
}

/// Every commit, or the busiest model's count if the total somehow trails it.
pub(super) fn share_total(credits: &Credits) -> u64 {
    credits.total_commits.max(credits.models.iter().map(|m| m.commits).max().unwrap_or(0))
}

/// Table rows in display order, keyed by position in the credits list.
struct RowOrder(Vec<usize>);

impl DisplayOrder<usize> for RowOrder {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn key(&self, at: usize) -> &usize {
        &self.0[at]
    }
}

/// The About window, 700 px wide on every tab so switching tabs doesn't resize it.
///
/// - **About**: the blurb, "Version …", the community button and the link row centred, the
///   built-with line faint. The three tabs are the library's tab strip.
/// - **Contributors**: Show (Username / Display name / Real name), Sort (the list) with ▲/▼ to
///   reverse, Grab bag / Table; the counts line; the names as links to `github.com/<login>`
///   flowing as a grab bag (14 × 8 spacing) or a sortable table (the controls part's
///   `list::Table` with `TableSort`, the header click sorts).
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
    contributor_layout: TableLayout<ContributorSort>,
    model_layout: TableLayout<ModelColumn>,
    state: DialogState<()>,
    clicked: Option<String>,
}

impl AboutDialog {
    /// Creates the dialog on the About tab with the default sort (first commit, ascending) and no hidden columns.
    pub fn new(info: AboutInfo, credits: Credits) -> Self {
        Self {
            id: dialog_id("about"),
            info,
            credits,
            tab: AboutTab::About,
            view: ContributorsView { names: NameMode::Username, sort: ContributorSort::FirstCommit, ascending: true, table: false },
            contributor_layout: TableLayout { order: ContributorSort::ALL.to_vec(), hidden: Vec::new(), widths: Vec::new(), sort: None },
            model_layout: TableLayout { order: ModelColumn::ALL.to_vec(), hidden: Vec::new(), widths: Vec::new(), sort: None },
            state: DialogState::new(),
            clicked: None,
        }
    }

    /// Opens on `tab` (automation's `ui.dialog.set`).
    pub fn tab(mut self, tab: AboutTab) -> Self {
        self.tab = tab;
        self
    }

    /// The tab currently showing.
    pub fn current_tab(&self) -> AboutTab {
        self.tab
    }

    /// Switches to `tab`.
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
    fn show(&mut self, ctx: &Context, stack: &DialogStack) -> Option<DialogResult<()>> {
        let labels = AboutLabels::of_ctx(ctx);
        let id = self.id;
        let title = craft_i18n::fmt(&labels.title, &[("app", self.info.app_name.as_str())]);
        let strip_label = title.clone();
        let Self { info, credits, tab, view, contributor_layout, model_layout, state, clicked, .. } = self;
        let response = Dialog::with_id(id, title)
            .stack(stack)
            .width(Width::Fixed(crate::style::dialog::DialogStyle::new(&crate::theme::current(ctx)).width_wide))
            .footer(Footer::standard().without_cancel())
            .show(ctx, state, &mut (), |ui, ()| {
                let tabs = [
                    Tab::new(AboutTab::About, labels.about_tab.as_str()),
                    Tab::new(AboutTab::Contributors, labels.contributors_tab.as_str()),
                    Tab::new(AboutTab::Models, labels.models_tab.as_str()),
                ];
                let strip = TabStrip::new(id.with("tabs"), strip_label.as_str(), TabKind::Text).selected(&*tab).show(ui, &tabs);
                for event in strip.events {
                    if let TabEvent::Activate(next) = event {
                        *tab = next;
                    }
                }
                ui.add_space(Tokens::of(ui).space.pad.dialog);
                match *tab {
                    AboutTab::About => about_tab(ui, &labels, info, clicked),
                    AboutTab::Contributors => contributors_tab(ui, &labels, credits, view, contributor_layout, clicked),
                    AboutTab::Models => models_tab(ui, &labels, credits, model_layout),
                }
            });
        response.outcome.filter(|outcome| outcome.closes()).map(|_| DialogResult::Ok(()))
    }

    fn confirm(&mut self, _ctx: &Context) -> Option<()> {
        Some(())
    }
}

/// The About tab: the blurb, the version, the community button and the link row centred, the
/// built-with line faint.
fn about_tab(ui: &mut Ui, labels: &AboutLabels, info: &AboutInfo, clicked: &mut Option<String>) {
    ui.scope(|ui| {
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
        ui.add(Text::new(info.blurb.as_str()));
        ui.add(Text::new(craft_i18n::fmt(&labels.version, &[("version", info.version.as_str())])));
    });
    ui.add_space(ui.spacing().item_spacing.y * 2.0);
    ui.vertical_centered(|ui| {
        if let Some(community) = &info.community
            && ui
                .add(
                    Button::new(community.label.as_str())
                        .variant(ButtonVariant::Primary)
                        .min_width(crate::style::dialog::DialogStyle::new(&Tokens::of(ui)).width_wide / 3.0),
                )
                .clicked()
        {
            *clicked = Some(community.url.clone());
        }
        ui.add_space(ui.spacing().item_spacing.y);
        link_row(ui, &info.links, clicked);
    });
    ui.add_space(ui.spacing().item_spacing.y);
    ui.add(Text::new(info.built_with.as_str()).content(Content::Faint));
}

/// `links` in one row, "·" between them, centred in the available width.
fn link_row(ui: &mut Ui, links: &[AboutLink], clicked: &mut Option<String>) {
    const SEPARATOR: &str = "·";
    let gap = ui.spacing().item_spacing.x;
    let total: f32 = links.iter().map(|link| measure(ui, &link.label, TypeRole::Body)).sum::<f32>()
        + links.len().saturating_sub(1) as f32 * (measure(ui, SEPARATOR, TypeRole::Body) + 2.0 * gap);
    ui.horizontal(|ui| {
        ui.add_space(((ui.available_width() - total) / 2.0).max(0.0));
        for (at, link) in links.iter().enumerate() {
            if at > 0 {
                ui.add(Text::new(SEPARATOR).content(Content::Faint));
            }
            if ui.add(Link::new(link.label.as_str()).id_salt(("link", at))).clicked() {
                *clicked = Some(link.url.clone());
            }
        }
    });
}

/// The width of `text` in `role`'s font, on one line.
fn measure(ui: &Ui, text: &str, role: TypeRole) -> f32 {
    let format = role_format(&Tokens::of(ui), role, egui::Color32::WHITE);
    ui.painter().layout_no_wrap(text.to_owned(), format.font_id, egui::Color32::WHITE).size().x
}

/// A column's width: its widest cell or its head with the sort chevron's room, and the padding.
fn column_width(ui: &Ui, head: &str, cells: impl Iterator<Item = String>) -> f32 {
    let style = Table::style(&Tokens::of(ui));
    let widest = cells.map(|cell| measure(ui, &cell, TypeRole::Body)).fold(0.0, f32::max);
    let head = measure(ui, head, TypeRole::Label) + style.sort_gap + style.sort_glyph;
    (widest.max(head) + 2.0 * style.cell_pad_x).ceil()
}

/// A contributor's name linking to their GitHub profile, with the full summary as its tooltip.
fn contributor_link(ui: &mut Ui, c: &Contributor, names: NameMode, salt: impl std::hash::Hash + std::fmt::Debug, clicked: &mut Option<String>) {
    let response = ui.add(Link::new(c.name(names)).id_salt(salt)).on_hover_ui(|ui| {
        ui.label(c.summary(ui.ctx()));
    });
    if response.clicked() {
        *clicked = Some(format!("https://github.com/{}", c.login));
    }
}

/// The Contributors tab: the Show, Sort and view controls, the counts line, and the list.
fn contributors_tab(
    ui: &mut Ui,
    labels: &AboutLabels,
    credits: &Credits,
    view: &mut ContributorsView,
    layout: &mut TableLayout<ContributorSort>,
    clicked: &mut Option<String>,
) {
    ui.horizontal_wrapped(|ui| {
        ui.add(Text::new(labels.show.as_str()));
        Segmented::new("about-names", labels.show.as_str(), &mut view.names)
            .item(NameMode::Username, labels.username.as_str())
            .item(NameMode::DisplayName, labels.display_name.as_str())
            .item(NameMode::RealName, labels.real_name.as_str())
            .show(ui);
        ui.separator();
        ui.add(Text::new(labels.sort.as_str()));
        let before = view.sort;
        let mut sort = Select::new("about-sort", labels.sort.as_str(), &mut view.sort);
        for key in ContributorSort::ALL {
            sort = sort.option(key, key.label(labels).0);
        }
        let _ = sort.show(ui);
        if view.sort != before {
            view.ascending = view.sort.default_ascending();
        }
        let chevron = if view.ascending { Icon::ChevronUp } else { Icon::ChevronDown };
        if ui.add(IconButton::new(chevron, labels.reverse.as_str()).id_salt("about-reverse")).clicked() {
            view.ascending = !view.ascending;
        }
        ui.separator();
        Segmented::new("about-view", labels.view.as_str(), &mut view.table)
            .item(false, labels.grab_bag.as_str())
            .item(true, labels.table.as_str())
            .show(ui);
    });
    let numbers = LocaleNumbers::of_ctx(ui.ctx());
    let list: &[Contributor] = &credits.contributors;
    let counts =
        craft_i18n::fmt(&labels.counts, &[("contributors", &numbers.count(list.len() as u64)), ("commits", &numbers.count(credits.total_commits))]);
    ui.add(Text::new(counts).role(TypeRole::Caption).content(Content::Faint));
    ui.separator();
    if list.is_empty() {
        ui.add(Text::new(labels.no_contributors.as_str()));
        return;
    }
    let list_height = list_height_of(ui);
    let order = sorted_indices(list, view.names, view.sort, view.ascending);
    if view.table {
        contributors_table(ui, labels, list, order, view, layout, clicked);
    } else {
        egui::ScrollArea::vertical().id_salt("about-bag").auto_shrink([false, false]).min_scrolled_height(list_height).max_height(list_height).show(
            ui,
            |ui| {
                ui.spacing_mut().item_spacing = {
                    let gap = Tokens::of(ui).space.gap;
                    egui::vec2(gap.section, gap.stack)
                };
                // Each name is one unit: a link that doesn't fit moves whole to the next line.
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                ui.horizontal_wrapped(|ui| {
                    for &at in &order {
                        contributor_link(ui, &list[at], view.names, ("bag", at), clicked);
                    }
                });
            },
        );
    }
}

/// The sortable table: a header click sorts by its column, and again reverses it.
fn contributors_table(
    ui: &mut Ui,
    labels: &AboutLabels,
    list: &[Contributor],
    order: Vec<usize>,
    view: &mut ContributorsView,
    layout: &mut TableLayout<ContributorSort>,
    clicked: &mut Option<String>,
) {
    let list_height = list_height_of(ui);
    let numbers = LocaleNumbers::of_ctx(ui.ctx());
    let dates = crate::localized::dates(ui.ctx());
    let columns: Vec<TableColumn<ContributorSort>> = ContributorSort::ALL
        .into_iter()
        .map(|key| {
            let (name, head) = key.label(labels);
            let first = if key.default_ascending() { SortOrder::Ascending } else { SortOrder::Descending };
            let column = TableColumn::new(key, head).first_order(first).tip(name);
            let column = if key.numeric() { column.numeric() } else { column };
            if key == ContributorSort::Name {
                column.min_width(FLEX_MIN)
            } else {
                let width = column_width(ui, head, list.iter().map(|c| key.cell(c, &numbers, &dates, labels)));
                column.min_width(width).initial_width(width)
            }
        })
        .collect();
    layout.sort = Some(TableSort { column: view.sort, order: if view.ascending { SortOrder::Ascending } else { SortOrder::Descending } });
    let header = Table::style(&Tokens::of(ui)).header_height;
    let rows = RowOrder(order);
    let names = view.names;
    let out = Table::new("about-contributors", labels.contributors_tab.as_str(), &columns, layout, &rows)
        .movable(false)
        .max_height(list_height - header)
        .show(ui, |ui, &at, column, cx| {
            let c = &list[at];
            if *column == ContributorSort::Name {
                contributor_link(ui, c, names, ("name", at), clicked);
            } else {
                cx.label(ui, column.cell(c, &numbers, &dates, labels));
            }
        });
    for event in out.events {
        if let TableEvent::Sort(sort) = event {
            view.sort = sort.column;
            view.ascending = sort.order == SortOrder::Ascending;
        }
    }
}

/// The Models tab: a table that doesn't sort, as tall as its rows up to `LIST_ROWS` rows of the control height.
fn models_tab(ui: &mut Ui, labels: &AboutLabels, credits: &Credits, layout: &mut TableLayout<ModelColumn>) {
    let models: &[ModelCredit] = &credits.models;
    if models.is_empty() {
        ui.add(Text::new(labels.no_models.as_str()));
        return;
    }
    let list_height = list_height_of(ui);
    let numbers = LocaleNumbers::of_ctx(ui.ctx());
    let total = share_total(credits);
    let columns: Vec<TableColumn<ModelColumn>> = ModelColumn::ALL
        .into_iter()
        .map(|key| {
            let head = key.head(labels);
            let column = TableColumn::new(key, head).sortable(false);
            let column = if key.numeric() { column.numeric() } else { column };
            if key == ModelColumn::Model {
                column.min_width(FLEX_MIN)
            } else {
                let width = column_width(ui, head, models.iter().map(|m| key.cell(m, total, &numbers, labels)));
                column.min_width(width).initial_width(width)
            }
        })
        .collect();
    let style = Table::style(&Tokens::of(ui));
    let body = (models.len() as f32 * style.row_height).min(list_height - style.header_height);
    let rows = RowOrder((0..models.len()).collect());
    let _ = Table::new("about-models", labels.models_tab.as_str(), &columns, layout, &rows).movable(false).max_height(body).show(
        ui,
        |ui, &at, column, cx| {
            cx.label(ui, column.cell(&models[at], total, &numbers, labels));
        },
    );
}
