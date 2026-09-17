use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rustscript_gpui_notepad::notepad_hosts::notepad_host_module;
use rustscript_gpui_notepad::rss_gpui::catalog::{
    NOTEPAD_HOST_COUNT, NOTEPAD_HOST_NAMES, PRODUCTION_RSS_HOST_COUNT, UI_HOST_COUNT,
    UI_HOST_NAMES, compose_catalog, production_host_catalog, production_host_modules,
};
use rustscript_gpui_notepad::rss_gpui::model::UiEvent;
use rustscript_gpui_notepad::rss_gpui::runtime::RssGpuiRuntime;
use rustscript_gpui_notepad::rss_gpui::ui_hosts::ui_host_module;
use rustscript_gpui_notepad::{FROZEN_RUSTSCRIPT_REV, notepad_runtime};
use vm::{
    CallOutcome, CallReturn, CompileSourceFileOptions, HostAdapterDescriptor, HostApiCatalog,
    HostBindingDescriptor, HostBindingKind, HostFunctionDescriptor, HostFunctionRegistry,
    HostFunctionSchema, HostModuleDescriptor, HostParamSchema, HostTypeSchema, SourceFlavor, Value,
    Vm, VmResult, compile_source_with_flavor_and_options,
};

const EXPECTED_BUNDLED_RSS: usize = 2;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn collect_rss(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap_or_else(|error| panic!("read {}: {error}", dir.display()))
    {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            if matches!(name, ".git" | "target") {
                continue;
            }
            collect_rss(&path, out);
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) == Some("rss") {
            out.push(path);
        }
    }
}

fn bundled_rss_files() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    collect_rss(&manifest_dir(), &mut paths);
    paths.sort();
    paths
}

