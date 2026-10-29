//! Uses every method and type the template disallows once. The test counts clippy's diagnostics.
#![allow(dead_code, unused_must_use, reason = "fixture: each line exists to be flagged")]

use egui::color_picker::Alpha;
use egui::{Color32, Rgba};

struct State {
    b: bool,
    i: u32,
    f: f32,
    text: String,
    color: Color32,
    hsva: egui::ecolor::Hsva,
    rgba: Rgba,
    rgb8: [u8; 3],
    rgb32: [f32; 3],
    rgba8: [u8; 4],
    rgba32: [f32; 4],
}

const A: Alpha = Alpha::Opaque;

fn methods(ui: &mut egui::Ui, s: &mut State, r: &egui::Response) {
    ui.button("x");
    ui.small_button("x");
    ui.checkbox(&mut s.b, "x");
    ui.toggle_value(&mut s.b, "x");
    ui.radio(s.b, "x");
    ui.radio_value(&mut s.i, 1, "x");
    ui.selectable_label(s.b, "x");
    ui.selectable_value(&mut s.i, 1, "x");
    ui.text_edit_singleline(&mut s.text);
    ui.text_edit_multiline(&mut s.text);
    ui.drag_angle(&mut s.f);
    ui.drag_angle_tau(&mut s.f);
    ui.separator();
    ui.spinner();
    ui.link("x");
    ui.hyperlink("https://example.org");
    ui.hyperlink_to("x", "https://example.org");
    ui.collapsing("x", |_| {});
    ui.menu_button("x", |_| {});
    ui.color_edit_button_srgba(&mut s.color);
    ui.color_edit_button_hsva(&mut s.hsva);
    ui.color_edit_button_srgb(&mut s.rgb8);
    ui.color_edit_button_rgb(&mut s.rgb32);
    ui.color_edit_button_srgba_premultiplied(&mut s.rgba8);
    ui.color_edit_button_srgba_unmultiplied(&mut s.rgba8);
    ui.color_edit_button_rgba_premultiplied(&mut s.rgba32);
    ui.color_edit_button_rgba_unmultiplied(&mut s.rgba32);
    egui::color_picker::color_edit_button_srgba(ui, &mut s.color, A);
    egui::color_picker::color_edit_button_hsva(ui, &mut s.hsva, A);
    egui::color_picker::color_edit_button_srgb(ui, &mut s.rgb8);
    egui::color_picker::color_edit_button_rgba(ui, &mut s.rgba, A);
    egui::color_picker::color_edit_button_rgb(ui, &mut s.rgb32);
    egui::color_picker::color_picker_color32(ui, &mut s.color, A);
    egui::color_picker::color_picker_hsva_2d(ui, &mut s.hsva, A);
    r.clone().on_hover_text("x");
    r.clone().on_hover_text_at_pointer("x");
    r.clone().on_disabled_hover_text("x");
    r.clone().on_hover_ui(|_| {});
    r.clone().on_hover_ui_at_pointer(|_| {});
    r.clone().on_disabled_hover_ui(|_| {});
    r.show_tooltip_text("x");
    r.show_tooltip_ui(|_| {});
    Color32::from_rgb(1, 2, 3);
    Color32::from_rgb_additive(1, 2, 3);
    Color32::from_rgba_unmultiplied(1, 2, 3, 4);
    Color32::from_rgba_premultiplied(1, 2, 3, 4);
    Color32::from_gray(1);
    Color32::from_black_alpha(1);
    Color32::from_white_alpha(1);
    let _ = Color32::from_hex("#010203");
    Rgba::from_rgb(0.1, 0.2, 0.3);
    Rgba::from_rgba_unmultiplied(0.1, 0.2, 0.3, 0.4);
}

fn type_0(_: Option<egui::Button>) {}
fn type_1(_: Option<egui::Checkbox>) {}
fn type_2(_: Option<egui::RadioButton>) {}
fn type_3(_: Option<egui::DragValue>) {}
fn type_4(_: Option<egui::Slider>) {}
fn type_5(_: Option<egui::RangeSlider>) {}
fn type_6(_: Option<egui::TextEdit>) {}
fn type_7(_: Option<egui::ProgressBar>) {}
fn type_8(_: Option<egui::Spinner>) {}
fn type_9(_: Option<egui::Hyperlink>) {}
fn type_10(_: Option<egui::Link>) {}
fn type_11(_: Option<egui::Separator>) {}
fn type_12(_: Option<egui::CollapsingHeader>) {}
fn type_13(_: Option<egui::ComboBox>) {}
fn type_14(_: Option<egui::containers::menu::MenuBar>) {}
fn type_15(_: Option<egui::containers::menu::MenuButton>) {}
fn type_16(_: Option<egui::containers::menu::SubMenu>) {}
fn type_17(_: Option<egui::containers::menu::SubMenuButton>) {}
