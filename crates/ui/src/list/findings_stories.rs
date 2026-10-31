//! Story of [`FindingsList`]: a preflight result with errors and warnings.

use egui::Ui;

use super::{Finding, FindingLevel, FindingsList};
use crate::story::{Level, Story, StoryArgs};
use crate::typed_story;

#[derive(StoryArgs, Debug, PartialEq)]
struct FindingsArgs {
    #[arg(range = 0..=4)]
    errors: usize,
    #[arg(range = 0..=5)]
    warnings: usize,
    summary: bool,
    #[arg(range = 240.0..=640.0)]
    width: f32,
}

pub(super) fn stories() -> Vec<Story> {
    vec![typed_story!(
        "composites/findings-list/list",
        Level::Composites,
        "FindingsList",
        "List",
        FindingsArgs { errors: 2, warnings: 3, summary: true, width: 420.0 },
        show_list
    )]
}

const ERRORS: [(&str, Option<&str>); 4] = [
    ("Missing font: Minion Pro Bold", Some("3")),
    ("Missing link: cover-photo.psd", Some("A-2")),
    ("Overset text in \u{201c}Body\u{201d}", Some("4")),
    ("Colour space CMYK expected, RGB found: logo.png", None),
];

const WARNINGS: [(&str, Option<&str>); 5] = [
    ("photo.jpg: effective 96 ppi (< 150)", Some("A-2")),
    ("Image scaled to 312 percent", Some("5")),
    ("Text on a hairline stroke", Some("1")),
    ("Overprint set on a white fill", Some("6")),
    ("Empty text frame", None),
];

fn finding(level: FindingLevel, (message, page): (&str, Option<&str>), target: u64) -> Finding {
    Finding { level, message: message.to_owned(), page: page.map(str::to_owned), target: Some(target) }
}

/// The findings in the app's order (errors first); the row the user activated is named under the list.
fn show_list(ui: &mut Ui, args: &mut FindingsArgs) {
    let mut findings: Vec<Finding> = ERRORS.iter().take(args.errors).enumerate().map(|(n, e)| finding(FindingLevel::Error, *e, n as u64)).collect();
    findings.extend(WARNINGS.iter().take(args.warnings).enumerate().map(|(n, w)| finding(FindingLevel::Warning, *w, 10 + n as u64)));
    let key = ui.make_persistent_id("craft_ui::list::findings-story");
    ui.allocate_ui(egui::vec2(args.width.min(ui.available_width()), 0.0), |ui| {
        let response = FindingsList::new("story-findings", &findings).summary(args.summary).show(ui);
        if let Some(index) = response.activated {
            ui.data_mut(|d| d.insert_temp(key, findings[index].message.clone()));
        }
        if let Some(message) = ui.data(|d| d.get_temp::<String>(key)) {
            ui.label(format!("Activated: {message}"));
        }
    });
}
