//! [`LiquifyPanel`]: Liquify's Properties panel (task WS-LQ2).

use std::ops::RangeInclusive;

use egui::{AsIdSalt, IdSalt, Response, Ui};

use craft_theme::TypeRole;
use craft_ui::composites::{SectionHeader, SliderRow};
use craft_ui::controls::{Button, Checkbox, Select};
use craft_ui::edit::EditEvent;
use craft_ui::localized::{LocaleNumbers, Unit};
use craft_ui::primitives::{Content, Gap, Row, Stack, Text};

use super::LiquifyLabels;
use super::session::{BackdropMode, BackdropUse, LiquifyGlobal, LiquifySettings, MeshSize};
use super::tools::LiquifyBrush;

/// What [`LiquifyPanel::show`] reports.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct LiquifyPanelOutput {
    pub response: Response,
    /// A setting changed this pass (already written to the settings).
    pub changed: bool,
    /// A slider is being dragged: the workspace's history waits.
    pub dragging: bool,
    /// Reconstruct (with the Amount), None, Mask All or Invert All pressed.
    pub global: Option<LiquifyGlobal>,
    /// Restore All pressed.
    pub restore_all: bool,
    /// Show Backdrop turned on, or Use changed while it is on: the backdrop to ask the app for
    /// (`None`: All Layers).
    pub backdrop_wanted: Option<Option<u64>>,
}

/// The Properties panel in the side column, top to bottom, as PhotoCraft (and Photoshop) order it:
///
/// - **Brush Tool Options** (`SectionHeader`): `SliderRow`s Size (1..=3000, the
///   [`LiquifyBrush::SLIDER_MAX`](super::LiquifyBrush::SLIDER_MAX)), Density, Pressure, Rate
///   (0..=100).
/// - **Brush Reconstruct Options**: Amount (0..=100 %); buttons Reconstruct and Restore All.
/// - **Mask Options**: buttons None, Mask All, Invert All.
/// - **View Options**: Show Mesh; Mesh Size (Small, Medium, Large); Show Mask; Show Backdrop and,
///   while it is on, Use (All Layers, then the source's layers bottom to top), Mode (In Front,
///   Behind, Blend), Opacity (0..=100).
///
/// PhotoCraft's stats line ("N stroke(s) · field … · dab … ms · preview … ms") is not drawn (lead
/// call 14). Every control is a Tab stop in that order; sliders report drags through `dragging`.
pub struct LiquifyPanel<'a> {
    id_salt: IdSalt,
    settings: &'a mut LiquifySettings,
    layers: &'a [(u64, String)],
    labels: &'a LiquifyLabels,
}

/// What the controls of one pass add up to.
#[derive(Default)]
struct Pass {
    changed: bool,
    dragging: bool,
    global: Option<LiquifyGlobal>,
    restore_all: bool,
    backdrop_wanted: Option<Option<u64>>,
}

const PERCENT: &str = "{n} %";

impl<'a> LiquifyPanel<'a> {
    pub fn new(id_salt: impl AsIdSalt, settings: &'a mut LiquifySettings, labels: &'a LiquifyLabels) -> Self {
        Self { id_salt: IdSalt::new(id_salt), settings, layers: &[], labels }
    }

    /// Show Backdrop › Use's layers, bottom to top ([`LiquifySource::layers`](super::LiquifySource::layers)).
    pub fn layers(mut self, layers: &'a [(u64, String)]) -> Self {
        self.layers = layers;
        self
    }

