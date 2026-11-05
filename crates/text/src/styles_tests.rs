//! Style names and folders.

use crate::styles::*;

// "Brand/Blue/500" is folder "Brand/Blue" and leaf "500"; with "Style 1" and "Style 3" taken the next is "Style 2"; groups keep first-appearance order with top-level styles first.
#[test]
fn folders_names_and_the_next_free_name() {
    assert_eq!(style_folder("Brand/Blue/500"), "Brand/Blue");
    assert_eq!(style_leaf("Brand/Blue/500"), "500");
    // A name without a separator is a leaf as typed; after a separator both parts are trimmed.
    assert_eq!((style_folder(" Plain "), style_leaf(" Plain ")), ("", " Plain "));
    assert_eq!((style_folder(" Brand / Red "), style_leaf(" Brand / Red ")), ("Brand", "Red"));
    assert_eq!(next_style_name(["Style 1", "Style 3"], "Style"), "Style 2");
    assert_eq!(next_style_name([], "Paragraph Style"), "Paragraph Style 1");

    let names = ["Brand/Blue/500", "Body", "Brand/Red", "Brand/Blue/600", "Lead"];
    let groups = group_by_folder(&names, |name| name);
    let folders: Vec<&str> = groups.iter().map(|(folder, _)| folder.as_str()).collect();
    assert_eq!(folders, ["", "Brand/Blue", "Brand"]);
    assert_eq!(groups[0].1, [&"Body", &"Lead"]);

    let listed = ["a", "b", "c", "d"];
    assert_eq!(dragged_styles(&["d", "b"], &listed, &"d"), ["b", "d"]);
    assert_eq!(dragged_styles(&["d", "b"], &listed, &"a"), ["a"]);
}
