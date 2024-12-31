use {
    super::state::State,
    crate::{
        el_general::{
            el_async,
            el_button_icon,
            el_button_icon_text,
            el_button_icon_toggle_auto,
            el_err_block,
            el_hbox,
            el_icon,
            el_spacer,
            el_vbox,
            CSS_FORM_BUTTONBOX,
            CSS_STATE_DELETED,
            CSS_STATE_INVALID,
            ICON_ADD,
            ICON_REMOVE,
            ICON_RESET,
            ICON_SAVE,
        },
        world::{
            self,
            req_post_json,
        },
    },
    flowcontrol::{
        exenum,
        shed,
        ta_return,
    },
    lunk::{
        link,
        HistPrim,
        Prim,
        ProcessingContext,
    },
    rooting::{
        el,
        spawn_rooted,
        El,
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
        rc::Rc,
        str::FromStr,
    },
    wasm_bindgen::JsCast,
    web_sys::{
        HtmlInputElement,
        HtmlSelectElement,
    },
};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
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

impl NodeState {
    fn as_node(&self) -> Node {
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

fn new_node_state(pc: &mut ProcessingContext, node: &Node) -> NodeState {
    let (type_, value) = node_to_type_value(node);
    return NodeState {
        type_: HistPrim::new(pc, type_),
        value: HistPrim::new(pc, value),
    };
}

struct BuildEditNodeRes {
    type_select: El,
    input: El,
}

struct PivotState_ {
    initial: Node,
    delete: HistPrim<bool>,
    node: NodeState,
}

#[derive(Clone)]
struct PivotState(Rc<PivotState_>);

fn new_pivot_state(pc: &mut ProcessingContext, n: &Node) -> PivotState {
    return PivotState(Rc::new(PivotState_ {
        initial: n.clone(),
        delete: HistPrim::new(pc, false),
        node: new_node_state(pc, &n),
    }));
}

struct TripleState_ {
    incoming: bool,
    initial: (String, Node),
    add: bool,
    delete: HistPrim<bool>,
    predicate: Prim<String>,
    node: NodeState,
}

#[derive(Clone)]
struct TripleState(Rc<TripleState_>);

fn new_triple_state(pc: &mut ProcessingContext, t: &Triple, incoming: bool) -> TripleState {
    return TripleState(Rc::new(TripleState_ {
        incoming: incoming,
        initial: (t.predicate.clone(), t.subject.clone()),
        add: false,
        delete: HistPrim::new(pc, false),
        predicate: Prim::new(t.predicate.clone()),
        node: new_node_state(pc, &t.subject),
    }));
}

fn build_edit_node(pc: &mut ProcessingContext, node: &NodeState) -> BuildEditNodeRes {
    let type_select = el("select");
    const OPT_VAL_STR: &str = "val_str";
    let opt_val_str = el("option").attr("value", OPT_VAL_STR).text("Value - text");
    type_select.ref_push(opt_val_str.clone());
    const OPT_VAL_NUM: &str = "val_num";
    let opt_val_num = el("option").attr("value", OPT_VAL_NUM).text("Value - number");
    type_select.ref_push(opt_val_num.clone());
    const OPT_VAL_BOOL: &str = "val_bool";
    let opt_val_bool = el("option").attr("value", OPT_VAL_BOOL).text("Value - bool");
    type_select.ref_push(opt_val_bool.clone());
    const OPT_VAL_JSON: &str = "val_json";
    let opt_val_json = el("option").attr("value", OPT_VAL_JSON).text("Value - JSON");
    type_select.ref_push(opt_val_json.clone());
    const OPT_VAL_FILE: &str = "file";
    let opt_val_file = el("option").attr("value", OPT_VAL_FILE).text("File");
    type_select.ref_push(opt_val_file.clone());
    match &*node.type_.borrow() {
        NodeEditType::Bool => {
            opt_val_bool.attr("selected", "selected");
        },
        NodeEditType::Num => {
            opt_val_num.attr("selected", "selected");
        },
        NodeEditType::Str => {
            opt_val_str.attr("selected", "selected");
        },
        NodeEditType::Json => {
            opt_val_json.attr("selected", "selected");
        },
        NodeEditType::File => {
            opt_val_file.attr("selected", "selected");
        },
    }
    let input_el = el_async();
    type_select.ref_on("change", {
        let node = node.clone();
        let eg = pc.eg();
        move |ev| eg.event(|pc| {
            let type_select = ev.target().unwrap().dyn_into::<HtmlSelectElement>().unwrap();
            let new_type_raw = type_select.value();
            let new_type;
            shed!{
                match new_type_raw.as_str() {
                    OPT_VAL_BOOL => {
                        new_type = NodeEditType::Bool;
                    },
                    OPT_VAL_NUM => {
                        new_type = NodeEditType::Num;
                    },
                    OPT_VAL_STR => {
                        new_type = NodeEditType::Str;
                    },
                    OPT_VAL_JSON => {
                        new_type = NodeEditType::Json;
                    },
                    _ => {
                        unreachable!();
                    },
                }
            }
            node.type_.set(pc, new_type);
        })
    });
    type_select.ref_own(
        |_| link!(
            (pc = pc),
            (node_type = node.type_.clone()),
            (node_value = node.value.clone()),
            (input_el = RefCell::new(input_el.weak())),
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
                let build_text_input =
                    |pc: &mut ProcessingContext, input_type: &str, validate: fn(&str) -> bool| -> El {
                        let input_value = Prim::new("".to_string());
                        return el("input").attr("type", input_type).on("change", {
                            let eg = pc.eg();
                            let input_value = input_value.clone();
                            move |ev| eg.event(|pc| {
                                let e = ev.target().unwrap().dyn_into::<HtmlInputElement>().unwrap();
                                input_value.set(pc, e.value());
                            })
                        }).own(|input_el| (
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
                    };
                let new_input;
                match &*node_type.borrow() {
                    NodeEditType::Num => {
                        convert_str_node_value(pc);
                        new_input = build_text_input(pc, "number", |x| x.parse::<f64>().is_ok());
                    },
                    NodeEditType::Bool => {
                        node_value.set(pc, NodeEditValue::Bool(false));
                        let input_value = Prim::new(false);
                        new_input = el("input").attr("type", "checkbox").on("change", {
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
                            })
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
                        new_input = build_text_input(pc, "text", |_| true);
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
                                "text",
                                |x| serde_json::from_str::<serde_json::Value>(x).is_ok(),
                            );
                    },
                    NodeEditType::File => {
                        convert_str_node_value(pc);
                        new_input = build_text_input(pc, "text", |v| FileHash::from_str(&v).is_ok());
                    },
                }
                if let Some(input_el) = input_el.borrow().upgrade() {
                    input_el.ref_replace(vec![new_input.clone()]);
                }
                *input_el.borrow_mut() = new_input.weak();
            }
        ),
    );
    return BuildEditNodeRes {
        type_select: type_select,
        input: input_el,
    };
}

fn build_edit_triple(pc: &mut ProcessingContext, triple: &TripleState) -> El {
    let out = el_vbox();
    let edit_node_res = build_edit_node(pc, &triple.0.node);
    let controls_row =
        el_hbox()
            .push(edit_node_res.type_select)
            .push(el_spacer())
            .push(el_button_icon(pc, el_icon(ICON_RESET), "Restore original value", {
                let triple = triple.clone();
                move |pc| {
                    triple.0.predicate.set(pc, triple.0.initial.0.clone());
                    let (node_type, node_value) = node_to_type_value(&triple.0.initial.1);
                    triple.0.node.type_.set(pc, node_type);
                    triple.0.node.value.set(pc, node_value);
                }
            }))
            .push(
                el_button_icon_toggle_auto(
                    pc,
                    ICON_REMOVE,
                    "Delete triple",
                    &triple.0.delete,
                ).own(|_| link!((_pc = pc), (deleted = triple.0.delete.clone()), (), (out = out.weak()), {
                    let out = out.upgrade()?;
                    out.ref_modify_classes(&[(CSS_STATE_DELETED, deleted.get())]);
                })),
            );
    let predicate_row = {
        let input_value = Prim::new("".to_string());
        el("input").attr("type", "text").on("change", {
            let eg = pc.eg();
            let input_value = input_value.clone();
            move |ev| eg.event(|pc| {
                input_value.set(pc, ev.target().unwrap().dyn_into::<HtmlInputElement>().unwrap().value());
            })
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
        return out.push(controls_row).push(edit_node_res.input).push(predicate_row);
    } else {
        return out.push(predicate_row).push(edit_node_res.input).push(controls_row);
    }
}

fn build_cont_incoming() -> El {
    return el("div").classes(&["g_edit_row_incoming"]);
}

fn build_cont_outgoing() -> El {
    return el("div").classes(&["g_edit_row_outgoing"]);
}

pub fn build_page_edit(pc: &mut ProcessingContext, outer_state: &State, edit_title: &str, node: &Node) {
    outer_state.page_title.upgrade().unwrap().ref_text(&edit_title);
    outer_state.page_body.upgrade().unwrap().ref_push(el_async().own(|async_el| {
        let async_el = async_el.weak();
        let eg = pc.eg();
        let outer_state = outer_state.clone();
        let node = node.clone();
        spawn_rooted(async move {
            let triples =
                match req_post_json(&outer_state.base_url, ReqGetTriplesAround { node: node.clone() }).await {
                    Ok(t) => t,
                    Err(e) => {
                        let Some(async_el) = async_el.upgrade() else {
                            return;
                        };
                        async_el.ref_replace(vec![el_err_block(e)]);
                        return;
                    },
                };
            eg.event(|pc| {
                match (|| {
                    ta_return!(Vec < El >, String);
                    let mut out = vec![];

                    // Incoming triples
                    let triple_states = Rc::new(RefCell::new(vec![] as Vec<TripleState>));
                    let incoming_triples_box = el_vbox();
                    for t in triples.incoming {
                        let triple = new_triple_state(pc, &t, true);
                        incoming_triples_box.ref_push(
                            build_cont_incoming().push(el_vbox().extend(vec![build_edit_triple(pc, &triple)])),
                        );
                        triple_states.borrow_mut().push(triple);
                    }
                    out.push(build_cont_incoming().push(el_button_icon(pc, el_icon(ICON_ADD), "Add incoming", {
                        let triple_states = triple_states.clone();
                        let incoming_triples_box = incoming_triples_box.clone();
                        move |pc| {
                            let triple = new_triple_state(pc, &Triple {
                                subject: Node::Value(serde_json::Value::String("".to_string())),
                                predicate: "".to_string(),
                                object: Node::Value(serde_json::Value::String("".to_string())),
                            }, true);
                            incoming_triples_box.ref_push(build_edit_triple(pc, &triple));
                            triple_states.borrow_mut().push(triple);
                        }
                    })));

                    // Pivot
                    let pivot_state = new_pivot_state(pc, &node);
                    {
                        let edit_pivot_res = build_edit_node(pc, &pivot_state.0.node);
                        let pivot_root = el_vbox();
                        pivot_root
                            .ref_push(
                                el_hbox()
                                    .push(edit_pivot_res.type_select)
                                    .push(el_spacer())
                                    .push(el_button_icon(pc, el_icon(ICON_RESET), "Restore original value", {
                                        let pivot_original = node;
                                        let pivot = pivot_state.clone();
                                        move |pc| {
                                            let (node_type, node_value) = node_to_type_value(&pivot_original);
                                            pivot.0.node.type_.set(pc, node_type);
                                            pivot.0.node.value.set(pc, node_value);
                                        }
                                    }))
                                    .push(
                                        el_button_icon_toggle_auto(
                                            pc,
                                            ICON_REMOVE,
                                            "Delete node",
                                            &pivot_state.0.delete,
                                        ).own(
                                            |_| link!(
                                                (_pc = pc),
                                                (deleted = pivot_state.0.delete.clone()),
                                                (),
                                                (pivot_root = pivot_root.weak()),
                                                {
                                                    let pivot_root = pivot_root.upgrade()?;
                                                    pivot_root.ref_modify_classes(
                                                        &[(CSS_STATE_DELETED, deleted.get())],
                                                    );
                                                }
                                            ),
                                        ),
                                    ),
                            )
                            .ref_push(edit_pivot_res.input);
                        out.push(pivot_root);
                    }

                    // Outgoing triples
                    let outgoing_triples_box = el_vbox();
                    for t in triples.outgoing {
                        let triple = new_triple_state(pc, &t, true);
                        outgoing_triples_box.ref_push(build_edit_triple(pc, &triple));
                        triple_states.borrow_mut().push(triple);
                    }
                    out.push(build_cont_outgoing().push(el_button_icon(pc, el_icon(ICON_ADD), "Add outgoing", {
                        let triple_states = triple_states.clone();
                        let outgoing_triples_box = outgoing_triples_box.clone();
                        move |pc| {
                            let triple = new_triple_state(pc, &Triple {
                                subject: Node::Value(serde_json::Value::String("".to_string())),
                                predicate: "".to_string(),
                                object: Node::Value(serde_json::Value::String("".to_string())),
                            }, false);
                            outgoing_triples_box.ref_push(build_edit_triple(pc, &triple));
                            triple_states.borrow_mut().push(triple);
                        }
                    })));

                    // Edit form controls
                    let error_el = el_err_block("");
                    out.push(error_el.clone());
                    out.push(
                        el("div").classes(&[CSS_FORM_BUTTONBOX]).push(el_button_icon_text(pc, ICON_SAVE, "Save", {
                            let triple_states = triple_states.clone();
                            let pivot_state = pivot_state.clone();
                            let bg = Rc::new(RefCell::new(None));
                            let outer_state = outer_state.clone();
                            let error_el = error_el.weak();
                            move |_pc| {
                                *bg.borrow_mut() = Some(spawn_rooted({
                                    let triple_states = triple_states.clone();
                                    let pivot_state = pivot_state.clone();
                                    let outer_state = outer_state.clone();
                                    let error_el = error_el.clone();
                                    async move {
                                        match async {
                                            ta_return!((), String);
                                            let mut add = vec![];
                                            let mut remove = vec![];
                                            if *pivot_state.0.delete.borrow() {
                                                for triple in &*RefCell::borrow(&triple_states) {
                                                    if triple.0.add {
                                                        continue;
                                                    }
                                                    let old_subject;
                                                    let old_object;
                                                    if triple.0.incoming {
                                                        old_subject = triple.0.initial.1.clone();
                                                        old_object = pivot_state.0.initial.clone();
                                                    } else {
                                                        old_subject = pivot_state.0.initial.clone();
                                                        old_object = triple.0.initial.1.clone();
                                                    }
                                                    remove.push(Triple {
                                                        subject: old_subject,
                                                        predicate: triple.0.initial.0.clone(),
                                                        object: old_object,
                                                    });
                                                }
                                            } else {
                                                let pivot_node = pivot_state.0.node.as_node();
                                                let pivot_changed = pivot_node != pivot_state.0.initial;
                                                for triple in &*RefCell::borrow(&triple_states) {
                                                    let triple_node = triple.0.node.as_node();
                                                    if !pivot_changed && triple_node == triple.0.initial.1 &&
                                                        triple.0.predicate.borrow().as_str() == &triple.0.initial.0 {
                                                        continue;
                                                    }
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
                                                    if !triple.0.add {
                                                        let old_subject;
                                                        let old_object;
                                                        if triple.0.incoming {
                                                            old_subject = triple.0.initial.1.clone();
                                                            old_object = pivot_state.0.initial.clone();
                                                        } else {
                                                            old_subject = pivot_state.0.initial.clone();
                                                            old_object = triple.0.initial.1.clone();
                                                        }
                                                        remove.push(Triple {
                                                            subject: old_subject,
                                                            predicate: triple.0.initial.0.clone(),
                                                            object: old_object,
                                                        });
                                                    }
                                                }
                                            }
                                            world::req_post_json(&outer_state.base_url, ReqCommit {
                                                add: add,
                                                remove: remove,
                                                files: vec![],
                                            }).await?;
                                            return Ok(());
                                        }.await {
                                            Ok(_) => {
                                                return;
                                            },
                                            Err(e) => {
                                                let Some(error_el) = error_el.upgrade() else {
                                                    return;
                                                };
                                                error_el.ref_text(&e);
                                                return;
                                            },
                                        }
                                    }
                                }));
                            }
                        })),
                    );
                    return Ok(out);
                })() {
                    Ok(e) => {
                        let Some(async_el) = async_el.upgrade() else {
                            return;
                        };
                        async_el.ref_replace(e);
                    },
                    Err(e) => {
                        let Some(async_el) = async_el.upgrade() else {
                            return;
                        };
                        async_el.ref_replace(vec![el_err_block(e)]);
                    },
                }
            });
        })
    }));
}