    pub fn show(self, ui: &mut Ui) -> LiquifyPanelOutput {
        // pc: crates/ui-egui/src/dialogs/liquify_ui.rs:995-1059 (the four sections).
        let Self { id_salt, settings, layers, labels } = self;
        let id = ui.make_persistent_id(id_salt);
        let numbers = LocaleNumbers::of(ui);
        let mut pass = Pass::default();
        let shown = Stack::new(Gap::Row).show(ui, |ui| {
            SectionHeader::new((id, "brush"), labels.brush_options.as_str()).show(ui, |ui| {
                let brush = &mut settings.brush;
                slider(ui, (id, "size"), &labels.size, &mut brush.size, 1.0..=LiquifyBrush::SLIDER_MAX, None, &numbers, &mut pass);
                slider(ui, (id, "density"), &labels.density, &mut brush.density, 0.0..=100.0, None, &numbers, &mut pass);
                slider(ui, (id, "pressure"), &labels.pressure, &mut brush.pressure, 0.0..=100.0, None, &numbers, &mut pass);
                slider(ui, (id, "rate"), &labels.rate, &mut brush.rate, 0.0..=100.0, None, &numbers, &mut pass);
            });
            SectionHeader::new((id, "reconstruct"), labels.reconstruct_options.as_str()).show(ui, |ui| {
                let amount = &mut settings.reconstruct_amount;
                slider(ui, (id, "amount"), &labels.amount, amount, 0.0..=100.0, Some(PERCENT), &numbers, &mut pass);
                let amount = *amount;
                Row::new(Gap::Inline).show(ui, |ui| {
                    if push(ui, (id, "reconstruct-button"), &labels.reconstruct) {
                        pass.global = Some(LiquifyGlobal::Reconstruct { amount });
                    }
                    if push(ui, (id, "restore-all"), &labels.restore_all) {
                        pass.restore_all = true;
                    }
                });
            });
            SectionHeader::new((id, "mask"), labels.mask_options.as_str()).show(ui, |ui| {
                Row::new(Gap::Inline).show(ui, |ui| {
                    if push(ui, (id, "mask-none"), &labels.mask_none) {
                        pass.global = Some(LiquifyGlobal::MaskNone);
                    }
                    if push(ui, (id, "mask-all"), &labels.mask_all) {
                        pass.global = Some(LiquifyGlobal::MaskAll);
                    }
                    if push(ui, (id, "invert-all"), &labels.invert_all) {
                        pass.global = Some(LiquifyGlobal::InvertAll);
                    }
                });
            });
            SectionHeader::new((id, "view"), labels.view_options.as_str()).show(ui, |ui| {
                view_options(ui, id, settings, layers, labels, &numbers, &mut pass);
            });
        });
        LiquifyPanelOutput {
            response: shown.response,
            changed: pass.changed,
            dragging: pass.dragging,
            global: pass.global,
            restore_all: pass.restore_all,
            backdrop_wanted: pass.backdrop_wanted,
        }
    }
}

/// Show Mesh, Mesh Size, Show Mask and the Show Backdrop group.
fn view_options(
    ui: &mut Ui,
    id: egui::Id,
    settings: &mut LiquifySettings,
    layers: &[(u64, String)],
    labels: &LiquifyLabels,
    numbers: &LocaleNumbers,
    pass: &mut Pass,
) {
    pass.changed |= check(ui, (id, "show-mesh"), &labels.show_mesh, &mut settings.show_mesh);
    let sizes = [(MeshSize::Small, &labels.small), (MeshSize::Medium, &labels.medium), (MeshSize::Large, &labels.large)];
    if let Some(size) = caption_row(ui, &labels.mesh_size, |ui| choose(ui, (id, "mesh-size"), &labels.mesh_size, settings.mesh_size, &sizes)) {
        settings.mesh_size = size;
        pass.changed = true;
    }
    pass.changed |= check(ui, (id, "show-mask"), &labels.show_mask, &mut settings.show_mask);
    if check(ui, (id, "show-backdrop"), &labels.show_backdrop, &mut settings.show_backdrop) {
        pass.changed = true;
        if settings.show_backdrop {
            pass.backdrop_wanted = Some(backdrop_id(settings.backdrop));
        }
    }
    if !settings.show_backdrop {
        return;
    }
    // A layer that is gone shows All Layers.
    let current = match settings.backdrop {
        BackdropUse::Layer(layer) if layers.iter().any(|(known, _)| *known == layer) => BackdropUse::Layer(layer),
        _ => BackdropUse::AllLayers,
    };
    let uses: Vec<(BackdropUse, &String)> = std::iter::once((BackdropUse::AllLayers, &labels.all_layers))
        .chain(layers.iter().map(|(layer, name)| (BackdropUse::Layer(*layer), name)))
        .collect();
    if let Some(picked) = caption_row(ui, &labels.backdrop_use, |ui| choose(ui, (id, "backdrop-use"), &labels.backdrop_use, current, &uses)) {
        settings.backdrop = picked;
        pass.changed = true;
        pass.backdrop_wanted = Some(backdrop_id(picked));
    }
    let modes = [(BackdropMode::InFront, &labels.in_front), (BackdropMode::Behind, &labels.behind), (BackdropMode::Blend, &labels.blend)];
    if let Some(mode) =
        caption_row(ui, &labels.backdrop_mode, |ui| choose(ui, (id, "backdrop-mode"), &labels.backdrop_mode, settings.backdrop_mode, &modes))
    {
        settings.backdrop_mode = mode;
        pass.changed = true;
    }
    slider(ui, (id, "opacity"), &labels.opacity, &mut settings.backdrop_opacity, 0.0..=100.0, None, numbers, pass);
}

