//! The live preview of an adjustment: preview keys, damage between keys, covered-area tracking, a
//! settle timer ([`LivePreview`]), the zoom proxy factor ([`ProxyPlan`]) and LightCraft's learned
//! draft scale ([`DraftScale`]).
//!
//! The library only keys and tracks damage; the host builds the preview document, renders it and
//! hands the damage to [`RasterDisplay::damage`](craft_ui::canvas::RasterDisplay::damage). One
//! policy serves an Image › Adjustments dialog and a Properties-panel drag (one [`LivePreview`]
//! each, in its own [`PreviewLane`]).
//!
//! ```ignore
//! // Once per pass while a dialog or a Properties drag is open.
//! live.begin(PreviewSession::new(doc_id, revision, layer_id), region);
//! let key = if preview_on { live.key(settings_hash(&[kind.as_bytes(), &params]), now) } else { PreviewKey::NONE };
//! if let Some(damage) = live.switch(shown_key, key, visible) { RasterDisplay::damage(ctx, canvas_id, damage); }
//! if let Some(damage) = live.uncovered(key, visible) { RasterDisplay::damage(ctx, canvas_id, damage); }
//! shown_key = key;
//! if live.settling(now, 0.2) { /* the Navigator waits */ }
//! ```
//!
//! The settled full-resolution render is the host's job on a
//! [`JobQueue<PreviewKey, R>`](craft_host::queue::JobQueue) at
//! [`Priority::VISIBLE`](craft_host::queue::Priority::VISIBLE): when a new key arrives the host
//! drops the previous key's job ([`JobQueue::reprioritize`](craft_host::queue::JobQueue::reprioritize)
//! returning `None` for it), which sets its [`Cancel`](craft_host::queue::Cancel), and a cancelled
//! result is never delivered. [`PreviewKey`] is `Copy + Eq + Hash + Send + 'static`, the queue's
//! key bounds (checked below).
//!
//! [`PreviewKey`] names one preview's settings for damage tracking. It is not
//! [`PreviewTicket`](craft_ui::dialog::full_window::PreviewTicket), which numbers the requests of a
//! full-window editor so older results are dropped.

mod damage;
mod draft;
mod key;
mod proxy;
mod session;
pub(crate) mod stories;
#[cfg(test)]
mod tests;

pub use draft::DraftScale;
pub use key::{PreviewKey, PreviewLane, PreviewSession, settings_hash};
pub use proxy::ProxyPlan;
pub use session::LivePreview;

// `PreviewKey` is a `JobQueue` key, as the module doc says.
const _: () = {
    const fn queue_key<K: Copy + Eq + std::hash::Hash + Send + 'static>() {}
    queue_key::<PreviewKey>();
};
