use {
    super::{
        api::req_post_json,
        state::set_page,
    },
    crate::libnonlink::{
        commit::{
            commit,
            CommitNode,
            CommitTriple,
        },
        ministate::{
            ministate_octothorpe,
            MinistateNodeView,
        },
        state::state,
    },
    flowcontrol::{
        exenum,
        superif,
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
            ReqGetNodeMeta,
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
    wasm::{
        js::{
            el_async,
            el_async_,
            style_export,
        },
        world::file_url,
    },
    wasm_bindgen::JsCast,
    web_sys::{
        File,
        HtmlElement,
        HtmlInputElement,
    },
};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
enum NodeEditType {
    // Value = str
    Str,
    // Value = str
    Num,
    // Value = bool
    Bool,
    // Value = str
    Json,
    // Value = str
    File,
    // Value = upload
    FileUpload,
}

#[derive(Clone, PartialEq, Eq)]
struct NodeEditValueUpload {
    // Keep if user accidentally selects
    old: String,
    new: Option<File>,
}

#[derive(Clone, PartialEq, Eq)]
enum NodeEditValue {
    String(String),
    Bool(bool),
    Upload(NodeEditValueUpload),
}

#[derive(Clone)]
struct NodeState {
    type_: HistPrim<NodeEditType>,
    value: HistPrim<NodeEditValue>,
    initial: HistPrim<(NodeEditType, NodeEditValue)>,
}

fn new_node_state(pc: &mut ProcessingContext, node: &Node) -> NodeState {
    let (type_, value) = node_to_type_value(node);
    return NodeState {
        type_: HistPrim::new(pc, type_.clone()),
        value: HistPrim::new(pc, value.clone()),
        initial: HistPrim::new(pc, (type_, value)),
    };
}

fn type_value_to_node(unique: usize, type_: &NodeEditType, value: &NodeEditValue) -> CommitNode {
    match type_ {
        NodeEditType::Str => {
            let v = exenum!(value, NodeEditValue:: String(v) => v).unwrap();
            return CommitNode::Node(Node::Value(serde_json::Value::String(v.clone())));
        },
        NodeEditType::Num => {
            let v = exenum!(value, NodeEditValue:: String(v) => v).unwrap();
            if let Ok(n) = serde_json::from_str::<serde_json::Number>(v) {
                return CommitNode::Node(Node::Value(serde_json::Value::Number(n)));
            } else {
                return CommitNode::Node(Node::Value(serde_json::Value::String(v.clone())));
            }
        },
        NodeEditType::Bool => {
            let v = exenum!(value, NodeEditValue:: Bool(v) => v).unwrap();
            return CommitNode::Node(Node::Value(serde_json::Value::Bool(*v)));
        },
        NodeEditType::Json => {
            let v = exenum!(value, NodeEditValue:: String(v) => v).unwrap();
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&v) {
                return CommitNode::Node(Node::Value(v));
            } else {
                return CommitNode::Node(Node::Value(serde_json::Value::String(v.clone())));
            }
        },
        NodeEditType::File => {
            let v = exenum!(value, NodeEditValue:: String(v) => v).unwrap();
            if let Ok(v) = FileHash::from_str(v) {
                return CommitNode::Node(Node::File(v));
            } else {
                return CommitNode::Node(Node::Value(serde_json::Value::String(v.clone())));
            }
        },
        NodeEditType::FileUpload => {
            let v = exenum!(value, NodeEditValue:: Upload(v) => v).unwrap();
            match &v.new {
                Some(n) => {
                    return CommitNode::File(unique, n.clone());
                },
                None => {
                    return CommitNode::Node(Node::Value(serde_json::Value::String(v.old.clone())));
                },
            }
        },
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
    delete: HistPrim<bool>,
    node: NodeState,
}

#[derive(Clone)]
struct PivotState(Rc<PivotState_>);

fn new_pivot_state(pc: &mut ProcessingContext, n: &Node) -> PivotState {
    return PivotState(Rc::new(PivotState_ {
        delete: HistPrim::new(pc, false),
        node: new_node_state(pc, &n),
    }));
}

struct TripleState_ {
    incoming: bool,
    add: bool,
    delete: HistPrim<bool>,
    delete_all: HistPrim<bool>,
    initial_predicate: HistPrim<String>,
    predicate: Prim<String>,
    node: NodeState,
}

#[derive(Clone)]
struct TripleState(Rc<TripleState_>);

fn new_triple_state(
    pc: &mut ProcessingContext,
    t: &Triple,
    incoming: bool,
    add: bool,
    delete_all: HistPrim<bool>,
) -> TripleState {
    let value = if incoming {
        t.subject.clone()
    } else {
        t.object.clone()
    };
    return TripleState(Rc::new(TripleState_ {
        incoming: incoming,
        initial_predicate: HistPrim::new(pc, t.predicate.clone()),
        add: add,
        delete: HistPrim::new(pc, false),
        delete_all: delete_all,
        predicate: Prim::new(t.predicate.clone()),
        node: new_node_state(pc, &value),
    }));
}

fn build_edit_node(pc: &mut ProcessingContext, node: &NodeState) -> El {
    let options =
        [
            (NodeEditType::Str, "Text"),
            (NodeEditType::Num, "Number"),
            (NodeEditType::Bool, "Bool"),
            (NodeEditType::Json, "JSON"),
            (NodeEditType::File, "File"),
            (NodeEditType::FileUpload, "Upload new file"),
        ]
            .into_iter()
            .map(|(k, v)| (serde_json::to_string(&k).unwrap(), v.to_string()))
            .collect::<HashMap<_, _>>();
    let inp_type_el = style_export::leaf_input_enum(style_export::LeafInputEnumArgs {
        id: None,
        title: "Node type".to_string(),
        options: options,
        value: serde_json::to_string(&node.type_.get()).unwrap(),
    }).root;
    let inp_value_group_el = style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root;
    inp_type_el.ref_on("input", {
        let node = node.clone();
        let eg = pc.eg();
        let inp_ele = inp_type_el.raw().dyn_into::<HtmlInputElement>().unwrap();
        move |_| eg.event(|pc| {
            node.type_.set(pc, serde_json::from_str::<NodeEditType>(&inp_ele.value()).unwrap());
        }).unwrap()
    });

    // When changing element type, munge the value to fit the new type and replace the
    // input element
    inp_type_el.ref_own(
        |_| link!(
            (pc = pc),
            (node_type = node.type_.clone()),
            (node_value = node.value.clone()),
            (inp_value_group_el = inp_value_group_el.clone()),
            {
                let node_value_as_string = || -> String {
                    let s = match &*node_value.borrow() {
                        NodeEditValue::String(s) => s.clone(),
                        NodeEditValue::Bool(v) => {
                            return if *v {
                                "true"
                            } else {
                                "false"
                            }.to_string();
                        },
                        NodeEditValue::Upload(v) => {
                            return v.old.clone();
                        },
                    };
                    match &*node_type.borrow() {
                        NodeEditType::Str => {
                            return s.clone();
                        },
                        NodeEditType::Num => {
                            return s.clone();
                        },
                        NodeEditType::Bool => {
                            unreachable!();
                        },
                        NodeEditType::Json => {
                            let Ok(serde_json::Value::String(v)) =
                                serde_json::from_str::<serde_json::Value>(&s) else {
                                    return s.clone();
                                };
                            return v;
                        },
                        NodeEditType::File => {
                            return s.clone();
                        },
                        NodeEditType::FileUpload => {
                            unreachable!();
                        },
                    }
                };
                let build_text_input = |pc: &mut ProcessingContext, input_: El| -> El {
                    let input_value = Prim::new("".to_string());
                    input_.ref_on("input", {
                        let eg = pc.eg();
                        let input_value = input_value.clone();
                        move |ev| {
                            let ele = ev.target().unwrap().dyn_into::<HtmlElement>().unwrap();
                            eg.event(|pc| {
                                input_value.set(pc, ele.text_content().unwrap_or_default());
                            }).unwrap();
                            if ele.text_content().unwrap_or_default() == "" {
                                // Remove `<br/>` :vomit:
                                ele.set_inner_html("");
                            }
                        }
                    });
                    input_.ref_own(|input_| (
                        //. .
                        link!((pc = pc), (node_value = node_value.clone()), (input_value = input_value.clone()), (input_ = input_.weak()), {
                            let input_ = input_.upgrade()?;
                            let v = exenum!(&*node_value.borrow(), NodeEditValue:: String(v) => v.clone()).unwrap();
                            input_value.set(pc, v.clone());
                            input_.ref_text(&v);
                        }),
                        link!((pc = pc), (input_value = input_value.clone()), (node_value = node_value.clone()), (), {
                            node_value.set(pc, NodeEditValue::String(input_value.borrow().clone()));
                        }),
                    ));
                    return input_;
                };

                // Convert to expected value type, create a new input for the new input type
                let new_input;
                let new_value;
                match &*node_type.borrow() {
                    NodeEditType::Num => {
                        let s = node_value_as_string();
                        new_value = NodeEditValue::String(s.clone());
                        let input_ = style_export::leaf_input_number(style_export::LeafInputNumberArgs {
                            id: None,
                            title: "Node".into(),
                            value: s,
                        }).root;
                        let input_value = Prim::new("".to_string());
                        input_.ref_on("input", {
                            let eg = pc.eg();
                            let input_value = input_value.clone();
                            move |ev| {
                                eg.event(|pc| {
                                    let e = ev.target().unwrap().dyn_into::<HtmlInputElement>().unwrap();
                                    input_value.set(pc, e.value());
                                }).unwrap();
                            }
                        });
                        input_.ref_own(|input_| (
                            //. .
                            link!((pc = pc), (node_value = node_value.clone()), (input_value = input_value.clone()), (input_ = input_.weak()), {
                                let input_ = input_.upgrade()?;
                                let v =
                                    exenum!(&*node_value.borrow(), NodeEditValue:: String(v) => v.clone()).unwrap();
                                input_value.set(pc, v.clone());
                                input_.ref_text(&v);
                            }),
                            link!(
                                (pc = pc),
                                (input_value = input_value.clone()),
                                (node_value = node_value.clone()),
                                (),
                                {
                                    node_value.set(pc, NodeEditValue::String(input_value.borrow().clone()));
                                }
                            ),
                        ));
                        new_input = input_;
                    },
                    NodeEditType::Bool => {
                        let new_value1 = (|| {
                            let s = match &*node_value.borrow() {
                                NodeEditValue::String(s) => s.clone(),
                                NodeEditValue::Bool(v) => {
                                    return *v;
                                },
                                NodeEditValue::Upload(v) => {
                                    v.old.clone()
                                },
                            };
                            match &*node_type.borrow() {
                                NodeEditType::Str => {
                                    return false;
                                },
                                NodeEditType::Num => {
                                    return false;
                                },
                                NodeEditType::Bool => {
                                    unreachable!();
                                },
                                NodeEditType::Json => {
                                    let Ok(serde_json::Value::Bool(v)) =
                                        serde_json::from_str::<serde_json::Value>(&s) else {
                                            return false;
                                        };
                                    return v;
                                },
                                NodeEditType::File => {
                                    return false;
                                },
                                NodeEditType::FileUpload => {
                                    return false;
                                },
                            }
                        })();
                        new_value = NodeEditValue::Bool(new_value1);
                        let input_value = Prim::new(false);
                        new_input = style_export::leaf_input_bool(style_export::LeafInputBoolArgs {
                            id: None,
                            title: "Value".to_string(),
                            value: new_value1,
                        }).root;
                        new_input.ref_on("input", {
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
                        });
                        new_input.ref_own(|input_el| (
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
                        let s = node_value_as_string();
                        new_value = NodeEditValue::String(s.clone());
                        new_input =
                            build_text_input(pc, style_export::leaf_input_text(style_export::LeafInputTextArgs {
                                id: None,
                                title: "Node".into(),
                                value: s,
                            }).root);
                    },
                    NodeEditType::Json => {
                        let s = node_value_as_string();
                        new_value = NodeEditValue::String(s.clone());
                        new_input =
                            build_text_input(pc, style_export::leaf_input_text(style_export::LeafInputTextArgs {
                                id: None,
                                title: "Node".into(),
                                value: s,
                            }).root);
                    },
                    NodeEditType::File => {
                        let s = node_value_as_string();
                        new_value = NodeEditValue::String(s.clone());
                        let input_value = Prim::new("".to_string());
                        let style_res = style_export::leaf_input_text_media(style_export::LeafInputTextMediaArgs {
                            id: None,
                            title: "Node".into(),
                            value: s,
                        });
                        let input_ = style_res.input;
                        input_.ref_on("input", {
                            let eg = pc.eg();
                            let input_value = input_value.clone();
                            let input_ = input_.weak();
                            move |_| eg.event(|pc| {
                                let Some(input_) = input_.upgrade() else {
                                    return;
                                };
                                let v = input_.raw().dyn_into::<HtmlInputElement>().unwrap().value();
                                input_value.set(pc, v);
                            }).unwrap()
                        });
                        input_.ref_own(|input_| (
                            //. .
                            link!((pc = pc), (node_value = node_value.clone()), (input_value = input_value.clone()), (input_ = input_.weak()), {
                                let input_ = input_.upgrade()?;
                                let v =
                                    exenum!(&*node_value.borrow(), NodeEditValue:: String(v) => v.clone()).unwrap();
                                input_value.set(pc, v.clone());
                                input_.ref_text(&v);
                            }),
                            link!(
                                (pc = pc),
                                (input_value = input_value.clone()),
                                (node_value = node_value.clone()),
                                (),
                                {
                                    node_value.set(pc, NodeEditValue::String(input_value.borrow().clone()));
                                }
                            ),
                            link!(
                                (_pc = pc),
                                (input_value = input_value.clone()),
                                (),
                                (media = style_res.media.weak()),
                                {
                                    let media = media.upgrade()?;
                                    media.ref_clear();
                                    let Ok(h) = FileHash::from_str(&*input_value.borrow()) else {
                                        return None;
                                    };
                                    let src_url = file_url(&state().env, &h);
                                    media.ref_push(el_async(async move {
                                        ta_return!(Vec < El >, String);
                                        let meta =
                                            req_post_json(
                                                &state().env.base_url,
                                                ReqGetNodeMeta { node: Node::File(h.clone()) },
                                            ).await?;
                                        match meta {
                                            Some(meta) => {
                                                match meta.mime.split("/").next().unwrap() {
                                                    "image" => {
                                                        return Ok(vec![el("img").attr("src", &src_url)]);
                                                    },
                                                    "video" => {
                                                        return Ok(
                                                            vec![
                                                                el("video")
                                                                    .attr("controls", "true")
                                                                    .attr("src", &src_url)
                                                            ],
                                                        );
                                                    },
                                                    "audio" => {
                                                        return Ok(
                                                            vec![
                                                                el("audio")
                                                                    .attr("controls", "true")
                                                                    .attr("src", &src_url)
                                                            ],
                                                        );
                                                    },
                                                    _ => {
                                                        return Ok(vec![]);
                                                    },
                                                }
                                            },
                                            None => {
                                                return Ok(vec![]);
                                            },
                                        }
                                    }));
                                }
                            ),
                        ));
                        new_input = style_res.root;
                    },
                    NodeEditType::FileUpload => {
                        new_value = NodeEditValue::Upload(NodeEditValueUpload {
                            old: node_value_as_string(),
                            new: None,
                        });
                        let style_res = style_export::leaf_input_file(style_export::LeafInputFileArgs {
                            id: None,
                            title: format!("Node"),
                        });
                        let input_ = style_res.input;
                        let input_value = Prim::new(None);
                        input_.ref_on("input", {
                            let eg = pc.eg();
                            let input_value = input_value.clone();
                            let input_ = input_.weak();
                            move |_| {
                                let Some(input_) = input_.upgrade() else {
                                    return;
                                };
                                eg.event(|pc| {
                                    superif!({
                                        let Some(files) =
                                            input_.raw().dyn_into::<HtmlInputElement>().unwrap().files() else {
                                                break 'nope;
                                            };
                                        let Some(file) = files.item(0) else {
                                            break 'nope;
                                        };
                                        input_value.set(pc, Some(file));
                                    } 'nope {
                                        input_value.set(pc, None);
                                    });
                                }).unwrap();
                            }
                        });
                        input_.ref_own(|input_| (
                            //. .
                            link!((_pc = pc), (_node_value = node_value.clone()), (_input_value = input_value.clone()), (_input_ = input_.weak()), {
                                // Should only be called on restore, but initial values should never be files
                                if true {
                                    unreachable!("restoring file input");
                                }
                            }),
                            link!(
                                (pc = pc),
                                (input_value = input_value.clone()),
                                (node_value = node_value.clone()),
                                (),
                                {
                                    let new_v = NodeEditValue::Upload(NodeEditValueUpload {
                                        old: exenum!(&*node_value.borrow(), NodeEditValue:: Upload(v) => v)
                                            .unwrap()
                                            .old
                                            .clone(),
                                        new: input_value.borrow().clone(),
                                    });
                                    node_value.set(pc, new_v);
                                }
                            ),
                        ));
                        new_input = style_res.root;
                    },
                }
                node_value.set(pc, new_value);
                inp_value_group_el.ref_clear();
                inp_value_group_el.ref_push(new_input);
            }
        ),
    );
    let out = style_export::leaf_node_edit_node(style_export::LeafNodeEditNodeArgs {
        input_type: inp_type_el,
        input_value: inp_value_group_el,
    }).root;
    out.ref_own(|out| (
        // Update modified/invalid flags
        link!(
            (_pc = pc),
            (input_type = node.type_.clone(), input_value = node.value.clone(), initial = node.initial.clone()),
            (),
            (out = out.weak()),
            {
                let input_el = out.upgrade()?;
                let initial = initial.borrow();
                let initial_type = &initial.0;
                let initial_value = &initial.1;
                let modified;
                let invalid;
                if !match input_type.get() {
                    NodeEditType::Str => {
                        true
                    },
                    NodeEditType::Num => {
                        exenum!(
                            serde_json::from_str::<serde_json::Value>(
                                exenum!(&*input_value.borrow(), NodeEditValue:: String(v) => v).unwrap(),
                            ),
                            Ok(serde_json::Value::Number(_)) =>()
                        ).is_some()
                    },
                    NodeEditType::Bool => {
                        true
                    },
                    NodeEditType::Json => {
                        serde_json::from_str::<serde_json::Value>(
                            exenum!(&*input_value.borrow(), NodeEditValue:: String(v) => v).unwrap(),
                        ).is_ok()
                    },
                    NodeEditType::File => {
                        FileHash::from_str(
                            exenum!(&*input_value.borrow(), NodeEditValue:: String(v) => v).unwrap(),
                        ).is_ok()
                    },
                    NodeEditType::FileUpload => {
                        true
                    },
                } {
                    modified = false;
                    invalid = true;
                } else if &*input_type.borrow() != initial_type || &*input_value.borrow() != initial_value {
                    modified = true;
                    invalid = false;
                } else {
                    modified = false;
                    invalid = false;
                }
                input_el.ref_modify_classes(
                    &[
                        (&style_export::class_state_invalid().value, invalid),
                        (&style_export::class_state_modified().value, modified),
                    ],
                );
            }
        ),
    ));
    return out;
}

fn build_edit_triple(pc: &mut ProcessingContext, triple: &TripleState, new: bool) -> El {
    let buttons_el = {
        let style_res = style_export::leaf_node_edit_buttons();
        let button_revert = style_res.button_revert;
        button_revert.ref_on("click", {
            let triple = triple.clone();
            let eg = pc.eg();
            move |_| eg.event(|pc| {
                triple.0.predicate.set(pc, triple.0.initial_predicate.borrow().clone());
                let (node_type, node_value) = triple.0.node.initial.borrow().clone();
                triple.0.node.type_.set(pc, node_type);
                triple.0.node.value.set(pc, node_value);
            }).unwrap()
        });
        let button_delete = style_res.button_delete;
        button_delete.ref_on("click", {
            let triple = triple.clone();
            let eg = pc.eg();
            move |_| eg.event(|pc| {
                triple.0.delete.set(pc, !triple.0.delete.get());
            }).unwrap()
        });
        button_delete.ref_own(
            |out| link!(
                (_pc = pc),
                (deleted = triple.0.delete.clone(), deleted_all = triple.0.delete_all.clone()),
                (),
                (out = out.weak()),
                {
                    let out = out.upgrade()?;
                    out.ref_modify_classes(
                        &[(&style_export::class_state_deleted().value, deleted.get() | deleted_all.get())],
                    );
                }
            ),
        );
        style_res.root
    };
    let node_el = build_edit_node(pc, &triple.0.node);
    let predicate_el = {
        let predicate_value = "".to_string();
        let predicate_res =
            style_export::leaf_node_edit_predicate(
                style_export::LeafNodeEditPredicateArgs { value: predicate_value.clone() },
            );
        let input_value = Prim::new(predicate_value);
        let out = predicate_res.root;
        out.ref_on("input", {
            let eg = pc.eg();
            let input_value = input_value.clone();
            move |ev| eg.event(|pc| {
                let ele = ev.target().unwrap().dyn_into::<HtmlElement>().unwrap();
                input_value.set(pc, ele.text_content().unwrap_or_default());
                if ele.text_content().unwrap_or_default() == "" {
                    // Remove `<br/>` :vomit:
                    ele.set_inner_html("");
                }
            }).unwrap()
        });
        out.ref_own(|out| (
            //. .
            link!(
                (pc = pc),
                (predicate_value = triple.0.predicate.clone()),
                (input_value = input_value.clone()),
                (out = out.weak()),
                {
                    let input_el = out.upgrade()?;
                    input_value.set(pc, predicate_value.borrow().clone());
                    input_el.ref_text(predicate_value.borrow().as_str());
                }
            ),
            link!((pc = pc), (input_value = input_value.clone()), (predicate_value = triple.0.predicate.clone()), (), {
                predicate_value.set(pc, input_value.borrow().clone());
            }),
            link!(
                (_pc = pc),
                (predicate_value = triple.0.predicate.clone(), initial_value = triple.0.initial_predicate.clone()),
                (),
                (out = out.weak()) {
                    let out = out.upgrade()?;
                    out.ref_modify_classes(
                        &[
                            (
                                &style_export::class_state_modified().value,
                                &*predicate_value.borrow() != &*initial_value.borrow(),
                            ),
                        ],
                    );
                }
            ),
        ));
        out
    };
    if triple.0.incoming {
        return style_export::cont_node_row_incoming(style_export::ContNodeRowIncomingArgs {
            children: vec![buttons_el, node_el, predicate_el],
            new: new,
        }).root;
    } else {
        return style_export::cont_node_row_outgoing(style_export::ContNodeRowOutgoingArgs {
            children: vec![buttons_el, predicate_el, node_el],
            new: new,
        }).root;
    }
}

pub fn build_page_node_edit(pc: &mut ProcessingContext, edit_title: &str, node: &Node) {
    set_page(pc, &format!("Edit {}", edit_title), el_async_(true, {
        let eg = pc.eg();
        let node = node.clone();
        let title = edit_title.to_string();
        async move {
            ta_return!(Vec < El >, String);
            let triples = req_post_json(&state().env.base_url, ReqGetTriplesAround { node: node.clone() }).await?;
            return eg.event(|pc| {
                let error_slot = style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root;
                let mut out = vec![error_slot.clone()];
                let mut bar_out = vec![];
                let pivot_state = new_pivot_state(pc, &node);
                let triple_states = Rc::new(RefCell::new(vec![] as Vec<TripleState>));

                // Top buttons
                let mut buttons_out = vec![];
                {
                    let style_res =
                        style_export::leaf_button_small_view(
                            style_export::LeafButtonSmallViewArgs {
                                link: ministate_octothorpe(
                                    &crate::libnonlink::ministate::Ministate::NodeView(MinistateNodeView {
                                        title: title.clone(),
                                        node: node.clone(),
                                    }),
                                ),
                            },
                        ).root;
                    buttons_out.push(style_res);
                }

                // Incoming triples
                {
                    let triples_box =
                        style_export::cont_page_node_section_rel(
                            style_export::ContPageNodeSectionRelArgs { children: vec![] },
                        ).root;
                    for t in triples.incoming {
                        let triple = new_triple_state(pc, &t, true, false, pivot_state.0.delete.clone());
                        triples_box.ref_push(build_edit_triple(pc, &triple, false));
                        triple_states.borrow_mut().push(triple);
                    }
                    let button_add =
                        style_export::leaf_button_node_edit_add(
                            style_export::LeafButtonNodeEditAddArgs { hint: "Add incoming".to_string() },
                        ).root;
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
                            }, true, true, pivot_state.0.delete.clone());
                            incoming_triples_box.ref_splice(0, 0, vec![build_edit_triple(pc, &triple, true)]);
                            triple_states.borrow_mut().push(triple);
                        }).unwrap()
                    });
                    out.push(style_export::cont_node_row_incoming(style_export::ContNodeRowIncomingArgs {
                        children: vec![button_add],
                        new: true,
                    }).root);
                    out.push(triples_box);
                }

                // Pivot
                {
                    let buttons_el = {
                        let style_res = style_export::leaf_node_edit_buttons();
                        let button_revert = style_res.button_revert;
                        button_revert.ref_on("click", {
                            let pivot_original = node;
                            let pivot = pivot_state.clone();
                            let eg = pc.eg();
                            move |_| eg.event(|pc| {
                                let (node_type, node_value) = node_to_type_value(&pivot_original);
                                pivot.0.node.type_.set(pc, node_type);
                                pivot.0.node.value.set(pc, node_value);
                            }).unwrap()
                        });
                        let button_delete = style_res.button_delete;
                        button_delete.ref_on("click", {
                            let pivot_state = pivot_state.clone();
                            let eg = pc.eg();
                            move |_| eg.event(|pc| {
                                pivot_state.0.delete.set(pc, !pivot_state.0.delete.get());
                            }).unwrap()
                        });
                        style_res.root
                    };
                    let style_res = build_edit_node(pc, &pivot_state.0.node);
                    let children = vec![buttons_el, style_res];
                    out.push(
                        style_export::cont_node_section_center(
                            style_export::ContNodeSectionCenterArgs { children: children },
                        )
                            .root
                            .own(
                                |ele| (
                                    link!(
                                        (_pc = pc),
                                        (deleted = pivot_state.0.delete.clone()),
                                        (),
                                        (ele = ele.weak()),
                                        {
                                            let pivot_root = ele.upgrade()?;
                                            pivot_root.ref_modify_classes(
                                                &[(&style_export::class_state_deleted().value, deleted.get())],
                                            );
                                        }
                                    ),
                                ),
                            ),
                    );
                }

                // Outgoing triples
                {
                    let triples_box =
                        style_export::cont_page_node_section_rel(
                            style_export::ContPageNodeSectionRelArgs { children: vec![] },
                        ).root;
                    for t in triples.outgoing {
                        let triple = new_triple_state(pc, &t, false, false, pivot_state.0.delete.clone());
                        triples_box.ref_push(build_edit_triple(pc, &triple, false));
                        triple_states.borrow_mut().push(triple);
                    }
                    out.push(triples_box.clone());
                    let button_add =
                        style_export::leaf_button_node_edit_add(
                            style_export::LeafButtonNodeEditAddArgs { hint: "Add outgoing".to_string() },
                        ).root;
                    button_add.ref_on("click", {
                        let eg = pc.eg();
                        let triple_states = triple_states.clone();
                        let triples_box = triples_box;
                        let pivot_state = pivot_state.clone();
                        move |_| eg.event(|pc| {
                            let triple = new_triple_state(pc, &Triple {
                                subject: Node::Value(serde_json::Value::String("".to_string())),
                                predicate: "".to_string(),
                                object: Node::Value(serde_json::Value::String("".to_string())),
                            }, false, true, pivot_state.0.delete.clone());
                            triples_box.ref_push(build_edit_triple(pc, &triple, true));
                            triple_states.borrow_mut().push(triple);
                        }).unwrap()
                    });
                    out.push(style_export::cont_node_row_outgoing(style_export::ContNodeRowOutgoingArgs {
                        children: vec![button_add],
                        new: true,
                    }).root);
                }

                // Edit form controls
                let button_save = style_export::leaf_button_big_save().root;
                button_save.ref_on("click", {
                    let triple_states = triple_states.clone();
                    let pivot_state = pivot_state.clone();
                    let error_slot = error_slot.weak();
                    let save_thinking = Rc::new(RefCell::new(None));
                    let eg = pc.eg();
                    move |ev| {
                        if save_thinking.borrow().is_some() {
                            return;
                        }
                        {
                            let Some(error_slot) = error_slot.upgrade() else {
                                return;
                            };
                            error_slot.ref_clear();
                        }
                        let button = ev.target().unwrap().dyn_into::<web_sys::HtmlElement>().unwrap();
                        button.class_list().add_1(&style_export::class_state_thinking().value).unwrap();
                        *save_thinking.borrow_mut() = Some(spawn_rooted({
                            let triple_states = triple_states.clone();
                            let pivot_state = pivot_state.clone();
                            let error_slot = error_slot.clone();
                            let eg = eg.clone();
                            async move {
                                let mut triple_nodes_predicates = vec![];
                                let mut file_unique = 0usize;
                                let pivot_node = type_value_to_node({
                                    file_unique += 1;
                                    file_unique
                                }, &*pivot_state.0.node.type_.borrow(), &*pivot_state.0.node.value.borrow());
                                let res = async {
                                    ta_return!(HashMap < usize, FileHash >, String);
                                    let mut add = vec![];
                                    let mut remove = vec![];
                                    let delete_all = *pivot_state.0.delete.borrow();
                                    let pivot_node_initial = {
                                        let pivot_node_initial = pivot_state.0.node.initial.borrow();
                                        type_value_to_node({
                                            file_unique += 1;
                                            file_unique
                                        }, &pivot_node_initial.0, &pivot_node_initial.1)
                                    };
                                    let pivot_changed = pivot_node != pivot_node_initial;
                                    for triple in &*RefCell::borrow(&triple_states) {
                                        // Get current values
                                        let triple_node = type_value_to_node({
                                            file_unique += 1;
                                            file_unique
                                        }, &*triple.0.node.type_.borrow(), &*triple.0.node.value.borrow());
                                        let triple_predicate = triple.0.predicate.borrow().clone();
                                        triple_nodes_predicates.push(
                                            (triple_predicate.clone(), triple_node.clone()),
                                        );

                                        // Get original values for comparison
                                        let triple_predicate_initial = triple.0.initial_predicate.borrow();
                                        let triple_node_initial = {
                                            let triple_node_initial = triple.0.node.initial.borrow();
                                            type_value_to_node({
                                                file_unique += 1;
                                                file_unique
                                            }, &triple_node_initial.0, &triple_node_initial.1)
                                        };

                                        // Classify if changed/deleted
                                        let changed =
                                            pivot_changed || triple_node != triple_node_initial ||
                                                triple.0.predicate.borrow().as_str() != &*triple_predicate_initial;

                                        // If not new but deleted or changed, delete first
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
                                                subject: exenum!(old_subject, CommitNode:: Node(v) => v).unwrap(),
                                                predicate: triple_predicate_initial.clone(),
                                                object: exenum!(old_object, CommitNode:: Node(v) => v).unwrap(),
                                            });
                                        }

                                        // If new or changed, write the new triple
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
                                            add.push(CommitTriple {
                                                subject: subject,
                                                predicate: triple.0.predicate.borrow().clone(),
                                                object: object,
                                            });
                                        }
                                    }

                                    // Send compiled changes
                                    return Ok(commit(add, remove).await?);
                                }.await;
                                button.class_list().remove_1(&style_export::class_state_thinking().value).unwrap();
                                match res {
                                    Ok(mut file_lookup) => {
                                        eg.event(|pc| {
                                            pivot_state.0.node.initial.set(pc, node_to_type_value(&match pivot_node {
                                                CommitNode::Node(n) => n,
                                                CommitNode::File(unique, _) => Node::File(
                                                    file_lookup.remove(&unique).unwrap(),
                                                ),
                                            }));
                                            for (
                                                triple,
                                                (sent_pred, sent_node),
                                            ) in Iterator::zip(
                                                RefCell::borrow(&triple_states).iter(),
                                                triple_nodes_predicates.into_iter(),
                                            ) {
                                                triple.0.initial_predicate.set(pc, sent_pred);
                                                triple.0.node.initial.set(pc, node_to_type_value(&match sent_node {
                                                    CommitNode::Node(n) => n,
                                                    CommitNode::File(unique, _) => Node::File(
                                                        file_lookup.remove(&unique).unwrap(),
                                                    ),
                                                }));
                                            }
                                        }).unwrap();
                                    },
                                    Err(e) => {
                                        let Some(error_slot) = error_slot.upgrade() else {
                                            return;
                                        };
                                        error_slot.ref_push(
                                            style_export::leaf_err_block(style_export::LeafErrBlockArgs {
                                                in_root: false,
                                                data: e,
                                            }).root,
                                        );
                                    },
                                }
                            }
                        }));
                    }
                });
                bar_out.push(button_save);
                return Ok(vec![style_export::cont_page_node_edit(style_export::ContPageNodeEditArgs {
                    page_button_children: buttons_out,
                    children: out,
                    bar_children: bar_out,
                }).root]);
            }).unwrap();
        }
    }));
}
