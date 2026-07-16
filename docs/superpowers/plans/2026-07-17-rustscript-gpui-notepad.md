# RustScript GPUI Notepad Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use codex-superpowers-subagent-driven-development (recommended) or codex-superpowers-executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a native GPUI notepad whose button clicks run RustScript and invoke per-run custom Rust host functions for formatting and saving notes.

**Architecture:** GPUI owns visual state and click listeners. A library runtime converts each UI event into typed RustScript source, creates a VM, binds dynamic host functions that capture the selected notes directory, runs the script, and returns the top-of-stack string to the view.

**Tech Stack:** Rust 2024, GPUI 0.2, local `pd-vm`, RustScript, `tempfile`.

---

## File structure

- `Cargo.toml`: package, GPUI, local `pd-vm`, and test dependencies.
- `src/lib.rs`: exposes `hosts` and `runtime`.
- `src/hosts.rs`: format/save host factories and deterministic persistence helpers.
- `src/runtime.rs`: event inputs, source construction, execution, and errors.
- `src/main.rs`: GPUI entity and buttons wired to `dispatch_script_event`.
- `scripts/notepad.rss`: editable event-to-host policy.
- `tests/runtime_tests.rs`: runtime behavior tests.
- `tests/script_smoke.rs`: checked-in script end-to-end coverage.

### Task 1: Bootstrap the reusable runtime boundary

**Files:**
- Create: `Cargo.toml`
- Create: `.gitignore`
- Create: `src/lib.rs`
- Create: `src/hosts.rs`
- Create: `src/runtime.rs`
- Test: `tests/runtime_tests.rs`

- [ ] **Step 1: Write a failing formatting test**

```rust
#[test]
fn format_event_returns_host_formatted_body() {
    let runtime = ScriptRuntime::new(tempdir().unwrap().path());
    assert_eq!(runtime.dispatch("format", "Draft", "a  \n\n\nb  ").unwrap(), "a\n\nb");
}
```

- [ ] **Step 2: Run the focused test and verify failure**

Run: `cargo test --test runtime_tests format_event_returns_host_formatted_body`

Expected: FAIL because `ScriptRuntime` does not exist.

- [ ] **Step 3: Add the smallest runtime implementation**

`ScriptRuntime::dispatch` concatenates an escaped typed prelude with a script body, compiles it, constructs a `Vm`, binds `notepad::format_note` and `notepad::save_note`, runs it, and converts one final `Value::String` to `String`.

- [ ] **Step 4: Run the focused test and verify success**

Run: `cargo test --test runtime_tests format_event_returns_host_formatted_body`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml .gitignore src/lib.rs src/hosts.rs src/runtime.rs tests/runtime_tests.rs
git commit -m "feat: add RustScript notepad runtime"
```

### Task 2: Add file persistence through a dynamic host function

**Files:**
- Modify: `src/hosts.rs`
- Modify: `src/runtime.rs`
- Modify: `tests/runtime_tests.rs`

- [ ] **Step 1: Write a failing save test**

```rust
#[test]
fn save_event_writes_sanitized_markdown_note() {
    let directory = tempdir().unwrap();
    let runtime = ScriptRuntime::new(directory.path());
    let saved = runtime.dispatch("save", "Ideas / 2026", "hello").unwrap();
    assert_eq!(directory.path().join("notes/ideas-2026.md").read_to_string().unwrap(), "# Ideas / 2026\n\nhello\n");
    assert!(saved.ends_with("notes/ideas-2026.md"));
}
```

- [ ] **Step 2: Run the focused test and verify failure**

Run: `cargo test --test runtime_tests save_event_writes_sanitized_markdown_note`

Expected: FAIL because `save` is unimplemented.

- [ ] **Step 3: Implement the save host factory**

Use `Vm::bind_args_function("notepad::save_note", ...)` with a closure that captures an `Arc<PathBuf>`. Validate exactly two `Value::String` arguments, create `notes/`, write Markdown, and return `CallOutcome::Return(CallReturn::one(Value::String(path)))`.

- [ ] **Step 4: Run runtime tests and verify success**

Run: `cargo test --test runtime_tests`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/hosts.rs src/runtime.rs tests/runtime_tests.rs
git commit -m "feat: save notes through dynamic host function"
```

### Task 3: Make the checked-in RustScript policy executable

**Files:**
- Create: `scripts/notepad.rss`
- Create: `tests/script_smoke.rs`
- Modify: `src/runtime.rs`

- [ ] **Step 1: Write a failing smoke test**

```rust
#[test]
fn checked_in_policy_routes_format_and_save_button_events() {
    let directory = tempdir().unwrap();
    let runtime = ScriptRuntime::from_project_script(directory.path()).unwrap();
    assert_eq!(runtime.dispatch("format", "", "x  \n\n\ny").unwrap(), "x\n\ny");
    runtime.dispatch("save", "Smoke", "saved").unwrap();
    assert!(directory.path().join("notes/smoke.md").exists());
}
```

- [ ] **Step 2: Run the focused test and verify failure**

Run: `cargo test --test script_smoke checked_in_policy_routes_format_and_save_button_events`

Expected: FAIL because `scripts/notepad.rss` does not exist.

- [ ] **Step 3: Add the event policy**

```rustscript
use notepad;
if event == "format" {
    notepad::format_note(body)
} else if event == "save" {
    notepad::save_note(title, body)
} else {
    "unknown UI event"
}
```

- [ ] **Step 4: Run smoke and all test targets**

Run: `cargo test --tests`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add scripts/notepad.rss src/runtime.rs tests/script_smoke.rs
git commit -m "feat: add editable RustScript event policy"
```

### Task 4: Add the GPUI notepad and visual event wiring

**Files:**
- Create: `src/main.rs`
- Modify: `Cargo.toml`
- Modify: `README.md`

- [ ] **Step 1: Add a failing unit test for the UI-independent action reducer**

```rust
#[test]
fn format_result_replaces_editor_body_and_save_result_updates_status() {
    assert_eq!(apply_script_result(Event::Format, "formatted", "before", ""), ("formatted", "formatted"));
    assert_eq!(apply_script_result(Event::Save, "path", "before", ""), ("before", "path"));
}
```

- [ ] **Step 2: Run the focused test and verify failure**

Run: `cargo test --bin rustscript-gpui-notepad format_result_replaces_editor_body_and_save_result_updates_status`

Expected: FAIL because `apply_script_result` does not exist.

- [ ] **Step 3: Implement GPUI entity and click callbacks**

Render native title/body editors, status text, and buttons. Button `on_click` callbacks capture the GPUI entity and call `dispatch_script_event(Event::Format)` or `dispatch_script_event(Event::Save)`, then notify the entity. Support `--script-smoke` to run the checked-in script without opening a window.

- [ ] **Step 4: Run tests, format, lints, build, and smoke executable**

Run:

```bash
cargo fmt --all -- --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo build
cargo run -- --script-smoke
```

Expected: all commands exit zero; smoke output reports both format and save activity.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src/main.rs README.md
git commit -m "feat: add GPUI notepad event integration"
```

### Task 5: Final verification

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Document run and scripted event flow**

Document `cargo run`, `cargo run -- --script-smoke`, button-to-script-to-host flow, editable RSS policy, and output location.

- [ ] **Step 2: Verify the working tree and runtime effect**

Run:

```bash
git diff --check
git status --short
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo run -- --script-smoke
```

Expected: no whitespace errors; all tests/lints pass; smoke creates a note and prints its path.
