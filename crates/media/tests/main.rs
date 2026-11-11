//! Integration tests, built into the library's own test executable.

#[cfg(any(feature = "isobmff", feature = "matroska"))]
mod common;
#[cfg(feature = "isobmff")]
mod fuzz;
#[cfg(feature = "matroska")]
mod fuzz_matroska;
#[cfg(feature = "isobmff")]
mod oracle_demux;
#[cfg(feature = "matroska")]
mod oracle_matroska;
#[cfg(feature = "isobmff")]
mod oracle_mux;
#[cfg(feature = "isobmff")]
mod roundtrip;
