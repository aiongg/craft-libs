//! [`GradientMapEditor`]: Photoshop's Gradient Map adjustment: the preset picker, the
//! [`GradientEditor`](super::GradientEditor) stop bar with colour stops only, Reverse and Dither (task AD-6).
//!
//! It lives here, beside the gradient editor it builds on, rather than with the other adjustment
//! editors in craft-imaging: a family crate depends on no other family crate but craft-color
//! (lead call L5).

use egui::accesskit::Role;
use egui::{AsIdSalt, Id, IdSalt, Modifiers, Sense, Ui, UiBuilder};

use craft_theme::ControlSize;
use craft_ui::controls::{Button, Checkbox, IconButton};
use craft_ui::edit::{EditEvent, EditPolicy, EditResponse};
use craft_ui::icons::Icon;
use craft_ui::localized::{Labels as _, LocaleNumbers};
use craft_ui::menu::{MenuEntry, MenuItem, PressMenu};
use craft_ui::popover::{Popover, PopoverAnchor};
use craft_ui::primitives::{Gap, Stack};

use super::{
    ColorStop, GradientEditor, GradientGroup, GradientLabels, GradientPalette, GradientRef, GradientStops, PaletteLayout, Sampler, StopRef, StopRows,
};

/// Most colour stops a Gradient Map keeps.
const MAX_STOPS: usize = 64;
/// The preset list's height before it scrolls (pc: `gradient_map.rs:166` `.height(420.0)`).
const PRESET_LIST_HEIGHT: f32 = 420.0;
/// Fewest colour stops a Gradient Map keeps.
const MIN_STOPS: usize = 2;

/// A Gradient Map value: the map from tone (0 black … 1 white) to colour.
#[derive(Clone, Debug, PartialEq)]
pub struct GradientMap {
    /// Colour stops only ([`StopRows::ColorOnly`](super::StopRows::ColorOnly)): every alpha 1,
    /// no opacity stops; 2..=64 stops, locations rounded to 0.001.
    pub stops: GradientStops,
    /// Reverse: the map runs white to black.
    pub reverse: bool,
    /// Dither: the host adds noise to hide banding.
    pub dither: bool,
}

craft_ui::words! {
    /// The Gradient Map editor's words, in the installed language unless set.
    #[non_exhaustive]
    pub struct GradientMapLabels {
        /// The editor's group name (AccessKit).
        gradient_map: "gradientMap.title" = "Gradient Map";
        /// The stop bar's own name (AccessKit), distinct from the group's.
        stops: "gradientMap.stops" = "Gradient stops";
        preset: "gradientMap.preset" = "Preset";
        /// The preset button's text while the stops match no preset (always: the editor keeps no
        /// preset identity).
        custom: "gradientMap.custom" = "Custom";
        preset_options: "gradientMap.presetOptions" = "Gradient Preset Options";
        import_presets: "gradientMap.importPresets" = "Import .GRD";
        reverse: "gradientMap.reverse" = "Reverse";
        dither: "gradientMap.dither" = "Dither";
    }
}

/// What [`GradientMapEditor::show`] reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct GradientMapOutput {
    /// The map's edit: the stop bar's gestures and sets with the stops replaced, a preset pick,
    /// Reverse and Dither as `Set`s. The response is the editor's group.
    pub edit: EditResponse<GradientMap>,
    /// Without [`color_popover`](GradientMapEditor::color_popover): the colour stop whose colour
    /// the host is asked to edit ([`GradientEditorResponse::color_requested`](super::GradientEditorResponse::color_requested)).
    pub color_requested: Option<usize>,
    /// With the popover: its eyedropper was pressed, and the popover's picker id while shown
    /// (as [`GradientEditorResponse`](super::GradientEditorResponse) reports them).
    pub eyedropper_requested: bool,
    pub picker_id: Option<Id>,
    /// The user chose Import .GRD in the preset menu: the host opens its gradient-file dialog
    /// and adds the file's presets to its library; the value is unchanged.
    pub import_presets: bool,
}

