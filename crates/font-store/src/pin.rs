//! The index this build trusts: a commit of the index repository and the index's sha256.

/// An index pinned by commit and sha256.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IndexPin {
    /// The GitHub repository holding the index (`owner/name`).
    pub repo: &'static str,
    /// The repository's commit.
    pub commit: &'static str,
    /// The sha256 of `google-fonts-index.json` at that commit, lowercase hex.
    pub sha256: &'static str,
}

impl IndexPin {
    /// Where the index is downloaded from: jsDelivr, then raw GitHub.
    pub fn urls(&self) -> Vec<String> {
        let (repo, commit) = (self.repo, self.commit);
        vec![
            format!("https://cdn.jsdelivr.net/gh/{repo}@{commit}/google-fonts-index.json"),
            format!("https://raw.githubusercontent.com/{repo}/{commit}/google-fonts-index.json"),
        ]
    }
}

/// The index this build uses: google/fonts at `45e3d52`, the copy `Index::bundled` compiles in.
pub const PINNED_INDEX: IndexPin = IndexPin {
    repo: "aiongg/google-fonts-index",
    commit: "8d52c53eaa40f17e7441f8234389cdbed196eb7b",
    sha256: "4e07695111691394b2d9084ccb9bf8627a9a391610c7d3e6f4320c726d05bf01",
};
