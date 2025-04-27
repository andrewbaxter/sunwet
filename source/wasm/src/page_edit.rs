use {
    super::state::State,
    crate::{
        el_general::{
            el_async,
            style_export,
            CSS_STATE_DELETED,
            CSS_STATE_INVALID,
            CSS_STATE_THINKING,
        },
        state::set_page,
        world::{
            self,
            req_post_json,
        },
    },
    flowcontrol::{
        exenum,
        ta_return,
    },
    lunk::{
        link,
        HistPrim,
        Prim,
        ProcessingContext,
    },
    rooting::{
        el_from_raw,
        spawn_rooted,
        El,
    },
    serde::{
        Deserialize,
        Serialize,
    },
    shared::interface::{
        triple::{
            FileHash,
            Node,
        },
        wire::{
            ReqCommit,
            ReqGetTriplesAround,
            Triple,
        },
    },
    std::{
        cell::RefCell,
        collections::HashMap,
        rc::Rc,
        str::FromStr,
    },
    wasm_bindgen::JsCast,
    web_sys::{
        HtmlInputElement,
    },
};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
enum NodeEditType {
    Str,
    Num,
    Bool,
    Json,
    File,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
enum NodeEditValue {
    String(String),
    Bool(bool),
}

#[derive(Clone)]
struct NodeState {
    type_: HistPrim<NodeEditType>,
    value: HistPrim<NodeEditValue>,
}

fn new_node_state(pc: &mut ProcessingContext, node: &Node) -> NodeState {
    let (type_, value) = node_to_type_value(node);
    return NodeState {
        type_: HistPrim::new(pc, type_),
        value: HistPrim::new(pc, value),
    };
}

impl NodeState {
    // Produce a valid node from whatever state this element is in
    fn type_value_to_node(&self) -> Node {
        match (&*self.type_.borrow(), &*self.value.borrow()) {
            (NodeEditType::File, NodeEditValue::String(v)) => {
                if let Ok(v) = FileHash::from_str(&v) {
                    return Node::File(v);
                } else {
                    return Node::Value(serde_json::Value::String(v.clone()));
                }
            },
            (NodeEditType::File, NodeEditValue::Bool(v)) => {
                return Node::Value(serde_json::Value::String(if *v {
                    "true"
                } else {
                    "false"
                }.to_string()));
            },
            (NodeEditType::Str, NodeEditValue::String(v)) => {
                return Node::Value(serde_json::Value::String(v.clone()));
            },
            (NodeEditType::Str, NodeEditValue::Bool(v)) => {
                return Node::Value(serde_json::Value::String(if *v {
                    "true"
                } else {
                    "false"
                }.to_string()));
            },
            (NodeEditType::Num, NodeEditValue::String(v)) => {
                if let Ok(n) = serde_json::from_str::<serde_json::Number>(&v) {
                    return Node::Value(serde_json::Value::Number(n));
                } else {
                    return Node::Value(serde_json::Value::String(v.clone()));
                }
            },
            (NodeEditType::Num, NodeEditValue::Bool(v)) => {
                return Node::Value(serde_json::Value::String(if *v {
                    "true"
                } else {
                    "false"
                }.to_string()));
            },
            (NodeEditType::Bool, NodeEditValue::String(v)) => {
                return Node::Value(serde_json::Value::String(v.clone()));
            },
            (NodeEditType::Bool, NodeEditValue::Bool(v)) => {
                return Node::Value(serde_json::Value::Bool(*v));
            },
            (NodeEditType::Json, NodeEditValue::String(v)) => {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&v) {
                    return Node::Value(v);
                } else {
                    return Node::Value(serde_json::Value::String(v.clone()));
                }
            },
            (NodeEditType::Json, NodeEditValue::Bool(v)) => {
                return Node::Value(serde_json::Value::Bool(*v));
            },
        }
    }
}