/// Gradient Map: Image › Adjustments › Gradient Map and a Gradient Map layer's Properties.
///
/// Top to bottom (PhotoCraft's order):
/// 1. **Preset**: a button reading "Custom" opens a popover with the host's
///    [`GradientGroup`]s in a [`GradientPalette`](super::GradientPalette) (List layout), a gear
///    menu with Import .GRD above it. A pick replaces the stops with the preset's colour stops
///    (alpha forced to 1, opacity stops dropped) as one `Set`, closes the popover and selects the
///    first stop. The host resolves Foreground and Background stops to its current colours
///    before passing the groups.
/// 2. **The stop bar**: [`GradientEditor`](super::GradientEditor) with [`StopRows::ColorOnly`](super::StopRows::ColorOnly),
///    2 to 64 stops, no midpoints, location fields in percent with PhotoCraft's 0.001 rounding of
///    locations. Its edits come out as edits of the map.
/// 3. **Reverse** and **Dither** check boxes, each one `Set`.
///
/// Keyboard and AccessKit: the preset button, the stop bar (as `GradientEditor`), then the two
/// check boxes are Tab stops; the editor is a `Group` named "Gradient Map".
#[must_use = "show the editor with `show`"]
pub struct GradientMapEditor<'a> {
    id_salt: IdSalt,
    value: &'a GradientMap,
    numbers: &'a LocaleNumbers,
    presets: &'a [GradientGroup],
    sampler: Option<&'a Sampler>,
    color_popover: bool,
    policy: EditPolicy,
    labels: Option<GradientMapLabels>,
    stop_labels: Option<GradientLabels>,
    enabled: bool,
}

