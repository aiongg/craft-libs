//! Size dialogs and their pure rules.

use egui::accesskit::Role;
use egui::{Key, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::sizing::{Field, arrow_direction};
use super::*;
use crate::theme::{Base, install};

fn size(width: u32, height: u32, pixels_per_inch: f64) -> PixelSize {
    PixelSize { width, height, pixels_per_inch }
}

fn harness<State>() -> HarnessBuilder<State> {
    Harness::<State>::builder().with_size(vec2(640.0, 640.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default()))
}

// Pixel, percent and physical units convert both ways at 72 and 300 ppi (pure).
#[test]
fn units_convert_both_ways() {
    for ppi in [72.0, 300.0] {
        for unit in SizeUnit::ALL {
            let value = unit.from_pixels(1200.0, 1200.0, ppi);
            assert!((unit.to_pixels(value, 1200.0, ppi) - 1200.0).abs() < 1e-9, "{unit:?} at {ppi} ppi");
        }
    }
    assert_eq!(SizeUnit::Inches.to_pixels(1.0, 0.0, 300.0), 300.0);
    assert_eq!(SizeUnit::Percent.to_pixels(50.0, 1200.0, 72.0), 600.0);
    assert_eq!(SizeUnit::Points.to_pixels(72.0, 0.0, 300.0), 300.0);
    assert_eq!(SizeUnit::Picas.to_pixels(6.0, 0.0, 300.0), 300.0);
    assert!((SizeUnit::Centimetres.from_pixels(72.0, 0.0, 72.0) - 2.54).abs() < 1e-12);
    assert!((SizeUnit::Millimetres.from_pixels(72.0, 0.0, 72.0) - 25.4).abs() < 1e-12);
    assert_eq!(SizeUnit::Pixels.from_pixels(1200.0, 1200.0, 72.0), 1200.0);
}

// Typing a width with the chain on scales the height; with it off the height stays.
#[test]
fn linked_fields_scale_the_other_side_while_constrained() {
    let mut fields = LinkedSizeFields::new(size(1200, 800, 72.0));
    fields.set_width(600.0);
    assert_eq!((fields.current.width, fields.current.height), (600, 400));
    fields.set_height(200.0);
    assert_eq!((fields.current.width, fields.current.height), (300, 200));
    // A value typed in a unit goes through the same rule.
    fields.unit = SizeUnit::Percent;
    fields.edit(Field::Width, 50.0);
    assert_eq!((fields.current.width, fields.current.height), (600, 400));

    fields.constrain = false;
    fields.set_width(900.0);
    assert_eq!((fields.current.width, fields.current.height), (900, 400));
    fields.set_height(0.0);
    assert_eq!(fields.current.height, 1, "a size is at least one pixel");

    // A relative size reads the typed value as the change.
    let mut canvas = LinkedSizeFields { relative: true, constrain: false, ..LinkedSizeFields::new(size(1200, 800, 72.0)) };
    canvas.edit(Field::Width, -200.0);
    assert_eq!(canvas.current.width, 1000);
    assert_eq!(canvas.shown(Field::Width), -200.0);
}

// Typing in the width field of the rows updates the height on each keystroke.
#[test]
fn typing_a_width_scales_the_height_on_each_keystroke() {
    struct Fields(LinkedSizeFields);
    let mut harness = harness().build_ui_state(
        |ui, fields: &mut Fields| {
            let labels = SizingLabels::default();
            crate::composites::FormGrid::new("size").show(ui, |form| {
                let _ = fields.0.rows(form, &labels, false);
            });
        },
        Fields(LinkedSizeFields::new(size(1200, 800, 72.0))),
    );
    harness.run();
    harness.get_by_role_and_label(Role::SpinButton, "Width:").focus();
    harness.run();
    harness.event(egui::Event::Text("6".into()));
    harness.run();
    assert_eq!(harness.state().0.current, size(6, 4, 72.0));
    harness.event(egui::Event::Text("00".into()));
    harness.run();
    assert_eq!(harness.state().0.current, size(600, 400, 72.0));
}

// Resample off disables the pixel fields and a physical width changes the resolution instead.
#[test]
fn resample_off_fixes_the_pixel_count() {
    let mut fields = LinkedSizeFields { resampling: false, ..LinkedSizeFields::new(size(1200, 800, 72.0)) };
    fields.edit(Field::Width, 600.0);
    assert_eq!(fields.current, size(1200, 800, 72.0), "pixels cannot change");

    fields.unit = SizeUnit::Inches;
    assert!((fields.shown(Field::Width) - 1200.0 / 72.0).abs() < 1e-9);
    fields.edit(Field::Width, 8.0);
    assert_eq!((fields.current.width, fields.current.height), (1200, 800));
    assert!((fields.current.pixels_per_inch - 150.0).abs() < 1e-9);
    assert!((fields.shown(Field::Width) - 8.0).abs() < 1e-9);
    assert!((fields.shown(Field::Height) - 800.0 / 150.0).abs() < 1e-9);

    // The fields are disabled in pixels and percent, not in inches.
    struct Host(LinkedSizeFields);
    let mut harness = harness().build_ui_state(
        |ui, host: &mut Host| {
            let labels = SizingLabels::default();
            crate::composites::FormGrid::new("size").show(ui, |form| {
                let _ = host.0.rows(form, &labels, true);
            });
        },
        Host(LinkedSizeFields { resampling: false, ..LinkedSizeFields::new(size(1200, 800, 72.0)) }),
    );
    harness.run();
    let enabled =
        |harness: &Harness<'_, Host>| harness.get_all_by_role(Role::SpinButton).map(|n| !n.accesskit_node().is_disabled()).collect::<Vec<_>>();
    assert_eq!(enabled(&harness), [false, false, true], "width, height, resolution");
    harness.state_mut().0.unit = SizeUnit::Inches;
    harness.run();
    assert_eq!(enabled(&harness), [true, true, true]);
}

// The anchor grid: a click picks a cell, arrow keys move the mark, arrows point outward when the
// canvas grows and inward when it shrinks.
#[test]
fn the_anchor_grid_picks_by_click_and_keyboard() {
    let mut harness = harness()
        .build_ui_state(|ui, state: &mut (Anchor, (f64, f64))| drop(AnchorGrid::new(&mut state.0, state.1).show(ui)), (Anchor::Center, (0.0, 0.0)));
    harness.run();
    harness.get_by_label("Top left").click();
    harness.run();
    assert_eq!(harness.state().0, Anchor::TopLeft);

    harness.get_by_label("Center").focus();
    harness.run();
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert_eq!(harness.state().0, Anchor::TopLeft, "an arrow moves the focus, not the pick");
    assert!(harness.get_by_label("Right").is_focused(), "the arrow moved the focus to the cell on the right");
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().0, Anchor::Right);

    // Arrows leave the anchor outward on growth and point back at it on shrinking; only the
    // neighbours have one.
    use Anchor::*;
    assert_eq!(arrow_direction(TopLeft, Right, (10.0, 10.0)), None);
    assert_eq!(arrow_direction(TopLeft, Top, (10.0, 10.0)), Some((1, 0)));
    assert_eq!(arrow_direction(TopLeft, Center, (10.0, 10.0)), Some((1, 1)));
    assert_eq!(arrow_direction(TopLeft, Center, (-10.0, 10.0)), Some((-1, 1)));
    assert_eq!(arrow_direction(Center, TopLeft, (-10.0, -10.0)), Some((1, 1)));
    assert_eq!(arrow_direction(Center, Center, (10.0, 10.0)), None);
}

