#[non_exhaustive]
pub struct Request {
    pub action: accesskit::Action,
    pub data: Option<accesskit::ActionData>,
}

pub fn click() -> accesskit::Action {
    accesskit::Action::Click
}
