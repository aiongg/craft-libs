//! Stories of the Channels panel: the panel over fake rows, a
//! `FakeThumbSource` and `ManualJobs`, with story words; each at 240 pt wide with its footer. The
//! story is the app: it keeps the rows and applies the panel's events (target, eyes, rename,
//! reorder, footer buttons, menu commands), so drag and drop run live.
//!
//! Ids start with their level (the gallery requires it), so the spec's `imaging/channels/*` are
//! `patterns/channels/rgb`, `/cmyk`, `/alpha-and-spot`, `/quick-mask-and-layer-mask`,
//! `/multichannel`, `/empty`, all of the component "Channels".

use std::sync::Arc;

use craft_host::ManualJobs;
use craft_theme::Srgba;
use craft_ui::behavior::{DropPosition, Move};
use craft_ui::canvas::textures::{FakeThumbSource, ThumbnailOptions, Thumbnails};
use craft_ui::controls::Button;
use craft_ui::edit::EditEvent;
use craft_ui::localized::Labels as _;
use craft_ui::story::{Level, Story, StoryArgs};
use craft_ui::typed_story;
use egui::{Ui, vec2};

use super::{ChannelKind, ChannelRow, ChannelsCommand, ChannelsEvent, ChannelsFooterButton, ChannelsPanel, ColorIndicates};

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Channels"];

craft_ui::words! {
    /// The channel names and texts of the Channels stories; the app translates built-in names.
    pub(crate) struct ChannelsStoryWords {
        finish_next: "channelsStory.finishNext" = "Finish next";
        reset: "channelsStory.reset" = "Reset";
        last_event: "channelsStory.lastEvent" = "Last event: {event}";
        no_event: "channelsStory.noEvent" = "No event yet";
        rgb: "channelsStory.rgb" = "RGB";
        red: "channelsStory.red" = "Red";
        green: "channelsStory.green" = "Green";
        blue: "channelsStory.blue" = "Blue";
        cmyk: "channelsStory.cmyk" = "CMYK";
        cyan: "channelsStory.cyan" = "Cyan";
        magenta: "channelsStory.magenta" = "Magenta";
        yellow: "channelsStory.yellow" = "Yellow";
        black: "channelsStory.black" = "Black";
        alpha: "channelsStory.alpha" = "Alpha {n}";
        spot: "channelsStory.spot" = "PANTONE {n} C";
        copy: "channelsStory.copy" = "{name} copy";
        layer_mask: "channelsStory.layerMask" = "{layer} Mask";
        layer: "channelsStory.layer" = "Sky";
        quick_mask: "channelsStory.quickMask" = "Quick Mask";
    }
}

/// The six stories above.
pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!("patterns/channels/rgb", Level::Patterns, "Channels", "RGB", ChannelsArgs { background: true }, show_rgb),
        typed_story!("patterns/channels/cmyk", Level::Patterns, "Channels", "CMYK", ChannelsArgs { background: true }, show_cmyk),
        typed_story!(
            "patterns/channels/alpha-and-spot",
            Level::Patterns,
            "Channels",
            "Alpha and spot",
            ChannelsArgs { background: true },
            show_alpha_and_spot
        ),
        typed_story!(
            "patterns/channels/quick-mask-and-layer-mask",
            Level::Patterns,
            "Channels",
            "Quick Mask and layer mask",
            ChannelsArgs { background: true },
            show_masks
        ),
        typed_story!(
            "patterns/channels/multichannel",
            Level::Patterns,
            "Channels",
            "Multichannel",
            ChannelsArgs { background: true },
            show_multichannel
        ),
        typed_story!("patterns/channels/empty", Level::Patterns, "Channels", "Empty", ChannelsArgs { background: true }, show_empty),
    ]
}

#[derive(StoryArgs, Debug, PartialEq)]
struct ChannelsArgs {
    /// Make the thumbnails as jobs, one per "Finish next".
    background: bool,
}

type Key = u32;
type Rows = Vec<ChannelRow<Key>>;

/// Keys of the rows the stories start with; rows the user adds count up from [`FIRST_NEW`].
const COMPOSITE: Key = 0;
const FIRST_NEW: Key = 100;

fn row(key: Key, kind: ChannelKind, name: &str) -> ChannelRow<Key> {
    ChannelRow::new(key, kind, name)
}

/// The composite, targeted, and its colour channels (highlighted with it).
fn colour_rows(composite: &str, colours: &[&str]) -> Rows {
    let mut rows = vec![ChannelRow { targeted: true, ..row(COMPOSITE, ChannelKind::Composite, composite) }];
    rows.extend(colours.iter().zip(1..).map(|(name, key)| ChannelRow { targeted: true, ..row(key, ChannelKind::Color, name) }));
    rows
}

