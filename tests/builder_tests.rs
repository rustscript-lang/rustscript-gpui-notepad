use rustscript_gpui_notepad::rss_gpui::builder::UiBuilder;

#[test]
fn builder_preserves_script_declared_hierarchy_and_click_binding() {
    let mut builder = UiBuilder::new();
    builder
        .window("Demo", 640, 480)
        .expect("window should build");
    builder.column_begin("root").expect("column should build");
    builder.button("save", "Save").expect("button should build");
    builder
        .bind_click("save", "save-note")
        .expect("binding should build");
    builder.column_end().expect("column should close");

    let tree = builder.finish().expect("tree should finish");

    assert_eq!(tree.click_event("save"), Some("save-note"));
    assert_eq!(tree.root.children.len(), 1);
}

#[test]
fn builder_rejects_click_binding_for_unknown_button() {
    let mut builder = UiBuilder::new();
    builder
        .window("Demo", 640, 480)
        .expect("window should build");

    let error = builder
        .bind_click("missing", "save-note")
        .expect_err("unknown button should fail");

    assert!(error.to_string().contains("missing"));
}
