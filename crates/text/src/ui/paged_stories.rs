//! Stories of the paged-text controls. Each draws the real control over sample values and applies
//! the events it reports to its demo state as an app would. The line under a story names the last
//! event. Sample texts are words of [`PagedStoryLabels`].
//!
//! | Story id | Shows |
//! |---|---|
//! | `composites/text-ruler/word` | Word's ruler over a 6.5 in column, a hanging indent, one stop of each kind, the selector |
//! | `composites/text-ruler/tabs-panel` | InDesign's Tabs panel strip fitted to its width, a selected decimal stop |
//! | `patterns/tabs/dialog` | the Tabs dialog with three stops, one with a dot leader |
//! | `patterns/paragraph/dialog` | the Paragraph dialog with a hanging indent and Exactly 18 pt |
//! | `patterns/notes/editor` | three notes paragraphs, a bulleted level-1 list and a bold run |
//! | `patterns/columns/dialog` | Word's More Columns over three unequal columns |
//! | `patterns/hyperlink/dialog` | Edit Hyperlink over a web address with a ScreenTip |
//! | `patterns/find/bar` | the bar with a query and 3 of 12, options on (with `find`) |
//! | `patterns/find/dialog` | Word's Find and Replace on the Replace tab (with `find`) |
//! | `composites/style-chip/picker` | a chip over a colour style, four styles in two folders |

use craft_ui::canvas::Rulers;
use craft_ui::edit::EditEvent;
use craft_ui::localized::{Labels as _, fmt};
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, NoArgs, Story, StoryArgs, StoryLabels, drive, lock, shared};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _};
use craft_ui::typed_story;
use egui::{Rect, Ui, vec2};

use super::character::{LineSpacing, ParagraphAlign, ParagraphDialog, ParagraphFormat, SpecialIndent};
use super::columns::{TextColumns, TextColumnsDialog};
use super::hyperlink::{CurrentLink, Hyperlink, HyperlinkDialog, LinkTarget};
use super::notes::{NotesEditor, NotesParagraph, NotesRun, NotesText};
use super::ruler::{RulerValues, TabAlign, TabLeader, TabStop, TextRuler};
use super::style_panel::{NamedStyleRow, StyleChip, StylePickerView, StylePreview};
use super::tabs_dialog::TabStopsDialog;
use crate::lists::ListMarker;

/// `composites/text-ruler/word`, `composites/text-ruler/tabs-panel`, `patterns/tabs/dialog`,
/// `patterns/paragraph/dialog`, `patterns/notes/editor`, `patterns/columns/dialog`,
/// `patterns/hyperlink/dialog`, `patterns/find/bar`, `patterns/find/dialog` (with `find`),
/// `composites/style-chip/picker`.
pub fn stories() -> Vec<Story> {
    #[cfg_attr(not(feature = "find"), expect(unused_mut, reason = "only the find stories are added after the list"))]
    let mut all = vec![
        typed_story!("composites/text-ruler/word", Level::Composites, "Text ruler", "Word", NoArgs {}, ruler_word),
        typed_story!("composites/text-ruler/tabs-panel", Level::Composites, "Text ruler", "Tabs panel", NoArgs {}, ruler_panel),
        typed_story!("patterns/tabs/dialog", Level::Patterns, "Tabs", "Dialog", DialogArgs { open: true }, tabs_dialog),
        typed_story!("patterns/paragraph/dialog", Level::Patterns, "Paragraph", "Dialog", DialogArgs { open: true }, paragraph_dialog),
        typed_story!("patterns/notes/editor", Level::Patterns, "Notes", "Editor", NoArgs {}, notes),
        typed_story!("patterns/columns/dialog", Level::Patterns, "Columns", "Dialog", DialogArgs { open: true }, columns),
        typed_story!("patterns/hyperlink/dialog", Level::Patterns, "Hyperlink", "Dialog", DialogArgs { open: true }, hyperlink),
        typed_story!("composites/style-chip/picker", Level::Composites, "Style chip", "Picker", NoArgs {}, style_chip),
    ];
    #[cfg(feature = "find")]
    all.extend([
        typed_story!("patterns/find/bar", Level::Patterns, "Find", "Bar", NoArgs {}, find_bar),
        typed_story!("patterns/find/dialog", Level::Patterns, "Find", "Dialog", DialogArgs { open: true }, find_dialog),
    ]);
    all
}