fn rgb_rows(words: &ChannelsStoryWords) -> Rows {
    colour_rows(&words.rgb, &[&words.red, &words.green, &words.blue])
}

fn cmyk_rows(words: &ChannelsStoryWords) -> Rows {
    colour_rows(&words.cmyk, &[&words.cyan, &words.magenta, &words.yellow, &words.black])
}

fn numbered(text: &str, key: &str, n: u32) -> String {
    craft_i18n::fmt(text, &[(key, &n.to_string())])
}

fn alpha_and_spot_rows(words: &ChannelsStoryWords) -> Rows {
    let mut rows = rgb_rows(words);
    rows.push(row(10, ChannelKind::Alpha { indicates: ColorIndicates::MaskedAreas }, &numbered(&words.alpha, "n", 1)));
    rows.push(ChannelRow {
        visible: false,
        ..row(11, ChannelKind::Alpha { indicates: ColorIndicates::SelectedAreas }, &numbered(&words.alpha, "n", 2))
    });
    rows.push(row(12, ChannelKind::Spot { ink: Srgba::rgb(0.88, 0.1, 0.2) }, &numbered(&words.spot, "n", 185)));
    rows.push(row(13, ChannelKind::Spot { ink: Srgba::rgb(0.1, 0.25, 0.75) }, &numbered(&words.spot, "n", 300)));
    rows
}

fn mask_rows(words: &ChannelsStoryWords) -> Rows {
    let mut rows = rgb_rows(words);
    for row in &mut rows {
        row.targeted = false;
    }
    let name = craft_i18n::fmt(&words.layer_mask, &[("layer", &words.layer)]);
    rows.push(ChannelRow { targeted: true, ..row(20, ChannelKind::LayerMask { enabled: true }, &name) });
    rows.push(row(10, ChannelKind::Alpha { indicates: ColorIndicates::MaskedAreas }, &numbered(&words.alpha, "n", 1)));
    rows.push(row(30, ChannelKind::QuickMask { indicates: ColorIndicates::MaskedAreas }, &words.quick_mask));
    rows
}

/// A Multichannel document lists its channels as spot channels, with no composite.
fn multichannel_rows(words: &ChannelsStoryWords) -> Rows {
    let inks = [(&words.cyan, [0.0, 0.7, 0.9]), (&words.magenta, [0.9, 0.0, 0.6]), (&words.yellow, [0.95, 0.85, 0.0])];
    let mut rows: Rows =
        inks.iter().zip(40..).map(|((name, [r, g, b]), key)| row(key, ChannelKind::Spot { ink: Srgba::rgb(*r, *g, *b) }, name)).collect();
    rows[0].targeted = true;
    rows
}

fn empty_rows(_: &ChannelsStoryWords) -> Rows {
    Vec::new()
}

fn show_rgb(ui: &mut Ui, args: &mut ChannelsArgs) {
    show_panel(ui, "rgb", args, rgb_rows);
}

fn show_cmyk(ui: &mut Ui, args: &mut ChannelsArgs) {
    show_panel(ui, "cmyk", args, cmyk_rows);
}

fn show_alpha_and_spot(ui: &mut Ui, args: &mut ChannelsArgs) {
    show_panel(ui, "alpha-and-spot", args, alpha_and_spot_rows);
}

fn show_masks(ui: &mut Ui, args: &mut ChannelsArgs) {
    show_panel(ui, "quick-mask-and-layer-mask", args, mask_rows);
}

fn show_multichannel(ui: &mut Ui, args: &mut ChannelsArgs) {
    show_panel(ui, "multichannel", args, multichannel_rows);
}

fn show_empty(ui: &mut Ui, args: &mut ChannelsArgs) {
    show_panel(ui, "empty", args, empty_rows);
}

/// What the story keeps between passes: the app's rows, its next new key and the last event.
#[derive(Clone)]
struct Demo {
    rows: Rows,
    next: Key,
    last: Option<&'static str>,
}

impl Demo {
    fn new(rows: Rows) -> Self {
        Self { rows, next: FIRST_NEW, last: None }
    }
}

