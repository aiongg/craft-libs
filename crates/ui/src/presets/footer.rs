//! `PresetFooter`: the size slider and action buttons (DS-44 task 4).

use egui::{AsIdSalt, IdSalt, Ui, UiBuilder};

use super::labels::PresetLabels;
use super::state::PresetState;
use super::style::PresetStyle;
use crate::behavior::ListDrag;
use crate::composites::{FooterAction, PanelFooter};
use crate::controls::Slider;
use crate::direction;
use crate::edit::EditEvent;
use crate::icons::Icon;
use crate::localized::words::counts;
use crate::localized::{Labels as _, Words};
use crate::theme::{Tokens, TokensOf as _};
use craft_theme::ControlSize;

use std::fmt::Debug;
use std::hash::Hash;
use std::ops::RangeInclusive;

use super::PresetBrowser;

/// What a [`PresetFooter`] reports. A browser's footer reports these as
/// [`PresetEvent::Footer`](super::PresetEvent::Footer), in order with its other events.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PresetFooterEvent<K> {
    /// The New button.
    New,
    /// The New Group button.
    NewGroup,
    /// The Delete button.
    Delete,
    /// A host action added with [`PresetFooter::action`] was pressed, by its id.
    Custom(&'static str),
    /// Presets or groups dragged from a list were dropped on a host action that
    /// [`accepts_drops`](FooterAction::accepts_drops), by its id.
    CustomDrop {
        /// The action's id.
        id: &'static str,
        /// The dragged keys.
        keys: Vec<K>,
    },
}

type Leading<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// The footer of a preset panel: the thumbnail size slider and the count or the host's own
/// controls at the reading start, the action buttons at the reading end.
///
/// ```
/// use craft_ui::presets::{PresetFooter, PresetFooterEvent, PresetState};
/// # egui::__run_test_ui(|ui| {
/// let mut state = PresetState::<u32>::new();
/// for event in PresetFooter::new("gradients", &mut state).can_delete(false).show(ui) {
///     match event {
///         PresetFooterEvent::New => { /* make a preset */ }
///         _ => {}
///     }
/// }
/// # });
/// ```
///
/// - **Placement.** Show it in the footer slot of a panel, or standalone: it draws the same
///   `PanelFooter` bar and rule.
/// - **Buttons.** Added from the reading end: Delete (disabled by `can_delete(false)`), New, New
///   Group (left out by `new_group(false)`), then the host's [`FooterAction`]s. Their tooltips and
///   names come from [`PresetLabels`] ([`labels`](Self::labels); the theme's words otherwise).
/// - **Host actions.** A disabled action takes no click. An action that
///   [`accepts_drops`](FooterAction::accepts_drops) takes presets dragged from any list and
///   reports [`PresetFooterEvent::CustomDrop`]; it reads enabled while such a drag is over it.
/// - **Slider.** Writes [`PresetState::thumb`] within `thumb_range`; omitted by `slider(false)`.
///   Its name is [`PresetLabels::thumb_size`], its id derives from `id_salt`.
#[must_use = "show the footer with `show`"]
pub struct PresetFooter<'a, K> {
    id_salt: IdSalt,
    state: &'a mut PresetState<K>,
    labels: Option<&'a PresetLabels>,
    thumb_range: RangeInclusive<f32>,
    slider: bool,
    count: Option<usize>,
    leading: Option<Leading<'a>>,
    can_delete: bool,
    new_group: bool,
    actions: Vec<FooterAction>,
}

impl<'a, K> PresetFooter<'a, K> {
    /// A footer writing the thumbnail size into `state`. `id_salt` is unique among the app's
    /// footers.
    pub fn new(id_salt: impl AsIdSalt, state: &'a mut PresetState<K>) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            state,
            labels: None,
            thumb_range: 16.0..=96.0,
            slider: true,
            count: None,
            leading: None,
            can_delete: true,
            new_group: true,
            actions: Vec::new(),
        }
    }

    /// The footer's words (default: the theme's).
    pub fn labels(mut self, labels: &'a PresetLabels) -> Self {
        self.labels = Some(labels);
        self
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

    /// A host button after New Group, reported as [`PresetFooterEvent::Custom`] with its id.
    pub fn action(mut self, action: FooterAction) -> Self {
        self.actions.push(action);
        self
    }
}

