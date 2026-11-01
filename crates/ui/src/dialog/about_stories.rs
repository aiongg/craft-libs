//! Story of the About dialog over sample credits: the About tab, the Contributors tab (a grab
//! bag of names or a sortable table) and the Models table. Links are not opened; the story
//! shows the last one the dialog handed over.

use std::borrow::Cow;

use egui::Ui;

use super::framework_stories::{claim, drive};
use super::{AboutDialog, AboutInfo, AboutLink, AboutTab, Contributor, Credits, ModelCredit};
use crate::localized::Labels as _;
use crate::story::{Level, Story, StoryArgs, StoryChoice, StoryLabels};
use crate::typed_story;

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum TabArg {
    About,
    Contributors,
    Models,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct AboutArgs {
    open: bool,
    tab: TabArg,
    #[arg(name = "community button")]
    community: bool,
}

pub(super) fn stories() -> Vec<Story> {
    vec![typed_story!(
        "patterns/dialog/about",
        Level::Patterns,
        "Dialog",
        "About",
        AboutArgs { open: true, tab: TabArg::About, community: true },
        show_about
    )]
}

fn link(label: &str, url: &str) -> AboutLink {
    AboutLink { label: label.to_owned(), url: url.to_owned() }
}

fn info(words: &StoryLabels, community: bool) -> AboutInfo {
    AboutInfo {
        app_name: words.app_name.clone(),
        blurb: words.about_blurb.clone(),
        version: "0.42.1 (a1b2c3d, 2026-10-30)".to_owned(),
        community: community.then(|| link(&words.about_community, "https://example.org/community")),
        links: vec![link(&words.about_website, "https://example.org"), link(&words.about_source, "https://example.org/source")],
        built_with: "egui · wgpu · craft-ui".to_owned(),
    }
}

/// Six sample contributors: some with a display or real name, one credited only through a
/// merged PR (no commit dates).
fn credits() -> Credits {
    let person =
        |login: &'static str, display: Option<&'static str>, real: Option<&'static str>, prs, commits, added, deleted, first, last| Contributor {
            login: Cow::Borrowed(login),
            display_name: display.map(Cow::Borrowed),
            real_name: real.map(Cow::Borrowed),
            prs,
            commits,
            lines_added: added,
            lines_deleted: deleted,
            binary_added: if commits > 100 { 2_400_000 } else { 0 },
            binary_deleted: if commits > 100 { 310_000 } else { 0 },
            first_commit: Cow::Borrowed(first),
            last_commit: Cow::Borrowed(last),
        };
    let contributors = vec![
        person("ada", Some("Ada L."), Some("Ada Lovelace"), 41, 212, 48_210, 12_044, "2025-03-02T09:14:00Z", "2026-10-29T17:40:00Z"),
        person("grace", Some("Grace H."), None, 18, 96, 15_730, 4_102, "2025-06-18T12:00:00Z", "2026-10-21T08:05:00Z"),
        person("linus", None, Some("Linus T."), 7, 31, 4_290, 980, "2026-01-11T20:31:00Z", "2026-09-30T11:11:00Z"),
        person("margaret", Some("Margaret H."), Some("Margaret Hamilton"), 12, 54, 9_875, 2_310, "2025-09-04T07:45:00Z", "2026-10-02T14:20:00Z"),
        person("tim", None, None, 3, 9, 640, 85, "2026-05-20T16:00:00Z", "2026-06-02T10:30:00Z"),
        person("guest", None, None, 1, 0, 0, 0, "", ""),
    ];
    let model = |company: &'static str, model: &'static str, version: &'static str, commits, added, deleted| ModelCredit {
        company: Cow::Borrowed(company),
        model: Cow::Borrowed(model),
        version: Cow::Borrowed(version),
        commits,
        lines_added: added,
        lines_deleted: deleted,
    };
    let models = vec![
        model("Example AI", "Atlas", "4.1", 88, 21_300, 5_020),
        model("Example AI", "Atlas Mini", "3.2", 12, 1_980, 410),
        model("Sample Labs", "Compass", "2.0", 3, 270, 40),
    ];
    Credits { contributors: Cow::Owned(contributors), models: Cow::Owned(models), total_commits: 520 }
}

/// About over sample credits; the tab argument picks the tab it opens on.
fn show_about(ui: &mut Ui, args: &mut AboutArgs) {
    if !claim(ui.ctx(), "about", ui.scope_id()) {
        return;
    }
    let (tab, community, words, ctx) = (args.tab, args.community, StoryLabels::of(ui), ui.ctx().clone());
    let link_key = ui.make_persistent_id("craft_ui::dialog::about_stories::link");
    drive(
        ui,
        "about",
        &mut args.open,
        (tab, community),
        || {
            let tab = match tab {
                TabArg::About => AboutTab::About,
                TabArg::Contributors => AboutTab::Contributors,
                TabArg::Models => AboutTab::Models,
            };
            ctx.data_mut(|d| d.remove_temp::<String>(link_key));
            AboutDialog::new(info(&words, community), credits()).tab(tab)
        },
        |dialog| {
            if let Some(url) = dialog.take_link() {
                ctx.data_mut(|d| d.insert_temp(link_key, url));
            }
        },
        |()| String::new(),
    );
    if let Some(url) = ui.data(|d| d.get_temp::<String>(link_key)) {
        ui.label(craft_i18n::fmt(&StoryLabels::of(ui).last_link, &[("url", &url)]));
    }
}
