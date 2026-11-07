pub struct Plain {
    common: Common,
}

impl Component for Plain {}

register_contract!(Plain);

pub struct Manual {
    common: Common,
}

impl Component for Manual {}