fn node_to_type_value(node: &Node) -> (NodeEditType, NodeEditValue) {
    match node {
        Node::File(v) => {
            return (NodeEditType::File, NodeEditValue::String(v.to_string()));
        },
        Node::Value(v) => match v {
            serde_json::Value::Bool(v) => {
                return (NodeEditType::Bool, NodeEditValue::Bool(*v));
            },
            serde_json::Value::Number(v) => {
                return (NodeEditType::Num, NodeEditValue::String(v.to_string()));
            },
            serde_json::Value::String(v) => {
                return (NodeEditType::Str, NodeEditValue::String(v.clone()));
            },
            _ => {
                return (NodeEditType::Json, NodeEditValue::String(serde_json::to_string_pretty(v).unwrap()));
            },
        },
    }
}

struct PivotState_ {
    initial: RefCell<Node>,
    delete: HistPrim<bool>,
    node: NodeState,
}

#[derive(Clone)]
struct PivotState(Rc<PivotState_>);

fn new_pivot_state(pc: &mut ProcessingContext, n: &Node) -> PivotState {
    return PivotState(Rc::new(PivotState_ {
        initial: RefCell::new(n.clone()),
        delete: HistPrim::new(pc, false),
        node: new_node_state(pc, &n),
    }));
}

struct TripleState_ {
    incoming: bool,
    initial: RefCell<(String, Node)>,
    add: bool,
    delete: HistPrim<bool>,
    delete_all: HistPrim<bool>,
    predicate: Prim<String>,
    node: NodeState,
}

#[derive(Clone)]
struct TripleState(Rc<TripleState_>);

fn new_triple_state(
    pc: &mut ProcessingContext,
    t: &Triple,
    incoming: bool,
    delete_all: HistPrim<bool>,
) -> TripleState {
    let value = if incoming {
        t.subject.clone()
    } else {
        t.object.clone()
    };
    return TripleState(Rc::new(TripleState_ {
        incoming: incoming,
        initial: RefCell::new((t.predicate.clone(), value.clone())),
        add: false,
        delete: HistPrim::new(pc, false),
        delete_all: delete_all,
        predicate: Prim::new(t.predicate.clone()),
        node: new_node_state(pc, &value),
    }));
}

struct BuildEditNodeRes {
    root: El,
    button_delete: El,
    button_revert: El,
}