impl<K: Clone + Send + Sync + 'static> PresetFooter<'_, K> {
    /// Shows the footer across the available width and returns what was pressed, in order.
    pub fn show(self, ui: &mut Ui) -> Vec<PresetFooterEvent<K>> {
        let Self { id_salt, state, labels, thumb_range, slider, count, leading, can_delete, new_group, actions } = self;
        let owned;
        let labels = match labels {
            Some(labels) => labels,
            None => {
                owned = PresetLabels::of(ui);
                &owned
            }
        };
        let slider_salt = IdSalt::new((id_salt, "thumb-size"));
        let mut events = Vec::new();
        let style = PresetStyle::from_tokens(&Tokens::of(ui));
        PanelFooter::show(ui, |ui| {
            let delete = ui.add_enabled_ui(can_delete, |ui| PanelFooter::button(ui, Icon::Trash, &labels.delete_button)).inner;
            if delete.clicked() {
                events.push(PresetFooterEvent::Delete);
            }
            if PanelFooter::button(ui, Icon::Plus, &labels.new).clicked() {
                events.push(PresetFooterEvent::New);
            }
            if new_group && PanelFooter::button(ui, Icon::NewFolder, &labels.new_group).clicked() {
                events.push(PresetFooterEvent::NewGroup);
            }
            for action in &actions {
                if action.accepts_drops {
                    let (button, dropped) =
                        PanelFooter::drop_button::<ListDrag<K>>(ui, action.icon, &action.label, action.enabled, |drag| !drag.items.is_empty());
                    if let Some(drag) = dropped {
                        events.push(PresetFooterEvent::CustomDrop { id: action.id, keys: drag.items.clone() });
                    } else if button.clicked() {
                        events.push(PresetFooterEvent::Custom(action.id));
                    }
                } else if ui.add_enabled_ui(action.enabled, |ui| PanelFooter::button(ui, action.icon, &action.label)).inner.clicked() {
                    events.push(PresetFooterEvent::Custom(action.id));
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
                    Slider::new(slider_salt, labels.thumb_size.as_str(), value, range).size(ControlSize::Md).width(style.slider_width).show(start);
                if let Some(
                    EditEvent::Begin { value: Some(value), .. }
                    | EditEvent::Update { value, .. }
                    | EditEvent::Set { value, .. }
                    | EditEvent::Commit { value, .. },
                ) = edit.event
                {
                    state.thumb = value as f32;
                }
            }
            if let Some(n) = count {
                start.label(egui::RichText::new(Words::of(start).counted(start.ctx(), counts::PRESETS, &labels.count, n as u64)).color(style.count));
            }
        });
        events
    }
}

/// The footer a [`PresetBrowser`] draws inside its own pass
/// ([`PresetBrowser::footer`]): the same buttons, slider, count and
/// leading slot as [`PresetFooter`], with the browser's state, words and thumbnail range. Its
/// presses come back in [`PresetEvent::Footer`](super::PresetEvent::Footer).
///
/// ```ignore
/// let out = PresetBrowser::new("gradients", "Gradients", &mut state, &rows)
///     .footer(PresetFooterOptions::new().count(library.total()).can_delete(selected.is_some()))
///     .show(ui, paint);
/// for event in out.events { /* PresetEvent::Footer(PresetFooterEvent::New), .. */ }
/// ```
#[must_use]
pub struct PresetFooterOptions<'a> {
    pub(super) slider: bool,
    pub(super) count: Option<usize>,
    pub(super) leading: Option<Leading<'a>>,
    pub(super) can_delete: bool,
    pub(super) new_group: bool,
    pub(super) actions: Vec<FooterAction>,
}

