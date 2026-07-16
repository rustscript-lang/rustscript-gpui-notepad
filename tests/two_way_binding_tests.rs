use rustscript_gpui_notepad::rss_gpui::model::UiEvent;
use rustscript_gpui_notepad::rss_gpui::runtime::RssGpuiRuntime;

fn two_way_binding_source() -> &'static str {
    r#"
        use ui;
        ui::window("Two-way binding", 400, 300);
        ui::text_input("a", "A", "", "Enter a value");
        ui::text_input("b", "B", "", "Mirrors field A");
        ui::bind_value("a", "b");
        ui::bind_value("b", "a");
        ui::finish();
    "#
}

#[test]
fn two_way_binding_propagates_input_change_across_fields() {
    let mut runtime = RssGpuiRuntime::from_source(two_way_binding_source(), vec![]).unwrap();
    runtime.render().unwrap();

    let result = runtime
        .dispatch(UiEvent::InputChanged {
            id: "a".into(),
            value: "hello world".into(),
        })
        .unwrap();

    assert_eq!(result.state.value("a"), Some("hello world"));
    assert_eq!(result.state.value("b"), Some("hello world"));
}

#[test]
fn two_way_binding_propagates_in_reverse_direction() {
    let mut runtime = RssGpuiRuntime::from_source(two_way_binding_source(), vec![]).unwrap();
    runtime.render().unwrap();

    let result = runtime
        .dispatch(UiEvent::InputChanged {
            id: "b".into(),
            value: "from b side".into(),
        })
        .unwrap();

    assert_eq!(result.state.value("b"), Some("from b side"));
    assert_eq!(result.state.value("a"), Some("from b side"));
}

#[test]
fn two_way_binding_does_not_mutate_when_source_is_unchanged() {
    let mut runtime = RssGpuiRuntime::from_source(two_way_binding_source(), vec![]).unwrap();
    runtime.render().unwrap();

    let seed = runtime
        .dispatch(UiEvent::InputChanged {
            id: "a".into(),
            value: "shared".into(),
        })
        .unwrap();
    assert_eq!(seed.state.value("b"), Some("shared"));

    let re_enter = runtime
        .dispatch(UiEvent::InputChanged {
            id: "a".into(),
            value: "shared".into(),
        })
        .unwrap();
    assert_eq!(re_enter.state.value("a"), Some("shared"));
    assert_eq!(re_enter.state.value("b"), Some("shared"));
}

#[test]
fn one_way_binding_runs_without_compiler_change() {
    let source = r#"
        use ui;
        ui::window("One-way binding", 400, 300);
        ui::text_input("left", "Left", "", "Driver");
        ui::text_input("right", "Right", "", "Follower");
        ui::bind_value("left", "right");
        ui::finish();
    "#;
    let mut runtime = RssGpuiRuntime::from_source(source, vec![]).unwrap();
    runtime.render().unwrap();

    let result = runtime
        .dispatch(UiEvent::InputChanged {
            id: "left".into(),
            value: "one way".into(),
        })
        .unwrap();

    assert_eq!(result.state.value("left"), Some("one way"));
    assert_eq!(result.state.value("right"), Some("one way"));

    let reverse = runtime
        .dispatch(UiEvent::InputChanged {
            id: "right".into(),
            value: "reverse ignored".into(),
        })
        .unwrap();

    assert_eq!(reverse.state.value("right"), Some("reverse ignored"));
    assert_eq!(reverse.state.value("left"), Some("one way"));
}

#[test]
fn bind_value_rejects_self_loop() {
    let source = r#"
        use ui;
        ui::window("loop", 400, 300);
        ui::text_input("solo", "Solo", "", "");
        ui::bind_value("solo", "solo");
        ui::finish();
    "#;
    let err = RssGpuiRuntime::from_source(source, vec![])
        .unwrap()
        .render()
        .expect_err("should fail for self-loop binding");
    assert_eq!(
        err.to_string(),
        "host error: ui::bind_value source and target must differ"
    );
}

#[test]
fn bind_value_rejects_when_target_does_not_exist() {
    let source = r#"
        use ui;
        ui::window("missing target", 400, 300);
        ui::text_input("src", "Src", "", "");
        ui::bind_value("src", "ghost");
        ui::finish();
    "#;
    let err = RssGpuiRuntime::from_source(source, vec![])
        .unwrap()
        .render()
        .expect_err("should fail for dangling binding");
    assert_eq!(
        err.to_string(),
        "host error: ui::bind_value references unknown id 'ghost'"
    );
}
