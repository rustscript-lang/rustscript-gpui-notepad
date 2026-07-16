# RustScript-driven GPUI Notepad Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use codex-superpowers-executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a reusable GPUI/RustScript UI runtime and demonstrate it with an RSS-authored notepad whose native buttons invoke script-defined events and custom host functions.

**Architecture:** `rss_gpui` is a reusable framework module. The RSS document calls restricted `ui::*` builder/state/event hosts to construct a native-agnostic `UiTree`; the framework renderer maps that tree into GPUI elements and callbacks. App-specific behavior enters only through `HostModule`, demonstrated by `notepad::*` format and save functions.

**Tech Stack:** Rust 2024, GPUI 0.2.2, local runtime-only `pd-vm`, RustScript, `tempfile`.

---

## File structure

- `src/rss_gpui/model.rs`: UI tree, state, event, and error data.
- `src/rss_gpui/builder.rs`: checked builder call ordering and tree materialization.
- `src/rss_gpui/dispatch.rs`: reusable parser and event-name table for `ui::on(event, || { ... })` handlers.
- `src/rss_gpui/runtime.rs`: source execution, import validation, limits, dynamic host modules.
- `src/rss_gpui/renderer.rs`: generic GPUI reconciliation and event callbacks.
- `src/notepad_hosts.rs`: example-only custom formatting/save hosts.
- `src/main.rs`: app shell that opens the generic `RssGpuiView`.
- `scripts/notepad.rss`: authoritative notepad declaration and behavior.
- `tests/builder_tests.rs`: generic builder behavior.
- `tests/runtime_tests.rs`: generic scripted event/state behavior.
- `tests/dispatch_sugar_tests.rs`: independent named handlers route through the shared event table.
- `tests/notepad_script_tests.rs`: end-to-end app host behavior.

### Task 1: Define generic script-owned UI data

**Files:**
- Create: `Cargo.toml`, `.gitignore`, `src/lib.rs`
- Create: `src/rss_gpui/mod.rs`, `src/rss_gpui/model.rs`, `src/rss_gpui/builder.rs`
- Test: `tests/builder_tests.rs`

- [ ] **Step 1: Write a failing builder test**

```rust
#[test]
fn builder_preserves_script_declared_hierarchy_and_click_binding() {
    let mut builder = UiBuilder::new();
    builder.window("Demo", 640, 480).unwrap();
    builder.column_begin("root").unwrap();
    builder.button("save", "Save").unwrap();
    builder.bind_click("save", "save-note").unwrap();
    builder.column_end().unwrap();
    let tree = builder.finish().unwrap();
    assert_eq!(tree.click_event("save"), Some("save-note"));
}
```

- [ ] **Step 2: Run RED**

Run: `cargo test --test builder_tests builder_preserves_script_declared_hierarchy_and_click_binding`

Expected: FAIL because `UiBuilder` does not exist.

- [ ] **Step 3: Implement only model and builder behavior**

Create node types for column, row, label, text input, text area, and button; validate one window, unique IDs, balanced containers, button-before-binding, and explicit `finish`.

- [ ] **Step 4: Run GREEN and complete builder suite**

Run: `cargo test --test builder_tests`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock .gitignore src/lib.rs src/rss_gpui tests/builder_tests.rs
git commit -m "feat: add RSS GPUI tree builder"
```

### Task 2: Execute RSS documents through reusable ui hosts

**Files:**
- Create: `src/rss_gpui/runtime.rs`, `src/rss_gpui/hosts.rs`
- Modify: `src/rss_gpui/mod.rs`
- Test: `tests/runtime_tests.rs`

- [ ] **Step 1: Write a failing event-dispatch test**

```rust
#[test]
fn script_declares_button_binding_and_updates_state_for_its_event() {
    let runtime = RssGpuiRuntime::from_source(SCRIPT, vec![]).unwrap();
    let result = runtime.dispatch(UiEvent::Click("format".into())).unwrap();
    assert_eq!(result.state.value("status"), Some("formatted"));
}
```

- [ ] **Step 2: Run RED**

Run: `cargo test --test runtime_tests script_declares_button_binding_and_updates_state_for_its_event`

Expected: FAIL because `RssGpuiRuntime` does not exist.

- [ ] **Step 3: Implement restricted execution**

`dispatch` resolves a click node ID to its script-declared event name, creates an execution context, binds `ui::*` dynamic `HostArgsFunction`s, verifies imports/arity, runs with bounded fuel, and returns a tree plus state. Implement `event_name`, `get_value`, `set_value`, widget declarations, and `finish` before adding any application host.

- [ ] **Step 4: Run GREEN and runtime suite**

Run: `cargo test --test runtime_tests`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/rss_gpui tests/runtime_tests.rs
git commit -m "feat: execute RSS UI events through reusable hosts"
```

