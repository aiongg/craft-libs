//! `arrange`: a custom placement, mirrored through the env's direction.

use craft_behavior::Arrangement;
use craft_ui::layout::{Placed, arrange};
use craft_ui::runtime::env::{Direction, with_env};
use craft_ui::testing::layout::Block;
use craft_ui::testing::{Frames, trial};
use egui::{Rect, Vec2};

/// Children on a diagonal from the start edge.
struct Stairs {
    mirrors: bool,
}

impl Arrangement for Stairs {
    fn place(&self, room: Rect, sizes: &[Vec2], out: &mut Vec<Rect>) -> Vec2 {
        let mut taken = Vec2::ZERO;
        for (i, size) in sizes.iter().enumerate() {
            let offset = Vec2::new(30.0 * i as f32, 15.0 * i as f32);
            out.push(Rect::from_min_size(room.min + offset, *size));
            taken = taken.max(offset + *size);
        }
        taken
    }

    fn mirrors(&self) -> bool {
        self.mirrors
    }
}

/// The rects of two children and the room, under `direction`.
fn stairs(direction: Direction, mirrors: bool) -> (Rect, Vec<Rect>) {
    let mut frames = Frames::new();
    let mut out = (Rect::NOTHING, Vec::new());
    frames.frame(vec![], |ui| {
        let room = ui.available_rect_before_wrap();
        let mut children = [Placed::new("a", Block(Vec2::new(40.0, 10.0))), Placed::new("b", Block(Vec2::new(50.0, 10.0)))];
        with_env(
            ui,
            |o| o.direction = Some(direction),
            |ui| {
                arrange(ui, &Stairs { mirrors }, &mut children);
            },
        );
        let rects = children.iter().filter_map(|child| child.response().map(|response| response.rect)).collect();
        out = (room, rects);
    });
    out
}

#[trial]
fn arrange_mirrors_a_placement_under_rtl_unless_it_opts_out() {
    let (room, ltr) = stairs(Direction::Ltr, true);
    assert_eq!(ltr.len(), 2, "each child is shown once");
    assert_eq!(ltr[0].min.x, room.min.x);
    assert_eq!(ltr[1].min.x, room.min.x + 30.0);
    let (room, rtl) = stairs(Direction::Rtl, true);
    assert_eq!(rtl[0].max.x, room.max.x, "the first child starts at the right edge");
    assert_eq!(rtl[1].max.x, room.max.x - 30.0);
    assert_eq!(rtl[1].min.y, ltr[1].min.y, "y is not mirrored");
    let (room, fixed) = stairs(Direction::Rtl, false);
    assert_eq!(fixed[0].min.x, room.min.x, "a placement that does not mirror is left as it places");
}
