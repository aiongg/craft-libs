//! The gradient editor's stories: PhotoCraft's split bar, DesignCraft's and VectorCraft's
//! combined one, and PhotoCraft's Gradient Editor dialog.

use crate::dialog::DialogLabels;
use crate::localized::Labels as _;
use craft_theme::{Rgba, Srgba};
use egui::{Id, Ui, vec2};

use super::editor::{GradientEditor, GradientEditorResponse};
use super::swatch::GradientSwatch;
use super::value::{ColorStop, GradientStops, OpacityStop, StopRows};
use crate::behavior::FieldTab;
use crate::controls::{Button, ButtonVariant};
use crate::edit::{self, EditEvent, OuterGesture};
use crate::localized::LocaleNumbers;
use crate::primitives::{Content, Gap, Row, Stack, Text};
use crate::story::{Level, Story, StoryArgs, StoryChoice};
use crate::typed_story;

pub(super) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "composites/gradient/editor-split",
            Level::Composites,
            "GradientEditor",
            "Split rows",
            SplitArgs { rows: RowsArg::Split, midpoints: true, fields: true, min: 2, max: 1024, enabled: true, width: 320.0 },
            show_split
        ),
        typed_story!(
            "composites/gradient/editor-combined",
            Level::Composites,
            "GradientEditor",
            "Combined rows",
            CombinedArgs { fields: true, width: 280.0 },
            show_combined
        ),
        typed_story!("composites/gradient/dialog", Level::Composites, "GradientEditor", "Dialog", DialogArgs { color_popover: true }, show_dialog),
    ]
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum RowsArg {
    #[choice(name = "Split")]
    Split,
    #[choice(name = "Combined")]
    Combined,
    #[choice(name = "Color only")]
    ColorOnly,
}