impl<'a> GradientMapEditor<'a> {
    /// The editor over `value`.
    pub fn new(id_salt: impl AsIdSalt, value: &'a GradientMap, numbers: &'a LocaleNumbers) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            value,
            numbers,
            presets: &[],
            sampler: None,
            color_popover: false,
            policy: EditPolicy::default(),
            labels: None,
            stop_labels: None,
            enabled: true,
        }
    }

    /// The host's gradient presets (the same library as the Gradient tool and the Gradients
    /// panel); the preset button is disabled without any.
    pub fn presets(mut self, groups: &'a [GradientGroup]) -> Self {
        self.presets = groups;
        self
    }

    /// The host's colour mixing for the bar and the presets ([`GradientEditor::sampler`](super::GradientEditor::sampler)).
    pub fn sampler(mut self, sampler: &'a Sampler) -> Self {
        self.sampler = Some(sampler);
        self
    }

    /// Edits a stop's colour in the bar's own colour popover ([`GradientEditor::color_popover`](super::GradientEditor::color_popover))
    /// instead of asking the host; off by default (PhotoCraft opens its Color Picker).
    pub fn color_popover(mut self, popover: bool) -> Self {
        self.color_popover = popover;
        self
    }

    /// The stop bar's edit policy.
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// The editor's words; the installed ones by default.
    pub fn labels(mut self, labels: GradientMapLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The stop bar's own words ([`GradientEditor::labels`](super::GradientEditor::labels)).
    pub fn stop_labels(mut self, labels: GradientLabels) -> Self {
        self.stop_labels = Some(labels);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Draws the editor and reports the map's edit.
    pub fn show(self, ui: &mut Ui) -> GradientMapOutput {
        // pc: crates/ui-egui/src/dialogs/adjust_editors/gradient_map.rs:151-269 `gradient_map` (preset
        // combo with the gear and Import .GRD :158-205, the strip :206, 64 stops at most :120 and
        // :243, two at least :212, Reverse and Dither :259-266); locations rounded to 0.001 (:65).
        let labels = self.labels.clone().unwrap_or_else(|| GradientMapLabels::of(ui));
        let enabled = self.enabled && ui.is_enabled();
        let id = ui.make_persistent_id(self.id_salt);
        let builder =
            UiBuilder::new().id_salt(id).sense(Sense::hover()).accessibility_role(Role::Group).accessibility_label(labels.gradient_map.as_str());
        let scoped = ui.scope_builder(builder, |ui| self.show_inside(ui, id, &labels, enabled));
        let mut output = scoped.inner;
        output.edit.response = scoped.response;
        output
    }

    fn show_inside(&self, ui: &mut Ui, id: Id, labels: &GradientMapLabels, enabled: bool) -> GradientMapOutput {
        let mut event: Option<EditEvent<GradientMap>> = None;
        let mut import_presets = false;
        let mut output = GradientMapOutput {
            edit: EditResponse::new(ui.response(), None),
            color_requested: None,
            eyedropper_requested: false,
            picker_id: None,
            import_presets: false,
        };
        let bar_id = GradientEditor::id_in(ui, "stops");
        Stack::new(Gap::Stack).show(ui, |ui| {
            ui.add_enabled_ui(enabled, |ui| {
                // 1. Preset.
                let (pick, import) = self.preset_row(ui, id, labels);
                import_presets = import;
                if let Some(pick) = pick {
                    GradientEditor::select(ui.ctx(), bar_id, Some(StopRef::Color(0)));
                    event = Some(EditEvent::set(GradientMap { stops: pick, ..self.value.clone() }, Modifiers::NONE));
                }

                // 2. The stop bar.
                let numbers = self.numbers;
                let mut bar = GradientEditor::new("stops", labels.stops.as_str(), &self.value.stops, numbers)
                    .rows(StopRows::ColorOnly)
                    .stops(MIN_STOPS, MAX_STOPS)
                    .midpoints(false)
                    .location_decimals(1)
                    .policy(self.policy)
                    .color_popover(self.color_popover)
                    .enabled(enabled);
                if let Some(sampler) = self.sampler {
                    bar = bar.sampler(sampler);
                }
                if let Some(stop_labels) = &self.stop_labels {
                    bar = bar.labels(stop_labels.clone());
                }
                let response = bar.show(ui);
                output.color_requested = response.color_requested;
                output.eyedropper_requested = response.eyedropper_requested;
                output.picker_id = response.picker_id;
                let value = self.value.clone();
                let bar_event = response.edit.map(|stops| GradientMap { stops, ..value }).event;
                event = event.take().or(bar_event);

                // 3. Reverse and Dither.
                let mut reverse = self.value.reverse;
                let toggled = Checkbox::bool(labels.reverse.as_str(), &mut reverse).id_salt((id, "reverse")).show(ui);
                let value = self.value.clone();
                event = event.take().or(toggled.map(|reverse| GradientMap { reverse, ..value }).event);
                let mut dither = self.value.dither;
                let toggled = Checkbox::bool(labels.dither.as_str(), &mut dither).id_salt((id, "dither")).show(ui);
                let value = self.value.clone();
                event = event.take().or(toggled.map(|dither| GradientMap { dither, ..value }).event);
            });
        });
        output.edit.event = event;
        output.import_presets = import_presets;
        output
    }

    /// The preset button, its popover with the gear menu and the list. Returns a picked preset's
    /// colour stops and whether Import .GRD was chosen.
    fn preset_row(&self, ui: &mut Ui, id: Id, labels: &GradientMapLabels) -> (Option<GradientStops>, bool) {
        let button =
            ui.add_enabled(!self.presets.is_empty(), Button::new(labels.custom.as_str()).trailing_icon(Icon::ChevronDown).id_salt((id, "preset")));
        let open_id = id.with("open");
        let mut open = ui.data(|d| d.get_temp::<bool>(open_id)).unwrap_or(false);
        if button.clicked() {
            open = !open;
        }
        let mut pick = None;
        let mut import = false;
        Popover::new((id, "popover"), PopoverAnchor::Control(button.clone())).toggle(button.id).label(labels.preset.as_str()).show(
            ui,
            &mut open,
            |ui, popover| {
                let gear = ui.add(IconButton::new(Icon::Settings, labels.preset_options.as_str()).size(ControlSize::Sm).id_salt((id, "gear")));
                let entries = [MenuEntry::from(MenuItem::new(labels.import_presets.as_str(), ()))];
                if PressMenu::new(&entries).size(ControlSize::Sm).show(ui, &gear).chosen.is_some() {
                    import = true;
                    popover.close();
                }
                let mut palette = GradientPalette::new((id, "presets"), labels.preset.as_str(), self.presets)
                    .layout(PaletteLayout::List)
                    .rows(StopRows::ColorOnly)
                    .max_height(PRESET_LIST_HEIGHT);
                if let Some(sampler) = self.sampler {
                    palette = palette.sampler(sampler);
                }
                if let Some(GradientRef::Item { group, item }) = palette.show(ui).event.and_then(|e| e.value().copied())
                    && let Some(preset) = self.presets.get(group).and_then(|g| g.items.get(item))
                    && let Some(stops) = color_only(&preset.stops)
                {
                    pick = Some(stops);
                    popover.close();
                }
            },
        );
        ui.data_mut(|d| d.insert_temp(open_id, open));
        (pick, import)
    }
}

/// A preset's colour stops as a Gradient Map keeps them: alpha 1, no opacity stops, locations
/// rounded to 0.001, at most 64. `None` for a preset with fewer than two colour stops.
fn color_only(stops: &GradientStops) -> Option<GradientStops> {
    let color: Vec<ColorStop> = stops
        .color
        .iter()
        .take(MAX_STOPS)
        .map(|s| {
            let mut color = s.color;
            color.a = 1.0;
            ColorStop { color, midpoint: 0.5, ..*s }
        })
        .collect();
    (color.len() >= MIN_STOPS).then(|| GradientStops::new(color))
}
