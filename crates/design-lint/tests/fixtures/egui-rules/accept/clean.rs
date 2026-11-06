use crate::widgets::{Button, Order};

#[non_exhaustive]
pub enum Mode {
    A,
    #[non_exhaustive]
    Moved { x: f32 },
}

pub enum Politeness {
    Polite,
}

#[non_exhaustive]
pub struct Newtype(pub f32);

pub struct Opaque(f32);

pub(crate) struct Hidden {
    pub width: f32,
}

#[doc(hidden)]
pub enum Internal {
    A,
}

mod private {
    pub enum Local {
        A,
    }
}

#[cfg(target_os_is_not_here)]
fn gated() {}

#[cfg(test)]
mod tests {
    fn helper() {}
}

fn f(cx: &mut Cx, ui: &mut Ui, tokens: &Tokens) {
    let _ = Button::new("x");
    let _ = Order::Front;
    cx.clipboard().copy("x");
    ui.add_space(tokens.gap);
    ui.add_space(0.0);
    ui.label("x");
    let _ = cx.press();
}
