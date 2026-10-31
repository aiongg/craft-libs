//! The export dialogs' pure rules.

// Reserved characters become `_`, the name is cut to 80 characters and trimmed, and an empty name
// becomes the fallback.
#[test]
#[ignore = "skeleton"]
fn file_name_replaces_reserved_characters_and_trims() {
    todo!()
}

// Later files of a multi-file export are numbered with their layer names and never collide.
#[test]
#[ignore = "skeleton"]
fn numbered_paths_add_layer_names_and_avoid_collisions() {
    todo!()
}

// Scale rounds each side to at least 1 px and refuses 0 % or 1001 %; a locked typed side keeps the
// source's aspect and 300,001 px is refused.
#[test]
#[ignore = "skeleton"]
fn resize_rules_round_keep_aspect_and_refuse_out_of_range() {
    todo!()
}

// 4-Up halves and quarters the quality of a JPEG and the colours of a GIF.
#[test]
#[ignore = "skeleton"]
fn four_up_variants_step_quality_or_colours() {
    todo!()
}

// The download time is Photoshop's 56.6 Kbps estimate, rounded up to whole seconds.
#[test]
#[ignore = "skeleton"]
fn download_time_rounds_up_at_56k() {
    todo!()
}

// JPEG quality 0–12 maps onto 1–100 and its label is Low, Medium, High or Maximum.
#[test]
#[ignore = "skeleton"]
fn jpeg_quality_maps_to_percent_and_label() {
    todo!()
}
