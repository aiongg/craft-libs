//! Stories of the review widgets. Not registered yet: registering them would change the registry
//! dump and run their bodies. The registration task adds them to the gallery, the words of
//! [`ReviewStoryLabels`] to the words registry, and regenerates the dump.
//!
//! Each story keeps its own demo state in egui memory and applies the widget's events to it as
//! an app would. The line under a story names the last event. Sample comments, names and
//! version titles are words of [`ReviewStoryLabels`].
//!
//! | Story id | Shows |
//! |---|---|
//! | `patterns/comments/panel` | six threads on three pages, the second selected |
//! | `patterns/comments/thread` | four pins (two clustered) and an open popover with reactions and a mention |
//! | `patterns/annotate/toolbar` | the toolbar over a page with a highlight, a rectangle (selected) and ink |
//! | `patterns/signature-pad/dialog` | the pad on the Draw tab with two strokes |
//! | `patterns/version-history/panel` | two branches, Current version, three named versions, an autosave group |
//! | `patterns/revisions/panel` | an inserted, a changed and a deleted slide |

use craft_ui::canvas::{CanvasId, CanvasView, ToolHost};
use craft_ui::edit::EditEvent;
use craft_ui::localized::{Labels as _, fmt};
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, NoArgs, Story, StoryArgs, StoryLabels, drive, lock, shared};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, UiTokens};
use craft_ui::typed_story;
use std::cell::RefCell;
use std::collections::HashMap;

use egui::{Color32, Id, Pos2, Rect as ScreenRect, Sense, Ui, Vec2, pos2, vec2};
use kurbo::{Point, Rect};

use super::ReviewStyle;
use super::annotate::{AnnotateTool, AnnotationOverlay, AnnotationToolbar};
use super::comments::{CommentEdit, CommentsPanel, CommentsPanelState};
use super::revisions::RevisionsPanel;
use super::signature::{SignaturePadDialog, SignaturePadResult};
use super::thread::{CommentPins, ThreadPopover};
use super::versions::{VersionAction, VersionHistoryPanel};
use crate::annotate::{Annotation, AnnotationEdit, AnnotationId, AnnotationKind, AnnotationTool};
use crate::comments::{CommentAnchor, CommentMessage, CommentThread, Reaction, ReviewStatus, ThreadId};
use crate::revisions::{Revision, RevisionAction, RevisionKind};
use crate::signature::{SignatureDraft, SignatureTab};
use crate::versions::{Branch, BranchId, BranchState, Version, VersionId};

pub(super) fn stories() -> Vec<Story> {
    vec![
        typed_story!("patterns/comments/panel", Level::Patterns, "Comments", "Panel", NoArgs {}, comments_panel),
        typed_story!("patterns/comments/thread", Level::Patterns, "Comments", "Thread", NoArgs {}, thread),
        typed_story!("patterns/annotate/toolbar", Level::Patterns, "Annotate", "Toolbar", NoArgs {}, annotate),
        typed_story!("patterns/signature-pad/dialog", Level::Patterns, "Signature pad", "Dialog", PadArgs { open: true }, signature_pad),
        typed_story!("patterns/version-history/panel", Level::Patterns, "Version history", "Panel", NoArgs {}, versions),
        typed_story!("patterns/revisions/panel", Level::Patterns, "Revisions", "Panel", NoArgs {}, revisions),
    ]
}

