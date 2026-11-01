//! The Brush Settings panel frame ([`BrushSettingsPanel`]).
use std::fmt::Debug;
use std::hash::Hash;

use std::sync::Arc;

use craft_ui::canvas::textures::Revision;
use craft_ui::controls::{ControlSize, IconButton};
use craft_ui::icons::Icon;
use craft_ui::localized::SharedLabels as _;
use craft_ui::primitives::Divider;
use craft_ui::theme::{Tokens, TokensOf as _};
use egui::{AsIdSalt, Color32, ColorImage, IdSalt, Response, ScrollArea, Ui, vec2};

use super::labels::BrushSettingsLabels;
use super::preview::BrushStrokeStrip;
use super::sections::{BrushSection, BrushSectionEvent, BrushSectionKind, BrushSectionList};
use super::style::BrushSettingsStyle;

/// The buttons beside the stroke strip.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BrushPanelAction {
    /// "Create new brush from these settings": the app opens its New Brush Preset dialog.
    NewPreset,
    /// "Reset the brush to the defaults": the app resets the brush, keeping its colours,
    /// smoothing and locks (PhotoCraft), as one preset command.
    Reset,
    /// "Update the current brush with these settings" (enabled with
    /// [`can_update`](BrushSettingsPanel::can_update)): the app overwrites the current preset.
    UpdatePreset,
}

type MakeStrip<'a> = Box<dyn FnOnce([usize; 2]) -> ColorImage + 'a>;

/// Photoshop's Brush Settings panel: the [`BrushSectionList`](super::BrushSectionList) at the
/// leading side, a vertical line, the selected section's pane (the app's content) in a scroll
/// area, a hairline, then the [`BrushStrokeStrip`](super::BrushStrokeStrip) with the New, Reset
/// and Update icon buttons beside it. An ordinary dockable panel; its preferred width is
/// `brush.panel_width` (540).
///
/// - **Pane.** The `body` closure draws the selected pane section's controls, with the shared rows
///   ([`BrushSizeField`](super::BrushSizeField), [`TipEllipse`](super::TipEllipse),
///   [`DynamicControlRow`](super::DynamicControlRow)) and the library's controls. A section that
///   is off ([`BrushSection::is_off`]) runs it in a disabled `Ui` (greyed). One scroll position
///   per section. No title: the selected row names the pane.
/// - **Edits.** The panel makes none. The list's events and the actions go to the app; the
///   controls in the pane report their own edit events, which the app sends to its tool options
///   host (`GestureHost<QuietMarks<_>>`): brush settings never become history states.
/// - **Keys.** The list, the pane's controls, the buttons, in Tab order.
/// - **AccessKit.** The strip is an image "Stroke preview"; the buttons are named by their
///   tooltips.
/// - **RTL.** Mirrors: the list at the right, the buttons left of the strip.
#[must_use = "show the panel with `show`"]
pub struct BrushSettingsPanel<'a, K> {
    id_salt: IdSalt,
    sections: &'a [BrushSection<K>],
    selected: Option<&'a K>,
    link: bool,
    can_update: bool,
    strip: Option<(Revision, MakeStrip<'a>)>,
    labels: Option<BrushSettingsLabels>,
}

