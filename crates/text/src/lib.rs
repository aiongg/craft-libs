#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

//! Shared text processing for the Craft apps. It sits below the composer: layout code calls it on
//! plain text and a language before it shapes and places glyphs.
//!
//! Modules, each behind the cargo feature of the same name:
//! - [`hyphen`] (default): hyphenation points, with bundled US English data and optional
//!   hypher patterns for 47 more languages (Slovak, under the GPL, only with `slovak-gpl`).
//! - [`segment`] (default): line-break opportunities (UAX #14 with kinsoku), and word breaks for
//!   Thai, Lao, Khmer and Burmese with their language features.
//! - [`bidi`] (default): bidirectional text (UAX #9): embedding levels, line reordering and
//!   mirroring.
//! - [`linebreak`] (default): line breaking on a font-independent box/glue/penalty model.
//! - [`spell`]: spelling, with a bundled US English word list and suggestions.
//! - [`quotes`]: smart quotes, the straight quotes typed into text turned into the language's own.
//! - [`ui`] (egui, on craft-ui): the canvas text editor, the Character and Paragraph controls, the
//!   text style panels and the Check Spelling dialog. With `font-picker`, the Character controls'
//!   family field is craft-font-manager's font picker.
//!
//! Language features (`spanish`, `german`, …, `all-languages`) add data for that language and
//! turn on the modules that use it.

#[cfg(feature = "bidi")]
pub mod bidi;
#[cfg(feature = "hyphen")]
pub mod hyphen;
#[cfg(feature = "linebreak")]
pub mod linebreak;
pub mod quotes;
#[cfg(test)]
mod quotes_tests;
#[cfg(feature = "segment")]
pub mod segment;
#[cfg(feature = "spell")]
pub mod spell;
#[cfg(all(test, feature = "spell"))]
mod spell_tests;
#[cfg(feature = "ui")]
pub mod ui;