craft_ui::words! {
    /// The words the review stories add: people, sample comments, version titles and slide
    /// changes.
    #[non_exhaustive]
    pub(crate) struct ReviewStoryLabels {
        /// The viewer, and two reviewers.
        me: "reviewStory.me" = "Maya Chen";
        ana: "reviewStory.ana" = "Ana Silva";
        bo: "reviewStory.bo" = "Bo Lindqvist";
        /// Annotation kinds of the sample threads.
        kind_note: "reviewStory.kindNote" = "Note";
        kind_highlight: "reviewStory.kindHighlight" = "Highlight";
        kind_text_box: "reviewStory.kindTextBox" = "Text box";
        kind_underline: "reviewStory.kindUnderline" = "Underline";
        /// Sample comments and replies.
        comment_logo: "reviewStory.commentLogo" = "Move the logo up to match the brand sheet.";
        comment_figure: "reviewStory.commentFigure" = "Is this figure final? The caption still says 2024.";
        comment_margin: "reviewStory.commentMargin" = "Check the margin on this side.";
        comment_legal: "reviewStory.commentLegal" = "Approved wording from legal.";
        comment_shorten: "reviewStory.commentShorten" = "Can we shorten this paragraph?";
        reply_two_lines: "reviewStory.replyTwoLines" = "I can cut it to two lines.";
        reply_keep_quote: "reviewStory.replyKeepQuote" = "Please do, and keep the quote.";
        comment_typo: "reviewStory.commentTypo" = "Typo in the heading.";
        /// The thread popover's sample.
        thread_check: "reviewStory.threadCheck" = "Can you check the spacing here, @Bo?";
        thread_reply: "reviewStory.threadReply" = "Fixed, thanks @B";
        thread_other_one: "reviewStory.threadOtherOne" = "Looks off-centre.";
        thread_other_two: "reviewStory.threadOtherTwo" = "Use the blue from the palette.";
        thread_other_three: "reviewStory.threadOtherThree" = "Add alt text.";
        thread_other_four: "reviewStory.threadOtherFour" = "Same here.";
        /// Sample version titles and descriptions.
        version_first: "reviewStory.versionFirst" = "First draft";
        version_review: "reviewStory.versionReview" = "Client review";
        version_final: "reviewStory.versionFinal" = "Launch candidate";
        version_description: "reviewStory.versionDescription" = "Sent to the client for sign-off.";
        branch_dark: "reviewStory.branchDark" = "Dark theme";
        branch_print: "reviewStory.branchPrint" = "Print layout";
        /// The compared copy and its changes.
        compared_with: "reviewStory.comparedWith" = "Quarterly review (Maya's copy)";
        revision_inserted: "reviewStory.revisionInserted" = "Slide inserted after 2";
        revision_inserted_detail: "reviewStory.revisionInsertedDetail" = "New slide: Market sizing";
        revision_changed: "reviewStory.revisionChanged" = "Slide 3 changed";
        revision_changed_title: "reviewStory.revisionChangedTitle" = "Title text changed";
        revision_changed_chart: "reviewStory.revisionChangedChart" = "Chart data changed";
        revision_deleted: "reviewStory.revisionDeleted" = "Slide 5 deleted";
        revision_deleted_detail: "reviewStory.revisionDeletedDetail" = "Removed slide: Appendix";
    }
}

/// Unix seconds of the sample history (2026-10-01 00:00 UTC) and a day's seconds.
const START: i64 = 1_790_812_800;
const DAY: i64 = 86_400;
const HOUR: i64 = 3_600;

/// The height of a canvas over its width.
const CANVAS_ASPECT: f32 = 0.6;

fn message(author: &str, text: &str, created: i64) -> CommentMessage {
    CommentMessage { author: author.into(), created, modified: None, text: text.into(), reactions: Vec::new(), read: true }
}

fn rgb(color: Color32) -> [u8; 3] {
    [color.r(), color.g(), color.b()]
}

fn last_event(ui: &mut Ui, event: Option<&str>) {
    if let Some(event) = event {
        let line = fmt(&StoryLabels::of(ui).last_event, &[("event", event)]);
        ui.add(Text::new(line).content(Content::Muted));
    }
}

fn next_id(ids: impl Iterator<Item = u64>) -> u64 {
    ids.max().unwrap_or(0) + 1
}