impl<'a, K: Clone + Eq + Hash + Debug + Send + Sync + 'static> BrushSettingsPanel<'a, K> {
    /// An empty panel; add the sections with [`sections`](Self::sections).
    pub fn new(id_salt: impl AsIdSalt) -> Self {
        Self { id_salt: IdSalt::new(id_salt), sections: &[], selected: None, link: false, can_update: false, strip: None, labels: None }
    }

    /// The section rows, in order.
    pub fn sections(mut self, sections: &'a [BrushSection<K>]) -> Self {
        self.sections = sections;
        self
    }

    /// The pane shown. `None`, or a key that isn't a pane section: the first pane section.
    pub fn selected(mut self, selected: Option<&'a K>) -> Self {
        self.selected = selected;
        self
    }

    /// Shows the list's "Brushes" link row.
    pub fn link(mut self, link: bool) -> Self {
        self.link = link;
        self
    }

    /// Enables Update (the brush was picked from a preset that still exists).
    pub fn can_update(mut self, can_update: bool) -> Self {
        self.can_update = can_update;
        self
    }

    /// The stroke strip's picture: `revision` of the brush settings and the app's render (see
    /// [`BrushStrokeStrip::show`](super::BrushStrokeStrip::show)). Without it the strip is empty.
    pub fn strip(mut self, revision: Revision, make: impl FnOnce([usize; 2]) -> ColorImage + 'a) -> Self {
        self.strip = Some((revision, Box::new(make)));
        self
    }

    /// The words; the installed language's by default.
    pub fn labels(mut self, labels: BrushSettingsLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Shows the panel; `body` draws the pane of the section it is given and its result comes back
    /// in [`BrushSettingsOutput::body`].
    pub fn show<R>(self, ui: &mut Ui, body: impl FnOnce(&mut Ui, &K) -> R) -> BrushSettingsOutput<K, R> {
        // pc: crates/ui-egui/src/panels/brush_panel.rs:351-412 `settings_tab` (list, vline, scroll pane max 450 greyed when off, hairline, strip and buttons)
        let Self { id_salt, sections, selected, link, can_update, strip, labels } = self;
        let labels = labels.map(Arc::new).unwrap_or_else(|| BrushSettingsLabels::shared(ui));
        let tokens = Tokens::of(ui);
        let style = BrushSettingsStyle::of(&tokens);
        let gap = tokens.space.gap.inline;
        let button_side = IconButton::style(&tokens, ControlSize::Md).side;

        // The pane shown: the selected pane section, else the first one.
        let is_pane = |section: &&BrushSection<K>| section.kind == BrushSectionKind::Pane;
        let shown =
            selected.and_then(|key| sections.iter().filter(is_pane).find(|section| section.key == *key)).or_else(|| sections.iter().find(is_pane));

        // The strip, its gaps and the hairline are reserved; the list and the pane share the rest.
        let hairline = ui.pixels_per_point().recip();
        let pane_height = (ui.available_height() - style.strip_height - 2.0 * gap - hairline).max(0.0);

        let mut body_result = None;
        let mut section_events = Vec::new();
        let mut action = None;
        let response = ui
            .vertical(|ui| {
                ui.allocate_ui(vec2(ui.available_width(), pane_height), |ui| {
                    ui.horizontal_top(|ui| {
                        let list = BrushSectionList::new((&id_salt, "sections"), sections)
                            .selected(shown.map(|section| &section.key))
                            .link(link)
                            .labels((*labels).clone())
                            .show(ui);
                        section_events = list.events;
                        Divider::vertical().show(ui);
                        ui.vertical(|ui| {
                            let Some(section) = shown else { return };
                            ScrollArea::vertical()
                                .id_salt((&id_salt, "pane", &section.key))
                                .max_height(pane_height)
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    body_result = Some(ui.add_enabled_ui(!section.is_off(), |ui| body(ui, &section.key)).inner);
                                });
                        });
                    });
                });
                ui.add_space(gap);
                Divider::horizontal().show(ui);
                ui.add_space(gap);
                ui.horizontal_top(|ui| {
                    let strip_width = (ui.available_width() - button_side - ui.spacing().item_spacing.x).max(0.0);
                    ui.allocate_ui(vec2(strip_width, style.strip_height), |ui| {
                        let (revision, make) =
                            strip.unwrap_or_else(|| (Revision::of(&()), Box::new(|[w, h]| ColorImage::filled([w, h], Color32::TRANSPARENT))));
                        BrushStrokeStrip::new((&id_salt, "strip"), revision).labels((*labels).clone()).show(ui, make);
                    });
                    ui.vertical(|ui| {
                        let button = |ui: &mut Ui, name: &str, icon: Icon, enabled: bool, which: BrushPanelAction| {
                            let response = ui.add(IconButton::new(icon, name).id_salt((&id_salt, "action", which)).enabled(enabled));
                            response.clicked().then_some(which)
                        };
                        let pressed = [
                            button(ui, &labels.new_preset, Icon::Plus, true, BrushPanelAction::NewPreset),
                            button(ui, &labels.reset, Icon::Undo, true, BrushPanelAction::Reset),
                            button(ui, &labels.update, Icon::Check, can_update, BrushPanelAction::UpdatePreset),
                        ];
                        action = pressed.into_iter().flatten().next();
                    });
                });
            })
            .response;
        BrushSettingsOutput { response, sections: section_events, action, body: body_result }
    }
}

/// What [`BrushSettingsPanel::show`] returns.
#[derive(Debug)]
#[non_exhaustive]
#[must_use = "the host applies the panel's events"]
pub struct BrushSettingsOutput<K, R> {
    /// The panel.
    pub response: Response,
    /// The section list's events, in order.
    pub sections: Vec<BrushSectionEvent<K>>,
    /// A button beside the strip, if one was pressed.
    pub action: Option<BrushPanelAction>,
    /// What `body` returned; `None` when no pane section exists.
    pub body: Option<R>,
}
