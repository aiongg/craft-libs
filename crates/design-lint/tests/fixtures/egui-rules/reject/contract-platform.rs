#[cfg(target_os = "macos")]
fn mac() {}

fn f(ctx: &Context) {
    if cfg!(target_arch = "wasm32") {}
    ctx.copy_text(text);
    ctx.open_url(url);
}
