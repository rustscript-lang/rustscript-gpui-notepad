use rustscript_gpui_notepad::rss_gpui::model::UiEvent;
use rustscript_gpui_notepad::rss_gpui::runtime::RssGpuiRuntime;

const SCRIPT: &str = r#"
use ui;

ui::window("Demo", 640, 480);
ui::column_begin("root");
ui::button("format", "Format", || ui::set_value("status", "formatted"));
ui::label("status", ui::get_value("status"));
ui::column_end();
ui::finish();
"#;

#[test]
fn script_keeps_anonymous_button_callback_and_updates_state() {
    let mut runtime = RssGpuiRuntime::from_source(SCRIPT, vec![]).expect("script should load");

    let result = runtime
        .dispatch(UiEvent::Click("format".into()))
        .expect("scripted click should run");

    assert!(result.tree.click_callback("format").is_some());
    assert_eq!(result.state.value("status"), Some("formatted"));
}
