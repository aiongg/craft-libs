//! The read record: which components read which binding in the running pass, in which run kind, and what their
//! measure took. An edit reruns the pass only when this record shows an earlier reader its change would make stale.

use crate::runtime::egui::pass;
use crate::runtime::frame::{PerViewport, Stamped};
use craft_behavior::Proposal;
use egui::{Context, Id, Vec2};
use std::any::TypeId;

/// The identity of a binding's value across the components of one pass: the bound place, or where a kept value
/// lives. A given value has none.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum BindKey {
    /// A write-through place: its address and the type stored there.
    Place(usize, TypeId),
    /// Session memory under this component id.
    Kept(Id),
    /// The saved tier under the key with this hash.
    Saved(Id),
}

/// One measure a reader made from the value it read: what it was offered and the size it took.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Sample {
    pub proposal: Proposal,
    pub room: Option<f32>,
    pub took: Vec2,
}

#[derive(Debug)]
struct Read {
    key: BindKey,
    reader: Id,
    measuring: bool,
    /// The innermost measuring run the read happened in, an index into `Reads::bodies`.
    body: Option<usize>,
    samples: Vec<Sample>,
}

#[derive(Default)]
struct Reads {
    reads: Vec<Read>,
    /// The measuring runs of the pass: the measured component's id and the run it ran inside.
    bodies: Vec<(Id, Option<usize>)>,
    /// The measuring runs in progress, innermost last.
    stack: Vec<usize>,
}

fn with<R>(ctx: &Context, f: impl FnOnce(&mut Reads) -> R) -> R {
    let viewport = pass::current_viewport(ctx);
    let pass_nr = pass::pass_nr(ctx, viewport);
    PerViewport::<Stamped<Reads>>::with_pass(ctx, viewport, pass_nr, |reads, _| *reads = Reads::default(), f)
}

/// Runs `f` as the measuring run of the component `id`: the reads inside it are attributed to it.
pub(crate) fn measuring<R>(ctx: &Context, id: Id, f: impl FnOnce() -> R) -> R {
    let depth = with(ctx, |reads| {
        let at = reads.bodies.len();
        reads.bodies.push((id, reads.stack.last().copied()));
        reads.stack.push(at);
        reads.stack.len() - 1
    });
    let out = f();
    with(ctx, |reads| reads.stack.truncate(depth));
    out
}

/// Notes that `reader` read the binding `key`, in a measuring run when `measuring`, and the measures it made from it.
pub(crate) fn note(ctx: &Context, key: BindKey, reader: Id, measuring: bool, samples: &[Sample]) {
    with(ctx, |reads| {
        let body = reads.stack.last().copied();
        reads.reads.push(Read { key, reader, measuring, body, samples: samples.to_vec() });
    });
}

/// The reads of a binding made so far in this pass, as an edit of it by one component sees them.
#[derive(Default)]
pub(crate) struct EarlierReads {
    /// Another component read it, in a real or a measuring run.
    pub other: bool,
    /// The editor read it in a measuring run.
    pub measured: bool,
    /// The measures the editor made from the value it read, each once.
    pub samples: Vec<Sample>,
}

/// The reads of `key` made so far in this pass, as `editor`'s edit of it sees them.
pub(crate) fn earlier(ctx: &Context, key: BindKey, editor: Id) -> EarlierReads {
    with(ctx, |reads| {
        let mut earlier = EarlierReads::default();
        for read in reads.reads.iter().filter(|read| read.key == key) {
            if read.reader != editor {
                earlier.other = true;
                continue;
            }
            earlier.measured |= read.measuring;
            for sample in &read.samples {
                if !earlier.samples.contains(sample) {
                    earlier.samples.push(*sample);
                }
            }
        }
        earlier
    })
}

/// The bindings read inside the measuring runs of `body` in this pass, nested measuring runs included.
#[expect(dead_code, reason = "the measure reuse of a body reads it")]
pub(crate) fn measured(ctx: &Context, body: Id) -> Vec<BindKey> {
    with(ctx, |reads| {
        let inside = |mut at: Option<usize>| {
            while let Some(index) = at {
                let (id, parent) = reads.bodies[index];
                if id == body {
                    return true;
                }
                at = parent;
            }
            false
        };
        let mut keys = Vec::new();
        for read in reads.reads.iter().filter(|read| read.measuring && inside(read.body)) {
            if !keys.contains(&read.key) {
                keys.push(read.key);
            }
        }
        keys
    })
}
