use {
    super::{
        api::req_post_json,
        state::set_page,
    },
    crate::libnonlink::{
        commit::{
            self,
            CommitNode,
            CommitTriple,
        },
        ministate::{
            ministate_octothorpe,
            Ministate,
            MinistateNodeView,
        },
        page_node_view::node_to_text,
        playlist::{
            categorize_mime_media,
            PlaylistEntryMediaType,
        },
        state::{
            change_ministate,
            state,
        },
    },
    by_address::ByAddress,
    flowcontrol::{
        exenum,
        shed,
        superif,
        ta_return,
    },
    gloo::storage::{
        LocalStorage,
        Storage,
    },
    lunk::{
        link,
        HistPrim,
        Prim,
        ProcessingContext,
    },
    rooting::{
        scope_any,
        spawn_rooted,
        El,
        ScopeValue,
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
            ReqGetNodeMeta,
            ReqGetTriplesAround,
            Triple,
        },
    },
    std::{
        cell::RefCell,
        collections::{
            HashMap,
            HashSet,
        },
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
        HtmlSelectElement,
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
    initial: HistPrim<Option<(NodeEditType, NodeEditValue)>>,
}

fn new_node_state(pc: &mut ProcessingContext, node: Option<&Node>, restore_node: Option<&Node>) -> NodeState {
    if let Some(restore_node) = restore_node {
        let initial;
        if let Some(node) = node {
            let (type_, value) = node_to_type_value(node);
            initial = HistPrim::new(pc, Some((type_, value)));
        } else {
            initial = HistPrim::new(pc, None);
        }
        let (restore_type, restore_value) = node_to_type_value(restore_node);
        return NodeState {
            type_: HistPrim::new(pc, restore_type),
            value: HistPrim::new(pc, restore_value),
            initial: initial,
        };
    } else if let Some(node) = node {
        let (type_, value) = node_to_type_value(node);
        return NodeState {
            type_: HistPrim::new(pc, type_.clone()),
            value: HistPrim::new(pc, value.clone()),
            initial: HistPrim::new(pc, Some((type_, value))),
        };
    } else {
        return NodeState {
            type_: HistPrim::new(pc, NodeEditType::Str),
            value: HistPrim::new(pc, NodeEditValue::String(format!(""))),
            initial: HistPrim::new(pc, None),
        };
    }
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

#[derive(Serialize, Deserialize, Clone, Debug)]
struct DraftRel {
    delete: bool,
    predicate: String,
    node: Node,
}

#[derive(Serialize, Deserialize, Default)]
struct DraftBody {
    // All: None if no modifications
    pivot: Option<(bool, Node)>,
    incoming: Vec<((Node, String), DraftRel)>,
    new_incoming: Vec<DraftRel>,
    outgoing: Vec<((String, Node), DraftRel)>,
    new_outgoing: Vec<DraftRel>,
}

#[derive(Clone, Hash, PartialEq, Eq, Default)]
struct MutDraftRel(ByAddress<Rc<RefCell<Option<DraftRel>>>>);

impl MutDraftRel {
    fn new(v: DraftRel) -> Self {
        return Self(ByAddress(Rc::new(RefCell::new(Some(v)))));
    }
}

struct Draft_ {
    key: RefCell<Node>,
    pivot: RefCell<Option<(bool, Node)>>,
    incoming: RefCell<HashMap<(Node, String), MutDraftRel>>,
    new_incoming: RefCell<HashSet<MutDraftRel>>,
    outgoing: RefCell<HashMap<(String, Node), MutDraftRel>>,
    new_outgoing: RefCell<HashSet<MutDraftRel>>,
}

#[derive(Clone)]
struct DraftData(Rc<Draft_>);

fn format_localstorage_draft_key(key: &Node) -> String {
    return format!("{}_{}", LOCALSTORAGE_NODE_EDIT_PREFIX, serde_json::to_string(key).unwrap());
}

pub const LOCALSTORAGE_NODE_EDIT_PREFIX: &str = "nodeedit";

fn restore_draft(key: Node) -> DraftData {
    let data;
    if let Ok(v) = LocalStorage::get::<DraftBody>(format_localstorage_draft_key(&key)) {
        data = v;
    } else {
        data = Default::default();
    };
    return DraftData(Rc::new(Draft_ {
        key: RefCell::new(key),
        pivot: RefCell::new(data.pivot),
        incoming: RefCell::new(data.incoming.into_iter().map(|(k, v)| (k, MutDraftRel::new(v))).collect()),
        new_incoming: RefCell::new(data.new_incoming.into_iter().map(|v| MutDraftRel::new(v)).collect()),
        outgoing: RefCell::new(data.outgoing.into_iter().map(|(k, v)| (k, MutDraftRel::new(v))).collect()),
        new_outgoing: RefCell::new(data.new_outgoing.into_iter().map(|v| MutDraftRel::new(v)).collect()),
    }));
}

fn persist_draft(s: &DraftData) {
    if let Err(e) = LocalStorage::set(format_localstorage_draft_key(&*s.0.key.borrow()), &DraftBody {
        pivot: s.0.pivot.borrow().clone(),
        incoming: s.0.incoming.borrow().iter().filter_map(|(k, v)| match &*v.0.borrow() {
            Some(v) => Some((k.clone(), v.clone())),
            None => None,
        }).collect(),
        new_incoming: s.0.new_incoming.borrow().iter().filter_map(|x| x.0.0.borrow().clone()).collect(),
        outgoing: s.0.outgoing.borrow().iter().filter_map(|(k, v)| {
            match &*v.0.borrow() {
                Some(v) => Some((k.clone(), v.clone())),
                None => None,
            }
        }).collect(),
        new_outgoing: s.0.new_outgoing.borrow().iter().filter_map(|x| x.0.0.borrow().clone()).collect(),
    }) {
        state().log.log(&format!("Error saving draft: {}", e));
    }
}

fn clear_draft(s: &DraftData, new_key: &Node) {
    LocalStorage::delete(format_localstorage_draft_key(&*s.0.key.borrow()));
    s.0.incoming.borrow_mut().clear();
    s.0.new_incoming.borrow_mut().clear();
    s.0.outgoing.borrow_mut().clear();
    s.0.new_outgoing.borrow_mut().clear();
    *s.0.pivot.borrow_mut() = None;
    *s.0.key.borrow_mut() = new_key.clone();
}

struct PivotState_ {
    delete_all: HistPrim<bool>,
    node: NodeState,
    _own: ScopeValue,
}

#[derive(Clone)]
struct PivotState(Rc<PivotState_>);

fn new_pivot_state(pc: &mut ProcessingContext, save_data: &DraftData, source_node: &Node) -> PivotState {
    let delete_all;
    let node;
    shed!{
        if let Some(restore) = &*save_data.0.pivot.borrow() {
            let (restore_delete, restore_node) = restore;
            delete_all = HistPrim::new(pc, *restore_delete);
            node = new_node_state(pc, Some(source_node), Some(&restore_node));
        } else {
            delete_all = HistPrim::new(pc, false);
            node = new_node_state(pc, Some(source_node), None);
        }
    }
    return PivotState(Rc::new(PivotState_ {
        _own: scope_any(
            // Persist data to draft after changes (remove if not changed or unstorable,
            // otherwise Some)
            link!(
                (_pc = pc),
                (delete_all = delete_all.clone(), node_type = node.type_.clone(), node_value = node.value.clone()),
                (),
                (save_data = save_data.clone(), node_initial = node.initial.clone()) {
                    *save_data.0.pivot.borrow_mut() = shed!{
                        let CommitNode::Node(node) =
                            type_value_to_node(0, &*node_type.borrow(), &*node_value.borrow()) else {
                                break None;
                            };
                        if let Some(node_initial) = &*node_initial.borrow() {
                            let node_initial_node =
                                exenum!(
                                    type_value_to_node(0, &node_initial.0, &node_initial.1),
                                    CommitNode:: Node(n) => n
                                ).unwrap();
                            if node == node_initial_node && !delete_all.get() {
                                break None;
                            }
                        }
                        break Some((delete_all.get(), node));
                    };
                    persist_draft(&save_data);
                }
            ),
        ),
        delete_all: delete_all,
        node: node,
    }));
}

struct TripleState_ {
    incoming: bool,
    delete: HistPrim<bool>,
    delete_all: HistPrim<bool>,
    initial_predicate: HistPrim<Option<String>>,
    predicate: Prim<String>,
    node: NodeState,
    _own: ScopeValue,
}

#[derive(Clone)]
struct TripleState(Rc<TripleState_>);

fn new_triple_state(
    pc: &mut ProcessingContext,
    draft_data: &DraftData,
    source_triple: Option<&Triple>,
    incoming: bool,
    draft_entry: MutDraftRel,
    delete_all: HistPrim<bool>,
) -> TripleState {
    let initial_predicate;
    if let Some(source_triple) = source_triple {
        initial_predicate = HistPrim::new(pc, Some(source_triple.predicate.clone()));
    } else {
        initial_predicate = HistPrim::new(pc, None);
    }
    let predicate;
    if let Some(d) = &*draft_entry.0.0.borrow() {
        predicate = Prim::new(d.predicate.clone());
    } else if let Some(source_triple) = source_triple {
        predicate = Prim::new(source_triple.predicate.clone());
    } else {
        predicate = Prim::new(format!(""));
    }
    let delete;
    if let Some(d) = &*draft_entry.0.0.borrow() {
        delete = HistPrim::new(pc, d.delete);
    } else {
        delete = HistPrim::new(pc, false);
    };
    let node = new_node_state(
        //. .
        pc,
        if let Some(source_triple) = source_triple {
            if incoming {
                Some(&source_triple.subject)
            } else {
                Some(&source_triple.object)
            }
        } else {
            None
        },
        if let Some(d) = &*draft_entry.0.0.borrow() {
            Some(&d.node)
        } else {
            None
        },
    );
    return TripleState(Rc::new(TripleState_ {
        _own: scope_any(
            // Persist data to draft after changes (remove if not changed or unstorable,
            // otherwise Some)
            link!(
                (_pc = pc),
                (
                    delete = delete.clone(),
                    predicate = predicate.clone(),
                    node_type = node.type_.clone(),
                    node_value = node.value.clone(),
                ),
                (),
                (
                    save_data = draft_data.clone(),
                    initial_node = node.initial.clone(),
                    initial_predicate = initial_predicate.clone(),
                    draft_entry = draft_entry,
                ) {
                    shed!{
                        let CommitNode::Node(node) =
                            type_value_to_node(0, &*node_type.borrow(), &*node_value.borrow()) else {
                                *draft_entry.0.borrow_mut() = None;
                                break;
                            };
                        if let Some((initial_predicate, initial_node)) =
                            Option::zip(initial_predicate.borrow().as_ref(), initial_node.borrow().as_ref()) {
                            let initial_node =
                                exenum!(
                                    type_value_to_node(0, &initial_node.0, &initial_node.1),
                                    CommitNode:: Node(n) => n
                                ).unwrap();
                            if *initial_predicate == *predicate.borrow() && node == initial_node && !delete.get() {
                                *draft_entry.0.borrow_mut() = None;
                                break;
                            }
                            let new_draft_rel = DraftRel {
                                delete: delete.get(),
                                predicate: predicate.borrow().clone(),
                                node: node,
                            };
                            *draft_entry.0.borrow_mut() = Some(new_draft_rel);
                        } else {
                            *draft_entry.0.borrow_mut() = Some(DraftRel {
                                delete: delete.get(),
                                predicate: predicate.borrow().clone(),
                                node: node,
                            });
                        }
                    }
                    persist_draft(&save_data);
                }
            ),
        ),
        incoming: incoming,
        initial_predicate: initial_predicate,
        delete: delete,
        delete_all: delete_all,
        predicate: predicate,
        node: node,
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
        let inp_ele = inp_type_el.raw().dyn_into::<HtmlSelectElement>().unwrap();
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
                    match node_type.get_old() {
                        NodeEditType::Str => {
                            return s.clone();
                        },
                        NodeEditType::Num => {
                            return s.clone();
                        },
                        NodeEditType::Bool => {
                            return s.clone();
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
                            return s.clone();
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
                                                match categorize_mime_media(
                                                    meta.mime.as_ref().map(|x| x.as_str()).unwrap_or(""),
                                                ) {
                                                    Some(PlaylistEntryMediaType::Audio) => {
                                                        return Ok(
                                                            vec![
                                                                style_export::leaf_media_audio(
                                                                    style_export::LeafMediaAudioArgs { src: src_url },
                                                                ).root
                                                            ],
                                                        );
                                                    },
                                                    Some(PlaylistEntryMediaType::Video) => {
                                                        return Ok(
                                                            vec![
                                                                style_export::leaf_media_video(
                                                                    style_export::LeafMediaVideoArgs { src: src_url },
                                                                ).root
                                                            ],
                                                        );
                                                    },
                                                    Some(PlaylistEntryMediaType::Image) => {
                                                        return Ok(
                                                            vec![
                                                                style_export::leaf_media_img(
                                                                    style_export::LeafMediaImgArgs { src: src_url },
                                                                ).root
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
                        input_.ref_own(|_| (
                            //. .
                            link!((pc = pc), (input_value = input_value.clone()), (node_value = node_value.clone()), (), {
                                let new_v = NodeEditValue::Upload(NodeEditValueUpload {
                                    old: exenum!(&*node_value.borrow(), NodeEditValue:: Upload(v) => v)
                                        .unwrap()
                                        .old
                                        .clone(),
                                    new: input_value.borrow().clone(),
                                });
                                node_value.set(pc, new_v);
                            }),
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
                } else if {
                    if let Some((initial_type, initial_value)) = &*initial.borrow() {
                        &*input_type.borrow() != initial_type || &*input_value.borrow() != initial_value
                    } else {
                        true
                    }
                } {
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
        let style_res = style_export::leaf_node_edit_toolbar(style_export::LeafNodeEditToolbarArgs { link: None });
        let button_revert = style_res.button_revert;
        button_revert.ref_on("click", {
            let triple = triple.clone();
            let eg = pc.eg();
            move |_| eg.event(|pc| {
                if let Some((initial_predicate, initial_node)) =
                    Option::zip(
                        triple.0.initial_predicate.borrow().as_ref(),
                        triple.0.node.initial.borrow().as_ref(),
                    ) {
                    triple.0.predicate.set(pc, initial_predicate.clone());
                    triple.0.node.type_.set(pc, initial_node.0.clone());
                    triple.0.node.value.set(pc, initial_node.1.clone());
                }
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
                        &[(&style_export::class_state_pressed().value, deleted.get() | deleted_all.get())],
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
                let new_value = ele.text_content().unwrap_or_default();
                input_value.set(pc, new_value);
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
                                if let Some(initial_value) = &*initial_value.borrow() {
                                    *predicate_value.borrow() != *initial_value
                                } else {
                                    true
                                },
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
            children: vec![node_el, predicate_el, buttons_el],
            new: new,
        }).root;
    } else {
        return style_export::cont_node_row_outgoing(style_export::ContNodeRowOutgoingArgs {
            children: vec![predicate_el, node_el, buttons_el],
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
                let draft_data = restore_draft(node.clone());
                let error_slot = style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root;
                let mut out = vec![error_slot.clone()];
                let mut bar_out = vec![];
                let pivot_state = new_pivot_state(pc, &draft_data, &node);
                let triple_states = Rc::new(RefCell::new(vec![] as Vec<TripleState>));

                // Incoming triples
                {
                    let triples_box =
                        style_export::cont_page_node_section_rel(
                            style_export::ContPageNodeSectionRelArgs { children: vec![] },
                        ).root;
                    for t in triples.incoming {
                        let triple =
                            new_triple_state(
                                pc,
                                &draft_data,
                                Some(&t),
                                true,
                                draft_data
                                    .0
                                    .incoming
                                    .borrow_mut()
                                    .entry((t.subject.clone(), t.predicate.clone()))
                                    .or_insert(Default::default())
                                    .clone(),
                                pivot_state.0.delete_all.clone(),
                            );
                        triples_box.ref_push(build_edit_triple(pc, &triple, false));
                        triple_states.borrow_mut().push(triple);
                    }
                    {
                        let draft_new = draft_data.0.new_incoming.borrow().clone();
                        for draft_value in draft_new {
                            let triple_state =
                                new_triple_state(
                                    pc,
                                    &draft_data,
                                    None,
                                    true,
                                    draft_value,
                                    pivot_state.0.delete_all.clone(),
                                );
                            triples_box.ref_push(build_edit_triple(pc, &triple_state, true));
                            triple_states.borrow_mut().push(triple_state);
                        }
                    }
                    let add_row_res =
                        style_export::cont_node_row_incoming_add(
                            style_export::ContNodeRowIncomingAddArgs { hint: format!("Add incoming") },
                        );
                    add_row_res.button.ref_on("click", {
                        let eg = pc.eg();
                        let pivot_state = pivot_state.clone();
                        let triple_states = triple_states.clone();
                        let incoming_triples_box = triples_box.clone();
                        let save_data = draft_data.clone();
                        move |_| eg.event(|pc| {
                            let draft_entry = MutDraftRel::default();
                            save_data.0.new_incoming.borrow_mut().insert(draft_entry.clone());
                            let triple =
                                new_triple_state(
                                    pc,
                                    &save_data,
                                    None,
                                    true,
                                    draft_entry,
                                    pivot_state.0.delete_all.clone(),
                                );
                            incoming_triples_box.ref_splice(0, 0, vec![build_edit_triple(pc, &triple, true)]);
                            triple_states.borrow_mut().push(triple);
                        }).unwrap()
                    });
                    out.push(add_row_res.root);
                    out.push(triples_box);
                }

                // Pivot
                {
                    let buttons_el = {
                        let style_res =
                            style_export::leaf_node_edit_buttons(
                                style_export::LeafNodeEditButtonsArgs {
                                    link: Some(
                                        ministate_octothorpe(&super::ministate::Ministate::NodeView(MinistateNodeView {
                                            title: shed!{
                                                'titled _;
                                                for t in &triples.outgoing {
                                                    if t.predicate == shared::interface::ont::PREDICATE_NAME {
                                                        break 'titled node_to_text(&t.object);
                                                    }
                                                }
                                                break 'titled node_to_text(&node);
                                            },
                                            node: node.clone(),
                                        })),
                                    ),
                                },
                            );
                        let button_revert = style_res.button_revert;
                        button_revert.ref_on("click", {
                            let pivot_original = node.clone();
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
                                pivot_state.0.delete_all.set(pc, !pivot_state.0.delete_all.get());
                            }).unwrap()
                        });
                        button_delete.ref_own(
                            |ele| (
                                link!(
                                    (_pc = pc),
                                    (deleted = pivot_state.0.delete_all.clone()),
                                    (),
                                    (ele = ele.weak()),
                                    {
                                        let pivot_root = ele.upgrade()?;
                                        pivot_root.ref_modify_classes(
                                            &[(&style_export::class_state_pressed().value, deleted.get())],
                                        );
                                    }
                                ),
                            ),
                        );
                        style_res.root
                    };
                    let style_res = build_edit_node(pc, &pivot_state.0.node);
                    let children = vec![buttons_el, style_res];
                    out.push(
                        style_export::cont_node_section_center(
                            style_export::ContNodeSectionCenterArgs { children: children },
                        ).root,
                    );
                }

                // Outgoing triples
                {
                    let triples_box =
                        style_export::cont_page_node_section_rel(
                            style_export::ContPageNodeSectionRelArgs { children: vec![] },
                        ).root;
                    for t in triples.outgoing {
                        let triple_state =
                            new_triple_state(
                                pc,
                                &draft_data,
                                Some(&t),
                                false,
                                draft_data
                                    .0
                                    .outgoing
                                    .borrow_mut()
                                    .entry((t.predicate.clone(), t.object.clone()))
                                    .or_insert(Default::default())
                                    .clone(),
                                pivot_state.0.delete_all.clone(),
                            );
                        triples_box.ref_push(build_edit_triple(pc, &triple_state, false));
                        triple_states.borrow_mut().push(triple_state);
                    }
                    {
                        let draft_new = draft_data.0.new_outgoing.borrow().clone();
                        for draft_value in draft_new {
                            let triple_state =
                                new_triple_state(
                                    pc,
                                    &draft_data,
                                    None,
                                    false,
                                    draft_value,
                                    pivot_state.0.delete_all.clone(),
                                );
                            triples_box.ref_push(build_edit_triple(pc, &triple_state, true));
                            triple_states.borrow_mut().push(triple_state);
                        }
                    }
                    out.push(triples_box.clone());
                    let add_row_res =
                        style_export::cont_node_row_outgoing_add(
                            style_export::ContNodeRowOutgoingAddArgs { hint: "Add outgoing".to_string() },
                        );
                    add_row_res.button.ref_on("click", {
                        let eg = pc.eg();
                        let triple_states = triple_states.clone();
                        let triples_box = triples_box;
                        let save_data = draft_data.clone();
                        let pivot_state = pivot_state.clone();
                        move |_| eg.event(|pc| {
                            let draft_entry = MutDraftRel::default();
                            save_data.0.new_outgoing.borrow_mut().insert(draft_entry.clone());
                            let triple =
                                new_triple_state(
                                    pc,
                                    &save_data,
                                    None,
                                    false,
                                    draft_entry,
                                    pivot_state.0.delete_all.clone(),
                                );
                            triples_box.ref_push(build_edit_triple(pc, &triple, true));
                            triple_states.borrow_mut().push(triple);
                        }).unwrap()
                    });
                    out.push(add_row_res.root);
                }

                // Edit form controls
                let button_view = style_export::leaf_button_big_view().root;
                button_view.ref_on("click", {
                    let eg = pc.eg();
                    let title = title.clone();
                    let node = node.clone();
                    move |_| eg.event(|pc| {
                        change_ministate(pc, &Ministate::NodeView(MinistateNodeView {
                            title: title.clone(),
                            node: node.clone(),
                        }));
                    }).unwrap()
                });
                let button_commit = style_export::leaf_button_big_commit().root;
                button_commit.ref_on("click", {
                    let triple_states = triple_states.clone();
                    let pivot_state = pivot_state.clone();
                    let error_slot = error_slot.weak();
                    let save_thinking = Rc::new(RefCell::new(None));
                    let draft_data = draft_data.clone();
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
                            let draft_data = draft_data.clone();
                            let title = title.clone();
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
                                    let delete_all = *pivot_state.0.delete_all.borrow();
                                    let pivot_node_initial = {
                                        let pivot_node_initial = pivot_state.0.node.initial.borrow();
                                        let pivot_node_initial = pivot_node_initial.as_ref().unwrap();
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
                                        let triple_node_initial = triple.0.node.initial.borrow();
                                        let triple_initial =
                                            match Option::zip(
                                                triple_predicate_initial.as_ref(),
                                                triple_node_initial.as_ref(),
                                            ) {
                                                Some((triple_predicate_initial, triple_node_initial)) => Some(
                                                    (triple_predicate_initial, type_value_to_node({
                                                        file_unique += 1;
                                                        file_unique
                                                    }, &triple_node_initial.0, &triple_node_initial.1)),
                                                ),
                                                None => None,
                                            };

                                        // Classify if changed/deleted
                                        let changed;
                                        let new;
                                        if let Some((triple_predicate_initial, triple_node_initial)) =
                                            &triple_initial {
                                            new = false;
                                            changed =
                                                pivot_changed || triple_node != *triple_node_initial ||
                                                    triple.0.predicate.borrow().as_str() !=
                                                        triple_predicate_initial.as_str();
                                        } else {
                                            new = true;
                                            changed = true;
                                        }

                                        // If not new but deleted or changed, delete first
                                        if let Some((triple_predicate_initial, triple_node_initial)) = triple_initial {
                                            if (delete_all || triple.0.delete.get()) || changed {
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
                                        }

                                        // If new or changed, write the new triple
                                        if new || changed {
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

                                    // Send compiled changes Preprocess
                                    let mut add1 = vec![];
                                    let mut files_to_return = HashMap::new();
                                    let mut files_to_commit = vec![];
                                    let mut files_to_upload = vec![];
                                    for triple in add {
                                        let Some(subject) =
                                            commit::prep_node(
                                                &mut files_to_return,
                                                &mut files_to_commit,
                                                &mut files_to_upload,
                                                triple.subject,
                                            ).await else {
                                                continue;
                                            };
                                        let Some(object) =
                                            commit::prep_node(
                                                &mut files_to_return,
                                                &mut files_to_commit,
                                                &mut files_to_upload,
                                                triple.object,
                                            ).await else {
                                                continue;
                                            };
                                        add1.push(Triple {
                                            subject: subject,
                                            predicate: triple.predicate,
                                            object: object,
                                        });
                                    }

                                    // Write commit
                                    req_post_json(&state().env.base_url, ReqCommit {
                                        comment: format!("Edit node [{}]", title),
                                        add: add1,
                                        remove: remove,
                                        files: files_to_commit,
                                    }).await?;

                                    // Upload files
                                    commit::upload_files(files_to_upload).await?;
                                    return Ok(files_to_return);
                                }.await;
                                button.class_list().remove_1(&style_export::class_state_thinking().value).unwrap();
                                match res {
                                    Ok(mut file_lookup) => {
                                        eg.event(|pc| {
                                            // Get committed nodes (files replaced with hashes) and update initial values,
                                            // clear draft
                                            let pivot_node = match pivot_node {
                                                CommitNode::Node(n) => n,
                                                CommitNode::File(unique, _) => Node::File(
                                                    file_lookup.remove(&unique).unwrap(),
                                                ),
                                            };
                                            clear_draft(&draft_data, &pivot_node);
                                            pivot_state
                                                .0
                                                .node
                                                .initial
                                                .set(pc, Some(node_to_type_value(&pivot_node)));
                                            for (
                                                triple,
                                                (sent_pred, sent_node),
                                            ) in Iterator::zip(
                                                RefCell::borrow(&triple_states).iter(),
                                                triple_nodes_predicates.into_iter(),
                                            ) {
                                                triple.0.initial_predicate.set(pc, Some(sent_pred));
                                                triple
                                                    .0
                                                    .node
                                                    .initial
                                                    .set(pc, Some(node_to_type_value(&match sent_node {
                                                        CommitNode::Node(n) => n,
                                                        CommitNode::File(unique, _) => Node::File(
                                                            file_lookup.remove(&unique).unwrap(),
                                                        ),
                                                    })));
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
                bar_out.push(button_commit);
                return Ok(vec![style_export::cont_page_node_edit(style_export::ContPageNodeEditArgs {
                    children: out,
                    bar_children: bar_out,
                }).root]);
            }).unwrap();
        }
    }));
}