craft_ui::words! {
    /// The words the paged-text stories add: sample notes, a link, a search and style names.
    #[non_exhaustive]
    pub(crate) struct PagedStoryLabels {
        /// The notes' paragraphs: an opening line, a bold run, a closing line and a list.
        notes_opening: "pagedStory.notesOpening" = "Welcome everyone and introduce the agenda for the next thirty minutes before taking questions.";
        notes_key: "pagedStory.notesKey" = "Key message:";
        notes_key_text: "pagedStory.notesKeyText" = " revenue grew in every region, led by the new subscription tier.";
        notes_item_one: "pagedStory.notesItemOne" = "Show the regional chart";
        notes_item_two: "pagedStory.notesItemTwo" = "Pause for questions on pricing";
        notes_closing: "pagedStory.notesClosing" = "Thank the team and hand over to the next speaker.";
        /// The hyperlink dialog's sample text, ScreenTip and bookmark names.
        link_text: "pagedStory.linkText" = "Pricing details";
        link_tip: "pagedStory.linkTip" = "Open the pricing page";
        place_intro: "pagedStory.placeIntro" = "Introduction";
        place_results: "pagedStory.placeResults" = "Results";
        /// The find bar's and dialog's query.
        find_query: "pagedStory.findQuery" = "page";
        /// Style names with their folders.
        style_blue_500: "pagedStory.styleBlue500" = "Brand/Blue/500";
        style_blue_700: "pagedStory.styleBlue700" = "Brand/Blue/700";
        style_red_500: "pagedStory.styleRed500" = "Brand/Red/500";
        style_neutral: "pagedStory.styleNeutral" = "Neutral";
    }
}

/// A story that opens a dialog.
#[derive(StoryArgs, Clone, Debug, PartialEq)]
struct DialogArgs {
    open: bool,
}

fn last_event(ui: &mut Ui, event: Option<&str>) {
    if let Some(event) = event {
        let line = fmt(&StoryLabels::of(ui).last_event, &[("event", event)]);
        ui.add(Text::new(line).content(Content::Muted));
    }
}

// --- Rulers ---

/// Word's column: 6.5 inches in points.
const COLUMN_WIDTH: f32 = 468.0;

struct RulerDemo {
    values: RulerValues,
    kind: TabAlign,
    selected: Option<usize>,
    last: Option<String>,
}

fn stop(position: f32, align: TabAlign) -> TabStop {
    TabStop { position, align, leader: TabLeader::None }
}

/// One stop of each kind, an inch apart from the first inch.
fn sample_stops() -> Vec<TabStop> {
    vec![
        stop(72.0, TabAlign::Left),
        stop(144.0, TabAlign::Center),
        stop(216.0, TabAlign::Right),
        stop(288.0, TabAlign::Decimal),
        stop(360.0, TabAlign::Bar),
    ]
}

fn apply_ruler(demo: &mut RulerDemo, out: super::ruler::TextRulerResponse) {
    if let Some(edit) = out.edit {
        demo.last = Some(format!("{edit:?}"));
        if let EditEvent::Commit { value, .. } | EditEvent::Set { value, .. } = edit {
            demo.values = value;
        }
    }
    if out.selected.is_some() {
        demo.selected = out.selected;
    }
    if out.open_tabs_dialog {
        demo.last = Some("OpenTabsDialog".to_owned());
    }
}