/// The edits of the panel and the popover applied to `threads` as an app would.
fn apply_comment(threads: &mut Vec<CommentThread>, me: &str, edit: &CommentEdit) {
    let find = |threads: &[CommentThread], id: ThreadId| threads.iter().position(|t| t.id == id);
    match edit {
        CommentEdit::Add { text } => {
            threads.push(CommentThread {
                id: ThreadId(next_id(threads.iter().map(|t| t.id.0))),
                anchor: CommentAnchor::Page { page: 1, at: [0.0, 0.0] },
                kind: String::new(),
                color: None,
                messages: vec![message(me, text, START + 30 * DAY)],
                status: ReviewStatus::None,
                marked: false,
                locked: false,
                resolved: false,
            });
        }
        CommentEdit::Reply { thread, text } => {
            if let Some(at) = find(threads, *thread) {
                threads[at].messages.push(message(me, text, START + 30 * DAY));
            }
        }
        CommentEdit::Edit { thread, index, text } => {
            if let Some(m) = find(threads, *thread).and_then(|at| threads[at].messages.get_mut(*index)) {
                m.text.clone_from(text);
            }
        }
        CommentEdit::Delete { thread, index } => {
            if let Some(at) = find(threads, *thread) {
                if *index == 0 {
                    threads.remove(at);
                } else if *index < threads[at].messages.len() {
                    threads[at].messages.remove(*index);
                }
            }
        }
        CommentEdit::SetStatus { thread, status } => {
            if let Some(at) = find(threads, *thread) {
                threads[at].status = *status;
            }
        }
        CommentEdit::Mark { thread, marked } => {
            if let Some(at) = find(threads, *thread) {
                threads[at].marked = *marked;
            }
        }
        CommentEdit::Resolve { thread, resolved } => {
            if let Some(at) = find(threads, *thread) {
                threads[at].resolved = *resolved;
            }
        }
        CommentEdit::React { thread, index, emoji } => {
            if let Some(m) = find(threads, *thread).and_then(|at| threads[at].messages.get_mut(*index)) {
                match m.reactions.iter().position(|r| r.emoji == *emoji) {
                    Some(r) => {
                        if let Some(p) = m.reactions[r].people.iter().position(|p| p == me) {
                            m.reactions[r].people.remove(p);
                        } else {
                            m.reactions[r].people.push(me.into());
                        }
                        if m.reactions[r].people.is_empty() {
                            m.reactions.remove(r);
                        }
                    }
                    None => m.reactions.push(Reaction { emoji: emoji.clone(), people: vec![me.into()] }),
                }
            }
        }
        CommentEdit::MarkUnread { thread } => {
            if let Some(m) = find(threads, *thread).and_then(|at| threads[at].messages.last_mut()) {
                m.read = false;
            }
        }
        CommentEdit::Select(_) | CommentEdit::CopyText { .. } | CommentEdit::CopyLink { .. } => {}
    }
}

struct CommentsDemo {
    threads: Vec<CommentThread>,
    state: CommentsPanelState,
    last: Option<String>,
}

fn sample_threads(labels: &ReviewStoryLabels, tokens: &UiTokens) -> Vec<CommentThread> {
    let status = &tokens.color.status;
    let make = |id: u64, page: usize, y: f64, kind: &str, color: Option<Color32>, messages: Vec<CommentMessage>| CommentThread {
        id: ThreadId(id),
        anchor: CommentAnchor::Page { page, at: [0.0, y] },
        kind: kind.into(),
        color: color.map(rgb),
        messages,
        status: ReviewStatus::None,
        marked: false,
        locked: false,
        resolved: false,
    };
    let day = |n: i64| START + n * DAY;
    let mut logo =
        make(1, 1, 40.0, &labels.kind_highlight, Some(status.warn.solid.to_egui()), vec![message(&labels.ana, &labels.comment_logo, day(1))]);
    logo.status = ReviewStatus::Accepted;
    let figure = make(2, 1, 200.0, &labels.kind_note, None, vec![message(&labels.bo, &labels.comment_figure, day(2))]);
    let mut margin =
        make(3, 2, 90.0, &labels.kind_note, Some(status.bad.solid.to_egui()), vec![message(&labels.ana, &labels.comment_margin, day(3))]);
    margin.marked = true;
    let mut legal = make(4, 2, 300.0, &labels.kind_text_box, None, vec![message(&labels.bo, &labels.comment_legal, day(4))]);
    legal.locked = true;
    let shorten = make(
        5,
        3,
        60.0,
        &labels.kind_note,
        Some(status.good.solid.to_egui()),
        vec![
            message(&labels.bo, &labels.comment_shorten, day(5)),
            message(&labels.ana, &labels.reply_two_lines, day(6)),
            message(&labels.bo, &labels.reply_keep_quote, day(7)),
        ],
    );
    let typo = make(6, 3, 250.0, &labels.kind_underline, None, vec![message(&labels.ana, &labels.comment_typo, day(8))]);
    vec![logo, figure, margin, legal, shorten, typo]
}

