//! What a harness painted, read from the display lists its recording renderer kept: the items of
//! the latest frame in paint order, each with the offset that puts it in window space and the clip
//! it paints under.

use craft_ui_rt::record::{FixedLayout, RecordedFrame, RecordingRenderer};
use craft_ui_rt::{Item, Rect, Rgba, Vec2};

use crate::contract::rt::harness::Harness;

/// One item of a frame's display lists.
#[derive(Clone, Debug)]
pub struct Painted {
    /// The item, in its node's coordinates.
    pub item: Item,
    /// Where the node's origin is in the window.
    pub origin: Vec2,
    /// The clip the item paints under, in window space.
    pub clip: Rect,
}

impl Painted {
    /// What the item can paint, in window space: geometry, stroke and anti-aliasing.
    pub fn bounds(&self) -> Option<Rect> {
        self.item.ink().map(|ink| ink.translate(self.origin))
    }

    /// Half the stroke width of the item's outline: how far it reaches past its geometry.
    pub fn overshoot(&self) -> f32 {
        match &self.item {
            Item::Rect { stroke, .. } | Item::Circle { stroke, .. } | Item::Path { stroke, .. } => stroke.map_or(0.0, |stroke| stroke.width / 2.0),
            Item::Line { stroke, .. } => stroke.width / 2.0,
            _ => 0.0,
        }
    }
}

/// A text run a frame painted, in window space.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintedText {
    /// The characters.
    pub text: String,
    /// The run's layout box.
    pub bounds: Rect,
    /// The clip it paints under.
    pub clip: Rect,
    /// The colour it paints in.
    pub color: Rgba,
    /// The text engine cut it short with an ellipsis.
    pub elided: bool,
}

/// The items of `frame` in paint order. Clips the lists push narrow the clip of what follows.
pub fn items_of(frame: &RecordedFrame) -> Vec<Painted> {
    let mut out = Vec::new();
    for draw in &frame.draws {
        let mut clips = vec![draw.clip];
        for item in &draw.list.items {
            let clip = clips.last().copied().unwrap_or(draw.clip);
            match item {
                Item::PushClip(next) => clips.push(next.rect.translate(draw.origin).intersect(clip)),
                Item::PopClip => {
                    clips.pop();
                }
                Item::PushOpacity { .. } | Item::PopOpacity => {}
                item => out.push(Painted { item: item.clone(), origin: draw.origin, clip }),
            }
        }
    }
    out
}

/// The text runs among `items` (the recording renderer's text engine keeps each run's characters).
pub fn texts_of(items: &[Painted]) -> Vec<PaintedText> {
    items
        .iter()
        .filter_map(|painted| match &painted.item {
            Item::Text { layout, origin, size, color, .. } => {
                let run = layout.downcast_ref::<FixedLayout>()?;
                Some(PaintedText {
                    text: run.text.to_string(),
                    bounds: Rect::from_min_size(*origin, *size).translate(painted.origin),
                    clip: painted.clip,
                    color: color.unwrap_or(run.color),
                    elided: run.text.ends_with('\u{2026}'),
                })
            }
            _ => None,
        })
        .collect()
}

impl<S> Harness<S, RecordingRenderer> {
    /// The items the latest frame painted, in paint order. Empty before the first frame.
    pub fn painted(&self) -> Vec<Painted> {
        self.driver.renderer.last_of(self.window()).map(items_of).unwrap_or_default()
    }

    /// The latest frame as the renderer recorded it, taken: later reads see only newer frames.
    pub fn take_paint(&mut self) -> Option<RecordedFrame> {
        let window = self.window();
        let frame = self.driver.renderer.last_of(window).cloned();
        self.driver.renderer.frames.clear();
        frame
    }

    /// One step with no input; what it painted, nested lists flattened.
    pub fn frame_shapes(&mut self) -> Vec<Item> {
        self.step();
        self.painted().into_iter().map(|painted| painted.item).collect()
    }

    /// One step with no input; what it painted, each item with the clip it paints under.
    pub fn frame_clipped_shapes(&mut self) -> Vec<(Rect, Item)> {
        self.step();
        self.painted().into_iter().map(|painted| (painted.clip, painted.item)).collect()
    }

    /// `input` runs (the pointer, the keys), then what the step that took the last input painted:
    /// not the frame after it.
    pub fn frame_shapes_with(&mut self, input: impl FnOnce(&mut Self)) -> Vec<Item> {
        input(self);
        self.painted().into_iter().map(|painted| painted.item).collect()
    }

    /// The text runs the latest frame painted.
    pub fn texts(&self) -> Vec<PaintedText> {
        texts_of(&self.painted())
    }

    /// The visual bounds of the items the latest frame painted, in window space; items with no
    /// ink (clips, empty meshes) have none.
    pub fn shape_bounds(&self) -> Vec<Rect> {
        self.painted().iter().filter_map(Painted::bounds).collect()
    }

    /// Half the stroke width of the item behind each of [`Harness::shape_bounds`].
    pub fn shape_overshoot(&self) -> Vec<f32> {
        self.painted().iter().filter(|painted| painted.bounds().is_some()).map(Painted::overshoot).collect()
    }
}
