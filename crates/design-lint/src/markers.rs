//! `// design: allow(<reason>)` markers: finding them in the source text and applying them to findings.

use std::ops::RangeInclusive;

use proc_macro2::{TokenStream, TokenTree};
use syn::spanned::Spanned;
use syn::visit::{self, Visit};

use crate::report::{Finding, Severity};
use crate::rules::analyze_source;

pub const EMPTY: &str = "marker/empty";
pub const UNUSED: &str = "marker/unused";

/// Shortest accepted marker or exemption reason, in characters.
pub const MIN_REASON: usize = 3;

const PREFIX: &str = "design: allow(";

/// The findings of one file after markers were applied.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Marked {
    /// Findings not covered by a marker, plus `marker/empty` and `marker/unused` findings.
    pub findings: Vec<Finding>,
    /// Markers found in the file.
    pub markers: usize,
}

#[derive(Debug)]
enum Scope {
    /// `//! design: allow(..)`: every finding in the file.
    File,
    /// End-of-line marker: findings starting on `line`.
    Line,
    /// Alone on its line: findings starting in the next code line's statement or item.
    Next,
}

#[derive(Debug)]
struct Marker {
    line: usize,
    column: usize,
    scope: Scope,
    /// Why the marker is invalid, when it is.
    problem: Option<&'static str>,
}

/// Checks one file's source text and applies its markers.
///
/// `exempt(rule)` tells whether a path exemption already silences a rule in this file;
/// those findings are dropped before markers are matched, so a marker that only covers them is unused.
pub fn analyze_with_markers(src: &str, include_tests: bool, deny_unused: bool, exempt: &dyn Fn(&str) -> bool) -> Marked {
    let analysis = analyze_source(src, include_tests);
    let mut findings: Vec<Finding> = analysis.findings.into_iter().filter(|f| !exempt(f.rule)).collect();
    if findings.iter().any(|f| f.rule == crate::rules::PARSE) {
        return Marked { findings, markers: 0 };
    }
    let lines: Vec<&str> = src.lines().collect();
    let markers = find_markers(src, &lines);
    let mut spans: Option<Vec<RangeInclusive<usize>>> = None;
    let mut used = vec![false; markers.len()];
    let mut kept = Vec::new();
    // A finding is covered when it starts inside the marker's range.
    let covering: Vec<Option<RangeInclusive<usize>>> = markers
        .iter()
        .map(|m| match m.scope {
            _ if m.problem.is_some() => None,
            Scope::File => Some(0..=usize::MAX),
            Scope::Line => Some(m.line..=m.line),
            Scope::Next => next_code_line(&lines, m.line).map(|line| {
                let item_spans = spans.get_or_insert_with(|| item_spans(src));
                let end = item_spans.iter().filter(|r| *r.start() == line).map(|r| *r.end()).max().unwrap_or(line);
                line..=end
            }),
        })
        .collect();
    for finding in findings.drain(..) {
        let hit = covering.iter().position(|c| c.as_ref().is_some_and(|r| r.contains(&finding.line)));
        match hit {
            Some(i) => used[i] = true,
            None => kept.push(finding),
        }
    }
    let severity = if deny_unused { Severity::Error } else { Severity::Warning };
    for (i, marker) in markers.iter().enumerate() {
        let (rule, message) = match marker.problem {
            Some(problem) => (EMPTY, problem.to_owned()),
            None if used[i] || analysis.skipped.iter().any(|r| r.contains(&marker.line)) => continue,
            None => (UNUSED, "marker suppresses nothing; remove it".to_owned()),
        };
        if exempt(rule) {
            continue;
        }
        let mut finding = Finding::new(rule, marker.line, marker.column, marker.line, message);
        if rule == UNUSED {
            finding.severity = severity;
        }
        kept.push(finding);
    }
    kept.sort_by_key(|f| (f.line, f.column));
    Marked { findings: kept, markers: markers.len() }
}

/// The first line after `line` that holds code: not blank and not a comment.
fn next_code_line(lines: &[&str], line: usize) -> Option<usize> {
    (line + 1..=lines.len()).find(|&n| {
        let text = lines[n - 1].trim_start();
        !text.is_empty() && !text.starts_with("//")
    })
}

