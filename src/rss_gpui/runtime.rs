use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use vm::{
    CallOutcome, CallReturn, HostArgsFunction, Value, Vm, VmError, VmResult, VmStatus,
    compile_source,
};

use super::builder::UiBuilder;
use super::model::{DispatchResult, ScriptError, UiEvent, UiState, UiTree};

const MAX_SOURCE_BYTES: usize = 64 * 1024;
const VM_FUEL: u64 = 100_000;
const VM_FUEL_CHECK_INTERVAL: u32 = 32;

pub type ExecutionContextHandle = Arc<Mutex<ExecutionContext>>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostSignature {
    pub name: &'static str,
    pub arity: u8,
}

pub trait HostModule: Send + Sync {
    fn signatures(&self) -> Vec<HostSignature>;
    fn bind(&self, vm: &mut Vm, context: ExecutionContextHandle) -> Result<(), ScriptError>;
}

pub struct CallbackHost {
    callback: Box<dyn Fn(&[Value]) -> VmResult<CallOutcome> + Send>,
}

impl CallbackHost {
    pub fn new(callback: impl Fn(&[Value]) -> VmResult<CallOutcome> + Send + 'static) -> Self {
        Self {
            callback: Box::new(callback),
        }
    }
}

impl HostArgsFunction for CallbackHost {
    fn call(&mut self, args: &[Value]) -> VmResult<CallOutcome> {
        (self.callback)(args)
    }
}

pub struct ExecutionContext {
    builder: Option<UiBuilder>,
    state: UiState,
    event_name: String,
    tree: Option<UiTree>,
}

impl ExecutionContext {
    fn new(state: UiState, event_name: impl Into<String>) -> Self {
        Self {
            builder: Some(UiBuilder::new()),
            state,
            event_name: event_name.into(),
            tree: None,
        }
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

    fn result(&self) -> Result<DispatchResult, ScriptError> {
        let tree = self
            .tree
            .clone()
            .ok_or_else(|| ScriptError::new("script must call ui::finish"))?;
        Ok(DispatchResult {
            tree,
            state: self.state.clone(),
        })
    }
}

#[derive(Clone, Copy)]
enum UiOperation {
    Window,
    ColumnBegin,
    ColumnEnd,
    RowBegin,
    RowEnd,
    Label,
    TextInput,
    TextArea,
    Button,
    BindClick,
    EventName,
    GetValue,
    SetValue,
    Finish,
}

impl UiOperation {
    fn signature(self) -> HostSignature {
        match self {
            Self::Window => HostSignature {
                name: "ui::window",
                arity: 3,
            },
            Self::ColumnBegin => HostSignature {
                name: "ui::column_begin",
                arity: 1,
            },
            Self::ColumnEnd => HostSignature {
                name: "ui::column_end",
                arity: 0,
            },
            Self::RowBegin => HostSignature {
                name: "ui::row_begin",
                arity: 1,
            },
            Self::RowEnd => HostSignature {
                name: "ui::row_end",
                arity: 0,
            },
            Self::Label => HostSignature {
                name: "ui::label",
                arity: 2,
            },
            Self::TextInput => HostSignature {
                name: "ui::text_input",
                arity: 4,
            },
            Self::TextArea => HostSignature {
                name: "ui::text_area",
                arity: 4,
            },
            Self::Button => HostSignature {
                name: "ui::button",
                arity: 2,
            },
            Self::BindClick => HostSignature {
                name: "ui::bind_click",
                arity: 2,
            },
            Self::EventName => HostSignature {
                name: "ui::event_name",
                arity: 0,
            },
            Self::GetValue => HostSignature {
                name: "ui::get_value",
                arity: 1,
            },
            Self::SetValue => HostSignature {
                name: "ui::set_value",
                arity: 2,
            },
            Self::Finish => HostSignature {
                name: "ui::finish",
                arity: 0,
            },
        }
    }
}

const UI_OPERATIONS: [UiOperation; 14] = [
    UiOperation::Window,
    UiOperation::ColumnBegin,
    UiOperation::ColumnEnd,
    UiOperation::RowBegin,
    UiOperation::RowEnd,
    UiOperation::Label,
    UiOperation::TextInput,
    UiOperation::TextArea,
    UiOperation::Button,
    UiOperation::BindClick,
    UiOperation::EventName,
    UiOperation::GetValue,
    UiOperation::SetValue,
    UiOperation::Finish,
];

pub struct RssGpuiRuntime {
    source: String,
    modules: Vec<Arc<dyn HostModule>>,
    state: UiState,
    last_tree: Option<UiTree>,
}

impl RssGpuiRuntime {
    pub fn from_source(
        source: impl Into<String>,
        modules: Vec<Arc<dyn HostModule>>,
    ) -> Result<Self, ScriptError> {
        let source = source.into();
        if source.len() > MAX_SOURCE_BYTES {
            return Err(ScriptError::new(format!(
                "RSS source exceeds {MAX_SOURCE_BYTES} byte limit"
            )));
        }
        Ok(Self {
            source,
            modules,
            state: UiState::default(),
            last_tree: None,
        })
    }

