//! Built-in words of the choice family.

craft_ui::words! {
    /// The words of the rating's stars.
    pub struct ChoiceWords {
        unrated: "choice.unrated" = "Unrated";
        stars: count "choice.stars" = "{n} star" | "{n} stars";
    }
}
