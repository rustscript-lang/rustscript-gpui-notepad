# RustScript GPUI Notepad Design

## Goal

Create a small native GPUI notepad that makes button-originated UI events execute RustScript and lets that script call application-provided Rust host functions.

## Scope

The app has a title input, a multiline body editor, **Format** and **Save** buttons, and a status line. Button handlers are regular GPUI click listeners. Each click turns the current editor state into a RustScript event invocation.

## Event flow

```text
GPUI click listener
  -> NotepadApp::dispatch_script_event(event, title, body)
  -> construct typed RustScript prelude + scripts/notepad.rss
  -> compile_source + Vm
  -> bind dynamic host functions with the per-run notes directory
  -> Vm::run
  -> final stack string
  -> update editor body/status through the GPUI entity
```

`Format` supplies the `format` event. The script branches on that event and invokes `notepad::format_note(body)`. Its string result replaces the body editor text.

`Save` supplies the `save` event. The script invokes `notepad::save_note(title, body)`. The host writes a Markdown file under the application-selected `notes/` directory and returns the saved path as the status text.

A fresh VM is created per UI event. The source remains editable at `scripts/notepad.rss`, inputs are generated as typed prelude declarations, and each event receives a dynamic `HostArgsFunction` binding that captures that run's output directory. This gives file writing an explicit per-runtime boundary and keeps GUI state outside the VM.

## Script contract

`src/runtime.rs` appends the script below to escaped typed values for `event`, `title`, and `body`:

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

Required host imports:

| Import | Arity | Effect | Result |
|---|---:|---|---|
| `notepad::format_note` | 1 | Normalize blank lines and trim trailing whitespace | formatted body |
| `notepad::save_note` | 2 | Sanitize the title, create `notes/`, write `<slug>.md` | saved file path |

## Code boundaries

| File | Responsibility |
|---|---|
| `src/lib.rs` | Public module surface |
| `src/runtime.rs` | Script prelude construction, VM lifecycle, host bindings, value/result conversion |
| `src/hosts.rs` | Dynamic host-function implementations plus title sanitization and file persistence |
| `src/main.rs` | GPUI application entity, editor state, visual tree, click listeners, status presentation |
| `scripts/notepad.rss` | Editable policy mapping UI event names to host calls |
| `tests/runtime_tests.rs` | Runtime-only integration tests for format, save, malformed scripts, and event errors |
| `tests/script_smoke.rs` | Loads the checked-in RSS script and verifies both host imports are invoked |

## Error handling

Compilation, VM, type-conversion, and file-system failures return `NotepadError`. GPUI retains current editor text and renders the error in the status line. Saves sanitize titles to a safe slug and use `untitled` when no title is supplied.

## Verification

Test the runtime without opening a GUI window: formatting must return transformed content, saving must create the expected Markdown file, and unsupported event names must return their script-produced status. Build the GPUI binary after tests, then run `--script-smoke` to prove the checked-in script executes custom host functions without requiring a display server.
