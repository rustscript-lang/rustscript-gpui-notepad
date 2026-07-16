# RustScript-driven GPUI Desktop Framework and Notepad Example

## Goal

Build a reusable, event-driven GPUI framework where `.rss` source is authoritative for UI declarations, widget construction, event bindings, and application behavior. The notepad is an example application module; Rust provides only the reusable renderer/runtime and explicitly registered custom host capabilities.

## Core rule

Rust never contains title/body/button business logic. It renders a generic `UiTree`, forwards native input and click events to `RssGpuiRuntime`, then applies the resulting tree/state. All widget IDs, labels, layout calls, click-to-event bindings, format behavior, save behavior, and status text originate in `scripts/notepad.rss`.

## Reusable architecture

```text
GPUI native callback
  -> RssGpuiController::dispatch(UiEvent)
  -> RssGpuiRuntime executes the selected .rss document
  -> dynamic ui::* hosts construct UiTree and mutate UiState
  -> optional app HostModule custom hosts run
  -> generic GPUI renderer reconciles UiTree into native elements
```

There is no polling or hand-written single-loop event dispatcher. Each GPUI click and text-change callback immediately dispatches a typed `UiEvent` through the controller. The controller is reusable by any desktop app that supplies an RSS document and optional host modules.

## Framework modules

| File | Responsibility |
|---|---|
| `src/rss_gpui/model.rs` | Public `UiTree`, `UiNode`, `NodeKind`, `Layout`, `UiState`, `UiEvent`, `ScriptError` types |
| `src/rss_gpui/builder.rs` | Validates ordered `ui::*` builder calls and produces one `UiTree` per script execution |
| `src/rss_gpui/runtime.rs` | Source loading, import allow-listing, fuel limits, per-execution context, host binding, and event dispatch |
| `src/rss_gpui/hosts.rs` | Reusable dynamic `ui::*` host factories: window, column, row, label, text input, text area, button, bind_click, get_value, set_value, set_status, event_name, finish |
| `src/rss_gpui/renderer.rs` | Generic GPUI renderer: maps `UiNode` types to GPUI elements and attaches callbacks from script-provided bindings |
| `src/notepad_hosts.rs` | Example-only `notepad::*` custom host functions for text formatting and Markdown persistence |
| `src/main.rs` | Minimal app shell: loads `scripts/notepad.rss`, registers `NotepadHostModule`, delegates rendering to `RssGpuiController` |

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
ui::bind_click("format", "format");
ui::button("save", "Save");
ui::bind_click("save", "save");
ui::row_end();
ui::label("status", ui::get_value("status"));
ui::column_end();

let event = ui::event_name();
if event == "format" {
    let formatted = notepad::format_note(ui::get_value("body"));
    ui::set_value("body", formatted);
    ui::set_value("status", "Formatted through RustScript");
} else if event == "save" {
    let saved = notepad::save_note(ui::get_value("title"), ui::get_value("body"));
    ui::set_value("status", saved);
}
ui::finish();
```

`text_input` and `text_area` initialize defaults only when a field has no value. Their native GPUI change callbacks emit `UiEvent::InputChanged { id, value }`. `bind_click` maps a button node ID to an arbitrary RSS event name. The generic renderer attaches each native listener from that mapping.

## Custom host functions

| Host | App owner | Behavior |
|---|---|---|
| `ui::*` | reusable framework | Builds UI, reads/writes UI state, binds events |
| `notepad::format_note(text)` | notepad example | Trims trailing whitespace and collapses excess blank lines |
| `notepad::save_note(title, body)` | notepad example | Sanitizes a filename and writes `notes/<slug>.md` |

The framework binds `ui::*` and the application module with `Vm::bind_args_function`, each capturing an `Arc<Mutex<ExecutionContext>>` that owns the current builder, state, event, and application services. There are no globals or thread-local state. The execution uses the runtime-only `pd-vm` profile, checks imported names and arities against the registered module list, rejects unbound imports, enforces a source-size limit, and enables bounded fuel.

## Verification

- Unit-test builder nesting, missing `finish`, duplicate node IDs, invalid click bindings, default values, and UI-state mutation.
- Unit-test event dispatch: a scripted button binding must invoke the matching RSS branch and produce state changes.
- Integration-test custom `notepad::*` calls through the RSS document: format alters `body`; save writes expected Markdown.
- Build the native GPUI binary and execute `--script-smoke`, which dispatches `format` then `save` through the same reusable controller without creating a window.
- Manually launch the desktop application when a display server is available and use the buttons to confirm native callbacks update the rendered tree.
