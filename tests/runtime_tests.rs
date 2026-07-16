use rustscript_gpui_notepad::rss_gpui::model::UiEvent;
use rustscript_gpui_notepad::rss_gpui::runtime::RssGpuiRuntime;

const SCRIPT: &str = r#"
use ui;

ui::window("Demo", 640, 480);
ui::column_begin("root");
ui::button("format", "Format");
ui::bind_click("format", "format-note");
ui::label("status", ui::get_value("status"));
ui::column_end();

if ui::event_name() == "format-note" {
    ui::set_value("status", "formatted");
}
ui::finish();
"#;

#[test]
fn script_declares_button_binding_and_updates_state_for_its_event() {
    let mut runtime = RssGpuiRuntime::from_source(SCRIPT, vec![]).expect("script should load");

    let result = runtime
        .dispatch(UiEvent::Click("format".into()))
        .expect("scripted click should run");

    assert_eq!(result.tree.click_event("format"), Some("format-note"));
    assert_eq!(result.state.value("status"), Some("formatted"));
}
