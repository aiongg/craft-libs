//! `ComboBox`: a text part with a chevron that filters a list of options as the user types.
//!
//! The text is the value: any text can be typed, and an option is a quick way to type its text. The list is the
//! component's own popup on the dropdown preset, so focus stays in the text part and the host hands the list keys to
//! the list; the component handles none of them.

#[cfg(feature = "testing")]
mod stories;

use super::words::ComboBoxWords;
use crate::buttons::IconButton;
use crate::contract::{
    Access, Bind, BindMut, CollectionEvent, Common, Component, Cx, Edit as _, EditResponse, Entry, Focus as _, Input, MeasureCx, Overlay, OverlayUi,
    Overlays as _, PaintUi, PartsCx, Pointer as _, Props as _, Remember as _, Role, ShowCollection as _, WordSetCx as _,
};
use crate::list::{List, ListRow};
use crate::primitives::{IconSpec, TextKey, TextOptions, TextRun};
use crate::runtime::env::Label;
use crate::text::{Adornment, place, reserve};
use craft_behavior::{DisplayOrder, HitLevel, HitPriority, IconId, ItemSource, NavMove, PartKey, PlainText, Selection, Sense};
use egui::{Rect, Vec2};
use std::borrow::Cow;
use std::hash::{DefaultHasher, Hash as _, Hasher as _};

/// The text part.
const TEXT: PartKey = PartKey::new("text");
/// The chevron that opens and closes the list.
const CHEVRON: PartKey = PartKey::new("chevron");
/// The chevron's icon.
const CHEVRON_ICON: &str = "chevron_down";

fn text_key() -> TextKey<String> {
    TextKey::new(TEXT)
}

/// The decoration of an option row: it gets the entry and the plain row and returns the row to show (a thumbnail
/// with a sample of the entry, a detail line).
pub struct RowDecor<'a>(Box<DecorFn<'a>>);

type DecorFn<'a> = dyn Fn(&Entry<String>, ListRow<'static>) -> ListRow<'static> + 'a;

impl<'a> RowDecor<'a> {
    /// The decoration that runs `decorate`.
    pub fn new(decorate: impl Fn(&Entry<String>, ListRow<'static>) -> ListRow<'static> + 'a) -> Self {
        RowDecor(Box::new(decorate))
    }
}

/// What the combo box keeps between passes: the draft the list was last filtered for.
#[derive(Clone, Default)]
struct Typing {
    draft: Option<String>,
}

/// A text field with a list of options. The typed text filters the list: the options whose text starts with it
/// first, then those containing it, ignoring case. The chevron and Alt+Down open the list on every option. Enter or a
/// click on a row puts the option's text in the field; Escape closes the list, and the next Escape restores the text.
/// Up, Down and the wheel on the focused, closed field step to the previous or next option.
#[derive(crate::contract::Component, crate::contract::Props)]
#[component(overlay)]
pub struct ComboBox<'a> {
    #[common]
    common: Common,
    value: Bind<'a, String>,
    options: &'a [Entry<String>],
    /// Shown while the text is empty.
    #[prop(into)]
    placeholder: Option<Label>,
    /// The decoration of each option row.
    #[prop(func = "Fn(&Entry<String>, ListRow<'static>) -> ListRow<'static>")]
    rows: Option<RowDecor<'a>>,
    empty: String,
}

impl<'a> ComboBox<'a> {
    /// A combo box named `label` over the text `value`, offering `options`; an option's value is the text it puts in
    /// the field.
    pub fn new(label: impl Into<Label>, value: Bind<'a, String>, options: &'a [Entry<String>]) -> Self {
        ComboBox { common: Common::new(), value, options, placeholder: None, rows: None, empty: String::new() }.label(label)
    }

    fn text_options(&self) -> TextOptions {
        let mut options = TextOptions::new();
        options.placeholder = self.placeholder.clone();
        options
    }

    fn trailing(name: Label) -> [Adornment; 1] {
        [Adornment::button(CHEVRON, IconSpec::svg(IconId::new(CHEVRON_ICON)), name)]
    }

