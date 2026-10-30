//! The search rule, shared by the body, hosts and tests.

use super::model::{PageSpec, RowSpec};

/// Whether `label`, `keywords` or `key` contains `query`: trimmed, case-insensitive substring.
/// An empty query matches everything.
pub fn matches(query: &str, label: &str, keywords: &str, key: &str) -> bool {
    let query = query.trim().to_lowercase();
    query.is_empty() || [label, keywords, key].iter().any(|text| text.to_lowercase().contains(&query))
}

/// Whether `row` matches `query`.
pub fn row_matches(query: &str, row: &RowSpec<'_>) -> bool {
    matches(query, row.label, row.keywords, row.key)
}

/// A page that the page list shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Listed {
    /// Its place in the host's pages.
    pub index: usize,
    /// Drawn dimmed: no visible row.
    pub dimmed: bool,
    /// The title matched (or there is no query): every visible row shows.
    pub all_rows: bool,
}

fn searching(query: &str) -> bool {
    !query.trim().is_empty()
}

/// The pages the list shows for `query`: a page whose title matches or that has a matching
/// visible row. Without a query every page is listed, the empty ones dimmed; with one an empty
/// page is listed only when its title matches.
pub fn list_pages(pages: &[PageSpec<'_>], query: &str) -> Vec<Listed> {
    pages
        .iter()
        .enumerate()
        .filter_map(|(index, page)| {
            let title = matches(query, page.title, "", "");
            let all_rows = title;
            let any_row = page.groups.iter().flat_map(|g| &g.rows).any(|r| r.visible && row_matches(query, r));
            let listed = !searching(query) || title || any_row;
            listed.then_some(Listed { index, dimmed: !page.has_content(), all_rows })
        })
        .collect()
}

/// Whether `row` shows on a page listed as `listed`, under `query`.
pub fn row_shown(query: &str, listed: &Listed, row: &RowSpec<'_>) -> bool {
    row.visible && (listed.all_rows || row_matches(query, row))
}

/// The place in `listed` of the page to show: the selected page, else the first listed one. The
/// selection itself is the user's and is kept while the search hides it.
pub fn shown_position(listed: &[Listed], pages: &[PageSpec<'_>], selected: &str) -> Option<usize> {
    listed.iter().position(|l| pages[l.index].id == selected).or(if listed.is_empty() { None } else { Some(0) })
}
