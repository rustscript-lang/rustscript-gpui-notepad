# RustScript-driven GPUI Desktop Framework and Notepad Example

## Goal

Build a reusable GPUI framework where `.rss` source is authoritative for UI declarations, widget construction, anonymous button callbacks, and application behavior. The notepad is an example application module; Rust provides the reusable renderer/runtime and explicitly registered custom host capabilities.

## Core rule

Rust never contains title/body/button business logic. It renders a generic `UiTree`, forwards native input and click events to `RssGpuiRuntime`, then applies the resulting tree/state. All widget IDs, labels, layout calls, callback behavior, format behavior, save behavior, and status text originate in `scripts/notepad.rss`.

## Reusable architecture

```text
GPUI native callback
  -> RssGpuiRuntime::dispatch(UiEvent)
  -> button click resolves its retained RSS callable
  -> Vm::invoke_callable executes the anonymous callback
  -> dynamic ui::* hosts mutate UiState
  -> runtime resets the VM and renders a fresh UiTree
  -> generic GPUI renderer reconciles native elements
```

There is no polling, script-side event chain, event-name table, or source-level dispatcher. Each rendered button owns its own anonymous callback value. The callback's Program ownership remains with the live VM; `reset_for_reuse` discards prior callback values only after the event finishes and before the next tree is built.

## Framework modules

| File | Responsibility |
|---|---|
| `src/rss_gpui/model.rs` | Public `UiTree`, `UiNode`, `NodeKind`, `UiState`, `UiEvent`, and `ScriptError` types; button callback values live in `UiTree`. |
| `src/rss_gpui/builder.rs` | Validates ordered `ui::*` builder calls and requires each button callback to be a callable value. |
| `src/rss_gpui/runtime.rs` | Source loading, import allow-listing, fuel limits, persistent VM/context, direct callable invocation, and tree rebuilds. |
| `src/rss_gpui/renderer.rs` | Generic GPUI renderer that attaches a native listener to each script-declared button. |
| `src/notepad_hosts.rs` | Example-only `notepad::*` custom host functions for text formatting and Markdown persistence. |
| `src/main.rs` | Minimal app shell that loads `scripts/notepad.rss`, registers `NotepadHostModule`, and opens `RssGpuiView`. |

`HostModule` is the framework extension point. Its `bind(vm, execution_context)` method adds an app namespace while the generic framework always binds `ui::*`. A new desktop project reuses `rss_gpui`, writes its own RSS document, and adds its own host module(s).

## RSS UI contract

The document uses imperative builder calls so host-side validation can report the exact malformed operation. From the app author's view, RSS declares the complete view and its anonymous callbacks.

```rustscript
use ui;
use notepad;

ui::window("RustScript GPUI Notepad", 760, 560);
ui::column_begin("root");
ui::text_input("title", "Title", "Untitled", "Note title");
ui::text_area("body", "Body", "", "Write here...");
ui::row_begin("actions");
ui::button("format", "Format", || format_note());
ui::button("save", "Save", || save_note());
ui::row_end();
ui::label("status", ui::get_value("status"));
ui::column_end();
ui::finish();

fn format_note() -> string {
    let formatted: string = notepad::format_note(ui::get_value("body"));
    ui::set_value("body", formatted);
    ui::set_value("status", "Formatted through RustScript");
    ui::get_value("status")
}

fn save_note() -> string {
    let saved: string = notepad::save_note(ui::get_value("title"), ui::get_value("body"));
    ui::set_value("status", saved);
    ui::get_value("status")
}
```

`text_input` and `text_area` initialize defaults only when a field has no value. Their native GPUI change callbacks emit `UiEvent::InputChanged { id, value }`. `ui::button` stores the provided anonymous callable on the button node. The generic renderer forwards the node ID; the runtime retrieves that callable, invokes it directly in the persistent VM, then rebuilds the UI with the updated state.

## Custom host functions

| Host | App owner | Behavior |
|---|---|---|
| `ui::*` | reusable framework | Builds UI, reads/writes UI state, stores callbacks, and links input values. |
| `notepad::format_note(text)` | notepad example | Trims trailing whitespace and collapses excess blank lines. |
| `notepad::save_note(title, body)` | notepad example | Sanitizes a filename and writes `notes/<slug>.md`. |

The framework binds `ui::*` and application modules with `Vm::bind_args_function`, each using the shared `Arc<Mutex<ExecutionContext>>` for the active builder and state. There are no globals or thread-local state. The runtime profile checks imported names and arities against the registered module list, rejects unbound imports, enforces a source-size limit, and uses bounded fuel.

## Verification

- Unit-test that `UiBuilder` rejects a non-callable button callback.
- Unit-test direct anonymous callback dispatch and renderer button discovery.
- Integration-test custom `notepad::*` calls through the RSS document: format alters `body`; save writes expected Markdown.
- Build the native GPUI binary and run the RSS event integration tests that dispatch `format` then `save` through the same reusable runtime.
- Launch the desktop application under a display server or Xvfb and confirm the GPUI window remains active.
