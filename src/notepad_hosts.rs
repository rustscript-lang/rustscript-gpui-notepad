use std::path::{Path, PathBuf};

use vm::{
    CallOutcome, CallReturn, HostAdapterDescriptor, HostBindingDescriptor, HostBindingKind,
    HostEffect, HostFunctionDescriptor, HostFunctionSchema, HostModuleDescriptor, HostParamSchema,
    HostState, HostStateEffect, HostTypeSchema, Value, Vm, VmError, VmResult,
};

pub struct NotesDirectory {
    path: PathBuf,
}

impl NotesDirectory {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    fn as_path(&self) -> &Path {
        &self.path
    }
}

impl HostState for NotesDirectory {
    const KEY: &'static str = "notepad.notes_directory";

    fn initialize() -> Result<Self, String> {
        Err("notepad notes directory is not configured".into())
    }
}

pub fn notepad_host_module() -> HostModuleDescriptor {
    HostModuleDescriptor {
        name: "notepad",
        functions: &[format_note_descriptor, save_note_descriptor],
        resources: &[],
    }
}

fn format_note_descriptor() -> HostFunctionDescriptor {
    HostFunctionDescriptor {
        schema: HostFunctionSchema::with_return(
            "notepad::format_note",
            vec![HostParamSchema::value("text", HostTypeSchema::String)],
            HostTypeSchema::String,
        )
        .with_description("Normalize notepad body whitespace"),
        binding: HostBindingDescriptor {
            kind: HostBindingKind::StaticNonYieldingArgs,
        },
        effects: Vec::new(),
        adapter: HostAdapterDescriptor::StaticNonYieldingArgs(format_note_host),
        resource_types: Vec::new(),
    }
}

fn save_note_descriptor() -> HostFunctionDescriptor {
    HostFunctionDescriptor {
        schema: HostFunctionSchema::with_return(
            "notepad::save_note",
            vec![
                HostParamSchema::value("title", HostTypeSchema::String),
                HostParamSchema::value("body", HostTypeSchema::String),
            ],
            HostTypeSchema::String,
        )
        .with_description("Write a markdown note and return its path"),
        binding: HostBindingDescriptor {
            kind: HostBindingKind::StaticStack,
        },
        effects: vec![HostEffect::HostState(
            HostStateEffect::read::<NotesDirectory>(),
        )],
        adapter: HostAdapterDescriptor::StaticStack(save_note_host),
        resource_types: Vec::new(),
    }
}

fn format_note_host(args: &[Value]) -> VmResult<CallOutcome> {
    let text = string_arg(args, 0, "notepad::format_note")?;
    Ok(CallOutcome::Return(CallReturn::one(Value::string(
        format_note(&text),
    ))))
}

fn save_note_host(vm: &mut Vm, args: &[Value]) -> VmResult<CallOutcome> {
    let title = string_arg(args, 0, "notepad::save_note")?;
    let body = string_arg(args, 1, "notepad::save_note")?;
    let mut context = vm.host_context();
    context
        .ensure_host_state::<NotesDirectory>("notepad::save_note", "read")
        .map_err(|error| VmError::HostError(error.to_string()))?;
    let directory = context
        .host_state_ref::<NotesDirectory>("notepad::save_note", "read")
        .map_err(|error| VmError::HostError(error.to_string()))?;
    let path = save_note(directory.as_path(), &title, &body)?;
    Ok(CallOutcome::Return(CallReturn::one(Value::string(
        path.to_string_lossy().into_owned(),
    ))))
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
