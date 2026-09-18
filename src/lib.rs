use std::path::PathBuf;

use rss_gpui::model::ScriptError;
use rss_gpui::runtime::RssGpuiRuntime;

pub mod notepad_hosts;
pub mod rss_gpui;

pub const FROZEN_RUSTSCRIPT_REV: &str = "b1d6cffede77f49410bf63525f30b9a46b02dc01";

pub fn notepad_runtime(notes_directory: impl Into<PathBuf>) -> Result<RssGpuiRuntime, ScriptError> {
    RssGpuiRuntime::from_source_with_notes(include_str!("../scripts/notepad.rss"), notes_directory)
}
