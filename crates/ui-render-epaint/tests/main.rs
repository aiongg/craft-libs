//! Trials for the epaint renderer: pixels from a headless wgpu device (lavapipe where there is
//! no GPU), compared as bytes.

#![allow(clippy::chunks_exact_to_as_chunks)]

mod boundary;
mod common;
mod fills;
mod images;
mod items;
mod surfaces;
