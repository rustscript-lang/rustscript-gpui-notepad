use std::path::PathBuf;
use std::sync::Arc;

use vm::{
    CompileSourceFileOptions, HostApiCatalog, HostFunctionRegistry, HostImport, HostTypeSchema,
    SourceFlavor, ValueType, Vm, VmStatus, compile_source_with_flavor_and_options,
};

use super::catalog::{production_host_catalog, production_host_modules};
use super::model::{DispatchResult, ScriptError, UiEvent, UiState, UiTree};
use super::ui_hosts::ExecutionContext;
use crate::notepad_hosts::NotesDirectory;

const MAX_SOURCE_BYTES: usize = 64 * 1024;
const VM_FUEL: u64 = 100_000;
const VM_FUEL_CHECK_INTERVAL: u32 = 32;

pub struct RssGpuiRuntime {
    vm: Vm,
    state: UiState,
    last_tree: Option<UiTree>,
    has_run: bool,
}

impl RssGpuiRuntime {
    pub fn from_source(source: impl Into<String>) -> Result<Self, ScriptError> {
        Self::from_source_inner(source.into(), None)
    }

    pub fn from_source_with_notes(
        source: impl Into<String>,
        notes_directory: impl Into<PathBuf>,
    ) -> Result<Self, ScriptError> {
        Self::from_source_inner(source.into(), Some(notes_directory.into()))
    }

    fn from_source_inner(
        source: String,
        notes_directory: Option<PathBuf>,
    ) -> Result<Self, ScriptError> {
        if source.len() > MAX_SOURCE_BYTES {
            return Err(ScriptError::new(format!(
                "RSS source exceeds {MAX_SOURCE_BYTES} byte limit"
            )));
        }

        let catalog = production_host_catalog();
        let compiled = compile_source_with_flavor_and_options(
            &source,
            SourceFlavor::RustScript,
            CompileSourceFileOptions::default().with_host_api_catalog(Arc::clone(&catalog)),
        )
        .map_err(|error| ScriptError::new(error.to_string()))?;
        validate_imports(&compiled.program.imports, catalog.as_ref())?;

        let mut registry = HostFunctionRegistry::restricted();
        for module in production_host_modules() {
            module
                .install_from_catalog(&mut registry, catalog.as_ref())
                .map_err(|error| ScriptError::new(error.to_string()))?;
        }

        let mut vm = Vm::new(compiled.program);
        vm.set_fuel_check_interval(VM_FUEL_CHECK_INTERVAL)
            .map_err(|error| ScriptError::new(error.to_string()))?;
        for module in production_host_modules() {
            module
                .install_state_requirements(&mut vm)
                .map_err(|error| ScriptError::new(error.to_string()))?;
        }
        if let Some(notes_directory) = notes_directory {
            vm.host_context()
                .set_host_state(NotesDirectory::new(notes_directory))
                .map_err(|error| ScriptError::new(error.to_string()))?;
        }
        registry
            .bind_vm_cached(&mut vm)
            .map_err(|error| ScriptError::new(error.to_string()))?;

        Ok(Self {
            vm,
            state: UiState::default(),
            last_tree: None,
            has_run: false,
        })
    }

    pub fn render(&mut self) -> Result<DispatchResult, ScriptError> {
        if self.has_run {
            self.reset_vm()?;
            self.clear_stale_tree();
        }
        self.replace_context()?;
        self.vm.set_fuel(VM_FUEL);
        let status = match self.vm.run() {
            Ok(status) => status,
            Err(error) => {
                self.clear_stale_tree();
                return Err(ScriptError::new(error.to_string()));
            }
        };
        self.has_run = true;
        if status != VmStatus::Halted {
            self.clear_stale_tree();
            return Err(ScriptError::new(format!(
                "RSS script stopped with unexpected VM status {status:?}"
            )));
        }
        let result = match self.context_result() {
            Ok(result) => result,
            Err(error) => {
                self.clear_stale_tree();
                return Err(error);
            }
        };
        self.state = result.state.clone();
        self.last_tree = Some(result.tree.clone());
        Ok(result)
    }

    pub fn dispatch(&mut self, event: UiEvent) -> Result<DispatchResult, ScriptError> {
        if self.last_tree.is_none() {
            self.render()?;
        }

        match event {
            UiEvent::InputChanged { id, value } => {
                self.state.set(id.clone(), value);
                self.propagate_value_bindings_from(&id);
                self.render()
            }
            UiEvent::Click(node_id) => {
                let callback = self
                    .last_tree
                    .as_ref()
                    .and_then(|tree| tree.click_callback(&node_id))
                    .cloned()
                    .ok_or_else(|| {
                        ScriptError::new(format!(
                            "script did not declare a callback for '{node_id}'"
                        ))
                    })?;
                self.replace_context()?;
                self.vm.set_fuel(VM_FUEL);
                self.vm
                    .invoke_callable(callback, &[])
                    .map_err(|error| ScriptError::new(error.to_string()))?;
                self.state = self.context_state()?;
                self.render()
            }
        }
    }

