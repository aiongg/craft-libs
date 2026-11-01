//! Comment threads.

use crate::comments::{CommentAnchor, CommentMessage, CommentSort, CommentThread, ReviewStatus, ThreadId, initials, matches, sort_threads};

fn message(author: &str, text: &str, created: i64) -> CommentMessage {
    CommentMessage { author: author.into(), created, modified: None, text: text.into(), reactions: Vec::new(), read: true }
}

fn thread(id: u64, anchor: CommentAnchor, kind: &str, color: Option<[u8; 3]>, author: &str, created: i64) -> CommentThread {
    CommentThread {
        id: ThreadId(id),
        anchor,
        kind: kind.into(),
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
        thread(0, page(2, 10.0), "Note", Some([9, 0, 0]), "bea", 5),
        thread(1, page(1, 300.0), "Highlight", None, "Ana", 30),
        thread(2, page(1, 20.0), "Note", Some([1, 0, 0]), "ana", 10),
    ];
    assert_eq!(sort_threads(&threads, CommentSort::Page), [2, 1, 0]);
    assert_eq!(sort_threads(&threads, CommentSort::Author), [1, 2, 0]);
    assert_eq!(sort_threads(&threads, CommentSort::Date), [1, 2, 0]);
    assert_eq!(sort_threads(&threads, CommentSort::Type), [1, 0, 2]);
    assert_eq!(sort_threads(&threads, CommentSort::Color), [1, 2, 0]);
}

// matches is case-insensitive over text, author and kind.
#[test]
fn search_matches_text_author_and_kind() {
    let mut t = thread(0, page(0, 0.0), "Highlight", None, "Ana María", 0);
    t.messages.push(message("Bo", "Fix the Margin", 1));
    assert!(matches(&t, "margin"));
    assert!(matches(&t, "MARÍA"));
    assert!(matches(&t, "highl"));
    assert!(!matches(&t, "absent"));
}

// initials of 'Ana María Ruiz' is AM, of '' is ?.
#[test]
fn initials_take_two_first_letters() {
    assert_eq!(initials("Ana María Ruiz"), "AM");
    assert_eq!(initials("cher"), "C");
    assert_eq!(initials("  "), "?");
}
