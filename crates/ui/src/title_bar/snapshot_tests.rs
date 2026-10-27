//! Renders of the caption buttons, compared with `tests/snapshots/title-bar-*.png`
//! (`UPDATE_SNAPSHOTS=1` rewrites them).

use craft_window::Layout;
use egui::accesskit::Role;
use egui::{Color32, Rect, Vec2, pos2, vec2};
use egui_kittest::kittest::Queryable as _;
use egui_kittest::{Harness, SnapshotResults};

use super::buttons::{Look, paint, place};
use super::*;

const CELL: Vec2 = vec2(160.0, 32.0);

struct Palette {
    bar: Color32,
    colors: CaptionColors,
}

fn light() -> Palette {
    Palette {
        bar: Color32::from_gray(0xF3),
        colors: CaptionColors {
            glyph: Color32::from_gray(0x1A),
            glyph_inactive: Color32::from_gray(0x8A),
            hover: Color32::from_gray(0xE0),
            pressed: Color32::from_gray(0xCF),
            bar: Color32::from_gray(0xF3),
        },
    }
}

fn dark() -> Palette {
    Palette {
        bar: Color32::from_gray(0x20),
        colors: CaptionColors {
            glyph: Color32::WHITE,
            glyph_inactive: Color32::from_gray(0x8F),
            hover: Color32::from_gray(0x2D),
            pressed: Color32::from_gray(0x38),
            bar: Color32::from_gray(0x20),
        },
    }
}

/// The grid's rows, top to bottom.
fn looks() -> [Look; 5] {
    let rest = Look { active: true, ..Look::default() };
    [
        rest,
        Look { hovered: true, ..rest },
        Look { hovered: true, pressed: true, ..rest },
        Look { active: false, ..rest },
        Look { maximized: true, ..rest },
    ]
}

/// Every button of `family` in the rows of [`looks`], light on the left, dark on the right.
fn grid(family: CaptionFamily) -> Harness<'static> {
    let looks = looks();
    Harness::builder().with_size(vec2(2.0 * CELL.x, looks.len() as f32 * CELL.y)).with_pixels_per_point(2.0).build_ui(move |ui| {
        for (col, palette) in [light(), dark()].iter().enumerate() {
            for (row, look) in looks.iter().enumerate() {
                let bar = Rect::from_min_size(pos2(col as f32 * CELL.x, row as f32 * CELL.y), CELL);
                ui.painter().rect_filled(bar, 0.0, palette.bar);
                let sides = (vec![], vec![Caption::Minimize, Caption::Maximize, Caption::Close]);
                for (c, r, centre) in place(family, &sides, bar, bar.left(), bar.right()) {
                    paint(ui, family, c, r, centre, *look, look.level(), &palette.colors, None);
                }
            }
        }
    })
}

/// Rows: rest, hover, pressed, inactive window, maximized (the restore glyph).
#[test]
fn title_bar_caption_snapshots() {
    let mut results = SnapshotResults::new();
    for (family, name) in [
        (CaptionFamily::Windows, "windows"),
        (CaptionFamily::Adwaita, "adwaita"),
        (CaptionFamily::Breeze, "breeze"),
        (CaptionFamily::Generic, "generic"),
    ] {
        let mut harness = grid(family);
        harness.run();
        results.add(harness.try_snapshot(format!("title-bar-{name}")));
    }
}

/// A whole bar in a right-to-left UI: the Windows captions at the left, Close hovered.
#[test]
fn title_bar_right_to_left_snapshot() {
    let frame = Frame::Integrated(Captions { family: CaptionFamily::Windows, layout: Layout::default() });
    let mut harness = Harness::builder()
        .with_size(vec2(400.0, CELL.y))
        .with_pixels_per_point(2.0)
        .with_context_setup(|ctx| {
            crate::direction::apply(ctx, crate::direction::Direction::Rtl);
            ctx.global_style_mut(|s| s.interaction.tooltip_delay = f32::INFINITY);
        })
        .with_render_cursor(false)
        .build_ui(move |ui| {
            let palette = light();
            // The whole window, without the harness panel's margin.
            let window = ui.ctx().content_rect();
            let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(window));
            ui.set_clip_rect(window);
            ui.painter().rect_filled(window, 0.0, palette.bar);
            TitleBar::new(&frame, CELL.y, palette.colors).show(&mut ui, |ui, slots| {
                // The app's content, at the reading start of its slot.
                let pill = Rect::from_min_size(pos2(slots.content.right() - 90.0, slots.centre_line - 8.0), vec2(80.0, 16.0));
                ui.painter().rect_filled(pill, 4.0, Color32::from_rgb(0x00, 0x5F, 0xB8));
            });
        });
    harness.run();
    harness.get_by_label("Close").hover();
    harness.run();
    harness.snapshot("title-bar-rtl");
}

#[test]
fn title_bar_captions_are_labelled_buttons() {
    let frame = Frame::Integrated(Captions { family: CaptionFamily::Adwaita, layout: Layout::default() });
    let labels = CaptionLabels::default().localized(|key| (key == "window.close").then(|| "Fermer".to_owned()));
    let mut harness = Harness::builder().with_size(vec2(400.0, CELL.y)).build_ui_state(
        move |ui, closed: &mut bool| {
            *closed |= TitleBar::new(&frame, CELL.y, light().colors).labels(&labels).show(ui, |_, _| {}).close_requested;
        },
        false,
    );
    harness.run();
    // `get_by_*` panics unless exactly one node matches.
    for label in ["Minimize", "Maximize", "Fermer"] {
        let _ = harness.get_by_role_and_label(Role::Button, label);
    }
    harness.input_mut().viewports.entry(egui::ViewportId::ROOT).or_default().maximized = Some(true);
    harness.run();
    let _ = harness.get_by_label("Restore");
    // Like the OS's own caption buttons, they are no Tab stops.
    harness.key_press(egui::Key::Tab);
    harness.run();
    assert_eq!(harness.ctx.memory(|m| m.focused()), None);
    // A screen reader presses them.
    harness.get_by_label("Fermer").click_accesskit();
    harness.run();
    assert!(*harness.state(), "Close asks the app");
}
