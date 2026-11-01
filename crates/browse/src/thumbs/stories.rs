//! Stories of file thumbnails (not registered; task ST1): `primitives/file-thumbnails/source`
//! (`Thumbnails` over a `FileThumbSource` with a fake decoder: loading, quick preview as stand-in,
//! stand-in during reload, Failed, Ready).
//!
//! The spec's id `browse/thumbs/file-source` becomes `primitives/file-thumbnails/source`: the gallery
//! requires a story id to start with its level. Registering it (task RG2) adds `stories()` to the
//! browse stories and the component to `COMPONENTS`.
//!
//! The story decodes nothing real. A job stats its file, so the tiles name the running executable
//! (a file that exists) with a variant per tile; the decoder answers by variant. One tile names a
//! path that does not exist. Jobs run on a `ManualJobs`, one per "Finish next".

#![expect(dead_code, reason = "not registered until RG2 adds `stories()` and `COMPONENTS` to the browse stories")]

use std::path::PathBuf;
use std::sync::Arc;

use craft_host::ManualJobs;
use craft_host::queue::Cancel;
use craft_ui::canvas::textures::{Priority, TextureState, ThumbPaint, ThumbRequest, ThumbnailOptions, Thumbnails};
use craft_ui::controls::Button;
use craft_ui::localized::Labels as _;
use craft_ui::story::{Level, Story, StoryArgs};
use craft_ui::typed_story;
use egui::ecolor::Hsva;
use egui::{Sense, Ui, vec2};

use super::{DecodeRequest, DecodedThumb, FileThumbKey, FileThumbOptions, FileThumbSource, ThumbDecoder, ThumbError};

/// The components this module has stories for. RG2 makes this `pub const COMPONENTS` and adds the
/// module to the browse stories.
pub(crate) const COMPONENTS: &[&str] = &["File thumbnails"];

craft_ui::words! {
    /// The words of the file thumbnail story.
    pub(crate) struct FileThumbStoryWords {
        finish_next: "fileThumbStory.finishNext" = "Finish next";
        reload: "fileThumbStory.reload" = "File changed";
        plain: "fileThumbStory.plain" = "Plain image";
        raw: "fileThumbStory.raw" = "Raw with preview";
        unsupported: "fileThumbStory.unsupported" = "Unsupported";
        broken: "fileThumbStory.broken" = "Decoder fails";
        missing_file: "fileThumbStory.missingFile" = "Missing file";
        decoder_failed: "fileThumbStory.decoderFailed" = "The decoder failed";
        ready: "fileThumbStory.ready" = "Ready";
        loading: "fileThumbStory.loading" = "Loading";
        loading_stand_in: "fileThumbStory.loadingStandIn" = "Loading, stand-in shown";
        missing: "fileThumbStory.missing" = "Missing";
        empty: "fileThumbStory.empty" = "No picture";
        failed: "fileThumbStory.failed" = "Failed";
    }
}

/// The story above.
pub(crate) fn stories() -> Vec<Story> {
    vec![typed_story!(
        "primitives/file-thumbnails/source",
        Level::Primitives,
        "File thumbnails",
        "Source",
        SourceArgs { quick_previews: true },
        show_source
    )]
}

#[derive(StoryArgs, Debug, PartialEq)]
struct SourceArgs {
    /// Show an embedded preview first, then the decoded picture.
    #[arg(name = "quick previews")]
    quick_previews: bool,
}

const PLAIN: u64 = 0;
const RAW: u64 = 1;
const UNSUPPORTED: u64 = 2;
const BROKEN: u64 = 3;

/// A decoder that answers by the key's variant, with 4:3 pictures.
struct FakeDecoder {
    failure: String,
}

/// A picture of side `side`: a gradient of the variant's hue, or for a quick preview one flat
/// pale colour of it.
fn picture(side: u32, variant: u64, quick: bool) -> DecodedThumb {
    let (width, height) = (side.max(4), (side * 3 / 4).max(3));
    let hue = (variant % 6) as f32 / 6.0;
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let t = (x as f32 + y as f32) / (width + height) as f32;
            let color = if quick { Hsva::new(hue, 0.15, 0.6, 1.0) } else { Hsva::new((hue + 0.3 * t).fract(), 0.6, 0.85, 1.0) };
            rgba.extend(color.to_srgba_unmultiplied());
        }
    }
    DecodedThumb { width, height, rgba }
}

