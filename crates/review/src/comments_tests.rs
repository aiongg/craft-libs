//! Comment threads.

use crate::annotate::AnnotationKind;
use crate::comments::{CommentAnchor, CommentFilter, CommentMessage, CommentSort, CommentThread, ReviewStatus, ThreadId, sort_threads};

fn name(kind: AnnotationKind) -> String {
    match kind {
        AnnotationKind::Note => "Note".into(),
        AnnotationKind::Highlight => "Highlight".into(),
        other => format!("{other:?}"),
    }
}

fn message(author: &str, text: &str, created: i64) -> CommentMessage {
    CommentMessage { author: author.into(), created, modified: None, text: text.into(), reactions: Vec::new(), read: true }
}

fn thread(id: u64, anchor: CommentAnchor, kind: Option<AnnotationKind>, color: Option<[u8; 3]>, author: &str, created: i64) -> CommentThread {
    CommentThread {
        id: ThreadId(id),
        anchor,
        kind,
        color,
        messages: vec![message(author, "text", created)],
        status: ReviewStatus::None,
        marked: false,
        locked: false,
        resolved: false,
    }
}

fn page(page: usize, y: f64) -> CommentAnchor {
    CommentAnchor::Page { page, at: [0.0, y] }
}

// sort_threads by each CommentSort, ties in document order.
#[test]
fn sort_orders_by_page_author_date_type_and_color() {
    let threads = [
        thread(0, page(2, 10.0), Some(AnnotationKind::Note), Some([9, 0, 0]), "bea", 5),
        thread(1, page(1, 300.0), Some(AnnotationKind::Highlight), None, "Ana", 30),
        thread(2, page(1, 20.0), Some(AnnotationKind::Note), Some([1, 0, 0]), "ana", 10),
    ];
    assert_eq!(sort_threads(&threads, CommentSort::Page, name), [2, 1, 0]);
    assert_eq!(sort_threads(&threads, CommentSort::Author, name), [1, 2, 0]);
    assert_eq!(sort_threads(&threads, CommentSort::Date, name), [1, 2, 0]);
    assert_eq!(sort_threads(&threads, CommentSort::Type, name), [1, 0, 2]);
    assert_eq!(sort_threads(&threads, CommentSort::Color, name), [1, 2, 0]);
}

// matches is case-insensitive over text, author and the kind's reader name.

// the search folds the Turkish dotted and dotless i the way PdfCraft's fold_case does.

// a thread shows unless its type, author, status, colour or checkmark is hidden.
#[test]
fn filter_hides_listed_values() {
    let mut t = thread(0, page(0, 0.0), Some(AnnotationKind::Note), Some([1, 2, 3]), "Ana", 0);
    let mut filter = CommentFilter::default();
    assert!(filter.shows(&t) && !filter.is_active());
    filter.hidden_types.push(Some(AnnotationKind::Note));
    assert!(!filter.shows(&t) && filter.is_active());
    filter.clear();
    filter.hidden_authors.push("Ana".into());
    assert!(!filter.shows(&t));
    filter.clear();
    filter.hidden_checks.push(true);
    assert!(filter.shows(&t));
    t.marked = true;
    assert!(!filter.shows(&t));
    filter.clear();
    filter.hidden_colors.push(None);
    assert!(filter.shows(&t));
    filter.hidden_statuses.push(ReviewStatus::None);
    assert!(!filter.shows(&t));
}
