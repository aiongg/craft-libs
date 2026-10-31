//! Stories of [`GradientPalette`]: the preset grid and the preset list.

use craft_theme::Srgba;
use egui::Ui;

use super::{ColorStop, GradientGroup, GradientItem, GradientPalette, GradientRef, GradientStops, OpacityStop, PaletteLayout};
use craft_ui::controls::slider;
use craft_ui::edit;
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, Story, StoryArgs};
use craft_ui::typed_story;

pub(super) fn stories() -> Vec<Story> {
    vec![
        typed_story!("composites/gradient/palette-grid", Level::Composites, "GradientPalette", "Grid", defaults(), show_grid),
        typed_story!("composites/gradient/palette-list", Level::Composites, "GradientPalette", "List", defaults(), show_list),
    ]
}

#[derive(StoryArgs, Debug, PartialEq)]
struct PaletteArgs {
    /// How many gradients the Recent group holds, newest first.
    #[arg(range = 0.0..=8.0)]
    recent: f32,
    #[arg(range = 40.0..=300.0)]
    max_height: f32,
    /// Marks the last pick as the host's current gradient.
    selected: bool,
    #[arg(range = 100.0..=400.0)]
    width: f32,
    disabled: bool,
}

fn defaults() -> PaletteArgs {
    PaletteArgs { recent: 4.0, max_height: 120.0, selected: true, width: 240.0, disabled: false }
}

fn two(a: [u8; 3], b: [u8; 3]) -> GradientStops {
    let color = |at: f32, [r, g, b]: [u8; 3]| ColorStop::new(at, Srgba::from(craft_theme::Rgba::rgb(r, g, b)));
    GradientStops::new(vec![color(0.0, a), color(1.0, b)])
}

fn presets() -> Vec<GradientGroup> {
    let fading = {
        let mut stops = two([0x3e, 0x63, 0xdd], [0x3e, 0x63, 0xdd]);
        stops.opacity = vec![OpacityStop::new(0.0, 1.0), OpacityStop::new(1.0, 0.0)];
        stops
    };
    vec![
        GradientGroup::new(
            "Basics",
            vec![
                GradientItem::new("Black, White", two([0, 0, 0], [255, 255, 255])),
                GradientItem::new("Red, Green", two([0xe5, 0x48, 0x4d], [0x30, 0xa4, 0x6c])),
                GradientItem::new("Blue, Yellow", two([0x3e, 0x63, 0xdd], [0xf5, 0xd9, 0x0a])),
                GradientItem::new("Blue to clear", fading),
            ],
        ),
        GradientGroup::new(
            "Warm and cool",
            vec![
                GradientItem::new("Sunset", two([0xf7, 0x6b, 0x15], [0x8e, 0x4e, 0xc6])),
                GradientItem::new("Ember", two([0xf5, 0xa5, 0x24], [0xe5, 0x48, 0x4d])),
                GradientItem::new("Lagoon", two([0x12, 0xa5, 0x94], [0x3e, 0x63, 0xdd])),
                GradientItem::new("Meadow", two([0xf5, 0xd9, 0x0a], [0x30, 0xa4, 0x6c])),
                GradientItem::new("Orchid", two([0xd6, 0x40, 0x9f], [0x3e, 0x63, 0xdd])),
                GradientItem::new("Slate", two([0x88, 0x8d, 0x91], [0x20, 0x22, 0x25])),
            ],
        ),
    ]
}

fn show_grid(ui: &mut Ui, args: &mut PaletteArgs) {
    show(ui, args, PaletteLayout::Grid);
}

fn show_list(ui: &mut Ui, args: &mut PaletteArgs) {
    show(ui, args, PaletteLayout::List);
}

/// The palette as a host uses it: a pick becomes the host's gradient and goes to the front of
/// the recent list.
fn show(ui: &mut Ui, args: &PaletteArgs, layout: PaletteLayout) {
    let groups = presets();
    let start = (None::<GradientRef>, vec![two([0xe5, 0x48, 0x4d], [0xf5, 0xd9, 0x0a]), two([0x00, 0x9e, 0xe1], [0x8e, 0x4e, 0xc6])], String::new());
    slider::with_values(ui, if layout == PaletteLayout::Grid { "gradient-palette-grid" } else { "gradient-palette-list" }, [start], |ui, state| {
        let (picked, recent, last) = &mut state[0];
        ui.set_width(args.width.min(ui.available_width()));
        let shown_recent: Vec<GradientStops> = recent.iter().cycle().take(args.recent as usize).cloned().collect();
        let response = GradientPalette::new(("gradient-palette", layout as u8), "Gradient presets", &groups)
            .recent(&shown_recent, "Recent")
            .layout(layout)
            .selected(picked.filter(|_| args.selected))
            .max_height(args.max_height)
            .enabled(!args.disabled)
            .show(ui);
        if let Some(event) = &response.event {
            if let Some(&pick) = event.value() {
                *picked = Some(pick);
                let stops = match pick {
                    GradientRef::Recent(i) => shown_recent.get(i).cloned(),
                    GradientRef::Item { group, item } => groups.get(group).and_then(|g| g.items.get(item)).map(|i| i.stops.clone()),
                };
                if let Some(stops) = stops {
                    recent.insert(0, stops);
                    recent.truncate(8);
                }
            }
            *last = edit::describe(&event.clone().map(|p| format!("{p:?}")));
        }
        ui.add(Text::new(format!("Last event: {last}")).content(Content::Muted));
    });
}
