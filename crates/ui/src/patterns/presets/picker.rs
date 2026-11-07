//! `PresetPicker`: a closed field that shows the current preset and opens a `PresetBrowser` in a popover.
//!
//! The closed field is a `SelectFace` over the field well, as `Select` shows it. Hovering a preset in the popover
//! proposes it, a pick commits and closes, and closing without a pick restores the value.

#[cfg(feature = "testing")]
mod stories;

use super::browser::PresetBrowser;
use super::event::PresetEvent;
use super::model::{CellShape, PresetRow};
use super::state::{DefaultOpen, PresetState, PresetView};
use super::words::PresetWords;
use crate::contract::{
    Access, Bind, BindMut, Common, Component, Cx, Edit as _, EditResponse, MeasureCx, Outcome, Overlay, OverlayUi, Overlays as _, PaintUi, PartsCx,
    Props as _, Role, WordSetCx as _,
};
use crate::list::RowTest;
use crate::pickers::SelectFace;
use crate::runtime::env::{EnvView, Label};
use crate::style::{ComponentStyle, FocusLook, StyleCx, SurfaceLook};
use craft_behavior::{CancelReason, GesturePhase, HitLevel, HitPriority, Metrics, PartKey, Sense, Value};
use egui::{Response, Vec2};
use std::hash::Hash;

/// The face under the whole field.
const FACE: PartKey = PartKey::new("face");

/// What the closed field shows of the current preset.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PickerField {
    /// The thumbnail alone; the name when the preset has none.
    #[default]
    Thumbnail,
    /// The name alone.
    Name,
    /// The thumbnail, then the name.
    ThumbnailAndName,
}

/// The style of a preset picker: the field well and the focus edge; the face carries the padding.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct PresetPickerStyle {
    /// The field well.
    #[style(surface)]
    surface: SurfaceLook,
    /// No padding of its own: the face carries the field's.
    #[style(metrics)]
    metrics: Metrics,
    #[style(focus)]
    focus: FocusLook,
}

/// What a preset picker reports.
#[derive(Debug)]
#[non_exhaustive]
#[must_use = "the host applies the picker's events"]
pub struct PresetPickerOutput<K> {
    /// The field's response and the edit of its value.
    pub edit: EditResponse<Option<K>>,
    /// The preset picked in this pass, which the binding now holds.
    pub picked: Option<K>,
    /// What the browser in the popover reported besides the pick and the open groups: renames, deletes, the
    /// context menu's rows and the footer's buttons.
    pub events: Vec<PresetEvent<K>>,
}

impl<K: Value> Outcome for PresetPickerOutput<K> {
    type Value = Option<K>;

    fn response(&self) -> &Response {
        &self.edit.response
    }

    #[cfg(feature = "testing")]
    fn story_facts(&self) -> crate::testing::StoryOutput {
        self.edit.story_facts()
    }
}

/// A field that shows the current preset and opens its presets. A click, Enter, Space or Alt+Down opens the popover
/// with the browser; hovering a preset proposes it, a click or Enter picks it and closes, and Escape closes and
/// restores the value.
///
/// - **Value.** The binding holds the picked preset's key, or none while the value is no preset of `rows`; the field
///   then shows `placeholder`, "Custom" by default.
/// - **State.** `state` is the browser's view settings; bind it with `Bind::bound`.
/// - **Thumbnails.** Each row's `preview` paints its thumbnail, in the field and in the browser.
///
/// ```ignore
/// let out = PresetPicker::new("Contour", Bind::bound(&mut shape), &rows, Bind::bound(&mut state)).field(PickerField::ThumbnailAndName).show(ui);
/// ```
#[derive(crate::contract::Component, crate::contract::Props)]
#[component(overlay, probe(PresetPicker<'static, u32>))]
pub struct PresetPicker<'a, K: Value + Eq + Hash + 'static> {
    #[common]
    common: Common,
    value: Bind<'a, Option<K>>,
    rows: &'a [PresetRow<K>],
    state: Bind<'a, PresetState<K>>,
    /// What the closed field shows.
    #[prop]
    field: PickerField,
    /// The shape of the browser's tiles.
    #[prop]
    cell_shape: CellShape,
    /// How the browser shows the presets.
    #[prop]
    view: PresetView,
    /// Which groups of the browser are open before the user toggles any.
    #[prop]
    default_open: DefaultOpen,
    /// A row above the presets with the search field.
    #[prop]
    toolbar: bool,
    /// Which presets and groups the browser may rename.
    #[prop(func = "Fn(&K) -> bool")]
    can_rename: Option<RowTest<'a, K>>,
    /// Which presets and groups the browser's context menu may delete.
    #[prop(func = "Fn(&K) -> bool")]
    can_delete: Option<RowTest<'a, K>>,
    /// Shown, muted, while the value is no preset of the rows.
    #[prop(into)]
    placeholder: Option<Label>,
    /// Shown instead of the current preset's name.
    #[prop(into)]
    display_name: Option<Label>,
    picked: Option<K>,
    events: Vec<PresetEvent<K>>,
}

