//! Print color: the blending color space of transparency groups
//! (`SerializeSettings::group_color_space`), overprint (`Surface::push_overprint`), knockout
//! groups (`Surface::push_group`), soft-mask backdrop and transfer function, DeviceN colors,
//! images and shadings, and the print output intent with the PDF/X markers.
#![cfg(feature = "write")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use backend::color::device_n::{self, Colorant, DeviceNSpace};
use backend::color::{cmyk, luma};
use backend::geom::{PathBuilder, Rect, Size, Transform};
use backend::mask::{Mask, MaskType};
use backend::num::NormalizedF32;
use backend::page::PageSettings;
use backend::paint::{Fill, LinearGradient, SpreadMethod, Stop};
use backend::surface::Surface;
use backend::{GroupColorSpace, SerializeSettings};
use craft_pdf::write::print;
use craft_pdf::write::{Metadata, WriteError, Writer, backend};
use hayro_syntax::object::{Array, Dict};

fn rect(surface: &mut Surface<'_>, x: f32, color: cmyk::Color, opacity: f32) {
    let mut path = PathBuilder::new();
    path.push_rect(Rect::from_xywh(x, 10.0, 40.0, 40.0).unwrap());
    surface.set_fill(Some(Fill { paint: color.into(), opacity: NormalizedF32::new(opacity).unwrap(), ..Default::default() }));
    surface.draw_path(&path.finish().unwrap());
}

fn k(value: u8) -> cmyk::Color {
    cmyk::Color::new(0, 0, 0, value)
}

/// Page 1: opacity group, isolated multiply group, soft mask, fill opacity and (with
/// `write-pdf`) a placed page. Page 2: opaque. Page 3: only the fill opacity of page 1, whose
/// graphics state is already written.
fn sample(settings: SerializeSettings, placed: Option<&[u8]>) -> Result<Vec<u8>, WriteError> {
    let mut writer = Writer::new(settings, Metadata::default());
    let page_settings = PageSettings::new(Size::from_wh(300.0, 60.0).unwrap());
    {
        let mut page = writer.document_mut().start_page_with(page_settings.clone());
        let mut surface = page.surface();
        rect(&mut surface, 0.0, cmyk::Color::new(255, 0, 0, 0), 1.0);
        surface.push_opacity(NormalizedF32::new(0.5).unwrap());
        rect(&mut surface, 20.0, k(255), 1.0);
        surface.pop();
        surface.push_isolated();
        surface.push_blend_mode(backend::blend::BlendMode::Multiply);
        rect(&mut surface, 60.0, cmyk::Color::new(0, 255, 0, 0), 1.0);
        surface.pop();
        surface.pop();
        let mut builder = surface.stream_builder();
        let mut mask_surface = builder.surface();
        let mut path = PathBuilder::new();
        path.push_rect(Rect::from_xywh(110.0, 10.0, 20.0, 40.0).unwrap());
        mask_surface.set_fill(Some(Fill { paint: luma::Color::white().into(), ..Default::default() }));
        mask_surface.draw_path(&path.finish().unwrap());
        mask_surface.finish();
        let stream = builder.finish();
        surface.push_mask(Mask::new(stream, MaskType::Luminosity));
        rect(&mut surface, 100.0, k(255), 1.0);
        surface.pop();
        rect(&mut surface, 150.0, k(255), 0.5);
        #[cfg(feature = "write-pdf")]
        if let Some(placed) = placed {
            let pdf = backend::pdf::Pdf::new(Arc::new(placed.to_vec())).unwrap();
            let doc = backend::pdf::PdfDocument::new(Arc::new(pdf));
            surface.draw_pdf_page(&doc, Size::from_wh(60.0, 12.0).unwrap(), 0);
        }
        #[cfg(not(feature = "write-pdf"))]
        let _ = placed;
        surface.finish();
        page.finish();
    }
    for page_index in [2, 3] {
        let mut page = writer.document_mut().start_page_with(page_settings.clone());
        let mut surface = page.surface();
        rect(&mut surface, 150.0, k(255), if page_index == 3 { 0.5 } else { 1.0 });
        surface.finish();
        page.finish();
    }
    writer.finish()
}