    pub fn dispatch(&mut self, event: UiEvent) -> Result<DispatchResult, ScriptError> {
        let event_name = match event {
            UiEvent::InputChanged { id, value } => {
                self.state.set(id.clone(), value);
                format!("input:{id}")
            }
            UiEvent::Click(node_id) => {
                if self.last_tree.is_none() {
                    let initial = self.execute("")?;
                    self.state = initial.state;
                    self.last_tree = Some(initial.tree);
                }
                self.last_tree
                    .as_ref()
                    .and_then(|tree| tree.click_event(&node_id))
                    .map(str::to_owned)
                    .ok_or_else(|| {
                        ScriptError::new(format!("script did not bind click event for '{node_id}'"))
                    })?
            }
        };

        let result = self.execute(&event_name)?;
        self.state = result.state.clone();
        self.last_tree = Some(result.tree.clone());
        Ok(result)
    }

    fn execute(&self, event_name: &str) -> Result<DispatchResult, ScriptError> {
        let compiled =
            compile_source(&self.source).map_err(|error| ScriptError::new(error.to_string()))?;
        let allowed = self.allowed_imports();
        for import in &compiled.program.imports {
            let expected_arity = allowed.get(import.name.as_str()).ok_or_else(|| {
                ScriptError::new(format!("RSS import '{}' is not registered", import.name))
            })?;
            if import.arity != *expected_arity {
                return Err(ScriptError::new(format!(
                    "RSS import '{}' uses arity {}, expected {}",
                    import.name, import.arity, expected_arity
                )));
            }
        }

        let context = Arc::new(Mutex::new(ExecutionContext::new(
            self.state.clone(),
            event_name,
        )));
        let mut vm = Vm::new(compiled.program);
        vm.set_fuel_check_interval(VM_FUEL_CHECK_INTERVAL)
            .map_err(|error| ScriptError::new(error.to_string()))?;
        vm.set_fuel(VM_FUEL);
        bind_ui_hosts(&mut vm, context.clone());
        for module in &self.modules {
            module.bind(&mut vm, context.clone())?;
        }

        let status = vm
            .run()
            .map_err(|error| ScriptError::new(error.to_string()))?;
        if status != VmStatus::Halted {
            return Err(ScriptError::new(format!(
                "RSS script stopped with unexpected VM status {status:?}"
            )));
        }
        drop(vm);

        context
            .lock()
            .map_err(|_| ScriptError::new("RSS execution context lock was poisoned"))?
            .result()
    }

    fn allowed_imports(&self) -> BTreeMap<&'static str, u8> {
        let mut imports = BTreeMap::new();
        for operation in UI_OPERATIONS {
            let signature = operation.signature();
            imports.insert(signature.name, signature.arity);
        }
        for module in &self.modules {
            for signature in module.signatures() {
                imports.insert(signature.name, signature.arity);
            }
        }
        imports
    }
}