fn show_panel(ui: &mut Ui, name: &'static str, args: &ChannelsArgs, initial: fn(&ChannelsStoryWords) -> Rows) {
    let words = ChannelsStoryWords::of(ui);
    let ctx = ui.ctx().clone();
    let demo_id = ui.make_persistent_id(("channels-story", name));
    let mut demo = ui.data_mut(|data| data.get_temp::<Demo>(demo_id)).unwrap_or_else(|| Demo::new(initial(&words)));
    let jobs_id = ui.make_persistent_id("channels-story-jobs");
    let jobs: Arc<ManualJobs> = ui.data_mut(|data| data.get_temp_mut_or_insert_with(jobs_id, || Arc::new(ManualJobs::new())).clone());

    ui.horizontal(|ui| {
        if args.background && ui.add(Button::new(words.finish_next.clone()).enabled(jobs.waiting() > 0)).clicked() {
            jobs.run_next();
        }
        if ui.add(Button::new(words.reset.clone())).clicked() {
            demo = Demo::new(initial(&words));
        }
    });
    let source = FakeThumbSource::<Key>::new().later(args.background).aspect(4, 3);
    let options = ThumbnailOptions { lanes: 1, jobs: Some(jobs), ..ThumbnailOptions::default() };
    let thumbs = Thumbnails::<Key>::in_context(&ctx, ("channels-story-thumbnails", args.background), options);
    let output =
        ui.allocate_ui(vec2(240.0, 320.0), |ui| ChannelsPanel::new(("channels-story", name), &demo.rows).thumbnails(&thumbs, &source).show(ui)).inner;
    for event in output.events {
        demo.last = Some(event_name(&event));
        apply(&mut demo, event, &words);
    }
    ui.label(match demo.last {
        Some(event) => craft_i18n::fmt(&words.last_event, &[("event", event)]),
        None => words.no_event.clone(),
    });
    ui.data_mut(|data| data.insert_temp(demo_id, demo));
}

/// The event's name, as the story's last-event readout shows it (an identifier, not prose).
fn event_name(event: &ChannelsEvent<Key>) -> &'static str {
    match event {
        ChannelsEvent::Target(_) => "Target",
        ChannelsEvent::Visibility(_) => "Visibility",
        ChannelsEvent::LoadSelection { .. } => "LoadSelection",
        ChannelsEvent::Rename { .. } => "Rename",
        ChannelsEvent::Options(_) => "Options",
        ChannelsEvent::Reorder(_) => "Reorder",
        ChannelsEvent::Footer { .. } => "Footer",
        ChannelsEvent::FooterDrop { .. } => "FooterDrop",
        ChannelsEvent::Command { .. } => "Command",
    }
}

/// What the app does with an event. Loads, options dialogs, split and merge need a document, so
/// the story only reads them out.
fn apply(demo: &mut Demo, event: ChannelsEvent<Key>, words: &ChannelsStoryWords) {
    match event {
        ChannelsEvent::Target(key) => target(&mut demo.rows, key),
        ChannelsEvent::Visibility(edit) => {
            if let Some(toggle) = edit.value()
                && let Some(row) = demo.rows.iter_mut().find(|row| row.key == toggle.row)
            {
                row.visible = toggle.on;
            }
        }
        ChannelsEvent::Rename { key, text } => {
            if let Some(row) = demo.rows.iter_mut().find(|row| row.key == key) {
                row.name = text;
            }
        }
        ChannelsEvent::Reorder(EditEvent::Commit { value, .. } | EditEvent::Set { value, .. }) => move_rows(&mut demo.rows, &value),
        ChannelsEvent::Footer { button, rows } => match (button, rows.into_iter().next()) {
            (ChannelsFooterButton::Delete, Some(key)) => remove(&mut demo.rows, key),
            (ChannelsFooterButton::New | ChannelsFooterButton::SaveSelection, _) => add_alpha(demo, words),
            _ => {}
        },
        ChannelsEvent::FooterDrop { button, rows } => {
            for key in rows {
                match button {
                    ChannelsFooterButton::New => duplicate(demo, key, words),
                    ChannelsFooterButton::Delete => remove(&mut demo.rows, key),
                    ChannelsFooterButton::SaveSelection | ChannelsFooterButton::LoadSelection => {}
                }
            }
        }
        ChannelsEvent::Command { command, key } => run_command(demo, command, key, words),
        _ => {}
    }
}