// new: six threads on three pages (one Accepted, one marked, one locked, one with two replies),
// the second selected.
fn comments_panel(ui: &mut Ui, _: &mut NoArgs) {
    let labels = ReviewStoryLabels::of(ui);
    let tokens = Tokens::of(ui);
    let demo = shared(ui, "comments", || CommentsDemo {
        threads: sample_threads(&labels, &tokens),
        state: CommentsPanelState { selected: Some(ThreadId(2)), ..CommentsPanelState::default() },
        last: None,
    });
    let mut demo = lock(&demo);
    let demo = &mut *demo;
    let out = CommentsPanel::new("comments", &demo.threads, &mut demo.state, &labels.me).show(ui);
    for edit in &out.edits {
        demo.last = Some(format!("{edit:?}"));
        apply_comment(&mut demo.threads, &labels.me, edit);
    }
    last_event(ui, demo.last.as_deref());
}

struct ThreadDemo {
    threads: Vec<CommentThread>,
    /// Pin points as fractions of the canvas.
    spots: Vec<Pos2>,
    open: Option<ThreadId>,
    draft: String,
    last: Option<String>,
}

// new: four pins (two clustered) over a grey canvas and an open popover with reactions and an
// @mention list.
fn thread(ui: &mut Ui, _: &mut NoArgs) {
    let labels = ReviewStoryLabels::of(ui);
    let tokens = Tokens::of(ui);
    let style = ReviewStyle::of(&tokens);
    let demo = shared(ui, "thread", || {
        let make = |id: u64, author: &str, text: &str| CommentThread {
            id: ThreadId(id),
            anchor: CommentAnchor::Canvas { at: [0.0, 0.0], node: None },
            kind: String::new(),
            color: None,
            messages: vec![message(author, text, START + id as i64 * DAY)],
            status: ReviewStatus::None,
            marked: false,
            locked: false,
            resolved: false,
        };
        let mut first = make(1, &labels.ana, &labels.thread_check);
        first.messages[0].reactions = vec![
            Reaction { emoji: "👍".into(), people: vec![labels.bo.clone(), labels.me.clone()] },
            Reaction { emoji: "👀".into(), people: vec![labels.bo.clone()] },
        ];
        first.messages.push(message(&labels.bo, &labels.thread_other_one, START + 2 * DAY));
        // The viewer's own pin is the third, so it drags.
        let threads = vec![
            first,
            make(2, &labels.bo, &labels.thread_other_two),
            make(3, &labels.me, &labels.thread_other_three),
            make(4, &labels.bo, &labels.thread_other_four),
        ];
        let spots = vec![pos2(0.18, 0.3), pos2(0.55, 0.45), pos2(0.55, 0.45), pos2(0.82, 0.75)];
        ThreadDemo { threads, spots, open: Some(ThreadId(1)), draft: labels.thread_reply.clone(), last: None }
    });
    let mut demo = lock(&demo);
    let demo = &mut *demo;

    let width = ui.available_width();
    let (canvas, _) = ui.allocate_exact_size(vec2(width, width * CANVAS_ASPECT), Sense::hover());
    ui.painter().rect_filled(canvas, 0.0, tokens.color.surface.canvas.to_egui());
    // The third pin starts a quarter of the cluster radius from the second.
    let nudge = |i: usize| if i == 2 { vec2(style.cluster_radius * 0.25, 0.0) } else { Vec2::ZERO };
    let to_screen = |i: usize, spot: Pos2| pos2(canvas.min.x + canvas.width() * spot.x, canvas.min.y + canvas.height() * spot.y) + nudge(i);
    let points: Vec<Pos2> = demo.spots.iter().enumerate().map(|(i, spot)| to_screen(i, *spot)).collect();
    let pins: Vec<(ThreadId, Pos2, &str)> = demo.threads.iter().zip(&points).map(|(t, p)| (t.id, *p, t.messages[0].author.as_str())).collect();
    let out = CommentPins::new("pins", &pins, &labels.me).open(demo.open).show(ui);
    if let Some(id) = out.opened {
        demo.open = Some(id);
        demo.last = Some(format!("Opened({id:?})"));
    }
    if let Some(moved) = &out.moved {
        demo.last = Some(format!("{moved:?}"));
        if let Some(value) = moved.value()
            && let Some(i) = demo.threads.iter().position(|t| t.id == value.thread)
        {
            let at = value.to - nudge(i);
            demo.spots[i] = pos2((at.x - canvas.min.x) / canvas.width(), (at.y - canvas.min.y) / canvas.height());
        }
    }
    if let Some(open) = demo.open.and_then(|id| demo.threads.iter().position(|t| t.id == id)) {
        let anchor = ScreenRect::from_min_size(pos2(points[open].x, points[open].y - style.pin), Vec2::splat(style.pin));
        let people = vec![labels.ana.clone(), labels.bo.clone(), labels.me.clone()];
        let out = ThreadPopover::new("popover", &demo.threads[open], anchor, &labels.me, &people, &mut demo.draft).show(ui);
        for edit in &out.edits {
            demo.last = Some(format!("{edit:?}"));
            apply_comment(&mut demo.threads, &labels.me, edit);
        }
        if out.closed {
            demo.open = None;
        }
    }
    last_event(ui, demo.last.as_deref());
}

