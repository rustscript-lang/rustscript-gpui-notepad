use vm::{
    CallOutcome, CallReturn, HostAdapterDescriptor, HostBindingDescriptor, HostBindingKind,
    HostEffect, HostFunctionDescriptor, HostFunctionSchema, HostModuleDescriptor, HostParamSchema,
    HostState, HostStateEffect, HostTypeSchema, Value, Vm, VmError, VmResult,
};

use super::builder::UiBuilder;
use super::model::{DispatchResult, ScriptError, UiState, UiTree};

pub(crate) struct ExecutionContext {
    builder: Option<UiBuilder>,
    state: UiState,
    tree: Option<UiTree>,
}

impl ExecutionContext {
    pub(crate) fn new(state: UiState) -> Self {
        Self {
            builder: Some(UiBuilder::new()),
            state,
            tree: None,
        }
    }

    pub(crate) fn state(&self) -> &UiState {
        &self.state
    }

    pub(crate) fn result(&self) -> Result<DispatchResult, ScriptError> {
        let tree = self
            .tree
            .clone()
            .ok_or_else(|| ScriptError::new("script must call ui::finish"))?;
        Ok(DispatchResult {
            tree,
            state: self.state.clone(),
        })
    }

    fn builder_mut(&mut self) -> Result<&mut UiBuilder, VmError> {
        self.builder
            .as_mut()
            .ok_or_else(|| VmError::HostError("ui::finish already completed this render".into()))
    }

    fn finish(&mut self) -> VmResult<()> {
        if self.tree.is_some() {
            return Err(VmError::HostError(
                "ui::finish may only be called once".into(),
            ));
        }
        let builder = self
            .builder
            .take()
            .ok_or_else(|| VmError::HostError("ui::finish has no active builder".into()))?;
        self.tree = Some(
            builder
                .finish()
                .map_err(|error| VmError::HostError(error.to_string()))?,
        );
        Ok(())
    }
}

impl HostState for ExecutionContext {
    const KEY: &'static str = "ui.execution_context";

    fn initialize() -> Result<Self, String> {
        Ok(Self::new(UiState::default()))
    }
}

pub fn ui_host_module() -> HostModuleDescriptor {
    HostModuleDescriptor {
        name: "ui",
        functions: &[
            ui_window_descriptor,
            ui_column_begin_descriptor,
            ui_column_end_descriptor,
            ui_row_begin_descriptor,
            ui_row_end_descriptor,
            ui_label_descriptor,
            ui_text_input_descriptor,
            ui_text_area_descriptor,
            ui_button_descriptor,
            ui_bind_value_descriptor,
            ui_get_value_descriptor,
            ui_set_value_descriptor,
            ui_finish_descriptor,
        ],
        resources: &[],
    }
}

fn ui_write_descriptor(
    schema: HostFunctionSchema,
    adapter: fn(&mut Vm, &[Value]) -> VmResult<CallOutcome>,
) -> HostFunctionDescriptor {
    HostFunctionDescriptor {
        schema,
        binding: HostBindingDescriptor {
            kind: HostBindingKind::StaticStack,
        },
        effects: vec![HostEffect::HostState(HostStateEffect::write::<
            ExecutionContext,
        >())],
        adapter: HostAdapterDescriptor::StaticStack(adapter),
        resource_types: Vec::new(),
    }
}

fn ui_window_descriptor() -> HostFunctionDescriptor {
    ui_write_descriptor(
        HostFunctionSchema::with_return(
            "ui::window",
            vec![
                HostParamSchema::value("title", HostTypeSchema::String),
                HostParamSchema::value("width", HostTypeSchema::Int),
                HostParamSchema::value("height", HostTypeSchema::Int),
            ],
            HostTypeSchema::Null,
        )
        .with_description("Declare the GPUI window title and size"),
        ui_window,
    )
}

fn ui_column_begin_descriptor() -> HostFunctionDescriptor {
    ui_write_descriptor(
        HostFunctionSchema::with_return(
            "ui::column_begin",
            vec![HostParamSchema::value("id", HostTypeSchema::String)],
            HostTypeSchema::Null,
        )
        .with_description("Open a column container"),
        ui_column_begin,
    )
}

