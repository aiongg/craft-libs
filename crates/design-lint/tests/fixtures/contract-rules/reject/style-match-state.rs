fn alpha(state: State) -> f32 {
    match state {
        State::Rest => 0.0,
        State::Hover | State::Pressed => 0.5,
        _ => 1.0,
    }
}