    /// The option whose text steps from the field's text in the direction of the move, over the options that can be
    /// picked. From text that is none of them, forward goes to the first and back to the last.
    fn neighbour(&self, text: &str, forward: bool) -> Option<usize> {
        let pickable: Vec<usize> =
            (0..self.options.len()).filter(|&at| self.options[at].value().is_some() && self.options[at].is_enabled()).collect();
        let here = pickable.iter().position(|&at| self.options[at].value().is_some_and(|value| value == text));
        let to = match (here, forward) {
            (Some(here), true) => (here + 1).min(pickable.len().checked_sub(1)?),
            (Some(here), false) => here.saturating_sub(1),
            (None, true) => 0,
            (None, false) => pickable.len().checked_sub(1)?,
        };
        pickable.get(to).copied()
    }
}

/// The positions of the options that match `query`: every position without one, else the options whose text starts
/// with the trimmed query, then those containing it, ignoring case. Separators match nothing.
fn matching(options: &[Entry<String>], query: Option<&str>) -> Vec<usize> {
    let query = query.map(|query| query.trim().to_lowercase()).filter(|query| !query.is_empty());
    let Some(query) = query else { return (0..options.len()).collect() };
    let (mut starting, mut containing) = (Vec::new(), Vec::new());
    for (at, option) in options.iter().enumerate() {
        let Some(text) = option.value() else { continue };
        let text = text.to_lowercase();
        if text.starts_with(&query) {
            starting.push(at);
        } else if text.contains(&query) {
            containing.push(at);
        }
    }
    starting.extend(containing);
    starting
}

/// The options the list shows, keyed by their position among all options.
struct Shown<'a> {
    options: &'a [Entry<String>],
    at: &'a [usize],
    labels: &'a [Label],
}

impl DisplayOrder<usize> for Shown<'_> {
    fn len(&self) -> usize {
        self.at.len()
    }

    fn key(&self, at: usize) -> usize {
        self.at[at]
    }

    fn index_of(&self, key: &usize) -> Option<usize> {
        self.at.iter().position(|shown| shown == key)
    }

    fn is_section(&self, at: usize) -> bool {
        self.options[self.at[at]].is_separator()
    }

    fn enabled(&self, at: usize) -> bool {
        self.options[self.at[at]].is_enabled()
    }

    fn text(&self, at: usize) -> Option<Cow<'_, str>> {
        Some(Cow::Borrowed(self.labels[self.at[at]].as_str()))
    }
}

impl ItemSource<usize> for Shown<'_> {}