    pub fn clear_tree_for_test(&mut self) {
        self.clear_stale_tree();
    }

    fn reset_vm(&mut self) -> Result<(), ScriptError> {
        self.vm.reset_for_reuse().map_err(|error| {
            self.clear_stale_tree();
            ScriptError::new(error.to_string())
        })?;
        if self.vm.scope_reset_pending() {
            self.clear_stale_tree();
            return Err(ScriptError::new(
                "RSS VM reset did not reach a reusable execution scope",
            ));
        }
        Ok(())
    }

    fn clear_stale_tree(&mut self) {
        self.last_tree = None;
    }

    fn replace_context(&mut self) -> Result<(), ScriptError> {
        let mut context = self.vm.host_context();
        context
            .ensure_host_state::<ExecutionContext>("rss_gpui::replace_context", "write")
            .map_err(|error| ScriptError::new(error.to_string()))?;
        let mut ui = context
            .host_state_mut::<ExecutionContext>("rss_gpui::replace_context", "write")
            .map_err(|error| ScriptError::new(error.to_string()))?;
        *ui = ExecutionContext::new(self.state.clone());
        Ok(())
    }

    fn context_result(&mut self) -> Result<DispatchResult, ScriptError> {
        let mut context = self.vm.host_context();
        context
            .ensure_host_state::<ExecutionContext>("rss_gpui::context_result", "read")
            .map_err(|error| ScriptError::new(error.to_string()))?;
        context
            .host_state_ref::<ExecutionContext>("rss_gpui::context_result", "read")
            .map_err(|error| ScriptError::new(error.to_string()))?
            .result()
    }

    fn context_state(&mut self) -> Result<UiState, ScriptError> {
        let mut context = self.vm.host_context();
        context
            .ensure_host_state::<ExecutionContext>("rss_gpui::context_state", "read")
            .map_err(|error| ScriptError::new(error.to_string()))?;
        Ok(context
            .host_state_ref::<ExecutionContext>("rss_gpui::context_state", "read")
            .map_err(|error| ScriptError::new(error.to_string()))?
            .state()
            .clone())
    }

    fn propagate_value_bindings_from(&mut self, source_id: &str) {
        let Some(tree) = self.last_tree.as_ref() else {
            return;
        };
        let Some(value) = self.state.value(source_id) else {
            return;
        };
        let value = value.to_owned();
        for target in tree.value_binding_targets(source_id) {
            self.state.set(target.to_string(), value.clone());
        }
    }
}

fn validate_imports(imports: &[HostImport], catalog: &HostApiCatalog) -> Result<(), ScriptError> {
    for import in imports {
        let Some(schema) = catalog.function(&import.name) else {
            return Err(ScriptError::new(format!(
                "RSS import '{}' is not registered",
                import.name
            )));
        };
        let expected_arity = u8::try_from(schema.params.len()).map_err(|_| {
            ScriptError::new(format!(
                "RSS import '{}' declares more parameters than a host import can encode",
                import.name
            ))
        })?;
        if import.arity != expected_arity {
            return Err(ScriptError::new(format!(
                "RSS import '{}' uses arity {}, expected {expected_arity}",
                import.name, import.arity
            )));
        }
        let expected_return = value_type_of(&schema.return_type)?;
        if import.return_type != expected_return {
            return Err(ScriptError::new(format!(
                "RSS import '{}' returns {:?}, expected {expected_return:?}",
                import.name, import.return_type
            )));
        }
    }
    Ok(())
}

fn value_type_of(schema: &HostTypeSchema) -> Result<ValueType, ScriptError> {
    match schema {
        HostTypeSchema::Unknown => Ok(ValueType::Unknown),
        HostTypeSchema::Null => Ok(ValueType::Null),
        HostTypeSchema::Int => Ok(ValueType::Int),
        HostTypeSchema::Float | HostTypeSchema::Number => Ok(ValueType::Float),
        HostTypeSchema::Bool => Ok(ValueType::Bool),
        HostTypeSchema::String => Ok(ValueType::String),
        HostTypeSchema::Bytes => Ok(ValueType::Bytes),
        HostTypeSchema::Array(_) => Ok(ValueType::Array),
        HostTypeSchema::Map(_) => Ok(ValueType::Map),
        HostTypeSchema::Optional(inner) => value_type_of(inner),
        HostTypeSchema::Callable { .. } => Ok(ValueType::Callable),
        HostTypeSchema::Named { .. } => Ok(ValueType::Map),
        HostTypeSchema::Resource(_) => Err(ScriptError::new(
            "RSS host catalogs must not expose resource returns from ui/notepad hosts",
        )),
    }
}