// new: Word's ruler over a 6.5 in column, a hanging indent, four stops of each kind, the selector.
fn ruler_word(ui: &mut Ui, _: &mut NoArgs) {
    let demo = shared(ui, "ruler-word", || RulerDemo {
        values: RulerValues { stops: sample_stops(), left_indent: 36.0, first_line_indent: -36.0, right_indent: 0.0 },
        kind: TabAlign::Left,
        selected: None,
        last: None,
    });
    let mut demo = lock(&demo);
    let demo = &mut *demo;
    let side = Rulers::style(&Tokens::of(ui)).size;
    let (_, row) = ui.allocate_space(vec2(ui.available_width(), side));
    // The selector sits in the corner square left of the strip.
    let strip = Rect::from_min_max(row.min + vec2(side, 0.0), row.max);
    let out = TextRuler::new("word", &demo.values, COLUMN_WIDTH).kind(&mut demo.kind).selected(demo.selected).show(ui, strip);
    apply_ruler(demo, out);
    last_event(ui, demo.last.as_deref());
}

// new: InDesign's Tabs panel strip fitted to its width, a selected decimal stop.
fn ruler_panel(ui: &mut Ui, _: &mut NoArgs) {
    let demo = shared(ui, "ruler-panel", || RulerDemo {
        values: RulerValues { stops: sample_stops(), ..RulerValues::default() },
        kind: TabAlign::Left,
        selected: Some(3),
        last: None,
    });
    let mut demo = lock(&demo);
    let demo = &mut *demo;
    let side = Rulers::style(&Tokens::of(ui)).size;
    let (_, strip) = ui.allocate_space(vec2(ui.available_width(), side));
    let out = TextRuler::new("panel", &demo.values, COLUMN_WIDTH).selected(demo.selected).show(ui, strip);
    apply_ruler(demo, out);
    last_event(ui, demo.last.as_deref());
}

// --- Dialogs ---

/// The default tab interval: half an inch.
const DEFAULT_INTERVAL: f32 = 36.0;

// new: the Tabs dialog with three stops, one with a dot leader.
fn tabs_dialog(ui: &mut Ui, args: &mut DialogArgs) {
    drive(
        ui,
        "tabs",
        &mut args.open,
        (),
        || {
            let mut stops = vec![stop(72.0, TabAlign::Left), stop(216.0, TabAlign::Right), stop(360.0, TabAlign::Decimal)];
            stops[1].leader = TabLeader::Dot;
            TabStopsDialog::new(stops, DEFAULT_INTERVAL)
        },
        |_| {},
        |result| format!("{} stops, {} cleared", result.stops.len(), result.cleared.len()),
    );
}

// new: the Paragraph dialog with a hanging indent and Exactly 18 pt.
fn paragraph_dialog(ui: &mut Ui, args: &mut DialogArgs) {
    drive(
        ui,
        "paragraph",
        &mut args.open,
        (),
        || {
            ParagraphDialog::new(ParagraphFormat {
                align: Some(ParagraphAlign::Left),
                before_text: Some(0.0),
                special: Some(SpecialIndent::Hanging),
                by: Some(36.0),
                space_before: Some(0.0),
                space_after: Some(6.0),
                line_spacing: Some(LineSpacing::Exactly(18.0)),
                ..ParagraphFormat::default()
            })
        },
        |_| {},
        |answer| format!("{:?}", answer.format.line_spacing),
    );
}

// new: Word's More Columns over three unequal columns.
fn columns(ui: &mut Ui, args: &mut DialogArgs) {
    drive(
        ui,
        "columns",
        &mut args.open,
        (),
        || {
            let columns = TextColumns {
                count: Some(3),
                spacing: Some(36.0),
                widths: vec![(150.0, 36.0), (150.0, 36.0), (96.0, 0.0)],
                equal_width: Some(false),
                line_between: Some(false),
                ..TextColumns::default()
            };
            TextColumnsDialog::new(columns).word(COLUMN_WIDTH)
        },
        |_| {},
        |columns| format!("{:?}", columns.map(|c| c.count)),
    );
}

