use rustscript_gpui_notepad::notepad_runtime;
use rustscript_gpui_notepad::rss_gpui::model::UiEvent;
use tempfile::tempdir;

#[test]
fn rss_format_button_calls_notepad_host_and_rewrites_body() {
    let directory = tempdir().expect("temporary directory should exist");
    let mut runtime = notepad_runtime(directory.path()).expect("notepad runtime should load");

    runtime
        .dispatch(UiEvent::InputChanged {
            id: "body".into(),
            value: "a  \n\n\nb  ".into(),
        })
        .expect("input event should render");
    let result = runtime
        .dispatch(UiEvent::Click("format".into()))
        .expect("format click should run");

    assert_eq!(result.state.value("body"), Some("a\n\nb"));
    assert_eq!(
        result.state.value("status"),
        Some("Formatted through RustScript")
    );
}

#[test]
fn rss_save_button_calls_notepad_host_and_writes_markdown() {
    let directory = tempdir().expect("temporary directory should exist");
    let mut runtime = notepad_runtime(directory.path()).expect("notepad runtime should load");

    runtime
        .dispatch(UiEvent::InputChanged {
            id: "title".into(),
            value: "Ideas / 2026".into(),
        })
        .expect("title input event should render");
    runtime
        .dispatch(UiEvent::InputChanged {
            id: "body".into(),
            value: "hello".into(),
        })
        .expect("body input event should render");
    let result = runtime
        .dispatch(UiEvent::Click("save".into()))
        .expect("save click should run");

    let saved = directory.path().join("ideas-2026.md");
    assert_eq!(
        std::fs::read_to_string(&saved).expect("note should be written"),
        "# Ideas / 2026\n\nhello\n"
    );
    assert_eq!(result.state.value("status"), saved.to_str());
}
