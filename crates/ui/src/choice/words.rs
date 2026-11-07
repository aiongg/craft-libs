//! Built-in words of the choice family.

craft_ui::words! {
    /// The words of the rating's stars.
    pub struct ChoiceWords {
        unrated: "choice.rating.unrated" = "Unrated";
        stars: count "choice.rating.stars" = "{n} star" | "{n} stars";
    }
}