impl ThumbDecoder for FakeDecoder {
    fn decode(&self, request: &DecodeRequest, _cancel: &Cancel) -> Result<DecodedThumb, ThumbError> {
        match request.key.variant {
            UNSUPPORTED => Err(ThumbError::Unsupported),
            BROKEN => Err(ThumbError::Failed(self.failure.clone())),
            variant => Ok(picture(request.side, variant, false)),
        }
    }

    fn quick(&self, request: &DecodeRequest, _cancel: &Cancel) -> Option<DecodedThumb> {
        (request.key.variant == RAW).then(|| picture(request.side, RAW, true))
    }
}

/// What the story keeps between passes.
#[derive(Clone)]
struct Demo {
    source: FileThumbSource,
    jobs: Arc<ManualJobs>,
}

/// A file that exists wherever the story runs natively: the running program.
fn existing_file() -> PathBuf {
    std::env::current_exe().unwrap_or_else(|_| PathBuf::from("story-file"))
}

fn show_source(ui: &mut Ui, args: &mut SourceArgs) {
    let words = FileThumbStoryWords::of(ui);
    let ctx = ui.ctx().clone();
    let salt = ("file-thumb-story", args.quick_previews);
    let demo_id = ui.make_persistent_id(salt);
    let demo = ui.data_mut(|data| {
        data.get_temp_mut_or_insert_with(demo_id, || {
            let decoder = Arc::new(FakeDecoder { failure: words.decoder_failed.clone() });
            let options = FileThumbOptions { quick_previews: args.quick_previews, ..FileThumbOptions::default() };
            Demo { source: FileThumbSource::new(decoder, None).options(options), jobs: Arc::new(ManualJobs::new()) }
        })
        .clone()
    });
    let options = ThumbnailOptions { lanes: 1, jobs: Some(demo.jobs.clone()), ..ThumbnailOptions::default() };
    let thumbs = Thumbnails::<FileThumbKey>::in_context(&ctx, salt, options);
    let file = existing_file();

    ui.horizontal(|ui| {
        if ui.add(Button::new(words.finish_next.clone()).enabled(demo.jobs.waiting() > 0)).clicked() {
            demo.jobs.run_next();
        }
        if ui.add(Button::new(words.reload.clone())).clicked() {
            demo.source.invalidate(&file);
        }
    });
    let tiles = [
        (FileThumbKey { path: file.clone(), variant: PLAIN }, words.plain.clone()),
        (FileThumbKey { path: file.clone(), variant: RAW }, words.raw.clone()),
        (FileThumbKey { path: file.clone(), variant: UNSUPPORTED }, words.unsupported.clone()),
        (FileThumbKey { path: file, variant: BROKEN }, words.broken.clone()),
        (FileThumbKey::file(PathBuf::from("missing-file.raw")), words.missing_file.clone()),
    ];
    let request = ThumbRequest::for_cell(64.0, ui.pixels_per_point());
    ui.horizontal_wrapped(|ui| {
        for (key, caption) in &tiles {
            ui.vertical(|ui| {
                ui.set_width(64.0);
                let (rect, response) = ui.allocate_exact_size(vec2(64.0, 64.0), Sense::hover());
                let state = thumbs.state(&ctx, &demo.source, key, &request, Priority::VISIBLE);
                thumbs.paint(ui.painter(), rect, &state, &ThumbPaint::default());
                if let TextureState::Failed { message } = &state {
                    response.on_hover_text(message);
                }
                ui.label(caption.clone());
                ui.label(match &state {
                    TextureState::Missing => &words.missing,
                    TextureState::Loading { stand_in: None } => &words.loading,
                    TextureState::Loading { stand_in: Some(_) } => &words.loading_stand_in,
                    TextureState::Ready(_) => &words.ready,
                    TextureState::Empty => &words.empty,
                    TextureState::Failed { .. } => &words.failed,
                });
            });
        }
    });
}