fn build_edit_node(pc: &mut ProcessingContext, node: &NodeState) -> BuildEditNodeRes {
    let options =
        [
            (NodeEditType::Str, "Value - text"),
            (NodeEditType::Num, "Value - number"),
            (NodeEditType::Bool, "Value - bool"),
            (NodeEditType::Json, "Value - JSON"),
            (NodeEditType::File, "File"),
        ]
            .into_iter()
            .map(|(k, v)| (serde_json::to_string(&k).unwrap(), v.to_string()))
            .collect::<HashMap<_, _>>();
    let inp_type_res = style_export::leaf_input_enum(style_export::LeafInputEnumArgs {
        id: None,
        title: "Node type".to_string(),
        options: options,
        value: serde_json::to_string(&node.type_.get()).unwrap(),
    });
    let inp_type_el = el_from_raw(inp_type_res.root.clone().into());
    let inp_value_group_el =
        el_from_raw(style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root.into());
    inp_type_el.ref_on("change", {
        let node = node.clone();
        let eg = pc.eg();
        move |_| eg.event(|pc| {
            node.type_.set(pc, serde_json::from_str::<NodeEditType>(&inp_type_res.root.value()).unwrap());
        }).unwrap()
    });
    inp_type_el.ref_own(
        |_| link!(
            (pc = pc),
            (node_type = node.type_.clone()),
            (node_value = node.value.clone()),
            (inp_value_group_el = inp_value_group_el.clone()),
            {
                let convert_str_node_value = |pc: &mut ProcessingContext| {
                    match &*node_type.borrow() {
                        NodeEditType::Str => {
                            // nop, leave as maybe invalid string
                        },
                        NodeEditType::Num => {
                            unreachable!();
                        },
                        NodeEditType::Bool => {
                            let NodeEditValue::Bool(v) = &*node_value.borrow() else {
                                unreachable!();
                            };
                            node_value.set(pc, NodeEditValue::String(if *v {
                                "true"
                            } else {
                                "false"
                            }.to_string()));
                        },
                        NodeEditType::Json => {
                            let NodeEditValue::String(v) = &*node_value.borrow() else {
                                unreachable!();
                            };
                            if let Ok(serde_json::Value::String(v)) = serde_json::from_str::<serde_json::Value>(v) {
                                node_value.set(pc, NodeEditValue::String(v.clone()));
                            } else {
                                // nop, leave as maybe valid json string (ok if number, invalid otherwise)
                            }
                        },
                        NodeEditType::File => {
                            // nop, leave as maybe invalid string
                        },
                    }
                };
                let build_text_input = |pc: &mut ProcessingContext, input_: El, validate: fn(&str) -> bool| -> El {
                    let input_value = Prim::new("".to_string());
                    input_.ref_on("change", {
                        let eg = pc.eg();
                        let input_value = input_value.clone();
                        move |ev| eg.event(|pc| {
                            let e = ev.target().unwrap().dyn_into::<HtmlInputElement>().unwrap();
                            input_value.set(pc, e.value());
                        }).unwrap()
                    });
                    input_.ref_own(|input_el| (
                        //. .
                        link!((pc = pc), (v = node_value.clone()), (input_value = input_value.clone()), (input_el = input_el.weak()), {
                            let input_el = input_el.upgrade()?;
                            match &*v.borrow() {
                                NodeEditValue::String(v) => {
                                    input_value.set(pc, v.clone());
                                    input_el.raw().dyn_ref::<HtmlInputElement>().unwrap().set_value(&v);
                                },
                                NodeEditValue::Bool(_) => unreachable!(),
                            }
                        }),
                        link!((pc = pc), (input_value = input_value.clone()), (v = node_value.clone()), (), {
                            v.set(pc, NodeEditValue::String(input_value.borrow().clone()));
                        }),
                        link!(
                            (_pc = pc),
                            (input_value = input_value),
                            (),
                            (input_el = input_el.weak(), validate = validate),
                            {
                                let input_el = input_el.upgrade()?;
                                input_el.ref_modify_classes(
                                    &[(CSS_STATE_INVALID, !validate(input_value.borrow().as_str()))],
                                );
                            }
                        ),
                    ));
                    return input_;
                };
                let new_input;
                match &*node_type.borrow() {
                    NodeEditType::Num => {
                        convert_str_node_value(pc);
                        new_input =
                            build_text_input(
                                pc,
                                el_from_raw(style_export::leaf_input_number(style_export::LeafInputNumberArgs {
                                    id: None,
                                    title: "Node".into(),
                                    value: "".into(),
                                }).root.into()),
                                |x| x.parse::<f64>().is_ok(),
                            );
                    },
                    NodeEditType::Bool => {
                        let new_value = false;
                        node_value.set(pc, NodeEditValue::Bool(new_value));
                        let input_value = Prim::new(false);
                        new_input = el_from_raw(style_export::leaf_input_bool(style_export::LeafInputBoolArgs {
                            id: None,
                            title: "Value".to_string(),
                            value: new_value,
                        }).root.into()).on("change", {
                            let eg = pc.eg();
                            let input_value = input_value.clone();
                            move |ev| eg.event(|pc| {
                                input_value.set(
                                    pc,
                                    ev
                                        .target()
                                        .unwrap()
                                        .dyn_ref::<HtmlInputElement>()
                                        .unwrap()
                                        .has_attribute("checked"),
                                );
                            }).unwrap()
                        }).own(|input_el| (
                            //. .
                            link!((pc = pc), (node_value = node_value.clone()), (input_value = input_value.clone()), (), {
                                input_value.set(
                                    pc,
                                    exenum!(&*node_value.borrow(), NodeEditValue:: Bool(v) =>* v).unwrap(),
                                );
                            }),
                            link!(
                                (pc = pc),
                                (input_value = input_value.clone()),
                                (node_value = node_value.clone()),
                                (input_el = input_el.weak()),
                                {
                                    let input_el = input_el.upgrade()?;
                                    node_value.set(pc, NodeEditValue::Bool(*input_value.borrow()));
                                    match *input_value.borrow() {
                                        true => input_el.ref_attr("checked", "checked"),
                                        false => input_el.ref_remove_attr("checked"),
                                    }
                                }
                            ),
                        ));
                    },
                    NodeEditType::Str => {
                        convert_str_node_value(pc);
                        new_input =
                            build_text_input(
                                pc,
                                el_from_raw(style_export::leaf_input_text(style_export::LeafInputTextArgs {
                                    id: None,
                                    title: "Node".into(),
                                    value: "".into(),
                                }).root.into()),
                                |_| true,
                            );
                    },
                    NodeEditType::Json => {
                        match &*node_type.borrow() {
                            NodeEditType::Str => {
                                node_value.set(
                                    pc,
                                    NodeEditValue::String(
                                        serde_json::to_string_pretty(
                                            exenum!(&*node_value.borrow(), NodeEditValue:: String(v) => v).unwrap(),
                                        ).unwrap(),
                                    ),
                                );
                            },
                            NodeEditType::Num => {
                                // nop
                            },
                            NodeEditType::Bool => {
                                node_value.set(
                                    pc,
                                    NodeEditValue::String(
                                        if exenum!(&*node_value.borrow(), NodeEditValue:: Bool(v) =>* v).unwrap() {
                                            "true"
                                        } else {
                                            "false"
                                        }.to_string(),
                                    ),
                                );
                            },
                            NodeEditType::Json => {
                                unreachable!();
                            },
                            NodeEditType::File => {
                                node_value.set(
                                    pc,
                                    NodeEditValue::String(
                                        serde_json::to_string_pretty(
                                            exenum!(&*node_value.borrow(), NodeEditValue:: String(v) => v).unwrap(),
                                        ).unwrap(),
                                    ),
                                );
                            },
                        }
                        new_input =
                            build_text_input(
                                pc,
                                el_from_raw(style_export::leaf_input_text(style_export::LeafInputTextArgs {
                                    id: None,
                                    title: "Node".into(),
                                    value: "".into(),
                                }).root.into()),
                                |x| serde_json::from_str::<serde_json::Value>(x).is_ok(),
                            );
                    },
                    NodeEditType::File => {
                        convert_str_node_value(pc);
                        new_input =
                            build_text_input(
                                pc,
                                el_from_raw(style_export::leaf_input_text(style_export::LeafInputTextArgs {
                                    id: None,
                                    title: "Node".into(),
                                    value: "".into(),
                                }).root.into()),
                                |v| FileHash::from_str(&v).is_ok(),
                            );
                    },
                }
                inp_value_group_el.ref_clear();
                inp_value_group_el.ref_push(new_input);
            }
        ),
    );
    let style_res = style_export::leaf_edit_node(style_export::LeafEditNodeArgs {
        input_type: inp_type_el.raw().dyn_into().unwrap(),
        input_value: inp_value_group_el.raw().dyn_into().unwrap(),
    });
    let button_delete = el_from_raw(style_res.button_delete.into());
    let button_revert = el_from_raw(style_res.button_revert.into());
    return BuildEditNodeRes {
        root: el_from_raw(
            style_res.root.into(),
        ).own(|_| (inp_type_el, inp_value_group_el, button_delete.clone(), button_revert.clone())),
        button_delete: button_delete,
        button_revert: button_revert,
    };
}