fn ui_column_end_descriptor() -> HostFunctionDescriptor {
    ui_write_descriptor(
        HostFunctionSchema::with_return("ui::column_end", Vec::new(), HostTypeSchema::Null)
            .with_description("Close the current column container"),
        ui_column_end,
    )
}

fn ui_row_begin_descriptor() -> HostFunctionDescriptor {
    ui_write_descriptor(
        HostFunctionSchema::with_return(
            "ui::row_begin",
            vec![HostParamSchema::value("id", HostTypeSchema::String)],
            HostTypeSchema::Null,
        )
        .with_description("Open a row container"),
        ui_row_begin,
    )
}

fn ui_row_end_descriptor() -> HostFunctionDescriptor {
    ui_write_descriptor(
        HostFunctionSchema::with_return("ui::row_end", Vec::new(), HostTypeSchema::Null)
            .with_description("Close the current row container"),
        ui_row_end,
    )
}

fn ui_label_descriptor() -> HostFunctionDescriptor {
    ui_write_descriptor(
        HostFunctionSchema::with_return(
            "ui::label",
            vec![
                HostParamSchema::value("id", HostTypeSchema::String),
                HostParamSchema::value("text", HostTypeSchema::String),
            ],
            HostTypeSchema::Null,
        )
        .with_description("Insert a text label"),
        ui_label,
    )
}

fn ui_text_input_descriptor() -> HostFunctionDescriptor {
    ui_write_descriptor(
        HostFunctionSchema::with_return(
            "ui::text_input",
            vec![
                HostParamSchema::value("id", HostTypeSchema::String),
                HostParamSchema::value("label", HostTypeSchema::String),
                HostParamSchema::value("default_value", HostTypeSchema::String),
                HostParamSchema::value("placeholder", HostTypeSchema::String),
            ],
            HostTypeSchema::Null,
        )
        .with_description("Insert a single-line text input"),
        ui_text_input,
    )
}

fn ui_text_area_descriptor() -> HostFunctionDescriptor {
    ui_write_descriptor(
        HostFunctionSchema::with_return(
            "ui::text_area",
            vec![
                HostParamSchema::value("id", HostTypeSchema::String),
                HostParamSchema::value("label", HostTypeSchema::String),
                HostParamSchema::value("default_value", HostTypeSchema::String),
                HostParamSchema::value("placeholder", HostTypeSchema::String),
            ],
            HostTypeSchema::Null,
        )
        .with_description("Insert a multi-line text area"),
        ui_text_area,
    )
}

fn ui_button_descriptor() -> HostFunctionDescriptor {
    ui_write_descriptor(
        HostFunctionSchema::with_return(
            "ui::button",
            vec![
                HostParamSchema::value("id", HostTypeSchema::String),
                HostParamSchema::value("label", HostTypeSchema::String),
                HostParamSchema::value(
                    "callback",
                    HostTypeSchema::Callable {
                        params: Vec::new(),
                        result: Box::new(HostTypeSchema::Unknown),
                    },
                ),
            ],
            HostTypeSchema::Null,
        )
        .with_description(
            "Insert a button whose callback is stored on UiTree and invoked with invoke_callable; \
             the Unknown callback result is allowlisted because GPUI discards it after invocation",
        ),
        ui_button,
    )
}

fn ui_bind_value_descriptor() -> HostFunctionDescriptor {
    ui_write_descriptor(
        HostFunctionSchema::with_return(
            "ui::bind_value",
            vec![
                HostParamSchema::value("source", HostTypeSchema::String),
                HostParamSchema::value("target", HostTypeSchema::String),
            ],
            HostTypeSchema::Null,
        )
        .with_description("Bind one input value onto another"),
        ui_bind_value,
    )
}

fn ui_get_value_descriptor() -> HostFunctionDescriptor {
    HostFunctionDescriptor {
        schema: HostFunctionSchema::with_return(
            "ui::get_value",
            vec![HostParamSchema::value("id", HostTypeSchema::String)],
            HostTypeSchema::String,
        )
        .with_description("Read a UI state string"),
        binding: HostBindingDescriptor {
            kind: HostBindingKind::StaticStack,
        },
        effects: vec![HostEffect::HostState(HostStateEffect::read::<
            ExecutionContext,
        >())],
        adapter: HostAdapterDescriptor::StaticStack(ui_get_value),
        resource_types: Vec::new(),
    }
}

