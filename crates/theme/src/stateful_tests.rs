use crate::parts::{State, States};
use crate::stateful::{FlagMask, Flags, Resolve, StateMix, Stateful};

fn look() -> Stateful<f32> {
    Stateful::new(0.0)
        .off(States::new(0.0, 1.0, 2.0, 9.0))
        .on(States::new(10.0, 11.0, 12.0, 19.0))
        .when(FlagMask::new(Flags::INVALID), 100.0)
        .when(FlagMask::new(Flags::FOCUSED), 200.0)
}

#[test]
fn disabled_wins_over_every_flag_look() {
    let all = Flags::INVALID | Flags::FOCUSED | Flags::CHECKED;
    assert_eq!(look().resolve(State::Disabled, all, StateMix::new(1.0, 1.0, 0.0)), 9.0);
    assert_eq!(look().resolve(State::Disabled, all, StateMix::new(0.0, 0.0, 1.0)), 19.0, "a disabled checked control keeps its on look");
}

#[test]
fn flag_looks_apply_in_list_order_and_a_mask_needs_all_its_flags() {
    let look = look();
    let at = |flags| look.resolve(State::Rest, flags, StateMix::REST);
    assert_eq!(at(Flags::INVALID | Flags::FOCUSED), 100.0, "the first listed look wins");
    assert_eq!(at(Flags::FOCUSED), 200.0);
    let both = Stateful::new(0.0).when(FlagMask::new(Flags::FOCUSED | Flags::FOCUS_VISIBLE), 5.0);
    assert_eq!(both.resolve(State::Rest, Flags::FOCUSED, StateMix::REST), 0.0);
    assert_eq!(both.resolve(State::Rest, Flags::FOCUSED | Flags::FOCUS_VISIBLE, StateMix::REST), 5.0);
}

#[test]
fn hover_press_and_on_blend_in_that_order() {
    let look = look();
    let at = |t| look.resolve(State::Hover, Flags::empty(), t);
    assert_eq!(at(StateMix::new(0.5, 0.0, 0.0)), 0.5);
    assert_eq!(at(StateMix::new(1.0, 1.0, 0.0)), 2.0, "pressed replaces hover");
    assert_eq!(at(StateMix::new(0.0, 0.0, 1.0)), 10.0, "on replaces off");
    assert_eq!(at(StateMix::new(0.0, 0.0, 0.5)), 5.0, "a toggle mid-animation is halfway");
}
