//! Every labels table, one line each: `<Struct as Labels>::WORDS,`. [`super::all`] walks this list.

use super::{Labels, Word};

pub(super) const TABLES: &[&[Word]] = &[
    <crate::preferences::PreferencesTexts as Labels>::WORDS,
    <crate::dialog::DialogLabels as Labels>::WORDS,
    <crate::menu::MenuBarLabels as Labels>::WORDS,
    <crate::behavior::EditMenuLabels as Labels>::WORDS,
    <crate::patterns::PaletteLabels as Labels>::WORDS,
    <crate::patterns::inspector::InspectorLabels as Labels>::WORDS,
    <crate::patterns::panel::ScrollBarLabels as Labels>::WORDS,
    <crate::patterns::rail::tool::RailLabels as Labels>::WORDS,
    <crate::tab_strip::TabStripText as Labels>::WORDS,
    <crate::panes::PaneLabels as Labels>::WORDS,
    <crate::composites::panel_header::PanelHeaderLabels as Labels>::WORDS,
    <crate::composites::toast::ToastLabels as Labels>::WORDS,
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