fn build_edit_triple(pc: &mut ProcessingContext, triple: &TripleState) -> El {
    let node_el = {
        let style_res = build_edit_node(pc, &triple.0.node);
        style_res.button_revert.ref_on("click", {
            let triple = triple.clone();
            let eg = pc.eg();
            move |_| eg.event(|pc| {
                triple.0.predicate.set(pc, triple.0.initial.borrow().0.clone());
                let (node_type, node_value) = node_to_type_value(&triple.0.initial.borrow().1);
                triple.0.node.type_.set(pc, node_type);
                triple.0.node.value.set(pc, node_value);
            }).unwrap()
        });
        style_res.button_delete.ref_on("click", {
            let triple = triple.clone();
            let eg = pc.eg();
            move |_| eg.event(|pc| {
                triple.0.delete.set(pc, !triple.0.delete.get());
            }).unwrap()
        });
        style_res
            .button_delete
            .ref_own(|out| link!((_pc = pc), (deleted = triple.0.delete.clone()), (), (out = out.weak()), {
                let out = out.upgrade()?;
                out.ref_modify_classes(&[(CSS_STATE_DELETED, deleted.get())]);
            }));
        style_res.root
    };
    let predicate_el = {
        let predicate_value = "".to_string();
        let predicate_res =
            style_export::leaf_edit_predicate(
                style_export::LeafEditPredicateArgs { value: predicate_value.clone() },
            );
        let input_value = Prim::new(predicate_value);
        el_from_raw(predicate_res.root.into()).on("change", {
            let eg = pc.eg();
            let input_value = input_value.clone();
            move |ev| eg.event(|pc| {
                input_value.set(pc, ev.target().unwrap().dyn_into::<HtmlInputElement>().unwrap().value());
            }).unwrap()
        }).own(|input_el| (
            //. .
            link!(
                (pc = pc),
                (predicate_value = triple.0.predicate.clone()),
                (input_value = input_value.clone()),
                (input_el = input_el.weak()),
                {
                    let input_el = input_el.upgrade()?;
                    input_value.set(pc, predicate_value.borrow().clone());
                    input_el.ref_attr("value", predicate_value.borrow().as_str());
                }
            ),
            link!((pc = pc), (input_value = input_value.clone()), (predicate_value = triple.0.predicate.clone()), (), {
                predicate_value.set(pc, input_value.borrow().clone());
            }),
        ))
    };
    if triple.0.incoming {
        return el_from_raw(
            style_export::cont_edit_row_incoming(
                style_export::ContEditRowIncomingArgs {
                    children: vec![node_el.raw().dyn_into().unwrap(), predicate_el.raw().dyn_into().unwrap()],
                },
            )
                .root
                .into(),
        ).own(|_| (node_el, predicate_el));
    } else {
        return el_from_raw(
            style_export::cont_edit_row_outgoing(
                style_export::ContEditRowOutgoingArgs {
                    children: vec![node_el.raw().dyn_into().unwrap(), predicate_el.raw().dyn_into().unwrap()],
                },
            )
                .root
                .into(),
        ).own(|_| (node_el, predicate_el));
    }
}

