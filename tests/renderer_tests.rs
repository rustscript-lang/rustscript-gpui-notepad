use rustscript_gpui_notepad::rss_gpui::builder::UiBuilder;
use rustscript_gpui_notepad::rss_gpui::renderer::RenderPlan;

#[test]
fn render_plan_keeps_script_button_event_name() {
    let mut builder = UiBuilder::new();
    builder
        .window("Demo", 640, 480)
        .expect("window should build");
    builder.button("save", "Save").expect("button should build");
    builder
        .bind_click("save", "save-note")
        .expect("binding should build");
    let tree = builder.finish().expect("tree should finish");

    let plan = RenderPlan::from_tree(&tree);

    assert_eq!(
        plan.button("save").expect("button exists").event_name,
        "save-note"
    );
}