// `human_bytes` formats 0, 850 K and 2.3 M as PhotoCraft does (pure).
#[test]
fn human_bytes_matches_the_readouts() {
    assert_eq!(human_bytes(0.0), "1K");
    assert_eq!(human_bytes(850.0 * 1024.0), "850K");
    assert_eq!(human_bytes(2.3 * 1024.0 * 1024.0), "2.3M");
    assert_eq!(human_bytes(1.5 * 1024.0 * 1024.0 * 1024.0), "1.50G");
}

struct Host {
    dialogs: DialogHost,
}

fn host_with(open: impl FnOnce(&mut DialogHost)) -> Harness<'static, Host> {
    let mut dialogs = DialogHost::new();
    open(&mut dialogs);
    let mut harness = harness().build_ui_state(|ui, host: &mut Host| host.dialogs.show(ui.ctx()), Host { dialogs });
    harness.run();
    harness
}

// Image Size's OK returns the pixel size, resample and scale-styles; Canvas Size's returns the
// anchor and extension.
#[test]
fn ok_returns_the_typed_size_results() {
    let mut harness = host_with(|dialogs| {
        dialogs.open(ImageSizeDialog::new(size(1200, 800, 72.0)));
    });
    harness.get_by_label("Scale Styles").click();
    harness.get_by_label("Resample method").click();
    harness.run();
    harness.get_by_label("Bilinear").click();
    harness.run();
    harness.get_by_label("OK").click();
    harness.run();
    harness.run();
    let result = harness.state_mut().dialogs.take::<ImageSizeDialog>().and_then(DialogResult::ok).expect("OK answered");
    assert_eq!(result, ImageSizeResult { size: size(1200, 800, 72.0), resample: Resample::Bilinear, scale_styles: false });

    let mut harness = host_with(|dialogs| {
        dialogs.open(CanvasSizeDialog::new(size(1200, 800, 72.0)));
    });
    harness.get_by_label("Bottom left").click();
    harness.run();
    harness.get_by_label("OK").click();
    harness.run();
    harness.run();
    let result = harness.state_mut().dialogs.take::<CanvasSizeDialog>().and_then(DialogResult::ok).expect("OK answered");
    assert_eq!(result, CanvasSizeResult { size: size(1200, 800, 72.0), anchor: Anchor::BottomLeft, extension: CanvasExtension::Background });
}

// Image Size links width and height with a chain; Canvas Size has none.
#[test]
fn only_image_size_draws_the_chain() {
    let image = host_with(|dialogs| {
        dialogs.open(ImageSizeDialog::new(size(1200, 800, 72.0)));
    });
    image.get_by_label("Constrain proportions");
    let canvas = host_with(|dialogs| {
        dialogs.open(CanvasSizeDialog::new(size(1200, 800, 72.0)));
    });
    assert!(canvas.query_by_label("Constrain proportions").is_none());
}
