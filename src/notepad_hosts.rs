use std::path::{Path, PathBuf};
use std::sync::Arc;

use vm::{CallOutcome, CallReturn, Value, Vm, VmError, VmResult};

use crate::rss_gpui::model::ScriptError;
use crate::rss_gpui::runtime::{
    CallbackHost, ExecutionContextHandle, HostModule, HostSignature, RssGpuiRuntime,
};

pub struct NotepadHostModule {
    notes_directory: Arc<PathBuf>,
}

impl NotepadHostModule {
    pub fn new(notes_directory: impl Into<PathBuf>) -> Self {
        Self {
            notes_directory: Arc::new(notes_directory.into()),
        }
    }
}

impl HostModule for NotepadHostModule {
    fn signatures(&self) -> Vec<HostSignature> {
        vec![
            HostSignature {
                name: "notepad::format_note",
                arity: 1,
            },
            HostSignature {
                name: "notepad::save_note",
                arity: 2,
            },
        ]
    }

    fn bind(&self, vm: &mut Vm, _context: ExecutionContextHandle) -> Result<(), ScriptError> {
        vm.bind_args_function(
            "notepad::format_note",
            Box::new(CallbackHost::new(|args| {
                let text = string_arg(args, 0, "notepad::format_note")?;
                Ok(CallOutcome::Return(CallReturn::one(Value::string(
                    format_note(&text),
                ))))
            })),
        );

        let notes_directory = self.notes_directory.clone();
        vm.bind_args_function(
            "notepad::save_note",
            Box::new(CallbackHost::new(move |args| {
                let title = string_arg(args, 0, "notepad::save_note")?;
                let body = string_arg(args, 1, "notepad::save_note")?;
                let path = save_note(notes_directory.as_ref(), &title, &body)?;
                Ok(CallOutcome::Return(CallReturn::one(Value::string(
                    path.to_string_lossy().into_owned(),
                ))))
            })),
        );

        Ok(())
    }
}

pub fn notepad_runtime(notes_directory: impl Into<PathBuf>) -> Result<RssGpuiRuntime, ScriptError> {
    RssGpuiRuntime::from_source(
        include_str!("../scripts/notepad.rss"),
        vec![Arc::new(NotepadHostModule::new(notes_directory))],
    )
}

fn format_note(text: &str) -> String {
    let mut output = String::new();
    let mut blank_lines = 0;
    for line in text.lines() {
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            blank_lines += 1;
            if blank_lines > 1 {
                continue;
            }
        } else {
            blank_lines = 0;
        }
        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str(trimmed);
    }
    output
}

fn save_note(notes_directory: &Path, title: &str, body: &str) -> VmResult<PathBuf> {
    let path = notes_directory.join(format!("{}.md", slugify(title)));
    std::fs::create_dir_all(notes_directory).map_err(|error| {
        VmError::HostError(format!("could not create notes directory: {error}"))
    })?;
    std::fs::write(&path, format!("# {title}\n\n{body}\n"))
        .map_err(|error| VmError::HostError(format!("could not save note: {error}")))?;
    Ok(path)
}

fn slugify(title: &str) -> String {
    let mut slug = String::new();
    let mut previous_was_dash = false;
    for character in title.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
            previous_was_dash = false;
        } else if !previous_was_dash && !slug.is_empty() {
            slug.push('-');
            previous_was_dash = true;
        }
    }
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "untitled".into()
    } else {
        slug.into()
    }
}

fn string_arg(args: &[Value], index: usize, host: &str) -> VmResult<String> {
    let Some(Value::String(value)) = args.get(index) else {
        return Err(VmError::HostError(format!(
            "{host} argument {index} must be a string"
        )));
    };
    Ok(value.as_str().into())
}