fn cmyk_settings() -> SerializeSettings {
    SerializeSettings { group_color_space: GroupColorSpace::Cmyk, ..Default::default() }
}

/// A copy of the sample to place on page 1, written with the same settings.
fn placed(settings: &SerializeSettings) -> Option<Vec<u8>> {
    cfg!(feature = "write-pdf").then(|| sample(settings.clone(), None).unwrap())
}

fn device_n(colorants: Vec<Colorant>, tints: Vec<u8>) -> backend::color::Color {
    device_n::Color::new(tints, DeviceNSpace::new(colorants).unwrap()).unwrap().into()
}

/// One page drawn by `draw`, written with CMYK blending.
fn page_with(draw: impl FnOnce(&mut Surface<'_>)) -> Vec<u8> {
    let mut writer = Writer::new(cmyk_settings(), Metadata::default());
    let mut page = writer.document_mut().start_page_with(PageSettings::new(Size::from_wh(300.0, 60.0).unwrap()));
    let mut surface = page.surface();
    draw(&mut surface);
    surface.finish();
    page.finish();
    writer.finish().unwrap()
}

fn stops(colors: [backend::color::Color; 2]) -> Vec<Stop> {
    let [a, b] = colors;
    vec![
        Stop { offset: NormalizedF32::ZERO, color: a, opacity: NormalizedF32::ONE },
        Stop { offset: NormalizedF32::ONE, color: b, opacity: NormalizedF32::ONE },
    ]
}

fn fill_rect(surface: &mut Surface<'_>, x: f32, paint: backend::paint::Paint) {
    let mut path = PathBuilder::new();
    path.push_rect(Rect::from_xywh(x, 10.0, 40.0, 40.0).unwrap());
    surface.set_fill(Some(Fill { paint, ..Default::default() }));
    surface.draw_path(&path.finish().unwrap());
}

fn linear(colors: [backend::color::Color; 2]) -> backend::paint::Paint {
    LinearGradient {
        x1: 0.0,
        y1: 0.0,
        x2: 40.0,
        y2: 0.0,
        transform: Transform::identity(),
        spread_method: SpreadMethod::Pad,
        stops: stops(colors),
        anti_alias: false,
    }
    .into()
}

/// The colors of a shading's function (`/C0` and `/C1` of the first shading's exponential
/// function).
fn shading_ends(bytes: &[u8]) -> (Vec<f32>, Vec<f32>) {
    let pdf = hayro_syntax::Pdf::new(Arc::new(bytes.to_vec())).unwrap();
    let page = pdf.pages().iter().next().unwrap();
    let patterns = &page.resources().patterns;
    let key = patterns.keys().next().unwrap();
    let pattern = patterns.get::<Dict<'_>>(key.as_ref()).unwrap();
    let shading = pattern.get::<Dict<'_>>(b"Shading".as_slice()).unwrap();
    let function = shading.get::<Dict<'_>>(b"Function".as_slice()).unwrap();
    let floats = |key: &[u8]| -> Vec<f32> { function.get::<Array<'_>>(key).unwrap().iter::<f32>().collect() };
    (floats(b"C0"), floats(b"C1"))
}

/// A gray stop in a CMYK gradient becomes the black ink `1 - gray`: gray 0.96 is K 0.04, and the
/// 8-bit gray 128 is K 127/255, as `cmyk::Color::new` writes it.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn gray_stops_keep_their_exact_black_in_cmyk_shadings() {
    let cyan = || -> backend::color::Color { cmyk::Color::new_f32(0.04, 0.0, 0.0, 0.0).into() };
    let bytes = page_with(|surface| fill_rect(surface, 0.0, linear([luma::Color::new_f32(0.96).into(), cyan()])));
    assert_eq!(shading_ends(&bytes), (vec![0.0, 0.0, 0.0, 0.04], vec![0.04, 0.0, 0.0, 0.0]));
    let bytes = page_with(|surface| fill_rect(surface, 0.0, linear([luma::Color::new(128).into(), cyan()])));
    assert_eq!(shading_ends(&bytes).0, vec![0.0, 0.0, 0.0, 127.0 / 255.0]);
}