impl Default for PresetFooterOptions<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> PresetFooterOptions<'a> {
    /// The footer with the slider, Delete, New and New Group, as [`PresetFooter::new`].
    pub fn new() -> Self {
        Self { slider: true, count: None, leading: None, can_delete: true, new_group: true, actions: Vec::new() }
    }

    /// As [`PresetFooter::slider`].
    pub fn slider(mut self, slider: bool) -> Self {
        self.slider = slider;
        self
    }

    /// As [`PresetFooter::count`].
    pub fn count(mut self, count: usize) -> Self {
        self.count = Some(count);
        self
    }

    /// As [`PresetFooter::leading`] (Tool Presets' "Current Tool Only" check box).
    pub fn leading(mut self, add: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.leading = Some(Box::new(add));
        self
    }

    /// As [`PresetFooter::can_delete`].
    pub fn can_delete(mut self, can_delete: bool) -> Self {
        self.can_delete = can_delete;
        self
    }

    /// As [`PresetFooter::new_group`].
    pub fn new_group(mut self, new_group: bool) -> Self {
        self.new_group = new_group;
        self
    }

    /// As [`PresetFooter::action`].
    pub fn action(mut self, action: FooterAction) -> Self {
        self.actions.push(action);
        self
    }
}

/// The height the browser keeps for its footer below the list: `PanelFooter::reserved_height`
/// (from `panel_footer.*` tokens, not measured, so there is no one-pass lag and no
/// `FOOTER_GUESS`), [`PresetStyle::footer_room`](super::style::PresetStyle::footer_room) and the
/// vertical item spacing that follows the list.
/// The list takes `max_height.min(available) - footer_height`.
pub(super) fn footer_height(ui: &Ui) -> f32 {
    // pc: crates/ui-egui/src/panels/preset_panels.rs:354 (`FOOTER_GUESS` 32, measured last frame); new: Lead call 4.
    let tokens = Tokens::of(ui);
    let style = PanelFooter::style(&tokens, ui.pixels_per_point());
    PanelFooter::reserved_height(&style) + super::style::PresetStyle::from_tokens(&tokens).footer_room + ui.spacing().item_spacing.y
}

/// [`show_options`] with `footer_room` above it and no other spacing, so the browser's height
/// is the list's plus [`footer_height`].
pub(super) fn show_below<K>(
    ui: &mut Ui,
    id_salt: IdSalt,
    options: PresetFooterOptions<'_>,
    state: &mut PresetState<K>,
    labels: &PresetLabels,
    thumb_range: RangeInclusive<f32>,
) -> Vec<PresetFooterEvent<K>>
where
    K: Clone + Send + Sync + 'static,
{
    let room = PresetStyle::from_tokens(&Tokens::of(ui)).footer_room;
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        ui.add_space(room);
        show_options(ui, id_salt, options, state, labels, thumb_range)
    })
    .inner
}

/// Draws `options` as the browser's footer, below the list, with `PresetFooter`'s layout:
/// writes `state.thumb` within `thumb_range` and returns the presses.
pub(super) fn show_options<K>(
    ui: &mut Ui,
    id_salt: IdSalt,
    options: PresetFooterOptions<'_>,
    state: &mut PresetState<K>,
    labels: &PresetLabels,
    thumb_range: RangeInclusive<f32>,
) -> Vec<PresetFooterEvent<K>>
where
    K: Clone + Send + Sync + 'static,
{
    let PresetFooterOptions { slider, count, leading, can_delete, new_group, actions } = options;
    let mut footer =
        PresetFooter::new(id_salt, state).labels(labels).thumb_range(thumb_range).slider(slider).can_delete(can_delete).new_group(new_group);
    if let Some(count) = count {
        footer = footer.count(count);
    }
    if let Some(add) = leading {
        footer.leading = Some(add);
    }
    for action in actions {
        footer = footer.action(action);
    }
    footer.show(ui)
}

impl<'a, K: Clone + Eq + Hash + Debug + Send + Sync + 'static> PresetBrowser<'a, K> {
    /// A footer drawn inside the browser's pass, below the list (`options` as
    /// [`PresetFooter`]'s): the list takes the height left after it, with no one-pass lag, and
    /// the presses come back in `PresetOutput::footer`. [`PresetFooter::new`] stays for hosts
    /// that lay out the footer themselves (`SwatchesPanel`).
    pub fn footer(mut self, options: PresetFooterOptions<'a>) -> Self {
        self.footer = Some(options);
        self
    }
}