fn rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap_or_else(|error| panic!("read {}: {error}", dir.display()))
    {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            rust_sources(&path, out);
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

fn compile_bundled_rss(path: &Path) {
    let source =
        fs::read_to_string(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    let notes = tempfile::tempdir().expect("temporary notes directory");
    let mut runtime = RssGpuiRuntime::from_source_with_notes(source, notes.path())
        .unwrap_or_else(|error| panic!("{} failed to compile: {error}", path.display()));
    runtime
        .render()
        .unwrap_or_else(|error| panic!("{} failed to render: {error}", path.display()));
}

fn production_function_names(catalog: &HostApiCatalog) -> Vec<String> {
    catalog
        .functions()
        .iter()
        .map(|function| function.name.clone())
        .collect()
}

fn compile_with_catalog(source: &str, catalog: Arc<HostApiCatalog>) -> vm::CompiledProgram {
    compile_source_with_flavor_and_options(
        source,
        SourceFlavor::RustScript,
        CompileSourceFileOptions::default().with_host_api_catalog(catalog),
    )
    .unwrap_or_else(|error| panic!("compile should succeed: {error}"))
}

fn null_adapter(_args: &[Value]) -> VmResult<CallOutcome> {
    Ok(CallOutcome::Return(CallReturn::one(Value::Null)))
}

fn probe_early_descriptor() -> HostFunctionDescriptor {
    HostFunctionDescriptor {
        schema: HostFunctionSchema::with_return("probe::early", Vec::new(), HostTypeSchema::Null)
            .with_description("First probe host used by rollback tests"),
        binding: HostBindingDescriptor {
            kind: HostBindingKind::StaticArgs,
        },
        effects: Vec::new(),
        adapter: HostAdapterDescriptor::StaticArgs(null_adapter),
        resource_types: Vec::new(),
    }
}

fn probe_late_ok_descriptor() -> HostFunctionDescriptor {
    HostFunctionDescriptor {
        schema: HostFunctionSchema::with_return("probe::late", Vec::new(), HostTypeSchema::Null)
            .with_description("Matching late probe host"),
        binding: HostBindingDescriptor {
            kind: HostBindingKind::StaticArgs,
        },
        effects: Vec::new(),
        adapter: HostAdapterDescriptor::StaticArgs(null_adapter),
        resource_types: Vec::new(),
    }
}

fn probe_late_mismatch_descriptor() -> HostFunctionDescriptor {
    HostFunctionDescriptor {
        schema: HostFunctionSchema::with_return("probe::late", Vec::new(), HostTypeSchema::Null)
            .with_description("Late probe host with a binding/adapter mismatch"),
        binding: HostBindingDescriptor {
            kind: HostBindingKind::StaticStack,
        },
        effects: Vec::new(),
        adapter: HostAdapterDescriptor::StaticArgs(null_adapter),
        resource_types: Vec::new(),
    }
}

#[test]
fn rustscript_crates_are_pinned_to_the_frozen_full_sha() {
    let cargo_toml = fs::read_to_string(manifest_dir().join("Cargo.toml")).expect("Cargo.toml");
    let cargo_lock = fs::read_to_string(manifest_dir().join("Cargo.lock")).expect("Cargo.lock");
    assert!(
        cargo_toml.contains(FROZEN_RUSTSCRIPT_REV),
        "Cargo.toml must pin the frozen full SHA"
    );
    assert!(
        !cargo_toml.contains("/home/wow"),
        "production pd-vm must not use a machine-specific path"
    );
    assert!(
        !cargo_toml.contains("path = \"../rustscript"),
        "production pd-vm must not use a sibling path dep"
    );
    let expected_source = format!(
        "git+https://github.com/rustscript-lang/rustscript?rev={FROZEN_RUSTSCRIPT_REV}#{FROZEN_RUSTSCRIPT_REV}"
    );
    assert!(
        cargo_lock.contains("name = \"pd-vm\""),
        "pd-vm must appear in Cargo.lock"
    );
    assert!(
        cargo_lock.contains(&expected_source),
        "pd-vm must resolve to the frozen git SHA"
    );
}

#[test]
fn production_sources_drive_one_descriptor_catalog_and_registry() {
    let mut sources = Vec::new();
    rust_sources(&manifest_dir().join("src"), &mut sources);
    assert!(!sources.is_empty(), "expected Rust sources under src/");
    let mut joined = String::new();
    for path in &sources {
        let source = fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
        assert!(
            !source.contains("bind_args_function"),
            "{} must not bind RSS-visible hosts through bind_args_function",
            path.display()
        );
        joined.push_str(&source);
        joined.push('\n');
    }
    for token in [
        "HostApiCatalog",
        "HostFunctionRegistry",
        "HostModuleDescriptor",
        "HostFunctionDescriptor",
        "install_from_catalog",
        "bind_vm_cached",
        "compile_source_with_flavor_and_options",
        "HostFunctionRegistry::restricted",
    ] {
        assert!(
            joined.contains(token),
            "production sources must drive one descriptor catalog/registry ({token})"
        );
    }
}

#[test]
fn production_catalog_exposes_exactly_the_fifteen_rss_hosts() {
    let modules = production_host_modules();
    assert_eq!(modules.len(), 2, "ui and notepad modules");
    assert_eq!(modules[0].name, "ui");
    assert_eq!(modules[1].name, "notepad");
    assert_eq!(modules[0].functions.len(), UI_HOST_COUNT);
    assert_eq!(modules[1].functions.len(), NOTEPAD_HOST_COUNT);

    let catalog = production_host_catalog();
    let names = production_function_names(&catalog);
    assert_eq!(names.len(), PRODUCTION_RSS_HOST_COUNT);
    let mut expected: Vec<&str> = UI_HOST_NAMES.to_vec();
    expected.extend(NOTEPAD_HOST_NAMES);
    assert_eq!(names, expected);

    let composed = compose_catalog(&[ui_host_module(), notepad_host_module()])
        .expect("composed production catalog");
    assert_eq!(composed.fingerprint(), catalog.fingerprint());
    assert_ne!(catalog.fingerprint().as_u64(), 0);

    let ui_only = compose_catalog(&[ui_host_module()]).expect("ui catalog");
    assert_ne!(
        ui_only.fingerprint(),
        catalog.fingerprint(),
        "fingerprint must change when the host set changes"
    );
}

#[test]
fn production_catalog_uses_typed_schemas_and_allowlists_button_callback_unknown() {
    let catalog = production_host_catalog();
    let mut unknown_slots = Vec::new();
    for function in catalog.functions() {
        if matches!(
            function.return_type,
            HostTypeSchema::Unknown | HostTypeSchema::Map(_)
        ) {
            unknown_slots.push(format!("{} return", function.name));
        }
        for param in &function.params {
            match &param.ty {
                HostTypeSchema::Unknown | HostTypeSchema::Map(_) => {
                    unknown_slots.push(format!("{} param {}", function.name, param.name));
                }
                HostTypeSchema::Callable { params, result } => {
                    assert_eq!(function.name, "ui::button");
                    assert_eq!(param.name, "callback");
                    assert!(params.is_empty(), "button callbacks take no arguments");
                    assert!(
                        matches!(result.as_ref(), HostTypeSchema::Unknown),
                        "button callback results are discarded after invoke_callable"
                    );
                }
                _ => {}
            }
        }
    }
    assert!(
        unknown_slots.is_empty(),
        "public host schemas must not use dynamic Value/Map/Unknown slots except the allowlisted button callback result: {unknown_slots:?}"
    );

    let format_note = catalog
        .function("notepad::format_note")
        .expect("format_note");
    assert_eq!(format_note.params.len(), 1);
    assert_eq!(format_note.params[0].ty, HostTypeSchema::String);
    assert_eq!(format_note.return_type, HostTypeSchema::String);

    let save_note = catalog.function("notepad::save_note").expect("save_note");
    assert_eq!(save_note.params.len(), 2);
    assert_eq!(save_note.params[0].ty, HostTypeSchema::String);
    assert_eq!(save_note.params[1].ty, HostTypeSchema::String);
    assert_eq!(save_note.return_type, HostTypeSchema::String);

    let get_value = catalog.function("ui::get_value").expect("get_value");
    assert_eq!(get_value.return_type, HostTypeSchema::String);
}

#[test]
fn restricted_registry_denies_before_install_and_binds_after() {
    let catalog = production_host_catalog();
    let compiled = compile_with_catalog(
        r#"
use ui;
ui::window("Demo", 640, 480);
ui::finish();
"#,
        Arc::clone(&catalog),
    );

    let mut denied = Vm::new(compiled.program.clone());
    let deny_error = HostFunctionRegistry::restricted()
        .bind_vm_cached(&mut denied)
        .expect_err("uninstalled production imports must fail at bind");
    let deny_message = deny_error.to_string();
    assert!(
        deny_message.contains("ui::window")
            || deny_message.contains("capability")
            || deny_message.contains("not registered")
            || deny_message.contains("missing"),
        "deny-before diagnostic should mention the unbound import: {deny_message}"
    );

    let mut allowed = Vm::new(compiled.program);
    let mut registry = HostFunctionRegistry::restricted();
    ui_host_module()
        .install_from_catalog(&mut registry, catalog.as_ref())
        .expect("ui module should install");
    notepad_host_module()
        .install_from_catalog(&mut registry, catalog.as_ref())
        .expect("notepad module should install");
    registry
        .bind_vm_cached(&mut allowed)
        .expect("allow-after install should bind");
}

#[test]
fn adapter_schema_mismatch_and_unlisted_import_fail_closed() {
    let catalog = production_host_catalog();
    let mismatch = HostModuleDescriptor {
        name: "ui-mismatch",
        functions: &[ui_get_value_int_mismatch_descriptor],
        resources: &[],
    };
    let mut registry = HostFunctionRegistry::restricted();
    let error = mismatch
        .install_from_catalog(&mut registry, catalog.as_ref())
        .expect_err("return-type mismatch must fail before bind");
    let message = error.to_string();
    assert!(
        message.contains("ui::get_value")
            || message.contains("mismatch")
            || message.contains("missing"),
        "schema mismatch should name the host: {message}"
    );

    let unknown = match RssGpuiRuntime::from_source(
        r#"
use ui;
ui::window("Demo", 640, 480);
missing_host::go();
ui::finish();
"#,
    ) {
        Ok(_) => panic!("unlisted import should fail"),
        Err(error) => error.to_string(),
    };
    assert!(
        unknown.contains("missing_host")
            || unknown.contains("not registered")
            || unknown.contains("unknown"),
        "unlisted-import diagnostic should name the import: {unknown}"
    );
}

fn ui_get_value_int_mismatch_descriptor() -> HostFunctionDescriptor {
    HostFunctionDescriptor {
        schema: HostFunctionSchema::with_return(
            "ui::get_value",
            vec![HostParamSchema::value("id", HostTypeSchema::String)],
            HostTypeSchema::Int,
        )
        .with_description("Mismatched get_value return used by schema-mismatch tests"),
        binding: HostBindingDescriptor {
            kind: HostBindingKind::StaticStack,
        },
        effects: Vec::new(),
        adapter: HostAdapterDescriptor::StaticArgs(null_adapter),
        resource_types: Vec::new(),
    }
}

#[test]
fn late_descriptor_failure_rolls_back_earlier_install() {
    let catalog = HostFunctionDescriptor::collect_catalog(&[
        probe_early_descriptor(),
        probe_late_ok_descriptor(),
    ])
    .expect("probe catalog");
    let catalog = Arc::new(catalog);
    let mismatch_module = HostModuleDescriptor {
        name: "probe-mismatch",
        functions: &[probe_early_descriptor, probe_late_mismatch_descriptor],
        resources: &[],
    };
    let mut registry = HostFunctionRegistry::restricted();
    let error = mismatch_module
        .install_from_catalog(&mut registry, catalog.as_ref())
        .expect_err("late adapter mismatch should fail the module");
    assert!(
        error.to_string().contains("mismatch") || error.to_string().contains("probe::late"),
        "rollback diagnostic should mention the late failure: {error}"
    );

    let compiled = compile_with_catalog(
        r#"
use probe;
probe::early();
"#,
        Arc::clone(&catalog),
    );
    let mut vm = Vm::new(compiled.program);
    registry
        .bind_vm_cached(&mut vm)
        .expect_err("rolled-back early host must not remain bound");

    let ok_module = HostModuleDescriptor {
        name: "probe",
        functions: &[probe_early_descriptor, probe_late_ok_descriptor],
        resources: &[],
    };
    let mut registry = HostFunctionRegistry::restricted();
    ok_module
        .install_from_catalog(&mut registry, catalog.as_ref())
        .expect("matching probe module should install");
    let compiled = compile_with_catalog(
        r#"
use probe;
probe::early();
probe::late();
"#,
        catalog,
    );
    let mut vm = Vm::new(compiled.program);
    registry
        .bind_vm_cached(&mut vm)
        .expect("matching probe hosts should bind");
}

#[test]
fn bundled_rss_scripts_compile_through_the_production_path() {
    let scripts = bundled_rss_files();
    assert_eq!(
        scripts.len(),
        EXPECTED_BUNDLED_RSS,
        "bundled RSS count drifted: {scripts:?}"
    );
    for path in scripts {
        compile_bundled_rss(&path);
    }
}

#[test]
fn notepad_vm_reuses_after_reload_and_repeated_dispatch() {
    let directory = tempfile::tempdir().expect("temporary directory should exist");
    let mut runtime = notepad_runtime(directory.path()).expect("notepad runtime should load");
    let first = runtime.render().expect("initial render should succeed");
    let second = runtime.render().expect("reused VM render should succeed");
    assert_eq!(first.state, second.state);

    runtime
        .dispatch(UiEvent::InputChanged {
            id: "body".into(),
            value: "alpha  \n\n\nbeta  ".into(),
        })
        .expect("body input should render");
    let formatted = runtime
        .dispatch(UiEvent::Click("format".into()))
        .expect("first format click should run");
    assert_eq!(formatted.state.value("body"), Some("alpha\n\nbeta"));
    let formatted_again = runtime
        .dispatch(UiEvent::Click("format".into()))
        .expect("second format click should reuse the VM");
    assert_eq!(formatted_again.state.value("body"), Some("alpha\n\nbeta"));
}

#[test]
fn reset_failure_clears_stale_tree_before_callback_dispatch() {
    let mut runtime = RssGpuiRuntime::from_source(
        r#"
use ui;
ui::window("Demo", 640, 480);
ui::button("go", "Go", || ui::set_value("status", "clicked"));
ui::finish();
"#,
    )
    .expect("script should load");
    runtime.render().expect("initial render");
    runtime.clear_tree_for_test();
    let result = runtime
        .dispatch(UiEvent::Click("go".into()))
        .expect("cleared tree must reset and rebuild before click");
    assert_eq!(result.state.value("status"), Some("clicked"));
}

#[test]
fn script_error_diagnostics_are_nonempty_and_portable() {
    let syntax = match RssGpuiRuntime::from_source("fn broken(") {
        Ok(_) => panic!("invalid RSS should fail at compile"),
        Err(error) => error.to_string(),
    };
    assert!(!syntax.is_empty(), "compile diagnostics must not be empty");
    assert!(
        !syntax.contains("/home/wow"),
        "compile diagnostics must not mention a machine-specific path: {syntax}"
    );

    let unknown = match RssGpuiRuntime::from_source(
        r#"
use ui;
ui::window("Demo", 640, 480);
missing_host::go();
ui::finish();
"#,
    ) {
        Ok(_) => panic!("unregistered import should fail"),
        Err(error) => error.to_string(),
    };
    assert!(
        unknown.contains("missing_host")
            || unknown.contains("not registered")
            || unknown.contains("unknown"),
        "unknown-import diagnostic should name the import: {unknown}"
    );
    assert!(
        !unknown.contains("/home/wow"),
        "import diagnostics must not mention a machine-specific path: {unknown}"
    );
}
