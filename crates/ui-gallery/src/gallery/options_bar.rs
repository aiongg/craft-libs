//! The options bar: the appearance options and the view, with Reset layout pinned at the end.
use craft_theme::{ButtonVariant, ContrastChoice, ControlSize, ModeChoice, MotionChoice, TextSize, builtins};
use craft_ui::composites::FormGrid;
use craft_ui::controls::{Button, IconButton, NumberField, Segmented, Select, ToggleButton, TooltipExt as _};
use craft_ui::icons::{Icon, IconSet};
use craft_ui::localized::{LocaleNumbers, Unit};
use craft_ui::menu::{MenuEntry, MenuItem, PressMenu};
use craft_ui::patterns::{CellWidth, OptionsBar};
use craft_ui::popover::{Popover, PopoverAnchor};
use egui::Ui;

use crate::appearance::{Appearance, SCALE, builtin};
use crate::faces;
use crate::fields;
use crate::layout::{GalleryWorkspace, PreviewView, VariantAxis};

/// What the bar needs to know about the rest of the gallery.
#[derive(Clone, Copy)]
pub(super) struct BarInput {
    /// A story is targeted, so Variants has something to repeat.
    pub(super) has_target: bool,
    /// The Variants view is open.
    pub(super) variants_shown: bool,
}

/// What the bar asks of the rest of the gallery in a frame.
#[derive(Default)]
pub(super) struct BarOutput {
    /// The axis of the Variants view that was chosen.
    pub(super) variants: Option<VariantAxis>,
    /// Reset layout was chosen.
    pub(super) reset_layout: bool,
}

#[derive(Default)]
pub(super) struct Bar {
    /// The More popover is open.
    more_open: bool,
}

impl Bar {
    /// Opens or closes the More popover.
    pub(super) fn set_more_open(&mut self, open: bool) {
        self.more_open = open;
    }

