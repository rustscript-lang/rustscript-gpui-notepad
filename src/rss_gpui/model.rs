use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};

use vm::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptError {
    message: String,
}

impl ScriptError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for ScriptError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ScriptError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowSpec {
    pub title: String,
    pub width: i64,
    pub height: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeKind {
    Root,
    Column,
    Row,
    Label { text: String },
    TextInput { label: String, placeholder: String },
    TextArea { label: String, placeholder: String },
    Button { label: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiNode {
    pub id: String,
    pub kind: NodeKind,
    pub children: Vec<UiNode>,
}

impl UiNode {
    pub fn root() -> Self {
        Self {
            id: "__root__".into(),
            kind: NodeKind::Root,
            children: Vec::new(),
        }
    }

    pub fn container(id: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            id: id.into(),
            kind,
            children: Vec::new(),
        }
    }

    pub fn leaf(id: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            id: id.into(),
            kind,
            children: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct UiTree {
    pub window: WindowSpec,
    pub root: UiNode,
    click_callbacks: BTreeMap<String, Value>,
    value_bindings: Vec<(String, String)>,
}

impl UiTree {
    pub(crate) fn new(
        window: WindowSpec,
        root: UiNode,
        click_callbacks: BTreeMap<String, Value>,
        value_bindings: Vec<(String, String)>,
    ) -> Self {
        Self {
            window,
            root,
            click_callbacks,
            value_bindings,
        }
    }

    pub fn click_callback(&self, node_id: &str) -> Option<&Value> {
        self.click_callbacks.get(node_id)
    }

    pub fn value_binding_targets(&self, source: &str) -> Vec<&str> {
        self.value_bindings
            .iter()
            .filter_map(|(from, to)| {
                if from == source {
                    Some(to.as_str())
                } else {
                    None
                }
            })
            .collect()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UiState {
    values: BTreeMap<String, String>,
}

impl UiState {
    pub fn value(&self, id: &str) -> Option<&str> {
        self.values.get(id).map(String::as_str)
    }

    pub fn set(&mut self, id: impl Into<String>, value: impl Into<String>) {
        self.values.insert(id.into(), value.into());
    }

    pub fn set_default(&mut self, id: impl Into<String>, value: impl Into<String>) {
        self.values.entry(id.into()).or_insert_with(|| value.into());
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UiEvent {
    Click(String),
    InputChanged { id: String, value: String },
}

#[derive(Clone, Debug)]
pub struct DispatchResult {
    pub tree: UiTree,
    pub state: UiState,
}
