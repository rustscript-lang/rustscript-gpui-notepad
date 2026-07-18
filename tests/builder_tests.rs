use rustscript_gpui_notepad::rss_gpui::builder::UiBuilder;
use vm::Value;

#[test]
fn builder_rejects_button_without_anonymous_callback() {
    let mut builder = UiBuilder::new();
    builder
        .window("Demo", 640, 480)
        .expect("window should build");

    let error = builder
        .button("save", "Save", Value::Null)
        .expect_err("non-callable button callback should fail");

    assert_eq!(error.to_string(), "ui::button callback must be callable");
}
