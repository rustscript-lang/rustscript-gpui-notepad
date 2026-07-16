use std::path::PathBuf;

use gpui::{App, AppContext as _, Application, Bounds, WindowBounds, WindowOptions, px, size};
use rustscript_gpui_notepad::notepad_runtime;
use rustscript_gpui_notepad::rss_gpui::renderer::RssGpuiView;

fn notes_directory() -> PathBuf {
    std::env::current_dir()
        .expect("current directory should be available")
        .join("notes")
}

fn main() {
    let notes_directory = notes_directory();

    Application::new().run(move |cx: &mut App| {
        gpui_component::init(cx);

        let mut runtime = notepad_runtime(notes_directory.clone())
            .expect("RustScript notepad runtime should initialize");
        let initial =
            RssGpuiView::initial(&mut runtime).expect("RSS should declare an initial GPUI tree");
        let bounds = Bounds::centered(None, size(px(760.0), px(560.0)), cx);

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            move |_, cx| cx.new(|_| RssGpuiView::from_initial(runtime, initial)),
        )
        .expect("GPUI window should open");
        cx.activate(true);
    });
}
