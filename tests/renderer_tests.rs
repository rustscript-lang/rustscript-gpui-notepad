use rustscript_gpui_notepad::rss_gpui::renderer::RenderPlan;
use rustscript_gpui_notepad::rss_gpui::runtime::RssGpuiRuntime;

#[test]
fn render_plan_keeps_script_button_callback() {
    let source = r#"
        use ui;
        ui::window("Demo", 640, 480);
        ui::button("save", "Save", || ui::set_value("status", "saved"));
        ui::finish();
    "#;
    let mut runtime = RssGpuiRuntime::from_source(source, vec![]).expect("script should load");
    let tree = runtime.render().expect("script should render").tree;

    let plan = RenderPlan::from_tree(&tree);

    assert!(plan.button("save").is_some());
}
