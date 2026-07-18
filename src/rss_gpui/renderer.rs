use std::collections::BTreeMap;

use gpui::{
    AnyElement, Context, Entity, IntoElement, Render, SharedString, Subscription, Window, div,
    prelude::*,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputEvent, InputState};

use super::model::{DispatchResult, NodeKind, ScriptError, UiEvent, UiNode, UiState, UiTree};
use super::runtime::RssGpuiRuntime;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RenderButton;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RenderPlan {
    buttons: BTreeMap<String, RenderButton>,
}

impl RenderPlan {
    pub fn from_tree(tree: &UiTree) -> Self {
        let mut plan = Self::default();
        collect_buttons(tree, &tree.root, &mut plan.buttons);
        plan
    }

    pub fn button(&self, id: &str) -> Option<&RenderButton> {
        self.buttons.get(id)
    }
}

fn collect_buttons(tree: &UiTree, node: &UiNode, buttons: &mut BTreeMap<String, RenderButton>) {
    if matches!(node.kind, NodeKind::Button { .. }) && tree.click_callback(&node.id).is_some() {
        buttons.insert(node.id.clone(), RenderButton);
    }
    for child in &node.children {
        collect_buttons(tree, child, buttons);
    }
}

pub struct RssGpuiView {
    runtime: RssGpuiRuntime,
    tree: UiTree,
    state: UiState,
    inputs: BTreeMap<String, Entity<InputState>>,
    subscriptions: Vec<Subscription>,
    error: Option<String>,
}

impl RssGpuiView {
    pub fn from_initial(runtime: RssGpuiRuntime, initial: DispatchResult) -> Self {
        Self {
            runtime,
            tree: initial.tree,
            state: initial.state,
            inputs: BTreeMap::new(),
            subscriptions: Vec::new(),
            error: None,
        }
    }

    pub fn initial(runtime: &mut RssGpuiRuntime) -> Result<DispatchResult, ScriptError> {
        runtime.render()
    }

    fn dispatch(&mut self, event: UiEvent, cx: &mut Context<Self>) {
        match self.runtime.dispatch(event) {
            Ok(result) => {
                self.tree = result.tree;
                self.state = result.state;
                self.error = None;
            }
            Err(error) => self.error = Some(error.to_string()),
        }
        cx.notify();
    }

    fn input(
        &mut self,
        id: &str,
        value: &str,
        placeholder: &str,
        multiline: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<InputState> {
        let input = if let Some(input) = self.inputs.get(id) {
            input.clone()
        } else {
            let input = cx.new(|cx| {
                InputState::new(window, cx)
                    .default_value(value.to_owned())
                    .placeholder(placeholder.to_owned())
                    .multi_line(multiline)
                    .rows(if multiline { 16 } else { 1 })
            });
            let input_id = id.to_string();
            let subscription = cx.subscribe(&input, move |view, input, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    view.dispatch(
                        UiEvent::InputChanged {
                            id: input_id.clone(),
                            value: input.read(cx).value().to_string(),
                        },
                        cx,
                    );
                }
            });
            self.subscriptions.push(subscription);
            self.inputs.insert(id.into(), input.clone());
            input
        };

        if input.read(cx).value().as_ref() != value {
            input.update(cx, |input, cx| {
                input.set_value(value.to_owned(), window, cx)
            });
        }
        input
    }

    fn render_node(
        &mut self,
        node: &UiNode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match &node.kind {
            NodeKind::Root | NodeKind::Column => div()
                .id(SharedString::from(node.id.clone()))
                .flex()
                .flex_col()
                .gap_3()
                .children(
                    node.children
                        .iter()
                        .map(|child| self.render_node(child, window, cx)),
                )
                .into_any_element(),
            NodeKind::Row => div()
                .id(SharedString::from(node.id.clone()))
                .flex()
                .flex_row()
                .gap_2()
                .children(
                    node.children
                        .iter()
                        .map(|child| self.render_node(child, window, cx)),
                )
                .into_any_element(),
            NodeKind::Label { text } => div()
                .id(SharedString::from(node.id.clone()))
                .text_sm()
                .child(text.clone())
                .into_any_element(),
            NodeKind::TextInput { label, placeholder } => {
                let value = self.state.value(&node.id).unwrap_or_default().to_owned();
                let input = self.input(&node.id, &value, placeholder, false, window, cx);
                div()
                    .id(SharedString::from(node.id.clone()))
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(label.clone())
                    .child(Input::new(&input).w_full())
                    .into_any_element()
            }
            NodeKind::TextArea { label, placeholder } => {
                let value = self.state.value(&node.id).unwrap_or_default().to_owned();
                let input = self.input(&node.id, &value, placeholder, true, window, cx);
                div()
                    .id(SharedString::from(node.id.clone()))
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(label.clone())
                    .child(Input::new(&input).w_full())
                    .into_any_element()
            }
            NodeKind::Button { label } => {
                let node_id = node.id.clone();
                Button::new(SharedString::from(node.id.clone()))
                    .label(label.clone())
                    .primary()
                    .on_click(cx.listener(move |view, _, _, cx| {
                        view.dispatch(UiEvent::Click(node_id.clone()), cx);
                    }))
                    .into_any_element()
            }
        }
    }
}

impl Render for RssGpuiView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let root = self.tree.root.clone();
        div()
            .size_full()
            .p_4()
            .flex()
            .flex_col()
            .gap_3()
            .child(self.render_node(&root, window, cx))
            .when_some(self.error.clone(), |element, error| {
                element.child(div().text_color(gpui::red()).child(error))
            })
    }
}