### Task 3: Add app-defined custom host modules

**Files:**
- Create: `src/notepad_hosts.rs`, `scripts/notepad.rss`
- Modify: `src/rss_gpui/runtime.rs`, `src/lib.rs`
- Test: `tests/notepad_script_tests.rs`

- [ ] **Step 1: Write a failing end-to-end format test**

```rust
#[test]
fn rss_format_button_calls_notepad_host_and_rewrites_body() {
    let runtime = notepad_runtime(tempdir().unwrap().path()).unwrap();
    runtime.dispatch(UiEvent::InputChanged { id: "body".into(), value: "a  \n\n\nb  ".into() }).unwrap();
    let result = runtime.dispatch(UiEvent::Click("format".into())).unwrap();
    assert_eq!(result.state.value("body"), Some("a\n\nb"));
}
```

- [ ] **Step 2: Run RED**

Run: `cargo test --test notepad_script_tests rss_format_button_calls_notepad_host_and_rewrites_body`

Expected: FAIL because the app module and RSS source do not exist.

- [ ] **Step 3: Implement `HostModule` and notepad module**

`HostModule` binds a namespace into the per-execution VM. `NotepadHostModule` captures a notes directory and supplies `notepad::format_note` and `notepad::save_note`. Write the notepad UI, button bindings, and its branches exclusively in `scripts/notepad.rss`.

- [ ] **Step 4: Add RED/GREEN save coverage**

Add a save-click test that asserts `notes/ideas-2026.md` contains `# Ideas / 2026\n\nhello\n`, first observe failure, then implement the save host.

Run: `cargo test --test notepad_script_tests`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/notepad_hosts.rs src/lib.rs src/rss_gpui/runtime.rs scripts/notepad.rss tests/notepad_script_tests.rs
git commit -m "feat: add RSS authored notepad host module"
```

### Task 4: Render generic script UI with GPUI

**Files:**
- Create: `src/rss_gpui/renderer.rs`, `src/main.rs`, `README.md`
- Modify: `src/rss_gpui/mod.rs`, `Cargo.toml`
- Test: `src/rss_gpui/renderer.rs`

- [ ] **Step 1: Write a failing pure render-plan test**

```rust
#[test]
fn render_plan_keeps_script_button_event_name() {
    let plan = RenderPlan::from_tree(&tree_with_button("save", "save-note"));
    assert_eq!(plan.button("save").unwrap().event_name, "save-note");
}
```

- [ ] **Step 2: Run RED**

Run: `cargo test --lib render_plan_keeps_script_button_event_name`

Expected: FAIL because `RenderPlan` does not exist.

- [ ] **Step 3: Implement renderer and app shell**

Render each `UiNode` with GPUI elements. Each text change sends `UiEvent::InputChanged`; each button callback sends `UiEvent::Click(node_id)`. The renderer holds no notepad-specific branch. `main.rs` loads the script, configures the notepad module, opens the window, and supplies `--script-smoke`.

- [ ] **Step 4: Verify native build and event path**

Run:

```bash
cargo fmt --all -- --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo build
cargo run -- --script-smoke
```

Expected: every command exits zero; smoke dispatches the same scripted click path for format and save and prints the saved path.

- [ ] **Step 5: Commit**

```bash
git add src/rss_gpui/renderer.rs src/rss_gpui/mod.rs src/main.rs README.md Cargo.toml
git commit -m "feat: render RSS UI through GPUI"
```

### Task 5: Final validation

- [ ] Run `git diff --check`, `cargo test --all-targets`, `cargo clippy --all-targets -- -D warnings`, and `cargo run -- --script-smoke`.
- [ ] Launch the desktop binary when a display server is available; verify text edits and both script-declared button events.
- [ ] Confirm `git status --short` has no unexpected files.
