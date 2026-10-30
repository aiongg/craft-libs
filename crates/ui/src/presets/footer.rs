//! `PresetFooter`: the size slider and action buttons (DS-44 task 4).

use egui::{Ui, UiBuilder};

use super::labels::PresetLabels;
use super::state::PresetState;
use super::style::PresetStyle;
use crate::composites::PanelFooter;
use crate::controls::Slider;
use crate::direction;
use crate::edit::EditEvent;
use crate::icons::Icon;
use crate::localized::Words;
use crate::localized::words::counts;
use crate::theme::{Tokens, TokensOf as _};
use craft_theme::ControlSize;

use std::ops::RangeInclusive;

/// What a [`PresetFooter`] reports.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FooterEvent {
    /// The New button.
    New,
    /// The New Group button.
    NewGroup,
    /// The Delete button.
    Delete,
    /// A host action added with [`PresetFooter::action`], by its id.
    Custom(&'static str),
}

type Leading<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// The footer of a preset panel: the thumbnail size slider and the count or the host's own
/// controls at the reading start, the action buttons at the reading end.
///
/// ```
/// use craft_ui::presets::{FooterEvent, PresetFooter, PresetLabels, PresetState};
/// # egui::__run_test_ui(|ui| {
/// let mut state = PresetState::<u32>::new();
/// let labels = PresetLabels::default();
/// for event in PresetFooter::new(&mut state, &labels).can_delete(false).show(ui) {
///     match event {
///         FooterEvent::New => { /* make a preset */ }
///         _ => {}
///     }
/// }
/// # });
/// ```
///
/// - **Placement.** Show it in the footer slot of a panel, or standalone: it draws the same
///   `PanelFooter` bar and rule.
/// - **Buttons.** Added from the reading end: Delete (disabled by `can_delete(false)`), New, New
///   Group (left out by `new_group(false)`), then the host's actions. Their tooltips and names
///   come from [`PresetLabels`].
/// - **Slider.** Writes [`PresetState::thumb`] within `thumb_range`; omitted by `slider(false)`.
///   Its name is [`PresetLabels::thumb_size`].
#[must_use = "show the footer with `show`"]
pub struct PresetFooter<'a, K> {
    state: &'a mut PresetState<K>,
    labels: &'a PresetLabels,
    thumb_range: RangeInclusive<f32>,
    slider: bool,
    count: Option<usize>,
    leading: Option<Leading<'a>>,
    can_delete: bool,
    new_group: bool,
    actions: Vec<(&'static str, Icon, String)>,
}

impl<'a, K> PresetFooter<'a, K> {
    /// A footer writing the thumbnail size into `state`, with `labels`' words.
    pub fn new(state: &'a mut PresetState<K>, labels: &'a PresetLabels) -> Self {
        Self {
            state,
            labels,
            thumb_range: 16.0..=96.0,
            slider: true,
            count: None,
            leading: None,
            can_delete: true,
            new_group: true,
            actions: Vec::new(),
        }
    }

    /// The range of the slider, in points (default 16 to 96): use the browser's.
    pub fn thumb_range(mut self, range: RangeInclusive<f32>) -> Self {
        self.thumb_range = range;
        self
    }

    /// Whether the size slider is shown (default yes).
    pub fn slider(mut self, slider: bool) -> Self {
        self.slider = slider;
        self
    }

    /// Shows [`PresetLabels::count`] with `count` after the slider.
    pub fn count(mut self, count: usize) -> Self {
        self.count = Some(count);
        self
    }

    /// The host's own controls at the reading start, before the slider (a "Current Tool Only"
    /// checkbox).
    pub fn leading(mut self, add: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.leading = Some(Box::new(add));
        self
    }

    /// Whether Delete can be used, which is when something is selected (default yes).
    pub fn can_delete(mut self, can_delete: bool) -> Self {
        self.can_delete = can_delete;
        self
    }

    /// Whether the New Group button is shown (default yes).
    pub fn new_group(mut self, new_group: bool) -> Self {
        self.new_group = new_group;
        self
    }

    /// A host button after New Group, reported as [`FooterEvent::Custom`] with `id`.
    pub fn action(mut self, id: &'static str, icon: Icon, label: impl Into<String>) -> Self {
        self.actions.push((id, icon, label.into()));
        self
    }

    /// Shows the footer across the available width and returns what was pressed, in order.
    pub fn show(self, ui: &mut Ui) -> Vec<FooterEvent> {
        let Self { state, labels, thumb_range, slider, count, leading, can_delete, new_group, actions } = self;
        let mut events = Vec::new();
        let style = PresetStyle::from_tokens(&Tokens::of(ui));
        PanelFooter::show(ui, |ui| {
            let delete = ui.add_enabled_ui(can_delete, |ui| PanelFooter::button(ui, Icon::Trash, &labels.delete_button)).inner;
            if delete.clicked() {
                events.push(FooterEvent::Delete);
            }
            if PanelFooter::button(ui, Icon::Plus, &labels.new).clicked() {
                events.push(FooterEvent::New);
            }
            if new_group && PanelFooter::button(ui, Icon::NewFolder, &labels.new_group).clicked() {
                events.push(FooterEvent::NewGroup);
            }
            for (id, icon, label) in &actions {
                if PanelFooter::button(ui, *icon, label).clicked() {
                    events.push(FooterEvent::Custom(id));
                }
            }
            let rest = ui.available_rect_before_wrap();
            let mut start = ui.new_child(UiBuilder::new().max_rect(rest).layout(direction::row(ui)));
            let start = &mut start;
            if let Some(add) = leading {
                add(start);
            }
            if slider {
                let value = state.thumb_in(&thumb_range);
                let range = *thumb_range.start()..=thumb_range.end().max(*thumb_range.start());
                let edit =
                    Slider::new("thumb-size", labels.thumb_size.as_str(), value, range).size(ControlSize::Md).width(style.slider_width).show(start);
                if let Some(EditEvent::Update { value, .. } | EditEvent::Set { value, .. } | EditEvent::Commit { value, .. }) = edit.event {
                    state.thumb = value as f32;
                }
            }
            if let Some(n) = count {
                start.label(egui::RichText::new(Words::of(start).counted(counts::PRESETS, &labels.count, n as u64)).color(style.count));
            }
        });
        events
    }
}
