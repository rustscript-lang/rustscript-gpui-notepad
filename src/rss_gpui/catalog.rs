use std::sync::Arc;

use vm::{HostApiCatalog, HostApiCatalogError, HostFunctionDescriptor, HostModuleDescriptor};

use super::ui_hosts::ui_host_module;
use crate::notepad_hosts::notepad_host_module;

pub const UI_HOST_COUNT: usize = 13;
pub const NOTEPAD_HOST_COUNT: usize = 2;
pub const PRODUCTION_RSS_HOST_COUNT: usize = UI_HOST_COUNT + NOTEPAD_HOST_COUNT;

pub const UI_HOST_NAMES: [&str; UI_HOST_COUNT] = [
    "ui::window",
    "ui::column_begin",
    "ui::column_end",
    "ui::row_begin",
    "ui::row_end",
    "ui::label",
    "ui::text_input",
    "ui::text_area",
    "ui::button",
    "ui::bind_value",
    "ui::get_value",
    "ui::set_value",
    "ui::finish",
];

pub const NOTEPAD_HOST_NAMES: [&str; NOTEPAD_HOST_COUNT] =
    ["notepad::format_note", "notepad::save_note"];

pub fn production_host_modules() -> [HostModuleDescriptor; 2] {
    [ui_host_module(), notepad_host_module()]
}

pub fn compose_catalog(
    modules: &[HostModuleDescriptor],
) -> Result<HostApiCatalog, HostApiCatalogError> {
    let mut descriptors = Vec::new();
    for module in modules {
        descriptors.extend(module.descriptors());
    }
    HostFunctionDescriptor::collect_catalog(&descriptors)
}

pub fn production_host_catalog() -> Arc<HostApiCatalog> {
    Arc::new(
        compose_catalog(&production_host_modules())
            .unwrap_or_else(|error| panic!("production RSS host catalog must be valid: {error}")),
    )
}