fn run_command(demo: &mut Demo, command: ChannelsCommand, key: Option<Key>, words: &ChannelsStoryWords) {
    match (command, key) {
        (ChannelsCommand::NewChannel, _) => add_alpha(demo, words),
        (ChannelsCommand::NewSpotChannel, _) => add_spot(demo, words),
        (ChannelsCommand::Duplicate, Some(key)) => duplicate(demo, key, words),
        (ChannelsCommand::Delete | ChannelsCommand::MergeSpot | ChannelsCommand::ExitQuickMask | ChannelsCommand::DeleteLayerMask, Some(key)) => {
            remove(&mut demo.rows, key)
        }
        (ChannelsCommand::ConvertToAlpha, _) => {
            if let Some(kind) = kind_mut(&mut demo.rows, key) {
                *kind = ChannelKind::Alpha { indicates: ColorIndicates::MaskedAreas };
            }
        }
        (ChannelsCommand::ColorIndicatesMasked | ChannelsCommand::ColorIndicatesSelected, _) => {
            let indicates =
                if command == ChannelsCommand::ColorIndicatesMasked { ColorIndicates::MaskedAreas } else { ColorIndicates::SelectedAreas };
            if let Some(ChannelKind::Alpha { indicates: current } | ChannelKind::QuickMask { indicates: current }) = kind_mut(&mut demo.rows, key) {
                *current = indicates;
            }
        }
        (ChannelsCommand::ToggleLayerMask, _) => {
            if let Some(ChannelKind::LayerMask { enabled }) = kind_mut(&mut demo.rows, key) {
                *enabled = !*enabled;
            }
        }
        _ => {}
    }
}

fn kind_mut(rows: &mut Rows, key: Option<Key>) -> Option<&mut ChannelKind> {
    let key = key?;
    rows.iter_mut().find(|row| row.key == key).map(|row| &mut row.kind)
}

/// Targets `key`; the composite targets every colour row with it.
fn target(rows: &mut Rows, key: Key) {
    let composite = rows.iter().any(|row| row.key == key && row.kind == ChannelKind::Composite);
    for row in rows {
        row.targeted = row.key == key || (composite && row.kind == ChannelKind::Color);
    }
}

/// Deletes an alpha, spot, Quick Mask or layer-mask row; the composite or the first row is
/// targeted when the target went.
fn remove(rows: &mut Rows, key: Key) {
    if rows.iter().any(|row| row.key == key && !matches!(row.kind, ChannelKind::Composite | ChannelKind::Color)) {
        rows.retain(|row| row.key != key);
        if !rows.iter().any(|row| row.targeted)
            && let Some(first) = rows.first().map(|row| row.key)
        {
            target(rows, first);
        }
    }
}

/// Inserts `row` before Quick Mask, or last, and targets it.
fn insert(demo: &mut Demo, row: ChannelRow<Key>) {
    let at = demo.rows.iter().position(|row| matches!(row.kind, ChannelKind::QuickMask { .. })).unwrap_or(demo.rows.len());
    let key = row.key;
    demo.rows.insert(at, row);
    target(&mut demo.rows, key);
}

fn add_alpha(demo: &mut Demo, words: &ChannelsStoryWords) {
    let count = demo.rows.iter().filter(|row| matches!(row.kind, ChannelKind::Alpha { .. })).count() as u32;
    let new = row(demo.next, ChannelKind::Alpha { indicates: ColorIndicates::MaskedAreas }, &numbered(&words.alpha, "n", count + 1));
    demo.next += 1;
    insert(demo, new);
}

fn add_spot(demo: &mut Demo, words: &ChannelsStoryWords) {
    let new = row(demo.next, ChannelKind::Spot { ink: Srgba::rgb(0.1, 0.6, 0.3) }, &numbered(&words.spot, "n", 347));
    demo.next += 1;
    insert(demo, new);
}

/// A copy of `key`: alpha and spot rows copy as they are, other rows become alpha channels.
fn duplicate(demo: &mut Demo, key: Key, words: &ChannelsStoryWords) {
    let Some(original) = demo.rows.iter().find(|row| row.key == key).cloned() else { return };
    let kind = if original.kind.renames() { original.kind } else { ChannelKind::Alpha { indicates: ColorIndicates::MaskedAreas } };
    let name = craft_i18n::fmt(&words.copy, &[("name", &original.name)]);
    let copy = row(demo.next, kind, &name);
    demo.next += 1;
    insert(demo, copy);
}

/// Applies a reorder: the rows go before or after the target. A move relative to one of the moved
/// rows changes nothing.
fn move_rows(rows: &mut Rows, mv: &Move<Key>) {
    if mv.items.contains(&mv.target) {
        return;
    }
    let moved: Rows = rows.iter().filter(|row| mv.items.contains(&row.key)).cloned().collect();
    rows.retain(|row| !mv.items.contains(&row.key));
    let Some(target) = rows.iter().position(|row| row.key == mv.target) else {
        rows.extend(moved);
        return;
    };
    let at = target + usize::from(mv.position == DropPosition::After);
    rows.splice(at..at, moved);
}
