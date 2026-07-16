use std::collections::{BTreeMap, BTreeSet};

use super::model::{NodeKind, ScriptError, UiNode, UiTree, WindowSpec};

pub struct UiBuilder {
    window: Option<WindowSpec>,
    stack: Vec<UiNode>,
    node_ids: BTreeSet<String>,
    click_bindings: BTreeMap<String, String>,
    value_bindings: Vec<(String, String)>,
    buttons: BTreeSet<String>,
    text_input_ids: BTreeSet<String>,
    text_area_ids: BTreeSet<String>,
    finished: bool,
}

impl UiBuilder {
    pub fn new() -> Self {
        Self {
            window: None,
            stack: vec![UiNode::root()],
            node_ids: BTreeSet::new(),
            click_bindings: BTreeMap::new(),
            value_bindings: Vec::new(),
            buttons: BTreeSet::new(),
            text_input_ids: BTreeSet::new(),
            text_area_ids: BTreeSet::new(),
            finished: false,
        }
    }

    pub fn window(&mut self, title: &str, width: i64, height: i64) -> Result<(), ScriptError> {
        if self.window.is_some() {
            return Err(ScriptError::new("ui::window may only be called once"));
        }
        if width <= 0 || height <= 0 {
            return Err(ScriptError::new("ui::window dimensions must be positive"));
        }
        self.window = Some(WindowSpec {
            title: title.into(),
            width,
            height,
        });
        Ok(())
    }

    pub fn column_begin(&mut self, id: &str) -> Result<(), ScriptError> {
        self.begin_container(id, NodeKind::Column)
    }

    pub fn row_begin(&mut self, id: &str) -> Result<(), ScriptError> {
        self.begin_container(id, NodeKind::Row)
    }

    pub fn column_end(&mut self) -> Result<(), ScriptError> {
        self.end_container(NodeKind::Column)
    }

    pub fn row_end(&mut self) -> Result<(), ScriptError> {
        self.end_container(NodeKind::Row)
    }

    pub fn label(&mut self, id: &str, text: &str) -> Result<(), ScriptError> {
        self.insert_leaf(id, NodeKind::Label { text: text.into() })
    }

    pub fn text_input(
        &mut self,
        id: &str,
        label: &str,
        placeholder: &str,
    ) -> Result<(), ScriptError> {
        self.text_input_ids.insert(id.into());
        self.insert_leaf(
            id,
            NodeKind::TextInput {
                label: label.into(),
                placeholder: placeholder.into(),
            },
        )
    }

    pub fn text_area(
        &mut self,
        id: &str,
        label: &str,
        placeholder: &str,
    ) -> Result<(), ScriptError> {
        self.text_area_ids.insert(id.into());
        self.insert_leaf(
            id,
            NodeKind::TextArea {
                label: label.into(),
                placeholder: placeholder.into(),
            },
        )
    }

    pub fn button(&mut self, id: &str, label: &str) -> Result<(), ScriptError> {
        self.insert_leaf(
            id,
            NodeKind::Button {
                label: label.into(),
            },
        )?;
        self.buttons.insert(id.into());
        Ok(())
    }

    pub fn bind_click(&mut self, node_id: &str, event_name: &str) -> Result<(), ScriptError> {
        if !self.buttons.contains(node_id) {
            return Err(ScriptError::new(format!(
                "ui::bind_click references unknown button '{node_id}'"
            )));
        }
        if event_name.is_empty() {
            return Err(ScriptError::new(
                "ui::bind_click event name must not be empty",
            ));
        }
        self.click_bindings
            .insert(node_id.into(), event_name.into());
        Ok(())
    }

    pub fn bind_value(&mut self, from: &str, to: &str) -> Result<(), ScriptError> {
        if from.is_empty() || to.is_empty() {
            return Err(ScriptError::new("ui::bind_value requires non-empty ids"));
        }
        if from == to {
            return Err(ScriptError::new(
                "ui::bind_value source and target must differ",
            ));
        }
        for id in [from, to] {
            if !self.text_input_ids.contains(id) && !self.text_area_ids.contains(id) {
                return Err(ScriptError::new(format!(
                    "ui::bind_value references unknown id '{id}'"
                )));
            }
        }
        self.value_bindings.push((from.into(), to.into()));
        Ok(())
    }

    pub fn finish(mut self) -> Result<UiTree, ScriptError> {
        if self.finished {
            return Err(ScriptError::new("ui::finish may only be called once"));
        }
        self.finished = true;
        let window = self
            .window
            .ok_or_else(|| ScriptError::new("ui::window must be called before ui::finish"))?;
        if self.stack.len() != 1 {
            return Err(ScriptError::new(
                "all UI containers must be closed before ui::finish",
            ));
        }
        let root = self.stack.pop().expect("root node is always present");
        Ok(UiTree::new(
            window,
            root,
            self.click_bindings,
            self.value_bindings,
        ))
    }

    fn begin_container(&mut self, id: &str, kind: NodeKind) -> Result<(), ScriptError> {
        self.ensure_open()?;
        self.reserve_id(id)?;
        self.stack.push(UiNode::container(id, kind));
        Ok(())
    }

    fn end_container(&mut self, expected: NodeKind) -> Result<(), ScriptError> {
        self.ensure_open()?;
        if self.stack.len() == 1 {
            return Err(ScriptError::new("cannot close the root UI container"));
        }
        let node = self.stack.pop().expect("checked stack length");
        if node.kind != expected {
            return Err(ScriptError::new(
                "UI containers must close in declaration order",
            ));
        }
        self.current_parent_mut()?.children.push(node);
        Ok(())
    }

    fn insert_leaf(&mut self, id: &str, kind: NodeKind) -> Result<(), ScriptError> {
        self.ensure_open()?;
        self.reserve_id(id)?;
        self.current_parent_mut()?
            .children
            .push(UiNode::leaf(id, kind));
        Ok(())
    }

    fn reserve_id(&mut self, id: &str) -> Result<(), ScriptError> {
        if id.is_empty() {
            return Err(ScriptError::new("UI node id must not be empty"));
        }
        if !self.node_ids.insert(id.into()) {
            return Err(ScriptError::new(format!("duplicate UI node id '{id}'")));
        }
        Ok(())
    }

    fn current_parent_mut(&mut self) -> Result<&mut UiNode, ScriptError> {
        self.stack
            .last_mut()
            .ok_or_else(|| ScriptError::new("UI builder has no active parent"))
    }

    fn ensure_open(&self) -> Result<(), ScriptError> {
        if self.finished {
            Err(ScriptError::new("UI builder has already finished"))
        } else {
            Ok(())
        }
    }
}

impl Default for UiBuilder {
    fn default() -> Self {
        Self::new()
    }
}