fn bind_ui_hosts(vm: &mut Vm, context: ExecutionContextHandle) {
    for operation in UI_OPERATIONS {
        let signature = operation.signature();
        let context = context.clone();
        vm.bind_args_function(
            signature.name,
            Box::new(CallbackHost::new(move |args| {
                invoke_ui(operation, &context, args)
            })),
        );
    }
}

fn invoke_ui(
    operation: UiOperation,
    context: &ExecutionContextHandle,
    args: &[Value],
) -> VmResult<CallOutcome> {
    let mut context = context
        .lock()
        .map_err(|_| VmError::HostError("RSS execution context lock was poisoned".into()))?;
    match operation {
        UiOperation::Window => {
            host_result(context.builder_mut()?.window(
                string_arg(args, 0, "ui::window")?.as_str(),
                int_arg(args, 1, "ui::window")?,
                int_arg(args, 2, "ui::window")?,
            ))?;
            unit()
        }
        UiOperation::ColumnBegin => {
            host_result(
                context
                    .builder_mut()?
                    .column_begin(string_arg(args, 0, "ui::column_begin")?.as_str()),
            )?;
            unit()
        }
        UiOperation::ColumnEnd => {
            host_result(context.builder_mut()?.column_end())?;
            unit()
        }
        UiOperation::RowBegin => {
            host_result(
                context
                    .builder_mut()?
                    .row_begin(string_arg(args, 0, "ui::row_begin")?.as_str()),
            )?;
            unit()
        }
        UiOperation::RowEnd => {
            host_result(context.builder_mut()?.row_end())?;
            unit()
        }
        UiOperation::Label => {
            host_result(context.builder_mut()?.label(
                string_arg(args, 0, "ui::label")?.as_str(),
                string_arg(args, 1, "ui::label")?.as_str(),
            ))?;
            unit()
        }
        UiOperation::TextInput => {
            let id = string_arg(args, 0, "ui::text_input")?;
            let label = string_arg(args, 1, "ui::text_input")?;
            let default_value = string_arg(args, 2, "ui::text_input")?;
            let placeholder = string_arg(args, 3, "ui::text_input")?;
            context.state.set_default(id.clone(), default_value);
            host_result(context.builder_mut()?.text_input(&id, &label, &placeholder))?;
            unit()
        }
        UiOperation::TextArea => {
            let id = string_arg(args, 0, "ui::text_area")?;
            let label = string_arg(args, 1, "ui::text_area")?;
            let default_value = string_arg(args, 2, "ui::text_area")?;
            let placeholder = string_arg(args, 3, "ui::text_area")?;
            context.state.set_default(id.clone(), default_value);
            host_result(context.builder_mut()?.text_area(&id, &label, &placeholder))?;
            unit()
        }
        UiOperation::Button => {
            host_result(context.builder_mut()?.button(
                string_arg(args, 0, "ui::button")?.as_str(),
                string_arg(args, 1, "ui::button")?.as_str(),
            ))?;
            unit()
        }
        UiOperation::BindClick => {
            host_result(context.builder_mut()?.bind_click(
                string_arg(args, 0, "ui::bind_click")?.as_str(),
                string_arg(args, 1, "ui::bind_click")?.as_str(),
            ))?;
            unit()
        }
        UiOperation::EventName => Ok(CallOutcome::Return(CallReturn::one(Value::string(
            context.event_name.clone(),
        )))),
        UiOperation::GetValue => {
            let id = string_arg(args, 0, "ui::get_value")?;
            Ok(CallOutcome::Return(CallReturn::one(Value::string(
                context.state.value(&id).unwrap_or_default(),
            ))))
        }
        UiOperation::SetValue => {
            context.state.set(
                string_arg(args, 0, "ui::set_value")?,
                string_arg(args, 1, "ui::set_value")?,
            );
            unit()
        }
        UiOperation::Finish => {
            context.finish()?;
            unit()
        }
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