/// Line ranges of every statement, item, match arm and field initialiser in the file.
fn item_spans(src: &str) -> Vec<RangeInclusive<usize>> {
    struct Spans(Vec<RangeInclusive<usize>>);
    impl Spans {
        fn add(&mut self, span: proc_macro2::Span) {
            self.0.push(span.start().line..=span.end().line);
        }
    }
    impl<'ast> Visit<'ast> for Spans {
        fn visit_stmt(&mut self, i: &'ast syn::Stmt) {
            self.add(i.span());
            visit::visit_stmt(self, i);
        }
        fn visit_item(&mut self, i: &'ast syn::Item) {
            self.add(i.span());
            visit::visit_item(self, i);
        }
        fn visit_impl_item(&mut self, i: &'ast syn::ImplItem) {
            self.add(i.span());
            visit::visit_impl_item(self, i);
        }
        fn visit_trait_item(&mut self, i: &'ast syn::TraitItem) {
            self.add(i.span());
            visit::visit_trait_item(self, i);
        }
        fn visit_arm(&mut self, i: &'ast syn::Arm) {
            self.add(i.span());
            visit::visit_arm(self, i);
        }
        fn visit_field_value(&mut self, i: &'ast syn::FieldValue) {
            self.add(i.span());
            visit::visit_field_value(self, i);
        }
    }
    let mut spans = Spans(Vec::new());
    if let Ok(file) = syn::parse_file(src) {
        spans.visit_file(&file);
    }
    spans.0
}

type LiteralSpan = ((usize, usize), (usize, usize));

/// `((line, column), (line, column))` of each string, char and byte literal token, columns in characters.
fn literal_spans(src: &str, lines: &[&str]) -> Vec<LiteralSpan> {
    fn walk(stream: TokenStream, lines: &[&str], out: &mut Vec<LiteralSpan>) {
        for tree in stream {
            match tree {
                TokenTree::Group(g) => walk(g.stream(), lines, out),
                TokenTree::Literal(l) => {
                    let (s, e) = (l.span().start(), l.span().end());
                    // Doc comments lex to a literal spanning the comment itself.
                    let text: String = lines.get(s.line.wrapping_sub(1)).map(|t| t.chars().skip(s.column).take(2).collect()).unwrap_or_default();
                    if text != "//" && text != "/*" {
                        out.push(((s.line, s.column), (e.line, e.column)));
                    }
                }
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    if let Ok(stream) = src.parse::<TokenStream>() {
        walk(stream, lines, &mut out);
    }
    out
}

fn find_markers(src: &str, lines: &[&str]) -> Vec<Marker> {
    let literals = literal_spans(src, lines);
    let mut out = Vec::new();
    for (idx, text) in lines.iter().enumerate() {
        let line = idx + 1;
        let in_literal = |column: usize| literals.iter().any(|(s, e)| (s.0, s.1) <= (line, column) && (line, column) < (e.0, e.1));
        let chars: Vec<(usize, char)> = text.char_indices().collect();
        let Some(column) = (0..chars.len().saturating_sub(1)).find(|&c| chars[c].1 == '/' && chars[c + 1].1 == '/' && !in_literal(c)) else {
            continue;
        };
        let after = &text[chars[column].0 + 2..];
        let (file_level, body) = match after.strip_prefix('!') {
            Some(rest) => (true, rest),
            None if after.starts_with('/') => continue,
            None => (false, after),
        };
        let Some(inner) = body.trim().strip_prefix(PREFIX) else { continue };
        let alone = text[..chars[column].0].trim().is_empty();
        let scope = match (file_level, alone) {
            (true, _) => Scope::File,
            (false, true) => Scope::Next,
            (false, false) => Scope::Line,
        };
        let problem = match inner.rfind(')') {
            None => Some("marker is missing the closing `)`"),
            Some(end) if inner[..end].trim().chars().count() < MIN_REASON => {
                Some("marker needs a reason of at least three characters: `// design: allow(<reason>)`")
            }
            Some(_) => None,
        };
        out.push(Marker { line, column: column + 1, scope, problem });
    }
    out
}
