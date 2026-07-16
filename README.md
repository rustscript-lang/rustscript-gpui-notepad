# RustScript GPUI Notepad

A reusable RSS-to-GPUI desktop layer plus a native notepad. The `.rss` file owns widget declarations, layout nesting, button event names, and event handling. Rust owns only the generic GPUI renderer and explicitly registered host capabilities.

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

RSS declares the controls and binds events:

```text
use ui;
use app;

ui::window("Tasks", 720, 520);
ui::column_begin("root");
ui::text_input("title", "Title", "", "Task title");
ui::button("create", "Create");
ui::on_click("create", "create-task");
ui::label("status", ui::get_value("status"));
ui::column_end();

let event: string = ui::event_name();
if event == "create-task" {
    let title: string = ui::get_value("title");
    let status: string = app::create_task(title);
    ui::set_value("status", status);
}
ui::finish();
```

## RSS `ui` surface

| RSS call | Result |
|---|---|
| `ui::window(title, width, height)` | Declares the window metadata. |
| `ui::column_begin(id)` / `ui::column_end()` | Declares a vertical container. |
| `ui::row_begin(id)` / `ui::row_end()` | Declares a horizontal container. |
| `ui::text_input(id, label, default, placeholder)` | Declares a single-line editable control. |
| `ui::text_area(id, label, default, placeholder)` | Declares a multi-line editable control. |
| `ui::button(id, label)` | Declares a clickable control. |
| `ui::on_click(id, event)` | Binds the button to an RSS event name. |
| `ui::event_name()` | Reads the active event name. |
| `ui::get_value(id)` / `ui::set_value(id, value)` | Reads and updates UI state. |
| `ui::label(id, text)` | Declares text. |
| `ui::finish()` | Completes the frame declaration. |

## Host capabilities

A desktop app supplies a `HostModule`. It exposes a fixed list of import names and arities, then binds each host function to the VM. The framework validates every RSS import before execution. The notepad module provides `notepad::format_note` and `notepad::save_note`; event handling in `scripts/notepad.rss` invokes both through regular button clicks.

The runtime has no application event loop. GPUI input subscriptions and click callbacks call `RssGpuiRuntime::dispatch`, which executes the declared RSS event and produces the next typed UI tree.

## Checks

```bash
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo run
```