impl RowsArg {
    fn rows(self) -> StopRows {
        match self {
            RowsArg::Split => StopRows::Split,
            RowsArg::Combined => StopRows::Combined,
            RowsArg::ColorOnly => StopRows::ColorOnly,
        }
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct SplitArgs {
    rows: RowsArg,
    midpoints: bool,
    fields: bool,
    #[arg(range = 1..=4)]
    min: usize,
    #[arg(range = 4..=1024)]
    max: usize,
    enabled: bool,
    #[arg(range = 160.0..=480.0)]
    width: f32,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct CombinedArgs {
    fields: bool,
    #[arg(range = 160.0..=480.0)]
    width: f32,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct DialogArgs {
    #[arg(name = "color popover")]
    color_popover: bool,
}

/// A host's stops, the "before" of the gesture under way and the last colour request, kept
/// between frames.
#[derive(Clone)]
struct StoryHost {
    stops: GradientStops,
    before: Option<GradientStops>,
    requested: Option<usize>,
}

impl StoryHost {
    fn load(ui: &Ui, key: &str, initial: impl FnOnce() -> GradientStops) -> (Id, Self) {
        let id = ui.scope_id().with(key);
        let host = ui.data(|d| d.get_temp::<StoryHost>(id)).unwrap_or_else(|| StoryHost { stops: initial(), before: None, requested: None });
        (id, host)
    }

    /// Applies the editor's edit as a host without undo does, restoring the before value on a
    /// cancel.
    fn apply(&mut self, response: GradientEditorResponse) {
        match response.edit.event {
            Some(EditEvent::Begin { .. }) => self.before = Some(self.stops.clone()),
            Some(EditEvent::Cancel { .. }) => {
                if let Some(before) = self.before.take() {
                    self.stops = before;
                }
            }
            Some(event) => {
                if let Some(value) = event.value() {
                    self.stops = value.clone();
                }
            }
            None => {}
        }
        self.requested = response.color_requested.or(self.requested);
    }

    fn show(self, ui: &mut Ui, id: Id, width: f32, editor: impl FnOnce(&mut Ui, &GradientStops) -> GradientEditorResponse) {
        let mut host = self;
        Stack::new(Gap::Stack).show(ui, |ui| {
            ui.allocate_ui(vec2(width, 0.0), |ui| {
                ui.set_max_width(width);
                let response = editor(ui, &host.stops.clone());
                host.apply(response);
            });
            let line = match host.requested {
                Some(i) => format!("Color requested for stop {}", i + 1),
                None => "Double-click a color stop, or press Enter on it, to edit its color".to_owned(),
            };
            ui.add(Text::new(line).content(Content::Muted));
        });
        ui.data_mut(|d| d.insert_temp(id, host));
    }
}

fn rgb(r: u8, g: u8, b: u8) -> Srgba {
    Srgba::from(Rgba::rgb(r, g, b))
}

/// PhotoCraft's Gradient Editor: opacity stops above the ramp, colour stops and midpoints below.
fn show_split(ui: &mut Ui, args: &mut SplitArgs) {
    let numbers = LocaleNumbers::from_tag("en");
    let rows = args.rows.rows();
    let (id, host) = StoryHost::load(ui, "gradient-editor-split", || {
        let mut stops = GradientStops::new(vec![
            ColorStop::new(0.0, rgb(0xf5, 0xa5, 0x24)),
            ColorStop { midpoint: 0.35, ..ColorStop::new(0.45, rgb(0xd9, 0x3a, 0x6a)) },
            ColorStop::new(1.0, rgb(0x3e, 0x63, 0xdd)),
        ]);
        stops.opacity = vec![OpacityStop::new(0.0, 1.0), OpacityStop::new(0.7, 0.3), OpacityStop::new(1.0, 1.0)];
        stops
    });
    let (min, max) = (args.min, args.max);
    let (midpoints, fields, enabled) = (args.midpoints, args.fields, args.enabled);
    host.show(ui, id, args.width.min(ui.available_width()), |ui, stops| {
        GradientEditor::new("split", "Gradient", stops, &numbers)
            .rows(rows)
            .midpoints(midpoints)
            .fields(fields)
            .stops(min, max)
            .enabled(enabled)
            .show(ui)
    });
}

/// DesignCraft's and VectorCraft's bar: one row, each stop's opacity in its colour, midpoints
/// 13–87 % above the ramp; a stop's colour in a popover, as VectorCraft's.
fn show_combined(ui: &mut Ui, args: &mut CombinedArgs) {
    let numbers = LocaleNumbers::from_tag("en");
    let (id, host) = StoryHost::load(ui, "gradient-editor-combined", || {
        GradientStops::new(vec![
            ColorStop::new(0.0, Srgba::new(0.9, 0.28, 0.3, 1.0)),
            ColorStop::new(0.5, Srgba::new(0.9, 0.28, 0.3, 0.0)),
            ColorStop::new(1.0, Srgba::new(0.19, 0.64, 0.42, 1.0)),
        ])
    });
    let fields = args.fields;
    host.show(ui, id, args.width.min(ui.available_width()), |ui, stops| {
        GradientEditor::new("combined", "Gradient", stops, &numbers)
            .rows(StopRows::Combined)
            .midpoint_range(0.13..=0.87)
            .fields(fields)
            .color_popover(true)
            .show(ui)
    });
}

/// The applied gradient and the dialog that edits it, kept between frames.
#[derive(Clone)]
struct Dialog {
    applied: GradientStops,
    open: Option<(GradientStops, OuterGesture)>,
    /// The dialog opens with the story.
    opened: bool,
    steps: usize,
    updates: usize,
}

/// PhotoCraft's Gradient Editor as a dialog: inside an [`edit::scope`] the editor reports only
/// updates, which the dialog applies to its draft; OK makes one undo step of the draft, Cancel
/// none. In its [`FieldTab`] scope Tab visits the fields and Ctrl+Tab reaches the bar.
fn show_dialog(ui: &mut Ui, args: &mut DialogArgs) {
    let numbers = LocaleNumbers::from_tag("en");
    let applied = GradientStops::new(vec![ColorStop::new(0.0, rgb(0x12, 0xa5, 0x94)), ColorStop::new(1.0, rgb(0x8e, 0x4e, 0xc6))]);
    let initial = Dialog { applied, open: None, opened: false, steps: 0, updates: 0 };
    crate::controls::slider::with_values(ui, "gradient-dialog", [initial], |ui, state| {
        let dialog = &mut state[0];
        Stack::new(Gap::Stack).show(ui, |ui| {
            Row::new(Gap::Inline).show(ui, |ui| {
                let clicked = GradientSwatch::new("gradient-dialog-swatch", "Gradient picker", &dialog.applied).width(96.0).show(ui).clicked();
                ui.add(Text::new(format!("Undo steps: {}", dialog.steps)));
                if (clicked || !dialog.opened) && dialog.open.is_none() {
                    dialog.open = Some((dialog.applied.clone(), OuterGesture::begin(ui.ctx())));
                    dialog.opened = true;
                    dialog.updates = 0;
                }
            });
            let Some((mut draft, outer)) = dialog.open.take() else {
                ui.add(Text::new("Click the gradient to open the dialog.").content(Content::Muted));
                return;
            };
            outer.keep(ui);
            let (mut ok, mut cancel) = (false, false);
            FieldTab::new("gradient-dialog").show(ui, |ui| {
                ui.allocate_ui(vec2(320.0, 0.0), |ui| {
                    ui.set_max_width(320.0);
                    let response = edit::scope(ui, |ui| {
                        GradientEditor::new("gradient-dialog-editor", "Gradient", &draft, &numbers).color_popover(args.color_popover).show(ui)
                    })
                    .inner;
                    if let Some(event) = &response.edit.event {
                        dialog.updates += usize::from(matches!(event, EditEvent::Update { .. }));
                        if let Some(value) = event.value() {
                            draft = value.clone();
                        }
                    }
                });
                Row::new(Gap::Inline).show(ui, |ui| {
                    ok = ui.add(Button::new(DialogLabels::of(ui).ok).variant(ButtonVariant::Primary)).clicked();
                    cancel = ui.add(Button::new(DialogLabels::of(ui).cancel)).clicked();
                });
            });
            ui.add(Text::new(format!("Updates: {}", dialog.updates)).content(Content::Muted));
            if ok {
                if draft != dialog.applied {
                    dialog.applied = draft;
                    dialog.steps += 1;
                }
            } else if !cancel {
                dialog.open = Some((draft, outer));
            }
        });
    });
}