/// The layer id the app is asked for: `None` for All Layers.
pub(super) fn backdrop_id(of: BackdropUse) -> Option<u64> {
    match of {
        BackdropUse::AllLayers => None,
        BackdropUse::Layer(id) => Some(id),
    }
}

/// A slider row over `value`, written as the user edits it.
#[expect(clippy::too_many_arguments, reason = "one row's parts, kept apart from the pass it adds to")]
fn slider(
    ui: &mut Ui,
    id_salt: impl AsIdSalt,
    label: &str,
    value: &mut f64,
    range: RangeInclusive<f64>,
    unit: Option<&str>,
    numbers: &LocaleNumbers,
    pass: &mut Pass,
) {
    let mut row = SliderRow::new(id_salt, label, *value, range, numbers).decimals(0).step(1.0);
    if let Some(unit) = unit {
        row = row.unit(Unit::new(unit));
    }
    let shown = row.show(ui);
    pass.dragging |= shown.slider.response.dragged() || shown.field.response.dragged();
    for event in shown.edits() {
        pass.dragging |= matches!(event, EditEvent::Begin { .. } | EditEvent::Update { .. });
        if let Some(next) = event.value().copied()
            && next != *value
        {
            *value = next;
            pass.changed = true;
        }
    }
}

/// A push button; whether it was clicked.
fn push(ui: &mut Ui, id_salt: impl AsIdSalt, label: &str) -> bool {
    ui.add(Button::new(label).id_salt(id_salt)).clicked()
}

/// A check box; whether the user toggled it.
fn check(ui: &mut Ui, id_salt: impl AsIdSalt, label: &str, value: &mut bool) -> bool {
    Checkbox::bool(label, value).id_salt(id_salt).show(ui).response.changed()
}

/// `label` muted before the control `add` draws.
fn caption_row<R>(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    Row::new(Gap::Inline)
        .show(ui, |ui| {
            ui.add(Text::new(label).role(TypeRole::Label).content(Content::Muted));
            add(ui)
        })
        .inner
}

/// A select over `options`; the value the user picked this pass.
fn choose<T: Clone + PartialEq + Send + Sync + 'static>(
    ui: &mut Ui,
    id_salt: impl AsIdSalt,
    label: &str,
    current: T,
    options: &[(T, &String)],
) -> Option<T> {
    let mut value = current;
    let mut select = Select::new(id_salt, label, &mut value);
    for (option, text) in options {
        select = select.option(option.clone(), text.as_str());
    }
    select.show(ui).event.and_then(|event| event.value().cloned())
}
