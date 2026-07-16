# RustScript-driven GPUI Desktop Framework and Notepad Example

## Goal

Build a reusable, event-driven GPUI framework where `.rss` source is authoritative for UI declarations, widget construction, event bindings, and application behavior. The notepad is an example application module; Rust provides only the reusable renderer/runtime and explicitly registered custom host capabilities.

## Core rule

Rust never contains title/body/button business logic. It renders a generic `UiTree`, forwards native input and click events to `RssGpuiRuntime`, then applies the resulting tree/state. All widget IDs, labels, layout calls, click-to-event bindings, format behavior, save behavior, and status text originate in `scripts/notepad.rss`.

## Reusable architecture

```text
GPUI native callback
  -> RssGpuiRuntime::dispatch(UiEvent)
  -> DispatchProgram resolves the event name through its handler table
  -> RssGpuiRuntime executes the selected RSS document
  -> dynamic ui::* hosts construct UiTree and mutate UiState
  -> optional app HostModule custom hosts run
  -> generic GPUI renderer reconciles UiTree into native elements
```

There is no polling, hand-written single-loop event dispatcher, or script-side event chain. Each GPUI click and text-change callback immediately dispatches a typed `UiEvent`. `DispatchProgram` is reusable by any desktop app that supplies an RSS document and optional host modules.

## Framework modules

| File | Responsibility |
|---|---|
| `src/rss_gpui/model.rs` | Public `UiTree`, `UiNode`, `NodeKind`, `Layout`, `UiState`, `UiEvent`, `ScriptError` types |
| `src/rss_gpui/builder.rs` | Validates ordered `ui::*` builder calls and produces one `UiTree` per script execution |
| `src/rss_gpui/dispatch.rs` | Parses `ui::on(event, || { ... })` syntax sugar and indexes independent handler sources by event name |
| `src/rss_gpui/runtime.rs` | Source loading, import allow-listing, fuel limits, per-execution context, host binding, and event dispatch |
| `src/rss_gpui/renderer.rs` | Generic GPUI renderer: maps `UiNode` types to GPUI elements and attaches callbacks from script-provided bindings |
| `src/notepad_hosts.rs` | Example-only `notepad::*` custom host functions for text formatting and Markdown persistence |
| `src/main.rs` | Minimal app shell: loads `scripts/notepad.rss`, registers `NotepadHostModule`, and opens `RssGpuiView` |

`HostModule` is the framework extension point. Its `bind(vm, execution_context)` method adds an app namespace while the generic framework always binds `ui::*`. A new desktop project reuses `rss_gpui`, writes its own RSS document, and adds only its own host module(s).

## RSS UI contract

The document uses imperative builder calls so host-side validation can report the exact malformed operation. This is declarative from the app author's view: the RSS source declares the complete current view and its event bindings on every execution.

```rustscript
use ui;
use notepad;

ui::window("RustScript GPUI Notepad", 760, 560);
ui::column_begin("root");
ui::text_input("title", "Title", "Untitled", "Note title");
ui::text_area("body", "Body", "", "Write here...");
ui::row_begin("actions");
ui::button("format", "Format");
ui::on_click("format", "format");
ui::button("save", "Save");
ui::on_click("save", "save");
ui::row_end();
ui::label("status", ui::get_value("status"));
ui::column_end();

ui::on("format", || {
    let formatted = notepad::format_note(ui::get_value("body"));
    ui::set_value("body", formatted);
    ui::set_value("status", "Formatted through RustScript");
});

ui::on("save", || {
    let saved = notepad::save_note(ui::get_value("title"), ui::get_value("body"));
    ui::set_value("status", saved);
});
ui::finish();
```

`text_input` and `text_area` initialize defaults only when a field has no value. Their native GPUI change callbacks emit `UiEvent::InputChanged { id, value }`. `on_click` maps a button node ID to an arbitrary RSS event name. `ui::on` registers an independent zero-argument handler body. `DispatchProgram` removes those handler declarations from the render source and selects one indexed handler source for a matching event. The generic renderer attaches each native listener from the button mapping.

## Custom host functions

| Host | App owner | Behavior |
|---|---|---|
| `ui::*` | reusable framework | Builds UI, reads/writes UI state, binds events |
| `notepad::format_note(text)` | notepad example | Trims trailing whitespace and collapses excess blank lines |
| `notepad::save_note(title, body)` | notepad example | Sanitizes a filename and writes `notes/<slug>.md` |

The framework binds `ui::*` and the application module with `Vm::bind_args_function`, each capturing an `Arc<Mutex<ExecutionContext>>` that owns the current builder, state, event, and application services. There are no globals or thread-local state. The execution uses the runtime-only `pd-vm` profile, checks imported names and arities against the registered module list, rejects unbound imports, enforces a source-size limit, and enables bounded fuel.

## Verification

- Unit-test builder nesting, missing `finish`, duplicate node IDs, invalid click bindings, default values, and UI-state mutation.
- Unit-test event dispatch: independent `ui::on` handlers must route through the shared dispatch table and produce state changes.
- Integration-test custom `notepad::*` calls through the RSS document: format alters `body`; save writes expected Markdown.
- Build the native GPUI binary and run the RSS event integration tests that dispatch `format` then `save` through the same reusable runtime.
- Launch the desktop application under a display server or Xvfb and confirm the GPUI window remains active.
