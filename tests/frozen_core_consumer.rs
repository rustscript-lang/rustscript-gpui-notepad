use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rustscript_gpui_notepad::notepad_hosts::NotepadHostModule;
use rustscript_gpui_notepad::rss_gpui::model::UiEvent;
use rustscript_gpui_notepad::rss_gpui::runtime::RssGpuiRuntime;
use rustscript_gpui_notepad::{FROZEN_RUSTSCRIPT_REV, notepad_runtime};

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
    let mut runtime =
        RssGpuiRuntime::from_source(source, vec![Arc::new(NotepadHostModule::new(notes.path()))])
            .unwrap_or_else(|error| panic!("{} failed to compile: {error}", path.display()));
    runtime
        .render()
        .unwrap_or_else(|error| panic!("{} failed to render: {error}", path.display()));
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
fn production_sources_do_not_build_a_parallel_host_catalog() {
    let mut sources = Vec::new();
    rust_sources(&manifest_dir().join("src"), &mut sources);
    assert!(!sources.is_empty(), "expected Rust sources under src/");
    for path in sources {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
        for token in [
            "HostApiBuilder",
            "HostApiCatalog",
            "HostFunctionRegistry",
            "HostModuleDescriptor",
            "HostFunctionDescriptor",
        ] {
            assert!(
                !source.contains(token),
                "{} must not construct a parallel host catalog ({token})",
                path.display()
            );
        }
    }
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
fn script_error_diagnostics_are_nonempty_and_portable() {
    let syntax = match RssGpuiRuntime::from_source("fn broken(", vec![]) {
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
        vec![],
    ) {
        Ok(_) => panic!("unregistered import should fail"),
        Err(error) => error.to_string(),
    };
    assert!(
        unknown.contains("missing_host") || unknown.contains("not registered"),
        "unknown-import diagnostic should name the import: {unknown}"
    );
    assert!(
        !unknown.contains("/home/wow"),
        "import diagnostics must not mention a machine-specific path: {unknown}"
    );
}
