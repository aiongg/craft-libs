//! Screen tests of the row mapping (L4): one row built with `row::build_row` and shown alone.

#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Eye, layer thumbnail, each mask with its chain, name, then the indicators, in reading order (and
// mirrored right to left).
#[test]
#[ignore = "skeleton"]
fn row_parts_follow_the_layer_info_in_reading_order() {
    todo!()
}

// A hidden layer's eye cell is empty and its name is drawn with `list.text.dimmed`.
#[test]
#[ignore = "skeleton"]
fn hidden_layer_has_an_empty_eye_and_a_faint_name() {
    todo!()
}

// The blend label drops before the name gets narrower than `list.min_label`; the other indicators
// stay.
#[test]
#[ignore = "skeleton"]
fn blend_label_drops_before_the_name_gets_narrow() {
    todo!()
}

// The active row's target thumbnail (pixels, layer mask, vector mask) carries the corner brackets;
// other rows have none.
#[test]
#[ignore = "skeleton"]
fn target_brackets_frame_the_active_rows_target_part() {
    todo!()
}

// A disabled mask is crossed out in `layers.mask.disabled`; badges show only with
// `show_layer_mask_badges`.
#[test]
#[ignore = "skeleton"]
fn disabled_mask_is_crossed_out_and_badges_follow_the_setting() {
    todo!()
}

// Effects rows sit under their layer at `list.row_height_dense`, their eye under the layer's eye.
#[test]
#[ignore = "skeleton"]
fn effect_rows_sit_under_the_layer_with_the_eye_indented() {
    todo!()
}

// A Smart Filters header with `mask` shows the filter-mask thumbnail; at thumbnail size None it
// shows none.
#[test]
#[ignore = "skeleton"]
fn smart_filter_header_shows_the_shared_mask_thumbnail() {
    todo!()
}

// The trailing furniture is drawn in its cell and the name keeps at least `list.min_label`.
#[test]
#[ignore = "skeleton"]
fn trailing_slot_hosts_app_furniture_without_losing_the_name() {
    todo!()
}

// `row_height` gives the 8-cell table: None, Small, Medium, Large at Compact (24, 32, 32, 56) and
// Comfortable (30, 40, 42, 56).
#[test]
#[ignore = "skeleton"]
fn row_height_follows_size_preset_and_thumbnail_setting() {
    todo!()
}

// The row's AccessKit description holds the kind word, the flags that hold and the target words.
#[test]
#[ignore = "skeleton"]
fn rows_are_list_box_options_with_state_in_the_description() {
    todo!()
}

// Only `info` of the rows the list draws is asked for, in a tree of 10,000 layers.
#[test]
#[ignore = "skeleton"]
fn tree_with_ten_thousand_layers_builds_rows_in_view_only() {
    todo!()
}