/// A page's size in page units.
const PAGE: (f64, f64) = (480.0, 320.0);

struct AnnotateDemo {
    tool: AnnotationTool,
    annotations: Vec<Annotation>,
    selected: Vec<AnnotationId>,
    last: Option<String>,
}

thread_local! {
    /// The annotation tools of the story instances: the tool holds a closure slot that is not
    /// `Send`, so it cannot live in egui memory, and its pending ink strokes must survive passes.
    static TOOLS: RefCell<HashMap<Id, AnnotateTool>> = RefCell::new(HashMap::new());
}

fn annotation(id: u64, kind: AnnotationKind, rect: Rect, color: [u8; 3]) -> Annotation {
    Annotation {
        id: AnnotationId(id),
        page: 0,
        kind,
        rect,
        points: Vec::new(),
        strokes: Vec::new(),
        color,
        opacity: 1.0,
        stroke_width: 2.0,
        text: String::new(),
        locked: false,
    }
}

/// A highlight over a line of text, a rectangle and a wavy ink stroke, in the tokens' colours.
fn sample_annotations(tokens: &UiTokens) -> Vec<Annotation> {
    let band = Rect::new(60.0, 50.0, 260.0, 74.0);
    let mut highlight = annotation(1, AnnotationKind::Highlight, band, rgb(tokens.color.status.warn.solid.to_egui()));
    highlight.opacity = 0.4;
    highlight.points = vec![Point::new(band.x0, band.y0), Point::new(band.x1, band.y0), Point::new(band.x0, band.y1), Point::new(band.x1, band.y1)];
    let rectangle = annotation(2, AnnotationKind::Rectangle, Rect::new(260.0, 110.0, 400.0, 200.0), rgb(tokens.color.status.bad.solid.to_egui()));
    let mut ink = annotation(3, AnnotationKind::Ink, Rect::new(70.0, 190.0, 210.0, 270.0), rgb(tokens.color.accent.solid.to_egui()));
    ink.strokes = vec![
        (0..=14)
            .map(|i| {
                let t = f64::from(i) / 14.0;
                Point::new(70.0 + 140.0 * t, 230.0 + 30.0 * (t * std::f64::consts::TAU).sin())
            })
            .collect(),
    ];
    vec![highlight, rectangle, ink]
}

