#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]
//! Media containers and bit-level I/O for the Craft apps.
//!
//! - `bitstream` (feature `bitstream`): MSB-first bit reader and writer, Exp-Golomb codes,
//!   emulation-prevention escaping and NAL unit splitting.
//! - `isobmff` (feature `isobmff`, enables `bitstream`): a clean-room ISO BMFF (MP4) and
//!   QuickTime (MOV) demuxer and muxer.
//!
//! Both modules are on by default. A crate that needs only bit I/O uses
//! `default-features = false, features = ["bitstream"]`. Codecs stay in the apps.

#[cfg(feature = "bitstream")]
pub mod bitstream;
#[cfg(feature = "isobmff")]
pub mod isobmff;