// new: Edit Hyperlink over a web address with a ScreenTip.
fn hyperlink(ui: &mut Ui, args: &mut DialogArgs) {
    let labels = PagedStoryLabels::of(ui);
    drive(
        ui,
        "hyperlink",
        &mut args.open,
        (),
        || {
            let link = Hyperlink {
                target: LinkTarget::Url("https://example.com/pricing".to_owned()),
                tooltip: labels.link_tip.clone(),
                highlight_click: false,
            };
            HyperlinkDialog::new(CurrentLink::One(link), Some(labels.link_text.clone()))
                .places(vec![labels.place_intro.clone(), labels.place_results.clone()])
        },
        |_| {},
        |answer| format!("{answer:?}"),
    );
}

// --- Notes ---

struct NotesDemo {
    notes: NotesText,
    last: Option<String>,
}

fn paragraph(runs: Vec<NotesRun>) -> NotesParagraph {
    NotesParagraph { runs, ..NotesParagraph::default() }
}

fn plain(text: &str) -> NotesRun {
    NotesRun { text: text.to_owned(), ..NotesRun::default() }
}

// new: three notes paragraphs, a bulleted level-1 list and a bold run, wrapping.
fn notes(ui: &mut Ui, _: &mut NoArgs) {
    let labels = PagedStoryLabels::of(ui);
    let demo = shared(ui, "notes", || {
        let item = |text: &str| NotesParagraph {
            runs: vec![plain(text)],
            level: 1,
            marker: Some(ListMarker::Bullet("•".to_owned())),
            ..NotesParagraph::default()
        };
        let key = paragraph(vec![NotesRun { bold: true, ..plain(&labels.notes_key) }, plain(&labels.notes_key_text)]);
        NotesDemo {
            notes: NotesText {
                paragraphs: vec![
                    paragraph(vec![plain(&labels.notes_opening)]),
                    key,
                    item(&labels.notes_item_one),
                    item(&labels.notes_item_two),
                    paragraph(vec![plain(&labels.notes_closing)]),
                ],
            },
            last: None,
        }
    });
    let mut demo = lock(&demo);
    let demo = &mut *demo;
    let out = NotesEditor::new("notes", &demo.notes).show(ui);
    if let Some(changed) = out.changed {
        demo.last = Some(format!("{} characters", changed.plain().chars().count()));
        demo.notes = changed;
    }
    last_event(ui, demo.last.as_deref());
}

// --- Find ---

#[cfg(feature = "find")]
mod find_demos {
    use super::*;
    use crate::find::{FindMatches, FindOptions};
    use crate::ui::find::{FindBar, FindBarState, FindReplaceDialog, FindTab};

    /// Four pages with three matches each: twelve in all.
    const MATCH_PAGES: usize = 4;
    const MATCHES_PER_PAGE: usize = 3;
    /// A match's length and the distance between matches on a page, in characters.
    const MATCH_LENGTH: usize = 4;
    const MATCH_PITCH: usize = 10;

    pub(super) struct FindDemo {
        query: String,
        options: FindOptions,
        matches: FindMatches,
        state: FindBarState,
        last: Option<String>,
    }

    /// Twelve matches, the third current.
    fn matches() -> FindMatches {
        let mut matches = FindMatches::new(0);
        for page in 0..MATCH_PAGES {
            matches.set_page(page, (0..MATCHES_PER_PAGE).map(|i| i * MATCH_PITCH..i * MATCH_PITCH + MATCH_LENGTH).collect::<Vec<_>>());
        }
        matches.step(false);
        matches.step(false);
        matches
    }

