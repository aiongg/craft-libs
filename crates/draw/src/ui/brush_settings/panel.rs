//! The Brush Settings panel frame ([`BrushSettingsPanel`]).
#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::fmt::Debug;
use std::hash::Hash;

use craft_ui::canvas::textures::Revision;
use egui::{AsIdSalt, ColorImage, IdSalt, Response, Ui};

use super::labels::BrushSettingsLabels;
use super::sections::{BrushSection, BrushSectionEvent};

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
        todo!()
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