impl<'a, K: Value + Eq + Hash + 'static> PresetPicker<'a, K> {
    /// A picker named `label` over `value`, choosing among the presets of `rows`.
    pub fn new(label: impl Into<Label>, value: Bind<'a, Option<K>>, rows: &'a [PresetRow<K>], state: Bind<'a, PresetState<K>>) -> Self {
        PresetPicker {
            common: Common::new(),
            value,
            rows,
            state,
            field: PickerField::default(),
            cell_shape: CellShape::default(),
            view: PresetView::Grid,
            default_open: DefaultOpen::All,
            toolbar: false,
            can_rename: None,
            can_delete: None,
            placeholder: None,
            display_name: None,
            picked: None,
            events: Vec::new(),
        }
        .label(label)
    }

    /// The rows the picker chooses among.
    pub(super) fn preset_rows(&self) -> &'a [PresetRow<K>] {
        self.rows
    }

    /// The key the binding holds, when it holds one.
    pub(super) fn held_key(&mut self) -> Option<K> {
        match self.value.reborrow().get() {
            Some(craft_behavior::Mixed::Same(held)) => held,
            _ => None,
        }
    }

    fn preset(&self, key: &K) -> Option<&PresetRow<K>> {
        self.rows.iter().find(|row| !row.is_group() && row.key == *key)
    }

    /// What the field shows: the current preset's thumbnail and name; else, in the muted look, the mixed word for a
    /// mixed value or the placeholder.
    fn face(&self, value: Option<&Option<K>>, words: &PresetWords, env: &EnvView<'_>) -> SelectFace<'static> {
        let Some(value) = value else {
            return SelectFace::new(env.resolve(&words.mixed)).placeholder();
        };
        let Some(row) = value.as_ref().and_then(|key| self.preset(key)) else {
            let text = self.placeholder.as_ref().unwrap_or(&words.custom);
            return SelectFace::new(env.resolve(text)).placeholder();
        };
        let text = self.display_name.as_ref().map_or_else(|| Label::from(row.label.clone()), |name| env.resolve(name));
        let face = SelectFace::new(text);
        match (&row.preview, self.field) {
            (_, PickerField::Name) | (None, _) => face,
            (Some(preview), field) => {
                let paint = preview.paint.clone();
                face.thumb(preview.aspect, move |cx, rect| (paint)(cx, rect)).name(field == PickerField::ThumbnailAndName)
            }
        }
    }
}

impl<K: Value + Eq + Hash + 'static> Component for PresetPicker<'_, K> {
    const ROLE: Role = Role::COMBO_BOX;
    type Style = PresetPickerStyle;
    type Output = PresetPickerOutput<K>;

    fn style(cx: &StyleCx<'_>, _: PresetPickerStyleKey<K>) -> PresetPickerStyle {
        PresetPickerStyle { surface: cx.field(), metrics: Metrics::new(), focus: cx.focus_look() }
    }

    fn sense(&self) -> Sense {
        Sense::CLICK | Sense::FOCUS | Sense::TAKE_FOCUS
    }