fn apply_annotation(demo: &mut AnnotateDemo, edit: &EditEvent<AnnotationEdit>) {
    let done = matches!(edit, EditEvent::Commit { .. } | EditEvent::Set { .. });
    let Some(value) = edit.value().filter(|_| done) else { return };
    let AnnotateDemo { annotations, selected, .. } = demo;
    match value {
        AnnotationEdit::Create(new) => {
            let id = AnnotationId(next_id(annotations.iter().map(|a| a.id.0)));
            annotations.push(Annotation { id, ..new.clone() });
            *selected = vec![id];
        }
        AnnotationEdit::Move { ids, delta } => {
            for a in annotations.iter_mut().filter(|a| ids.contains(&a.id)) {
                a.rect = a.rect + *delta;
                for p in a.points.iter_mut().chain(a.strokes.iter_mut().flatten()) {
                    *p += *delta;
                }
            }
        }
        AnnotationEdit::Resize { id, rect } => {
            if let Some(a) = annotations.iter_mut().find(|a| a.id == *id) {
                a.rect = *rect;
            }
        }
        AnnotationEdit::Reshape { id, points } => {
            if let Some(a) = annotations.iter_mut().find(|a| a.id == *id) {
                a.points.clone_from(points);
            }
        }
        AnnotationEdit::Delete(ids) => {
            annotations.retain(|a| !ids.contains(&a.id));
            selected.retain(|id| !ids.contains(id));
        }
        AnnotationEdit::Erase { id, strokes } => {
            if let Some(a) = annotations.iter_mut().find(|a| a.id == *id) {
                let mut at = 0;
                a.strokes.retain(|_| {
                    at += 1;
                    !strokes.contains(&(at - 1))
                });
            }
        }
        AnnotationEdit::Selection(change) => selected.clone_from(&change.selection.items),
        AnnotationEdit::Open(_) => {}
    }
}

// new: the toolbar with the Draw flyout open over a page with a highlight, a rectangle and ink,
// the rectangle selected with its handles.
fn annotate(ui: &mut Ui, _: &mut NoArgs) {
    let tokens = Tokens::of(ui);
    let demo = shared(ui, "annotate", || AnnotateDemo {
        tool: AnnotationTool::Select,
        annotations: sample_annotations(&tokens),
        selected: vec![AnnotationId(2)],
        last: None,
    });
    let mut demo = lock(&demo);
    let demo = &mut *demo;
    let canvas_id = CanvasId::new(ui, "annotations");
    let key = canvas_id.id();
    let mut tool = TOOLS.with(|tools| tools.borrow_mut().remove(&key)).unwrap_or_else(|| AnnotateTool::new(demo.tool, 0));
    tool.tool = demo.tool;
    AnnotationToolbar::new("toolbar", &mut tool).show(ui);
    demo.tool = tool.tool;

    let width = ui.available_width();
    let (area, _) = ui.allocate_exact_size(vec2(width, width * CANVAS_ASPECT), Sense::hover());
    ui.painter().rect_filled(area, 0.0, tokens.color.surface.canvas.to_egui());
    let zoom = f64::from(area.width()) / PAGE.0;
    let zoom = zoom.min(f64::from(area.height()) / PAGE.1);
    let view = CanvasView::new(area, Point::new(PAGE.0 / 2.0, PAGE.1 / 2.0), zoom);
    let page = ScreenRect::from_min_max(view.to_screen(Point::ORIGIN), view.to_screen(Point::new(PAGE.0, PAGE.1)));
    ui.painter().rect_filled(page, 0.0, tokens.color.surface.raised.to_egui());

    let canvas = canvas_id.interact(ui, area, Sense::click_and_drag() | Sense::FOCUSABLE);
    tool.annotations.clone_from(&demo.annotations);
    tool.selected.clone_from(&demo.selected);
    let _ = ToolHost::new(canvas_id).show(ui, &view, &canvas, &mut tool);
    for edit in tool.take_edits() {
        demo.last = Some(format!("{edit:?}"));
        apply_annotation(demo, &edit);
    }
    TOOLS.with(|tools| tools.borrow_mut().insert(key, tool));
    AnnotationOverlay::new(&demo.annotations, &demo.selected).paint(&ui.painter_at(area), &Tokens::of(ui), |p| view.to_screen(p));
    last_event(ui, demo.last.as_deref());
}