pub fn build_page_edit(pc: &mut ProcessingContext, outer_state: &State, edit_title: &str, node: &Node) {
    set_page(outer_state, edit_title, el_async({
        let eg = pc.eg();
        let outer_state = outer_state.clone();
        let node = node.clone();
        async move {
            ta_return!(El, String);
            let triples = req_post_json(&outer_state.base_url, ReqGetTriplesAround { node: node.clone() }).await?;
            return eg.event(|pc| {
                let error_slot =
                    el_from_raw(
                        style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root.into(),
                    );
                let mut out = vec![error_slot.clone()];
                let mut bar_out = vec![];
                let pivot_state = new_pivot_state(pc, &node);
                let triple_states = Rc::new(RefCell::new(vec![] as Vec<TripleState>));

                // Incoming triples
                {
                    let triples_box =
                        el_from_raw(
                            style_export::cont_page_edit_section_rel(
                                style_export::ContPageEditSectionRelArgs { children: vec![] },
                            )
                                .root
                                .into(),
                        );
                    for t in triples.incoming {
                        let triple = new_triple_state(pc, &t, true, pivot_state.0.delete.clone());
                        triples_box.ref_push(build_edit_triple(pc, &triple));
                        triple_states.borrow_mut().push(triple);
                    }
                    let button_add =
                        el_from_raw(
                            style_export::leaf_button_edit_add(
                                style_export::LeafButtonEditAddArgs { hint: "Add incoming".to_string() },
                            )
                                .root
                                .into(),
                        );
                    button_add.ref_on("click", {
                        let eg = pc.eg();
                        let pivot_state = pivot_state.clone();
                        let triple_states = triple_states.clone();
                        let incoming_triples_box = triples_box.clone();
                        move |_| eg.event(|pc| {
                            let triple = new_triple_state(pc, &Triple {
                                subject: Node::Value(serde_json::Value::String("".to_string())),
                                predicate: "".to_string(),
                                object: Node::Value(serde_json::Value::String("".to_string())),
                            }, true, pivot_state.0.delete.clone());
                            incoming_triples_box.ref_push(build_edit_triple(pc, &triple));
                            triple_states.borrow_mut().push(triple);
                        }).unwrap()
                    });
                    out.push(button_add);
                    out.push(triples_box);
                }

                // Pivot
                {
                    let style_res = build_edit_node(pc, &pivot_state.0.node);
                    style_res.button_revert.ref_on("click", {
                        let pivot_original = node;
                        let pivot = pivot_state.clone();
                        let eg = pc.eg();
                        move |_| eg.event(|pc| {
                            let (node_type, node_value) = node_to_type_value(&pivot_original);
                            pivot.0.node.type_.set(pc, node_type);
                            pivot.0.node.value.set(pc, node_value);
                        }).unwrap()
                    });
                    style_res.button_delete.ref_on("click", {
                        let pivot_state = pivot_state.clone();
                        let eg = pc.eg();
                        move |_| eg.event(|pc| {
                            pivot_state.0.delete.set(pc, !pivot_state.0.delete.get());
                        }).unwrap()
                    });
                    out.push(
                        el_from_raw(
                            style_export::cont_edit_section_center(
                                style_export::ContEditSectionCenterArgs {
                                    child: style_res.root.raw().dyn_into().unwrap(),
                                },
                            )
                                .root
                                .into(),
                        ).own(
                            |ele| (
                                style_res.root,
                                link!((_pc = pc), (deleted = pivot_state.0.delete.clone()), (), (ele = ele.weak()), {
                                    let pivot_root = ele.upgrade()?;
                                    pivot_root.ref_modify_classes(&[(CSS_STATE_DELETED, deleted.get())]);
                                }),
                            ),
                        ),
                    );
                }

                // Outgoing triples
                {
                    let triples_box =
                        el_from_raw(
                            style_export::cont_page_edit_section_rel(
                                style_export::ContPageEditSectionRelArgs { children: vec![] },
                            )
                                .root
                                .into(),
                        );
                    for t in triples.outgoing {
                        let triple = new_triple_state(pc, &t, false, pivot_state.0.delete.clone());
                        triples_box.ref_push(build_edit_triple(pc, &triple));
                        triple_states.borrow_mut().push(triple);
                    }
                    let button_add =
                        el_from_raw(
                            style_export::leaf_button_edit_add(
                                style_export::LeafButtonEditAddArgs { hint: "Add outgoing".to_string() },
                            )
                                .root
                                .into(),
                        );
                    button_add.ref_on("click", {
                        let eg = pc.eg();
                        let triple_states = triple_states.clone();
                        let triples_box = triples_box.clone();
                        let pivot_state = pivot_state.clone();
                        move |_| eg.event(|pc| {
                            let triple = new_triple_state(pc, &Triple {
                                subject: Node::Value(serde_json::Value::String("".to_string())),
                                predicate: "".to_string(),
                                object: Node::Value(serde_json::Value::String("".to_string())),
                            }, false, pivot_state.0.delete.clone());
                            triples_box.ref_push(build_edit_triple(pc, &triple));
                            triple_states.borrow_mut().push(triple);
                        }).unwrap()
                    });
                    out.push(triples_box);
                    out.push(button_add);
                }

                // Edit form controls
                let button_save = el_from_raw(style_export::leaf_bar_button_big(style_export::LeafBarButtonBigArgs {
                    title: "Save".into(),
                    text: "Save".into(),
                }).root.into());
                let save_thinking = Rc::new(RefCell::new(None));
                button_save.ref_own(|_| save_thinking.clone());
                button_save.ref_on("click", {
                    let triple_states = triple_states.clone();
                    let pivot_state = pivot_state.clone();
                    let outer_state = outer_state.clone();
                    let error_slot = error_slot.weak();
                    move |ev| {
                        {
                            let Some(error_slot) = error_slot.upgrade() else {
                                return;
                            };
                            error_slot.ref_clear();
                        }
                        let button = ev.target().unwrap().dyn_into::<web_sys::HtmlElement>().unwrap();
                        button.class_list().add_1(CSS_STATE_THINKING).unwrap();
                        *save_thinking.borrow_mut() = Some(spawn_rooted({
                            let triple_states = triple_states.clone();
                            let pivot_state = pivot_state.clone();
                            let outer_state = outer_state.clone();
                            let error_slot = error_slot.clone();
                            async move {
                                let mut triple_nodes_predicates = vec![];
                                let pivot_node = pivot_state.0.node.type_value_to_node();
                                let res = async {
                                    ta_return!((), String);
                                    let mut add = vec![];
                                    let mut remove = vec![];
                                    let delete_all = *pivot_state.0.delete.borrow();
                                    let pivot_node_initial = pivot_state.0.initial.borrow();
                                    let pivot_changed = pivot_node != *pivot_node_initial;
                                    for triple in &*RefCell::borrow(&triple_states) {
                                        let triple_node = triple.0.node.type_value_to_node();
                                        let triple_predicate = triple.0.predicate.borrow().clone();
                                        triple_nodes_predicates.push(
                                            (triple_predicate.clone(), triple_node.clone()),
                                        );
                                        let triple_initial = triple.0.initial.borrow();
                                        let triple_predicate_initial = &triple_initial.0;
                                        let triple_node_initial = &triple_initial.1;
                                        let changed =
                                            pivot_changed || triple_node == *triple_node_initial ||
                                                triple.0.predicate.borrow().as_str() == triple_predicate_initial;
                                        if !triple.0.add && (delete_all || triple.0.delete.get()) || changed {
                                            let old_subject;
                                            let old_object;
                                            if triple.0.incoming {
                                                old_subject = triple_node_initial.clone();
                                                old_object = pivot_node_initial.clone();
                                            } else {
                                                old_subject = pivot_node_initial.clone();
                                                old_object = triple_node_initial.clone();
                                            }
                                            remove.push(Triple {
                                                subject: old_subject,
                                                predicate: triple_predicate_initial.clone(),
                                                object: old_object,
                                            });
                                        }
                                        if triple.0.add || changed {
                                            let subject;
                                            let object;
                                            if triple.0.incoming {
                                                subject = triple_node;
                                                object = pivot_node.clone();
                                            } else {
                                                subject = pivot_node.clone();
                                                object = triple_node;
                                            }
                                            add.push(Triple {
                                                subject: subject,
                                                predicate: triple.0.predicate.borrow().clone(),
                                                object: object,
                                            });
                                        }
                                    }
                                    world::req_post_json(&outer_state.base_url, ReqCommit {
                                        add: add,
                                        remove: remove,
                                        files: vec![],
                                    }).await?;
                                    return Ok(());
                                }.await;
                                button.class_list().remove_1(CSS_STATE_THINKING).unwrap();
                                match res {
                                    Ok(_) => {
                                        *pivot_state.0.initial.borrow_mut() = pivot_node;
                                        for (
                                            triple,
                                            sent_triple,
                                        ) in Iterator::zip(
                                            RefCell::borrow(&triple_states).iter(),
                                            triple_nodes_predicates.into_iter(),
                                        ) {
                                            *triple.0.initial.borrow_mut() = sent_triple;
                                        }
                                    },
                                    Err(e) => {
                                        let Some(error_slot) = error_slot.upgrade() else {
                                            return;
                                        };
                                        error_slot.ref_push(
                                            el_from_raw(
                                                style_export::leaf_err_block(
                                                    style_export::LeafErrBlockArgs { data: e },
                                                )
                                                    .root
                                                    .into(),
                                            ),
                                        );
                                    },
                                }
                            }
                        }));
                    }
                });
                bar_out.push(button_save);
                return Ok(el_from_raw(style_export::cont_page_edit(style_export::ContPageEditArgs {
                    children: out.iter().map(|x| x.raw().dyn_into().unwrap()).collect(),
                    bar_children: bar_out.iter().map(|x| x.raw().dyn_into().unwrap()).collect(),
                }).root.into()).own(|_| (out, bar_out)));
            }).unwrap();
        }
    }));
}