    /// Draws the bar. It edits the appearance, the workspace and the preview view in place.
    pub(super) fn show(
        &mut self,
        ui: &mut Ui,
        a: &mut Appearance,
        workspace: &mut GalleryWorkspace,
        preview: &mut PreviewView,
        input: &BarInput,
    ) -> BarOutput {
        let BarInput { has_target, variants_shown } = *input;
        let mut out = BarOutput::default();
        let more_open = &mut self.more_open;
        let current = *workspace;
        let numbers = LocaleNumbers::of(ui);
        let faces_shown =
            faces::available().then(|| (faces::shown_ui_face(ui.ctx(), &a.ui_face), faces::shown_mono_face(&a.mono_face), faces::ui_faces(ui.ctx())));
        let icon_set_id = a.icon_set().id().to_owned();
        OptionsBar::new("gallery.options").label("Appearance").show(ui, |bar| {
            bar.row(|ui| {
                OptionsBar::cell(ui, "Theme:", CellWidth::Content, |ui, _| {
                    let current = builtin(&a.theme).map_or(a.theme.as_str(), |t| t.name.as_str()).to_owned();
                    let mut theme = Select::new("gallery.theme", "Theme", &mut a.theme).placeholder(current).fit(true).size(ControlSize::Sm);
                    for t in builtins() {
                        theme = theme.option(t.id.as_str().to_owned(), &t.name);
                    }
                    let _ = theme.policy(fields::policy()).show(ui);
                });
                segmented(ui, "Mode", &mut a.mode, [("Light", ModeChoice::Light), ("Dark", ModeChoice::Dark), ("System", ModeChoice::System)]);
                segmented(
                    ui,
                    "Contrast",
                    &mut a.contrast,
                    [("Standard", ContrastChoice::Standard), ("High", ContrastChoice::High), ("System", ContrastChoice::System)],
                );
                segmented(ui, "Size", &mut a.comfortable, [("Compact", false), ("Comfortable", true)]);
                let more = ui.add(IconButton::new(Icon::More, "More appearance settings").size(ControlSize::Sm));
                if more.clicked() {
                    *more_open = !*more_open;
                }
                Popover::new("gallery.more", PopoverAnchor::Control(more.clone())).toggle(more.id).label("More appearance settings").show(
                    ui,
                    more_open,
                    |ui, _| {
                        FormGrid::new("gallery.more.form").show(ui, |form| {
                            form.row("Reduced motion", |ui, _| {
                                choice(
                                    ui,
                                    "Reduced motion",
                                    &mut a.motion,
                                    [("Off", MotionChoice::Full), ("On", MotionChoice::Reduced), ("System", MotionChoice::System)],
                                )
                            });
                            form.row("Direction", |ui, _| choice(ui, "Direction", &mut a.rtl, [("LTR", false), ("RTL", true)]));
                            form.row("Text", |ui, _| {
                                let mut text = Segmented::new("gallery.text_size", "Text", &mut a.text_size).size(ControlSize::Sm);
                                for (i, size) in TextSize::ALL.iter().enumerate() {
                                    text = text.item(i, text_size_name(*size));
                                }
                                text.show(ui)
                            });
                            if let Some((ui_shown, mono_shown, ui_faces)) = &faces_shown {
                                form.row("UI face", |ui, _| face_select(ui, "UI face", "gallery.ui_face", &mut a.ui_face, ui_shown, ui_faces));
                                form.row("Mono face", |ui, _| {
                                    face_select(ui, "Mono face", "gallery.mono_face", &mut a.mono_face, mono_shown, faces::MONO_FACES)
                                });
                            }
                            form.row("Icons", |ui, _| {
                                let mut icon_set = icon_set_id;
                                let mut icons = Select::new("gallery.icon_set", "Icons", &mut icon_set).fit(true).size(ControlSize::Sm);
                                for set in IconSet::ALL {
                                    icons = icons.option(set.id().to_owned(), set.title());
                                }
                                let shown = icons.policy(fields::policy()).show(ui);
                                if shown.response.changed() {
                                    a.icon_set = icon_set;
                                }
                                shown.response
                            });
                            form.row("Scale", |ui, label| {
                                let mut field = NumberField::new("gallery.scale", "Scale", a.scale, &numbers)
                                    .range(SCALE)
                                    .decimals(0..=0)
                                    .unit(Unit::new("{n}%"))
                                    .popup_slider("Scale slider")
                                    .size(ControlSize::Sm)
                                    .policy(fields::policy());
                                if let Some(label) = label.scrub() {
                                    field = field.scrub_by(label);
                                }
                                let scale = field.show(ui);
                                let mut percent = f64::from(a.scale);
                                if fields::apply(&scale, &mut percent) {
                                    a.scale = percent.round() as u32;
                                }
                                scale.response
                            });
                        });
                    },
                );
                OptionsBar::divider(ui);
                match current {
                    GalleryWorkspace::Components => {
                        // The toggle only shows the state; the menu opens on the press.
                        let mut shown = variants_shown;
                        let button = ui.add(ToggleButton::new("Variants", &mut shown).trailing_icon(Icon::ChevronDown).size(ControlSize::Sm));
                        let button = if has_target { button } else { button.tooltip("Select a component first") };
                        let entries: Vec<MenuEntry<VariantAxis>> =
                            VariantAxis::ALL.into_iter().map(|axis| MenuItem::new(axis.menu_label(), axis).enabled(has_target).into()).collect();
                        if let Some(axis) = PressMenu::new(&entries).size(ControlSize::Sm).show(ui, &button).chosen {
                            out.variants = Some(axis);
                        }
                    }
                    GalleryWorkspace::Theme => {
                        segmented(ui, "Preview", preview, [("App", PreviewView::App), ("Components", PreviewView::Components)]);
                        let mut picking = false;
                        ui.add(ToggleButton::new("Pick from preview", &mut picking).icon(Icon::Eyedropper).enabled(false).size(ControlSize::Sm))
                            .tooltip("Comes with token picking");
                    }
                }
            });
            bar.trailing(|ui| {
                if current == GalleryWorkspace::Theme {
                    ui.add(Button::new("Revert all").enabled(false).size(ControlSize::Sm)).tooltip("Comes with theme editing");
                    ui.add(Button::new("Save as…").variant(ButtonVariant::Primary).enabled(false).size(ControlSize::Sm))
                        .tooltip("Comes with theme editing");
                    OptionsBar::divider(ui);
                }
                let reset = ui.add(IconButton::new(Icon::Undo, "Reset layout").show_tooltip(false).size(ControlSize::Sm));
                if reset.tooltip("Reset this workspace's layout").clicked() {
                    out.reset_layout = true;
                }
                OptionsBar::cell(ui, "Workspace:", CellWidth::Content, |ui, _| {
                    let mut segmented = Segmented::new(("gallery.segmented", "Workspace"), "Workspace", workspace).size(ControlSize::Sm);
                    for option in GalleryWorkspace::ALL {
                        segmented = segmented.item(option, option.title());
                    }
                    segmented.show(ui);
                });
            });
        });
        out
    }
}

fn text_size_name(size: TextSize) -> &'static str {
    match size {
        TextSize::Small => "Small",
        TextSize::Default => "Default",
        TextSize::Large => "Large",
        TextSize::Larger => "Larger",
    }
}

/// A face picker showing `shown`, the face in use for the stored pick `value`; choosing an option
/// stores it.
fn face_select(ui: &mut Ui, label: &str, id: &str, value: &mut String, shown: &str, options: &[&str]) -> egui::Response {
    let mut pick = shown.to_owned();
    let mut select = Select::new(id, label, &mut pick).fit(true).size(ControlSize::Sm);
    for option in options {
        select = select.option((*option).to_owned(), *option);
    }
    let out = select.policy(fields::policy()).show(ui);
    if out.response.changed() {
        *value = pick;
    }
    out.response
}

/// A captioned set of exclusive choices in the options bar.
fn segmented<T: PartialEq + Clone, const N: usize>(ui: &mut Ui, label: &str, value: &mut T, options: [(&str, T); N]) {
    OptionsBar::cell(ui, &format!("{label}:"), CellWidth::Content, |ui, _| {
        choice(ui, label, value, options);
    });
}

/// Exclusive choices as a segmented control.
fn choice<T: PartialEq + Clone, const N: usize>(ui: &mut Ui, label: &str, value: &mut T, options: [(&str, T); N]) -> egui::Response {
    let mut segmented = Segmented::new(("gallery.segmented", label), label, value).size(ControlSize::Sm);
    for (name, option) in options {
        segmented = segmented.item(option, name);
    }
    segmented.show(ui)
}