    /// The widest face that can show, so the field keeps its width whichever preset is picked.
    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let words = cx.words::<PresetWords>();
        let shows_name = self.field != PickerField::Thumbnail;
        let aspect = self.cell_shape.aspect();
        let sized = |text: Label| {
            let face = SelectFace::new(cx.env.resolve(&text));
            cx.face(if self.field == PickerField::Name { face } else { face.thumb(aspect, |_, _| {}).name(shows_name) })
        };
        let mut texts = vec![Label::Empty];
        if shows_name {
            let names = self.rows.iter().filter(|row| !row.is_group()).map(|row| Label::from(row.label.clone()));
            let fixed = [words.custom.clone(), words.mixed.clone()];
            texts.extend(names.chain(fixed).chain(self.placeholder.clone()).chain(self.display_name.clone()));
        }
        texts.into_iter().map(sized).fold(Vec2::ZERO, Vec2::max)
    }

    fn parts(&mut self, rect: egui::Rect, cx: &mut PartsCx<'_, Self>) {
        cx.part(FACE, rect)
            .sense(Sense::CLICK | Sense::HOVER | Sense::TAKE_FOCUS)
            .priority(HitPriority::new(HitLevel::Body, 0))
            .pointer_only("Enter, Space and Alt+Down open the presets")
            .toggles("browser")
            .on_press();
    }

    fn bind(&mut self) -> Option<BindMut<'_, Option<K>>> {
        Some(self.value.reborrow())
    }

    /// A press on the face and Alt+Down open the popover through the face's `toggles`; Enter and Space toggle it here.
    /// A preview that the popover closed on without a pick is cancelled.
    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let popup = cx.popup("browser");
        if cx.activation().is_some_and(|activation| activation.by_key) {
            if popup.is_open(cx) {
                popup.close(cx);
            } else {
                popup.open(cx);
            }
        }
        if !popup.is_open(cx) && matches!(cx.gesture(), Some(GesturePhase::Begin | GesturePhase::Update)) {
            cx.cancel(CancelReason::Dismissed);
        }
    }

    /// The popover: the browser over the rows. The preset the pointer moves the lead to is proposed; a click or Enter
    /// on a preset proposes it, commits and closes the popover.
    fn overlay(&mut self, ui: &mut OverlayUi<'_>, cx: &mut Cx<'_, Self>) {
        let popup = cx.popup("browser");
        let name = cx.label();
        let current = cx.value().and_then(Clone::clone);
        let menu = self.can_rename.is_some() || self.can_delete.is_some();
        let (rows, shape, view, default_open, toolbar) = (self.rows, self.cell_shape, self.view, self.default_open, self.toolbar);
        let (can_rename, can_delete) = (self.can_rename.as_ref(), self.can_delete.as_ref());
        let state = &mut self.state;
        let out = ui.show(popup, Overlay::popover(), |ui, _| {
            let mut browser = PresetBrowser::new(name.clone(), state.reborrowed(), rows)
                .cell_shape(shape)
                .view_lock(view)
                .default_open(default_open)
                .toolbar(toolbar)
                .context_menu(menu);
            if let Some(key) = current.clone() {
                browser = browser.selected(key);
            }
            if let Some(test) = can_rename {
                browser = browser.can_rename(|key| test.passes(key));
            }
            if let Some(test) = can_delete {
                browser = browser.can_delete(|key| test.passes(key));
            }
            browser.show(ui)
        });
        let Some(out) = out else { return };
        let mut pick = None;
        for event in out.events {
            match event {
                PresetEvent::Select { key, .. } | PresetEvent::Activate { key, .. } => pick = Some(key),
                PresetEvent::Open(_) => {}
                other => self.events.push(other),
            }
        }
        match (pick, out.hovered) {
            (Some(key), _) => {
                cx.propose(Some(key.clone()));
                cx.commit();
                popup.close(cx);
                self.picked = Some(key);
            }
            (None, Some(key)) if current.as_ref() != Some(&key) => cx.propose(Some(key)),
            _ => {}
        }
    }

    /// The current preset's name as the field's value.
    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        let words = cx.words::<PresetWords>();
        let text = match cx.value() {
            None => cx.env.resolve(&words.mixed),
            Some(value) => match value.as_ref().and_then(|key| self.preset(key)) {
                Some(row) => self.display_name.as_ref().map_or_else(|| Label::from(row.label.clone()), |name| cx.env.resolve(name)),
                None => cx.env.resolve(self.placeholder.as_ref().unwrap_or(&words.custom)),
            },
        };
        access.value(text.as_str().to_owned());
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let words = cx.words::<PresetWords>();
        ui.face(FACE, self.face(cx.value(), &words, &cx.env));
    }

    fn finish(self, cx: Cx<'_, Self>) -> PresetPickerOutput<K> {
        PresetPickerOutput { edit: cx.edit_response(), picked: self.picked, events: self.events }
    }
}
