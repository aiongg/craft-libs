//! The Variants view: the target story repeated over one axis of the appearance, a card for each
//! value.
use craft_theme::{ButtonVariant, ContrastChoice, ControlSize, Mode, ModeChoice, SystemAppearance, TypeRole, builtins};
use craft_ui::controls::Button;
use craft_ui::direction;
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Args, Story};
use craft_ui::theme::{Tokens, TokensOf};
use egui::{Align, ScrollArea, Ui};

use super::applied::shown_mode;
use super::canvas::{canvas, canvas_frame};
use super::tiles::TILE_MIN_WIDTH;
use crate::appearance::Appearance;
use crate::layout::{PanelId, VariantAxis};

/// Which Variants view is open, and where.
pub(super) struct Variants {
    /// The axis the view repeats the story over; not saved.
    axis: Option<VariantAxis>,
    /// The centre tab the view shows in; it closes when another centre tab comes to the front.
    tab: PanelId,
}

impl Variants {
    pub(super) fn closed() -> Self {
        Self { axis: None, tab: PanelId::View }
    }

    pub(super) fn axis(&self) -> Option<VariantAxis> {
        self.axis
    }

    /// The tab the view shows in.
    pub(super) fn tab(&self) -> PanelId {
        self.tab
    }

    /// Opens the view over `axis` in `tab`, or closes it.
    pub(super) fn set(&mut self, axis: Option<VariantAxis>, tab: PanelId) {
        self.axis = axis;
        self.tab = tab;
    }

    pub(super) fn close(&mut self) {
        self.axis = None;
    }

    /// The axis of the view when it shows in `tab`.
    pub(super) fn in_tab(&self, tab: PanelId) -> Option<VariantAxis> {
        self.axis.filter(|_| self.tab == tab)
    }
}

/// The Variants view of `story`: a header with Close, and a card for each value of `axis`. Picking
/// a card sets the appearance's value on `axis` and closes the view. Returns whether the view
/// closed.
pub(super) fn show(ui: &mut Ui, story: &Story, axis: VariantAxis, args: &mut Args, appearance: &mut Appearance, system: SystemAppearance) -> bool {
    let current = appearance.clone();
    let mut applied = None;
    let mut closed = false;
    canvas_frame(&Tokens::of(ui)).show(ui, |ui| {
        ui.set_min_size(ui.available_size());
        closed = variants_header(ui, story, axis);
        ScrollArea::vertical().id_salt("gallery.variants").auto_shrink([false, false]).show(ui, |ui| {
            applied = variants_grid(ui, story, args, &current, system, axis);
        });
    });
    if let Some(from) = applied {
        copy_axis(axis, &from, appearance);
        closed = true;
    }
    closed
}

/// The cards of a Variants view: the appearance with each value of `axis` set, named by the value.
fn variant_values(axis: VariantAxis, appearance: &Appearance) -> Vec<(String, Appearance)> {
    let with = |name: &str, set: &dyn Fn(&mut Appearance)| {
        let mut a = appearance.clone();
        set(&mut a);
        (name.to_owned(), a)
    };
    match axis {
        VariantAxis::Theme => builtins().iter().map(|t| with(&t.name, &|a| a.theme = t.id.as_str().to_owned())).collect(),
        VariantAxis::Mode => vec![with("Light", &|a| a.mode = ModeChoice::Light), with("Dark", &|a| a.mode = ModeChoice::Dark)],
        VariantAxis::Contrast => {
            vec![with("Standard", &|a| a.contrast = ContrastChoice::Standard), with("High", &|a| a.contrast = ContrastChoice::High)]
        }
        VariantAxis::Size => vec![with("Compact", &|a| a.comfortable = false), with("Comfortable", &|a| a.comfortable = true)],
    }
}

/// Sets `to`'s value on `axis` to `from`'s.
fn copy_axis(axis: VariantAxis, from: &Appearance, to: &mut Appearance) {
    match axis {
        VariantAxis::Theme => to.theme.clone_from(&from.theme),
        VariantAxis::Mode => to.mode = from.mode,
        VariantAxis::Contrast => to.contrast = from.contrast,
        VariantAxis::Size => to.comfortable = from.comfortable,
    }
}

/// Whether `a` and `b` show the same value on `axis`; the System choices count as what they show.
fn same_on(axis: VariantAxis, a: &Appearance, b: &Appearance, shown: Mode, system: SystemAppearance) -> bool {
    match axis {
        VariantAxis::Theme => a.theme == b.theme,
        VariantAxis::Mode => a.mode(shown) == b.mode(shown),
        VariantAxis::Contrast => a.contrast(system) == b.contrast(system),
        VariantAxis::Size => a.comfortable == b.comfortable,
    }
}

/// The Variants view's header: what is shown and the hint, with Close at the trailing end. Returns
/// whether Close was clicked.
fn variants_header(ui: &mut Ui, story: &Story, axis: VariantAxis) -> bool {
    let mut closed = false;
    let gap = Tokens::of(ui).space.gap.section;
    direction::horizontal(ui, |ui| {
        ui.add(Text::new(format!("{} · {} in every {}", story.component, story.name, axis.noun())).role(TypeRole::Heading));
        ui.add(Text::new("Click one to use it").content(Content::Muted));
        // The strip's leftover width goes before Close, so it ends at the trailing edge.
        let rtl = direction::is_rtl(ui);
        let layout = if rtl { egui::Layout::left_to_right(Align::Center) } else { egui::Layout::right_to_left(Align::Center) };
        ui.with_layout(layout, |ui| closed = ui.add(Button::new("Close").size(ControlSize::Sm)).clicked());
    });
    ui.add_space(gap);
    closed
}

/// One card per value of `axis`, in one or two columns like the Overview's tiles. Each draws the
/// story as the Component view does, under the gallery's appearance with the value set; the story
/// stays live. Returns the appearance of the card whose header was clicked.
fn variants_grid(
    ui: &mut Ui,
    story: &Story,
    args: &mut Args,
    appearance: &Appearance,
    system: SystemAppearance,
    axis: VariantAxis,
) -> Option<Appearance> {
    let shown = shown_mode(ui.ctx());
    let gap = Tokens::of(ui).space.gap.section;
    let columns = if ui.available_width() >= 2.0 * TILE_MIN_WIDTH + gap { 2 } else { 1 };
    let cards = variant_values(axis, appearance);
    let mut picked = None;
    for row in cards.chunks(columns) {
        ui.columns(columns, |uis| {
            for (ui, (name, card)) in uis.iter_mut().zip(row) {
                // The header sits on the gallery's background, so it keeps the gallery's colours.
                ui.horizontal(|ui| {
                    if ui
                        .add(Button::new(name.as_str()).variant(ButtonVariant::Ghost).size(ControlSize::Sm).id_salt(("gallery.variant", name)))
                        .clicked()
                    {
                        picked = Some(card.clone());
                    }
                    if same_on(axis, card, appearance, shown, system) {
                        ui.add(Text::new("shown now").content(Content::Muted));
                    }
                });
                ui.push_id((axis.noun(), name), |ui| canvas(ui, story, args, card, system, None));
                // A card does not change the gallery's scale.
                let _ = craft_ui::story::take_pixels_per_point_request(ui.ctx());
            }
        });
        ui.add_space(gap);
    }
    picked
}
