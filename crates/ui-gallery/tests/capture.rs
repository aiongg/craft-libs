use craft_ui_gallery::capture::{parse_spec, render, resolve};

fn shot(json: &str) -> Result<craft_ui_gallery::capture::Resolved, String> {
    resolve(&parse_spec(json)?.remove(0))
}

#[test]
fn a_shot_renders_to_a_png_of_the_requested_size() {
    let resolved = shot(
        r#"[{"name": "button", "component": "button", "theme": "craft.artcraft", "mode": "dark", "size": "compact",
             "width": 320, "height": 120, "pixels_per_point": 2, "state": "hover"}]"#,
    )
    .unwrap();
    let image = render(&resolved).unwrap();
    assert_eq!((image.width(), image.height()), (640, 240));
    let path = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("capture-button.png");
    image.save(&path).unwrap();
    assert!(std::fs::metadata(&path).unwrap().len() > 0);
}

#[test]
fn a_mistake_in_the_spec_names_the_valid_values() {
    let error = |json: &str| shot(json).err().expect("the spec is wrong");
    assert!(error(r#"[{"name": "a", "component": "button", "theme": "nope"}]"#).contains("craft.artcraft"));
    assert!(error(r#"[{"name": "a", "component": "nope"}]"#).contains("Button"));
    assert!(error(r#"[{"name": "a", "component": "button", "mode": "dim"}]"#).contains("dark"));
    assert!(error(r#"[{"name": "a", "component": "button", "size": "huge"}]"#).contains("comfortable"));
    assert!(error(r#"[{"name": "a", "component": "button", "args": {"nope": 1}}]"#).contains("args:"));
}

#[test]
fn an_overview_shot_draws_a_level_as_wide_as_asked() {
    let resolved = shot(r#"[{"name": "controls", "level": "controls", "width": 700}]"#).unwrap();
    let image = render(&resolved).unwrap();
    assert_eq!(image.width(), 700);
    assert!(image.height() > 1000, "every control of the level is drawn: {}", image.height());
    let error = shot(r#"[{"name": "a", "level": "nope"}]"#).err().expect("the level is wrong");
    assert!(error.contains("controls"));
}

/// A pixel of a shot on its row 60, `at` pixels from the left.
fn pixel(json: &str, at: u32) -> [u8; 3] {
    let image = render(&shot(json).unwrap()).unwrap();
    let [r, g, b, _] = image.get_pixel(at, 60).0;
    [r, g, b]
}

#[test]
fn a_shot_paints_the_surface_of_its_story_in_the_dock_colour() {
    let artcraft = r#""theme": "craft.artcraft", "mode": "dark", "width": 320, "height": 120"#;
    let on = |component: &str, extra: &str| pixel(&format!(r#"[{{"name": "a", "component": "{component}", {extra} {artcraft}}}]"#), 20);
    // The dock colour surrounds the story's frame; a button sits in a panel body, the rulers on the canvas.
    assert_eq!(pixel(&format!(r#"[{{"name": "a", "component": "button", {artcraft}}}]"#), 4), [0x42, 0x42, 0x42]);
    assert_eq!(on("button", ""), [0x53, 0x53, 0x53]);
    assert_eq!(on("rulers", ""), [0x28, 0x28, 0x28]);
    assert_eq!(on("button", r#""surface": "canvas","#), [0x28, 0x28, 0x28]);
    let error = shot(r#"[{"name": "a", "component": "button", "surface": "floor"}]"#).err().expect("the surface is wrong");
    assert!(error.contains("chrome"));
}
