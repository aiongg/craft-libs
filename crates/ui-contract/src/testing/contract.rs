//! Helpers the contract trials use: responses, a raw allocation and the parts-declaration check.

/// An edit response with no events, for a trial component that has no `cx.edit_response()` to call.
pub fn edit_response<T>(response: crate::contract::Response) -> crate::contract::EditResponse<T> {
    crate::contract::EditResponse::new(response, craft_behavior::EditEvents::new(), crate::contract::Validity::Ok)
}

/// Whether `response` reports a click, by pointer or by keyboard activation.
pub fn clicked(response: &crate::contract::Response) -> bool {
    crate::contract::clicked(response)
}

/// Allocates a block of `size` in `ui` that senses nothing.
pub fn allocate(ui: &mut crate::contract::Ui, size: craft_ui_rt::Vec2) {
    ui.allocate(size);
}

thread_local! {
    static PARTS_CHECK: std::cell::RefCell<Option<Vec<&'static str>>> = const { std::cell::RefCell::new(None) };
}

/// Run a story with the parts declaration check enabled, and return any offending component types.
pub fn check_parts_declarations(run: impl FnOnce()) -> Vec<&'static str> {
    struct Reset(Option<Vec<&'static str>>);
    impl Drop for Reset {
        fn drop(&mut self) {
            PARTS_CHECK.with(|check| *check.borrow_mut() = self.0.take());
        }
    }
    let previous = PARTS_CHECK.with(|check| check.replace(Some(Vec::new())));
    let _reset = Reset(previous);
    run();
    PARTS_CHECK.with(|check| check.borrow_mut().take().unwrap_or_default())
}

pub(crate) fn checking_parts() -> bool {
    PARTS_CHECK.with(|check| check.borrow().is_some())
}

pub(crate) fn bad_parts(component: &'static str) {
    PARTS_CHECK.with(|check| {
        if let Some(bad) = check.borrow_mut().as_mut() {
            bad.push(component);
        }
    });
}
