use rustscript_gpui_notepad::rss_gpui::model::UiEvent;
use rustscript_gpui_notepad::rss_gpui::runtime::RssGpuiRuntime;

const SCRIPT: &str = r#"
use ui;

ui::window("Demo", 640, 480);
ui::column_begin("root");
ui::button("format", "Format");
ui::on_click("format", "format-note");
ui::button("save", "Save");
ui::on_click("save", "save-note");
ui::label("status", ui::get_value("status"));
ui::column_end();

ui::on("format-note", || {
    ui::set_value("status", "formatted");
});

ui::on("save-note", || {
    ui::set_value("status", "saved");
});

ui::finish();
"#;

#[test]
fn dispatch_sugar_routes_each_named_handler_without_script_side_event_chain() {
    let mut runtime = RssGpuiRuntime::from_source(SCRIPT, vec![]).expect("script should load");

    let formatted = runtime
        .dispatch(UiEvent::Click("format".into()))
        .expect("format handler should run");
    assert_eq!(formatted.state.value("status"), Some("formatted"));

    let saved = runtime
        .dispatch(UiEvent::Click("save".into()))
        .expect("save handler should run");
    assert_eq!(saved.state.value("status"), Some("saved"));
}