#[derive(StoryArgs, Clone, Debug, PartialEq)]
struct PadArgs {
    open: bool,
}

/// A stroke across the pad from `from` to `to` (fractions of its width), swinging by `lift` of
/// its height.
fn stroke(from: f32, to: f32, lift: f32) -> Vec<[f32; 2]> {
    const STEPS: u8 = 16;
    (0..=STEPS)
        .map(|i| {
            let t = f32::from(i) / f32::from(STEPS);
            [from + (to - from) * t, 0.5 - lift * (t * std::f32::consts::TAU).sin()]
        })
        .collect()
}

// new: the pad on the Draw tab with two strokes.
fn signature_pad(ui: &mut Ui, args: &mut PadArgs) {
    let labels = ReviewStoryLabels::of(ui);
    drive(
        ui,
        "signature-pad",
        &mut args.open,
        (),
        || {
            let draft = SignatureDraft {
                tab: SignatureTab::Draw,
                strokes: vec![stroke(0.1, 0.6, 0.25), stroke(0.6, 0.9, 0.1)],
                ..SignatureDraft::new(false, &labels.me)
            };
            SignaturePadDialog::new(draft)
        },
        |_| {},
        |SignaturePadResult { signature, initials }| format!("{signature:?}, initials: {initials}"),
    );
}

struct VersionsDemo {
    versions: Vec<Version>,
    branches: Vec<Branch>,
    previewing: Option<VersionId>,
    last: Option<String>,
}

// new: two branches, Current version, three named versions and an autosave group.
fn versions(ui: &mut Ui, _: &mut NoArgs) {
    let labels = ReviewStoryLabels::of(ui);
    let demo = shared(ui, "versions", || {
        let named = |id: u64, title: &str, description: &str, day: i64, author: &str| Version {
            id: VersionId(id),
            title: title.into(),
            description: description.into(),
            created: START + day * DAY,
            author: author.into(),
            autosave: false,
        };
        let auto = |id: u64, hours: i64| Version {
            id: VersionId(id),
            title: String::new(),
            description: String::new(),
            created: START + 20 * DAY + hours * HOUR,
            author: labels.me.clone(),
            autosave: true,
        };
        let branch = |id: u64, name: &str, state: BranchState, day: i64, author: &str| Branch {
            id: BranchId(id),
            name: name.into(),
            state,
            created: START + day * DAY,
            author: author.into(),
        };
        VersionsDemo {
            versions: vec![
                named(1, &labels.version_first, "", 2, &labels.me),
                named(2, &labels.version_review, &labels.version_description, 9, &labels.ana),
                named(3, &labels.version_final, "", 16, &labels.me),
                auto(4, 1),
                auto(5, 2),
                auto(6, 3),
            ],
            branches: vec![
                branch(1, &labels.branch_dark, BranchState::Active, 12, &labels.ana),
                branch(2, &labels.branch_print, BranchState::Merged, 6, &labels.bo),
            ],
            previewing: None,
            last: None,
        }
    });
    let mut demo = lock(&demo);
    let demo = &mut *demo;
    let out = VersionHistoryPanel::new("versions", &demo.versions, &demo.branches).previewing(demo.previewing).show(ui);
    if let Some(action) = out.action {
        demo.last = Some(format!("{action:?}"));
        match action {
            VersionAction::Save { title, description } => {
                let id = VersionId(next_id(demo.versions.iter().map(|v| v.id.0)));
                let created = demo.versions.iter().map(|v| v.created).max().unwrap_or(START) + DAY;
                demo.versions.push(Version { id, title, description, created, author: labels.me.clone(), autosave: false });
            }
            VersionAction::Preview(id) => demo.previewing = Some(id),
            VersionAction::Current => demo.previewing = None,
            VersionAction::Rename { id, title } => {
                if let Some(v) = demo.versions.iter_mut().find(|v| v.id == id) {
                    v.title = title;
                }
            }
            VersionAction::Duplicate(id) => {
                if let Some(v) = demo.versions.iter().find(|v| v.id == id).cloned() {
                    let new = VersionId(next_id(demo.versions.iter().map(|v| v.id.0)));
                    demo.versions.push(Version { id: new, created: v.created + 1, ..v });
                }
            }
            VersionAction::Delete(id) => demo.versions.retain(|v| v.id != id),
            VersionAction::CreateBranch { name } => {
                let id = BranchId(next_id(demo.branches.iter().map(|b| b.id.0)));
                demo.branches.push(Branch { id, name, state: BranchState::Active, created: START + 30 * DAY, author: labels.me.clone() });
            }
            VersionAction::MergeBranch(id) => set_branch(&mut demo.branches, id, BranchState::Merged),
            VersionAction::ArchiveBranch(id) => set_branch(&mut demo.branches, id, BranchState::Archived),
            VersionAction::Restore(_) | VersionAction::OpenBranch(_) => {}
        }
    }
    last_event(ui, demo.last.as_deref());
}

