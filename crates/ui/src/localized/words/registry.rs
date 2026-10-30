//! Every labels table, one line each: `<Struct as Labels>::WORDS,`. [`super::all`] walks this list.

use super::{Labels, Word};

pub(super) const TABLES: &[&[Word]] = &[
    <crate::docking::DockConfig as Labels>::WORDS,
    <crate::presets::PresetLabels as Labels>::WORDS,
    <crate::list::TreeText as Labels>::WORDS,
    #[cfg(feature = "window-frame")]
    <crate::title_bar::CaptionLabels as Labels>::WORDS,
    <crate::controls::ThumbRowLabels as Labels>::WORDS,
    <crate::list::ThumbSizeLabels as Labels>::WORDS,
    <crate::canvas::RefPointLabels as Labels>::WORDS,
    crate::controls::kbd::KEY_WORDS,
];