    // new: the bar with "page" and 3 of 12, options on.
    pub(in super::super) fn find_bar(ui: &mut Ui, _: &mut NoArgs) {
        let labels = PagedStoryLabels::of(ui);
        let demo = shared(ui, "find-bar", || FindDemo {
            query: labels.find_query.clone(),
            options: FindOptions { match_case: true, ..FindOptions::default() },
            matches: matches(),
            state: {
                let mut state = FindBarState::default();
                state.open();
                state
            },
            last: None,
        });
        let mut demo = lock(&demo);
        let demo = &mut *demo;
        let out = FindBar::new("bar", &mut demo.query, &mut demo.options, &mut demo.matches).state(&mut demo.state).show(ui);
        if out.search {
            demo.last = Some(format!("Search({:?})", demo.query));
        }
        if out.stepped {
            demo.last = Some(format!("Step({:?})", demo.matches.current()));
        }
        if out.closed {
            demo.last = Some("Closed".to_owned());
        }
        super::last_event(ui, demo.last.as_deref());
    }

    // new: Word's Find and Replace, Replace tab, Search Options open.
    pub(in super::super) fn find_dialog(ui: &mut Ui, args: &mut DialogArgs) {
        let labels = PagedStoryLabels::of(ui);
        let demo = shared(ui, "find-dialog", || None::<String>);
        let mut demo = lock(&demo);
        let last = &mut *demo;
        drive(
            ui,
            "find",
            &mut args.open,
            (),
            || FindReplaceDialog::new(labels.find_query.clone(), FindTab::Replace).word(true),
            |dialog| {
                if let Some(request) = dialog.take_request() {
                    *last = Some(format!("{request:?}"));
                }
            },
            |()| String::new(),
        );
        super::last_event(ui, demo.as_deref());
    }
}
#[cfg(feature = "find")]
use find_demos::{find_bar, find_dialog};

// --- Style chip ---

struct ChipDemo {
    applied: Option<u64>,
    view: StylePickerView,
    last: Option<String>,
}

// new: a chip over "Brand/Blue/500" with the picker open in list view.
fn style_chip(ui: &mut Ui, _: &mut NoArgs) {
    let labels = PagedStoryLabels::of(ui);
    let tokens = Tokens::of(ui);
    let demo = shared(ui, "style-chip", || ChipDemo { applied: Some(1), view: StylePickerView::List, last: None });
    let mut demo = lock(&demo);
    let demo = &mut *demo;
    let swatch = |id: u64, name: &String, color: egui::Color32| {
        let hex = format!("#{:02X}{:02X}{:02X}", color.r(), color.g(), color.b());
        NamedStyleRow { id, name: name.clone(), meta: hex, preview: StylePreview::Color(color) }
    };
    let styles = [
        swatch(1, &labels.style_blue_500, tokens.color.accent.solid.to_egui()),
        swatch(2, &labels.style_blue_700, tokens.color.accent.pressed.to_egui()),
        swatch(3, &labels.style_red_500, tokens.color.status.bad.solid.to_egui()),
        swatch(4, &labels.style_neutral, tokens.color.text.muted.to_egui()),
    ];
    let out = StyleChip::new("chip", &styles, demo.applied.map(Some), &mut demo.view).show(ui);
    if let Some(id) = out.picked {
        demo.applied = Some(id);
        demo.last = Some(format!("Picked({id})"));
    }
    if out.detach {
        demo.applied = None;
        demo.last = Some("Detach".to_owned());
    }
    if out.create {
        demo.last = Some("Create".to_owned());
    }
    if let Some(id) = out.edit {
        demo.last = Some(format!("Edit({id})"));
    }
    last_event(ui, demo.last.as_deref());
}

#[cfg(test)]
mod tests {
    use craft_ui::story::Args;
    use craft_ui::theme::{Base, install};
    use egui_kittest::Harness;

    // every story draws its widget over its sample data without panicking.
    #[test]
    fn every_story_draws() {
        for story in super::stories() {
            let mut harness = Harness::<Args>::builder()
                .with_size([720.0, 520.0])
                .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
                .build_ui_state(move |ui, args| (story.show)(ui, args), story.defaults());
            harness.run_steps(4);
        }
    }
}