fn set_branch(branches: &mut [Branch], id: BranchId, state: BranchState) {
    if let Some(branch) = branches.iter_mut().find(|b| b.id == id) {
        branch.state = state;
    }
}

struct RevisionsDemo {
    revisions: Vec<Revision>,
    last: Option<String>,
}

fn sample_revisions(labels: &ReviewStoryLabels) -> Vec<Revision> {
    vec![
        Revision { kind: RevisionKind::Inserted, heading: labels.revision_inserted.clone(), details: vec![labels.revision_inserted_detail.clone()] },
        Revision {
            kind: RevisionKind::Changed,
            heading: labels.revision_changed.clone(),
            details: vec![labels.revision_changed_title.clone(), labels.revision_changed_chart.clone()],
        },
        Revision { kind: RevisionKind::Deleted, heading: labels.revision_deleted.clone(), details: vec![labels.revision_deleted_detail.clone()] },
    ]
}

// new: inserted, changed and deleted slides.
fn revisions(ui: &mut Ui, _: &mut NoArgs) {
    let labels = ReviewStoryLabels::of(ui);
    let demo = shared(ui, "revisions", || RevisionsDemo { revisions: sample_revisions(&labels), last: None });
    let mut demo = lock(&demo);
    let demo = &mut *demo;
    let out = RevisionsPanel::new("revisions", &labels.compared_with, &demo.revisions).show(ui);
    if let Some(action) = out.action {
        demo.last = Some(format!("{action:?}"));
        match action {
            RevisionAction::Accept(i) | RevisionAction::Reject(i) => {
                if i < demo.revisions.len() {
                    demo.revisions.remove(i);
                }
            }
            RevisionAction::AcceptAll | RevisionAction::RejectAll => demo.revisions.clear(),
            RevisionAction::EndReview => demo.revisions = sample_revisions(&labels),
            RevisionAction::Go(_) => {}
        }
    }
    last_event(ui, demo.last.as_deref());
}

#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod tests {
    use craft_ui::story::Args;
    use craft_ui::testing::symbols;
    use craft_ui::theme::{Base, install};
    use egui_kittest::Harness;

    // every story draws its widget over its sample data without panicking.
    #[test]
    fn every_story_draws() {
        for story in super::stories() {
            let mut harness = Harness::<Args>::builder()
                .with_size([720.0, 520.0])
                .with_context_setup(|ctx| {
                    install(ctx, &Base::suite_default());
                    symbols::install(ctx);
                })
                .build_ui_state(
                    move |ui, args| {
                        // The reaction emoji have no face in the test fonts.
                        ui.ctx().set_missing_glyph_policy(egui::epaint::text::MissingGlyphPolicy::Tofu);
                        (story.show)(ui, args);
                    },
                    story.defaults(),
                );
            harness.run_steps(4);
        }
    }
}
