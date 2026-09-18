use rustscript_gpui_notepad::rss_gpui::model::UiEvent;
use rustscript_gpui_notepad::rss_gpui::runtime::RssGpuiRuntime;

const SCRIPT: &str = r#"
use ui;

ui::window("Demo", 640, 480);
ui::column_begin("root");
ui::button("format", "Format", || ui::set_value("status", "formatted"));
ui::button("save", "Save", || ui::set_value("status", "saved"));
ui::label("status", ui::get_value("status"));
ui::column_end();
ui::finish();
"#;

#[test]
fn anonymous_button_callbacks_run_without_an_event_dispatcher() {
    let mut runtime = RssGpuiRuntime::from_source(SCRIPT).expect("script should load");

    let formatted = runtime
        .dispatch(UiEvent::Click("format".into()))
        .expect("format callback should run");
    assert_eq!(formatted.state.value("status"), Some("formatted"));

    let saved = runtime
        .dispatch(UiEvent::Click("save".into()))
        .expect("save callback should run");
    assert_eq!(saved.state.value("status"), Some("saved"));
}