impl Component for ComboBox<'_> {
    const ROLE: Role = Role::COMBO_BOX;
    const INPUT: Input = Input::Text;
    type Style = crate::text::FieldStyle;
    type Output = EditResponse<String>;

    fn style(cx: &crate::style::StyleCx<'_>, _: ComboBoxStyleKey) -> crate::text::FieldStyle {
        crate::text::FieldStyle::at(cx)
    }

    fn sense(&self) -> Sense {
        Sense::CLICK | Sense::FOCUS | Sense::TAKE_FOCUS
    }

    /// The widest of the current text, the placeholder and the options, and the room of the chevron: the field keeps
    /// its size whichever option is picked.
    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        let current = cx.value().map_or("", String::as_str);
        let placeholder = self.placeholder.as_ref().map_or("", Label::as_str);
        let texts = self.options.iter().filter_map(|option| option.value().map(String::as_str)).chain([current, placeholder]);
        let widest = texts.map(|text| cx.layout(&TextRun::new(text)).size()).fold(Vec2::ZERO, Vec2::max);
        let name = cx.words::<ComboBoxWords>().options.clone();
        Vec2::new(widest.x + reserve(cx.style, &[], &Self::trailing(name)), widest.y)
    }

    fn parts(&mut self, _: Rect, cx: &mut PartsCx<'_, Self>) {
        let value = cx.value();
        let name = cx.words::<ComboBoxWords>().options.clone();
        let trailing = Self::trailing(name);
        let placed = place(cx.content_rect, cx.style, &[], &trailing, cx.flow.is_rtl());
        if let Some(at) = placed.trailing.first() {
            cx.part(CHEVRON, *at)
                .sense(Sense::CLICK | Sense::KEEP_FOCUS)
                .priority(HitPriority::new(HitLevel::Handle, 0))
                .pointer_only("Alt+Down opens the list")
                .toggles("options");
        }
        if let Some(value) = value.or(Some(&self.empty)) {
            cx.text(text_key(), placed.text, &PlainText, value, &self.text_options());
        }
    }

    fn bind(&mut self) -> Option<BindMut<'_, String>> {
        Some(self.value.reborrow())
    }

    /// A changed draft filters the list: it opens on the matches, and closes when there are none. A draft that ended
    /// (committed, cancelled or replaced by a pick) closes it. While the list is closed, Up, Down and the wheel step
    /// through the options.
    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let list = cx.popup("options");
        let draft = cx.text(&text_key()).and_then(|outcome| outcome.draft.clone());
        let before = std::mem::replace(&mut cx.memory::<Typing>().draft, draft.clone());
        if before != draft {
            match &draft {
                Some(text) if !matching(self.options, Some(text)).is_empty() => list.open(cx),
                _ => list.close(cx),
            }
        }
        if list.is_open(cx) {
            return;
        }
        let text = cx.value().cloned().unwrap_or_default();
        while let Some((step, mods)) = cx.intents().take_move() {
            let forward = match step {
                NavMove::Down if !mods.fine => true,
                NavMove::Up if !mods.fine => false,
                _ => continue,
            };
            if let Some(value) = self.neighbour(&text, forward).and_then(|at| self.options[at].value()) {
                cx.set(value.clone());
            }
        }
        if let Some(wheel) = cx.take_wheel()
            && wheel.scroll.y != 0.0
            && let Some(value) = self.neighbour(&text, wheel.scroll.y < 0.0).and_then(|at| self.options[at].value())
        {
            cx.set(value.clone());
        }
    }

    /// The list: one option row per match, the row of the field's text checked and active when the list opens. A
    /// pick comes back as `Activate` (Enter, Tab or a click) and puts the option's text in the field.
    fn overlay(&mut self, ui: &mut OverlayUi<'_>, cx: &mut Cx<'_, Self>) {
        let popup = cx.popup("options");
        let options = self.options;
        let query = cx.memory::<Typing>().draft.clone();
        let at = matching(options, query.as_deref());
        let labels: Vec<Label> = options.iter().map(|option| cx.env.resolve(option.label())).collect();
        let current = cx.value().and_then(|text| options.iter().position(|option| option.value() == Some(text))).filter(|key| at.contains(key));
        let stamp = {
            let mut hasher = DefaultHasher::new();
            at.hash(&mut hasher);
            hasher.finish()
        };
        let name = cx.label();
        let decorate = self.rows.as_ref();
        let source = Shown { options, at: &at, labels: &labels };
        let events = ui.show(popup, Overlay::dropdown(), |ui, _| {
            let selection = current.map_or_else(Selection::default, |key| Selection::new(vec![key], Some(key), Some(key)));
            List::<usize>::options(name.clone())
                .rows_stamp(stamp)
                .selection(Bind::given(selection))
                .show_items(ui, &source, &mut |key, _| {
                    let option = &options[*key];
                    if option.is_separator() {
                        return ListRow::spacer();
                    }
                    let mut row = ListRow::new(labels[*key].clone()).option().enabled(option.is_enabled());
                    if let Some(icon) = option.icon_spec() {
                        row = row.icon(icon.clone());
                    }
                    match decorate {
                        Some(RowDecor(decorate)) => decorate(option, row),
                        None => row,
                    }
                })
                .events
        });
        let picked = events.into_iter().flatten().find_map(|event| match event {
            CollectionEvent::Activate(key) => Some(key),
            _ => None,
        });
        if let Some(value) = picked.and_then(|key| options.get(key)).and_then(Entry::value) {
            cx.set(value.clone());
            popup.close(cx);
        }
    }

    /// The chevron's node, which reports whether the list is open.
    fn access(&self, access: &mut Access, _: &Cx<'_, Self>) {
        access.node(CHEVRON, Role::BUTTON);
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let name = cx.words::<ComboBoxWords>().options.clone();
        ui.face(CHEVRON, IconButton::new(IconId::new(CHEVRON_ICON)).label(name));
    }

    fn finish(self, cx: Cx<'_, Self>) -> EditResponse<String> {
        cx.edit_response()
    }
}