fn ui_set_value_descriptor() -> HostFunctionDescriptor {
    ui_write_descriptor(
        HostFunctionSchema::with_return(
            "ui::set_value",
            vec![
                HostParamSchema::value("id", HostTypeSchema::String),
                HostParamSchema::value("value", HostTypeSchema::String),
            ],
            HostTypeSchema::Null,
        )
        .with_description("Write a UI state string"),
        ui_set_value,
    )
}

fn ui_finish_descriptor() -> HostFunctionDescriptor {
    ui_write_descriptor(
        HostFunctionSchema::with_return("ui::finish", Vec::new(), HostTypeSchema::Null)
            .with_description("Finish the current UI tree"),
        ui_finish,
    )
}

fn ui_window(vm: &mut Vm, args: &[Value]) -> VmResult<CallOutcome> {
    with_ui_mut(vm, "ui::window", |ui| {
        host_result(ui.builder_mut()?.window(
            string_arg(args, 0, "ui::window")?.as_str(),
            int_arg(args, 1, "ui::window")?,
            int_arg(args, 2, "ui::window")?,
        ))?;
        unit()
    })
}

fn ui_column_begin(vm: &mut Vm, args: &[Value]) -> VmResult<CallOutcome> {
    with_ui_mut(vm, "ui::column_begin", |ui| {
        host_result(
            ui.builder_mut()?
                .column_begin(string_arg(args, 0, "ui::column_begin")?.as_str()),
        )?;
        unit()
    })
}

fn ui_column_end(vm: &mut Vm, _args: &[Value]) -> VmResult<CallOutcome> {
    with_ui_mut(vm, "ui::column_end", |ui| {
        host_result(ui.builder_mut()?.column_end())?;
        unit()
    })
}

fn ui_row_begin(vm: &mut Vm, args: &[Value]) -> VmResult<CallOutcome> {
    with_ui_mut(vm, "ui::row_begin", |ui| {
        host_result(
            ui.builder_mut()?
                .row_begin(string_arg(args, 0, "ui::row_begin")?.as_str()),
        )?;
        unit()
    })
}

fn ui_row_end(vm: &mut Vm, _args: &[Value]) -> VmResult<CallOutcome> {
    with_ui_mut(vm, "ui::row_end", |ui| {
        host_result(ui.builder_mut()?.row_end())?;
        unit()
    })
}

fn ui_label(vm: &mut Vm, args: &[Value]) -> VmResult<CallOutcome> {
    with_ui_mut(vm, "ui::label", |ui| {
        host_result(ui.builder_mut()?.label(
            string_arg(args, 0, "ui::label")?.as_str(),
            string_arg(args, 1, "ui::label")?.as_str(),
        ))?;
        unit()
    })
}

fn ui_text_input(vm: &mut Vm, args: &[Value]) -> VmResult<CallOutcome> {
    with_ui_mut(vm, "ui::text_input", |ui| {
        let id = string_arg(args, 0, "ui::text_input")?;
        let label = string_arg(args, 1, "ui::text_input")?;
        let default_value = string_arg(args, 2, "ui::text_input")?;
        let placeholder = string_arg(args, 3, "ui::text_input")?;
        ui.state.set_default(id.clone(), default_value);
        host_result(ui.builder_mut()?.text_input(&id, &label, &placeholder))?;
        unit()
    })
}

fn ui_text_area(vm: &mut Vm, args: &[Value]) -> VmResult<CallOutcome> {
    with_ui_mut(vm, "ui::text_area", |ui| {
        let id = string_arg(args, 0, "ui::text_area")?;
        let label = string_arg(args, 1, "ui::text_area")?;
        let default_value = string_arg(args, 2, "ui::text_area")?;
        let placeholder = string_arg(args, 3, "ui::text_area")?;
        ui.state.set_default(id.clone(), default_value);
        host_result(ui.builder_mut()?.text_area(&id, &label, &placeholder))?;
        unit()
    })
}

