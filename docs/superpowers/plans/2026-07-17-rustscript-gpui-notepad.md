# RustScript-driven GPUI Notepad Implementation Plan

## Goal

Keep `rss_gpui` reusable while making every GPUI button callback an anonymous RSS callable. Remove the event-name dispatcher and execute retained callbacks directly through the persistent VM.

## Architecture

1. `ui::button(id, label, callback)` accepts an anonymous zero-argument RSS callable.
2. `UiBuilder` validates that callback arguments are `Value::Callable` and stores them with button IDs in `UiTree`.
3. `RssGpuiRuntime` compiles and binds one VM per source document. A click retrieves the current tree's callable and invokes it through `Vm::invoke_callable`.
4. The callback updates the shared execution context state through existing `ui::*` hosts. The runtime then calls `Vm::reset_for_reuse`, renders the root document again, and replaces the old tree/callback values.
5. Input changes preserve two-way bindings, then follow the same root re-render path.

## File structure

- `src/rss_gpui/model.rs`: stores button callback values in `UiTree`.
- `src/rss_gpui/builder.rs`: validates `ui::button` callbacks.
- `src/rss_gpui/runtime.rs`: persistent VM, import validation, direct callable dispatch, and rebuild lifecycle.
- `src/rss_gpui/renderer.rs`: renders button nodes and forwards click IDs.
- `scripts/notepad.rss`, `scripts/two_way.rss`: use anonymous `ui::button` callbacks.
- `tests/dispatch_sugar_tests.rs`: validates independent anonymous callbacks without dispatcher routing.
- `tests/runtime_tests.rs`, `tests/builder_tests.rs`, `tests/renderer_tests.rs`: validate callable storage, validation, and rendering.

## Acceptance checks

- [x] Legacy named-event APIs and their source preprocessor are removed.
- [x] Button callbacks are callable values from the live program instance.
- [x] Input, format, and save behavior rebuild the tree after a state change.
- [x] Focused unit and integration tests cover anonymous callbacks, state updates, two-way binding, and notepad host side effects.
- [ ] `cargo fmt --all -- --check`, `cargo test --all-targets`, and Clippy complete cleanly.
- [ ] The native smoke path is exercised under the available display environment.
