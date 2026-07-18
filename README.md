# RustScript GPUI Notepad

A reusable RSS-to-GPUI desktop layer plus a native notepad. The `.rss` document owns widget declarations, layout nesting, anonymous button callbacks, and application behavior. Rust owns the generic GPUI renderer and explicitly registered host capabilities.

## Run

```bash
cargo run
```

Notes are written under `./notes/`.

## Define a desktop UI in RSS

`RssGpuiRuntime` executes an RSS source file, turns `ui::*` declarations into a typed `UiTree`, and the reusable `RssGpuiView` renders that tree. To create another desktop app, provide a source file and any application host modules:

```rust
let runtime = RssGpuiRuntime::from_source(
    include_str!("../scripts/app.rss"),
    vec![Arc::new(MyHostModule::new(app_services))],
)?;
```

Buttons receive anonymous RSS callbacks directly:

```text
use ui;
use app;

ui::window("Tasks", 720, 520);
ui::column_begin("root");
ui::text_input("title", "Title", "", "Task title");
ui::button("create", "Create", || create_task());
ui::label("status", ui::get_value("status"));
ui::column_end();
ui::finish();

fn create_task() -> string {
    let title: string = ui::get_value("title");
    let status: string = app::create_task(title);
    ui::set_value("status", status);
    ui::get_value("status")
}
```

The anonymous callback is retained with its rendered button and invoked through the same VM. After an input or callback changes state, the runtime resets the VM, rebuilds the `UiTree`, and replaces the prior callback values.

## RSS `ui` surface

| RSS call | Result |
|---|---|
| `ui::window(title, width, height)` | Declares the window metadata. |
| `ui::column_begin(id)` / `ui::column_end()` | Declares a vertical container. |
| `ui::row_begin(id)` / `ui::row_end()` | Declares a horizontal container. |
| `ui::text_input(id, label, default, placeholder)` | Declares a single-line editable control. |
| `ui::text_area(id, label, default, placeholder)` | Declares a multi-line editable control. |
| `ui::button(id, label, callback)` | Declares a clickable control with an anonymous zero-argument RSS callback. |
| `ui::get_value(id)` / `ui::set_value(id, value)` | Reads and updates UI state. |
| `ui::label(id, text)` | Declares text. |
| `ui::finish()` | Completes the frame declaration. |
| `ui::bind_value(from, to)` | Links two editable ids. Changing either field updates the linked partner without script-side assignment. Both ids must already be registered as `text_input` or `text_area`, and self-loops are rejected. |

## Host capabilities

A desktop app supplies a `HostModule`. It exposes a fixed list of import names and arities, then binds each host function to the VM. The framework validates every RSS import before execution. The notepad module provides `notepad::format_note` and `notepad::save_note`; the anonymous callbacks in `scripts/notepad.rss` invoke both.

There is no event-name dispatcher or source preprocessor. GPUI input subscriptions and click listeners call `RssGpuiRuntime::dispatch`; a click resolves the button's retained callable and invokes it directly. The runtime then renders a fresh typed UI tree from the updated state.

## Two-way binding example

`scripts/two_way.rss` demonstrates how `ui::bind_value` mirrors two inputs without manual assignments:

```text
ui::bind_value("a", "b");
ui::bind_value("b", "a");
```

When the user types in `a`, the runtime:

1. Writes the new value to `UiState`.
2. Walks the `value_bindings` stored in `UiTree` and copies the value into each linked target.
3. Renders a fresh UI tree and reconciles each target `InputState`.

One-way binding remains available for read-only mirrors:

```text
ui::bind_value("source", "read_only_display");
```

## Checks

```bash
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo run
```