fn ui_button(vm: &mut Vm, args: &[Value]) -> VmResult<CallOutcome> {
    with_ui_mut(vm, "ui::button", |ui| {
        host_result(ui.builder_mut()?.button(
            string_arg(args, 0, "ui::button")?.as_str(),
            string_arg(args, 1, "ui::button")?.as_str(),
            callable_arg(args, 2, "ui::button")?,
        ))?;
        unit()
    })
}

fn ui_bind_value(vm: &mut Vm, args: &[Value]) -> VmResult<CallOutcome> {
    with_ui_mut(vm, "ui::bind_value", |ui| {
        host_result(ui.builder_mut()?.bind_value(
            string_arg(args, 0, "ui::bind_value")?.as_str(),
            string_arg(args, 1, "ui::bind_value")?.as_str(),
        ))?;
        unit()
    })
}

fn ui_get_value(vm: &mut Vm, args: &[Value]) -> VmResult<CallOutcome> {
    with_ui_ref(vm, "ui::get_value", |ui| {
        let id = string_arg(args, 0, "ui::get_value")?;
        Ok(CallOutcome::Return(CallReturn::one(Value::string(
            ui.state.value(&id).unwrap_or_default(),
        ))))
    })
}

fn ui_set_value(vm: &mut Vm, args: &[Value]) -> VmResult<CallOutcome> {
    with_ui_mut(vm, "ui::set_value", |ui| {
        ui.state.set(
            string_arg(args, 0, "ui::set_value")?,
            string_arg(args, 1, "ui::set_value")?,
        );
        unit()
    })
}

fn ui_finish(vm: &mut Vm, _args: &[Value]) -> VmResult<CallOutcome> {
    with_ui_mut(vm, "ui::finish", |ui| {
        ui.finish()?;
        unit()
    })
}

fn with_ui_mut(
    vm: &mut Vm,
    function: &'static str,
    op: impl FnOnce(&mut ExecutionContext) -> VmResult<CallOutcome>,
) -> VmResult<CallOutcome> {
    let mut context = vm.host_context();
    context
        .ensure_host_state::<ExecutionContext>(function, "write")
        .map_err(host_context_error)?;
    let mut ui = context
        .host_state_mut::<ExecutionContext>(function, "write")
        .map_err(host_context_error)?;
    op(&mut ui)
}

fn with_ui_ref(
    vm: &mut Vm,
    function: &'static str,
    op: impl FnOnce(&ExecutionContext) -> VmResult<CallOutcome>,
) -> VmResult<CallOutcome> {
    let mut context = vm.host_context();
    context
        .ensure_host_state::<ExecutionContext>(function, "read")
        .map_err(host_context_error)?;
    let ui = context
        .host_state_ref::<ExecutionContext>(function, "read")
        .map_err(host_context_error)?;
    op(&ui)
}

fn string_arg(args: &[Value], index: usize, host: &str) -> VmResult<String> {
    let Some(Value::String(value)) = args.get(index) else {
        return Err(VmError::HostError(format!(
            "{host} argument {index} must be a string"
        )));
    };
    Ok(value.as_str().into())
}

fn callable_arg(args: &[Value], index: usize, host: &str) -> VmResult<Value> {
    let Some(value @ Value::Callable(_)) = args.get(index) else {
        return Err(VmError::HostError(format!(
            "{host} argument {index} must be callable"
        )));
    };
    Ok(value.clone())
}

fn int_arg(args: &[Value], index: usize, host: &str) -> VmResult<i64> {
    let Some(Value::Int(value)) = args.get(index) else {
        return Err(VmError::HostError(format!(
            "{host} argument {index} must be an integer"
        )));
    };
    Ok(*value)
}

fn unit() -> VmResult<CallOutcome> {
    Ok(CallOutcome::Return(CallReturn::one(Value::Null)))
}

fn host_result<T>(result: Result<T, ScriptError>) -> VmResult<T> {
    result.map_err(|error| VmError::HostError(error.to_string()))
}

fn host_context_error(error: vm::HostContextError) -> VmError {
    VmError::HostError(error.to_string())
}
