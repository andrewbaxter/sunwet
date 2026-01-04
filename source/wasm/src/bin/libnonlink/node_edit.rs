use {
    super::api::req_post_json,
    crate::libnonlink::{
        commit::{
            self,
            CommitNode,
            CommitTriple,
        },
        ministate::{
            MinistateNodeView,
            ministate_octothorpe,
        },
        playlist::{
            PlaylistEntryMediaType,
            categorize_mime_media,
        },
        state::state,
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
        EventGraph,
        HistPrim,
        Prim,
        ProcessingContext,
        link,
    },
    rooting::{
        El,
        ScopeValue,
        WeakEl,
        scope_any,
        spawn_rooted,
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
        cell::{
            Cell,
            RefCell,
        },
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
            env_preferred_audio_url,
            env_preferred_video_url,
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

fn initial_type_value_to_node(type_: &NodeEditType, value: &NodeEditValue) -> Node {
    let CommitNode::Node(n) = type_value_to_node(0, type_, value) else {
        unreachable!();
    };
    return n;
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
    fill: bool,
    predicate: String,
    node: Node,
}

#[derive(Serialize, Deserialize, Default)]
struct DraftBody {
    // All: None if no modifications
    delete_all: bool,
    pivot: Option<Node>,
    incoming: Vec<((String, Node), DraftRel)>,
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
    delete_all: Cell<bool>,
    pivot: RefCell<Option<Node>>,
    incoming: RefCell<HashMap<(String, Node), MutDraftRel>>,
    outgoing: RefCell<HashMap<(String, Node), MutDraftRel>>,
    // Non-none immediately after deserializing; may become none after first commit
    new_incoming: RefCell<HashSet<MutDraftRel>>,
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
        delete_all: Cell::new(data.delete_all),
        pivot: RefCell::new(data.pivot),
        incoming: RefCell::new(data.incoming.into_iter().map(|(k, v)| (k, MutDraftRel::new(v))).collect()),
        new_incoming: RefCell::new(data.new_incoming.into_iter().map(|v| MutDraftRel::new(v)).collect()),
        outgoing: RefCell::new(data.outgoing.into_iter().map(|(k, v)| (k, MutDraftRel::new(v))).collect()),
        new_outgoing: RefCell::new(data.new_outgoing.into_iter().map(|v| MutDraftRel::new(v)).collect()),
    }));
}

fn persist_draft(s: &DraftData) {
    if let Err(e) = LocalStorage::set(format_localstorage_draft_key(&*s.0.key.borrow()), &DraftBody {
        delete_all: s.0.delete_all.get(),
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

struct PivotStateSingle {
    initial: Prim<InitialNode>,
    type_: HistPrim<NodeEditType>,
    value: HistPrim<NodeEditValue>,
    _own: ScopeValue,
}

struct PivotStateMulti {
    initial: Prim<Vec<Node>>,
}

enum PivotState_ {
    Single(PivotStateSingle),
    Multi(PivotStateMulti),
}

#[derive(Clone)]
struct PivotState(Rc<PivotState_>);

struct PivotStateArgsSingle<'a> {
    save_data: &'a DraftData,
    source_node: Node,
}

struct PivotStateArgsMulti {
    source_nodes: Vec<Node>,
}

enum PivotStateArgs<'a> {
    Single(PivotStateArgsSingle<'a>),
    Multi(PivotStateArgsMulti),
}

fn new_pivot_state(pc: &mut ProcessingContext, args: PivotStateArgs) -> PivotState {
    match args {
        PivotStateArgs::Single(args) => {
            let (source_node_type_, source_node_value) = node_to_type_value(&args.source_node);
            let node_type_;
            let node_value;
            if let Some(restore_node) = &*args.save_data.0.pivot.borrow() {
                let (restore_node_type, restore_node_value) = node_to_type_value(restore_node);
                node_type_ = HistPrim::new(pc, restore_node_type);
                node_value = HistPrim::new(pc, restore_node_value);
            } else {
                node_type_ = HistPrim::new(pc, source_node_type_.clone());
                node_value = HistPrim::new(pc, source_node_value.clone());
            }
            let initial = Prim::new(InitialNode {
                node_type_: source_node_type_,
                node_value: source_node_value,
            });
            return PivotState(Rc::new(PivotState_::Single(PivotStateSingle {
                _own: scope_any(
                    // Persist data to draft after changes (remove if not changed or unstorable,
                    // otherwise Some)
                    link!(
                        (_pc = pc),
                        (node_type = node_type_.clone(), node_value = node_value.clone()),
                        (),
                        (save_data = args.save_data.clone(), initial_node = initial.clone()) {
                            *save_data.0.pivot.borrow_mut() = shed!{
                                let node_type = node_type.borrow();
                                let node_value = node_value.borrow();
                                let CommitNode::Node(node) = type_value_to_node(0, &*node_type, &*node_value) else {
                                    break None;
                                };
                                let initial_node = initial_node.borrow();
                                if *node_type == initial_node.node_type_ && *node_value == initial_node.node_value {
                                    break None;
                                }
                                Some(node)
                            };
                            persist_draft(&save_data);
                        }
                    ),
                ),
                initial: initial,
                type_: node_type_,
                value: node_value,
            })));
        },
        PivotStateArgs::Multi(args) => {
            return PivotState(
                Rc::new(PivotState_::Multi(PivotStateMulti { initial: Prim::new(args.source_nodes) })),
            );
        },
    }
}

#[derive(Clone)]
struct InitialNode {
    node_type_: NodeEditType,
    node_value: NodeEditValue,
}

#[derive(Clone)]
struct RelInitialPredNode {
    predicate: String,
    node_type_: NodeEditType,
    node_value: NodeEditValue,
}

enum RelInitialPivot {
    Single,
    Multi(Prim<Vec<Node>>),
}

struct RelState_ {
    incoming: bool,
    delete: HistPrim<bool>,
    predicate: Prim<String>,
    node_type: HistPrim<NodeEditType>,
    node_value: HistPrim<NodeEditValue>,
    fill: HistPrim<bool>,
    initial_pivot: RelInitialPivot,
    initial_pred_node: Prim<Option<RelInitialPredNode>>,
    initial_fill: Prim<bool>,
    _own: ScopeValue,
}

#[derive(Clone)]
struct RelState(Rc<RelState_>);

struct RelStateArgs<'a> {
    draft_data: Option<&'a DraftData>,
    draft_entry: Option<MutDraftRel>,
    initial_fill: bool,
    initial_pivot: RelInitialPivot,
    initial_pred_node: Option<(String, Node)>,
    new: bool,
    incoming: bool,
}

fn new_rel_state(pc: &mut ProcessingContext, args: RelStateArgs) -> RelState {
    let initial_pred_node = Prim::new(args.initial_pred_node.as_ref().map(|x| {
        let (type_, value) = node_to_type_value(&x.1);
        RelInitialPredNode {
            predicate: x.0.clone(),
            node_type_: type_,
            node_value: value,
        }
    }));
    let predicate;
    let delete;
    let fill;
    let node_type;
    let node_value;
    shed!{
        'done _;
        // From draft
        shed!{
            let Some(d) = args.draft_entry.as_ref() else {
                break;
            };
            let Some(d) = &*d.0.0.borrow() else {
                break;
            };
            predicate = Prim::new(d.predicate.clone());
            delete = HistPrim::new(pc, d.delete);
            fill = HistPrim::new(pc, d.fill);
            let (restore_type, restore_value) = node_to_type_value(&d.node);
            node_type = HistPrim::new(pc, restore_type);
            node_value = HistPrim::new(pc, restore_value);
            break 'done;
        }
        // From existing rel
        shed!{
            let Some((initial_pred, initial_node)) = args.initial_pred_node else {
                break;
            };
            predicate = Prim::new(initial_pred);
            delete = HistPrim::new(pc, false);
            fill = HistPrim::new(pc, args.initial_fill);
            let (type_, value) = node_to_type_value(&initial_node);
            node_type = HistPrim::new(pc, type_.clone());
            node_value = HistPrim::new(pc, value.clone());
            break 'done;
        }
        // Defaults, new rel
        predicate = Prim::new(format!(""));
        delete = HistPrim::new(pc, false);
        fill = HistPrim::new(pc, true);
        let (type_, value) = node_to_type_value(&Node::Value(serde_json::Value::String(format!(""))));
        node_type = HistPrim::new(pc, type_.clone());
        node_value = HistPrim::new(pc, value.clone());
        break 'done;
    };
    return RelState(Rc::new(RelState_ {
        _own: scope_any(
            // Persist data to draft after changes (remove if not changed or unstorable,
            // otherwise Some)
            link!(
                (_pc = pc),
                (
                    delete = delete.clone(),
                    fill = fill.clone(),
                    predicate = predicate.clone(),
                    node_type = node_type.clone(),
                    node_value = node_value.clone(),
                ),
                (),
                (
                    save_data = args.draft_data.map(|x| x.clone()),
                    initial_pred_node = initial_pred_node.clone(),
                    draft_entry = args.draft_entry
                ) {
                    shed!{
                        let CommitNode::Node(node) =
                            type_value_to_node(0, &*node_type.borrow(), &*node_value.borrow()) else {
                                if let Some(d) = draft_entry.as_ref() {
                                    *d.0.borrow_mut() = None;
                                }
                                break;
                            };
                        if let Some(initial_pred_node) = &*initial_pred_node.borrow() {
                            let initial_node =
                                initial_type_value_to_node(
                                    &initial_pred_node.node_type_,
                                    &initial_pred_node.node_value,
                                );
                            if *initial_pred_node.predicate == *predicate.borrow() && node == initial_node &&
                                !delete.get() {
                                if let Some(d) = draft_entry.as_ref() {
                                    *d.0.borrow_mut() = None;
                                }
                                break;
                            }
                            let new_draft_rel = DraftRel {
                                delete: delete.get(),
                                fill: fill.get(),
                                predicate: predicate.borrow().clone(),
                                node: node,
                            };
                            if let Some(d) = draft_entry.as_ref() {
                                *d.0.borrow_mut() = Some(new_draft_rel);
                            }
                        }
                    }
                    if let Some(save_data) = save_data {
                        persist_draft(save_data);
                    }
                }
            ),
        ),
        fill: fill,
        incoming: args.incoming,
        delete: delete,
        predicate: predicate,
        node_type: node_type,
        node_value: node_value,
        initial_pivot: args.initial_pivot,
        initial_pred_node: initial_pred_node,
        initial_fill: Prim::new(args.initial_fill),
    }));
}

enum BuildEditInitialNode {
    Pivot(Prim<InitialNode>),
    Rel(Prim<Option<RelInitialPredNode>>),
}

fn build_edit_node(
    pc: &mut ProcessingContext,
    node_type: &HistPrim<NodeEditType>,
    node_value: &HistPrim<NodeEditValue>,
    initial_node: BuildEditInitialNode,
) -> El {
    let options =
        [
            (NodeEditType::Str, "Text"),
            (NodeEditType::Num, "Number"),
            (NodeEditType::Bool, "Bool"),
            (NodeEditType::Json, "JSON"),
            (NodeEditType::File, "File"),
            (NodeEditType::FileUpload, "File, upload new"),
        ]
            .into_iter()
            .map(|(k, v)| (serde_json::to_string(&k).unwrap(), v.to_string()))
            .collect::<HashMap<_, _>>();
    let inp_type_el = style_export::leaf_input_enum(style_export::LeafInputEnumArgs {
        id: None,
        title: "Node type".to_string(),
        options: options,
        value: serde_json::to_string(&node_type.get()).unwrap(),
    }).root;
    let inp_value_group_el = style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root;
    inp_type_el.ref_on("input", {
        let node_type = node_type.clone();
        let eg = pc.eg();
        let inp_ele = inp_type_el.raw().dyn_into::<HtmlSelectElement>().unwrap();
        move |_| eg.event(|pc| {
            node_type.set(pc, serde_json::from_str::<NodeEditType>(&inp_ele.value()).unwrap());
        }).unwrap()
    });

    // When changing element type, munge the value to fit the new type and replace the
    // input element
    inp_type_el.ref_own(
        |_| link!(
            (pc = pc),
            (node_type = node_type.clone()),
            (node_value = node_value.clone()),
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
                            let Ok(serde_json::Value::String(v)) = serde_json::from_str::<serde_json::Value>(&s) else {
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
                                    media.ref_push(el_async(async move {
                                        ta_return!(Vec < El >, String);
                                        let meta =
                                            req_post_json(ReqGetNodeMeta { node: Node::File(h.clone()) },).await?;
                                        match meta {
                                            Some(meta) => {
                                                match categorize_mime_media(
                                                    meta.mime.as_ref().map(|x| x.as_str()).unwrap_or(""),
                                                ) {
                                                    Some(PlaylistEntryMediaType::Audio) => {
                                                        return Ok(
                                                            vec![
                                                                style_export::leaf_media_audio(
                                                                    style_export::LeafMediaAudioArgs {
                                                                        src: env_preferred_audio_url(&state().env, &h)
                                                                    },
                                                                ).root
                                                            ],
                                                        );
                                                    },
                                                    Some(PlaylistEntryMediaType::Video) => {
                                                        return Ok(
                                                            vec![
                                                                style_export::leaf_media_video(
                                                                    style_export::LeafMediaVideoArgs {
                                                                        src: env_preferred_video_url(&state().env, &h)
                                                                    },
                                                                ).root
                                                            ],
                                                        );
                                                    },
                                                    Some(PlaylistEntryMediaType::Image) => {
                                                        return Ok(
                                                            vec![
                                                                style_export::leaf_media_img(
                                                                    style_export::LeafMediaImgArgs {
                                                                        src: file_url(&state().env, &h)
                                                                    },
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
    out.ref_own(|out| ({
        fn do_update_modified_invalid(
            out: &WeakEl,
            input_type: &HistPrim<NodeEditType>,
            input_value: &HistPrim<NodeEditValue>,
            initial_node_type: Option<&NodeEditType>,
            initial_node_value: Option<&NodeEditValue>,
        ) {
            let Some(input_el) = out.upgrade() else {
                return;
            };
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
            } else if Some(&*input_type.borrow()) != initial_node_type ||
                Some(&*input_value.borrow()) != initial_node_value {
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

        match initial_node {
            BuildEditInitialNode::Pivot(prim) => link!(
                (_pc = pc),
                (input_type = node_type.clone(), input_value = node_value.clone(), initial_node = prim),
                (),
                (out = out.weak()),
                {
                    let initial_node = initial_node.borrow();
                    do_update_modified_invalid(
                        out,
                        input_type,
                        input_value,
                        Some(&initial_node.node_type_),
                        Some(&initial_node.node_value),
                    );
                }
            ),
            BuildEditInitialNode::Rel(prim) => link!(
                (_pc = pc),
                (input_type = node_type.clone(), input_value = node_value.clone(), initial_node = prim),
                (),
                (out = out.weak()),
                {
                    let initial_node = initial_node.borrow();
                    match &*initial_node {
                        Some(initial_node) => do_update_modified_invalid(
                            out,
                            input_type,
                            input_value,
                            Some(&initial_node.node_type_),
                            Some(&initial_node.node_value),
                        ),
                        None => do_update_modified_invalid(out, input_type, input_value, None, None),
                    }
                }
            ),
        }
        // Update modified/invalid flags
    },));
    return out;
}

fn build_edit_rel(pc: &mut ProcessingContext, total_pivot_nodes: usize, rel: &RelState, new: bool) -> El {
    let buttons_el = {
        let mut left = vec![];
        let mut right = vec![];
        match &rel.0.initial_pivot {
            RelInitialPivot::Single => { },
            RelInitialPivot::Multi(nodes) => {
                let count_text = style_export::leaf_node_edit_toolbar_count_text().root;
                count_text.ref_own(
                    |this| link!(
                        (_pc = pc),
                        (fill = rel.0.fill.clone(), nodes = nodes.clone()),
                        (),
                        (count_text = this.weak(), total_pivot_nodes = total_pivot_nodes) {
                            let count_el = count_text.upgrade()?;
                            count_el.ref_text(&format!("{} / {} nodes", if fill.get() {
                                *total_pivot_nodes
                            } else {
                                nodes.borrow().len()
                            }, total_pivot_nodes));
                            count_el.ref_modify_classes(
                                &[
                                    (&style_export::class_state_modified().value, fill.get()),
                                    (
                                        &style_export::class_state_hide().value,
                                        nodes.borrow().len() == *total_pivot_nodes,
                                    ),
                                ],
                            );
                        }
                    ),
                );
                left.push(count_text);
                let button_fill = style_export::leaf_node_edit_toolbar_fill_toggle().root;
                button_fill.ref_on("click", {
                    let rel = rel.clone();
                    let eg = pc.eg();
                    move |_| eg.event(|pc| {
                        rel.0.fill.set(pc, !rel.0.fill.get());
                    }).unwrap()
                });
                button_fill.ref_own(
                    |out| link!(
                        (_pc = pc),
                        (fill = rel.0.fill.clone(), nodes = nodes.clone()),
                        (),
                        (out = out.weak(), total_pivot_nodes = total_pivot_nodes),
                        {
                            let button_fill = out.upgrade()?;
                            button_fill.ref_modify_classes(
                                &[
                                    (&style_export::class_state_pressed().value, fill.get()),
                                    (
                                        &style_export::class_state_hide().value,
                                        nodes.borrow().len() == *total_pivot_nodes,
                                    ),
                                ],
                            );
                        }
                    ),
                );
                right.push(button_fill);
            },
        }
        let button_revert = style_export::leaf_node_edit_toolbar_revert_button().root;
        button_revert.ref_on("click", {
            let rel = rel.clone();
            let eg = pc.eg();
            move |_| eg.event(|pc| {
                let initial = rel.0.initial_pred_node.borrow();
                if let Some(initial) = &*initial {
                    rel.0.predicate.set(pc, initial.predicate.clone());
                    rel.0.node_type.set(pc, initial.node_type_.clone());
                    rel.0.node_value.set(pc, initial.node_value.clone());
                }
            }).unwrap()
        });
        right.push(button_revert);
        let button_delete = style_export::leaf_node_edit_toolbar_delete_toggle().root;
        button_delete.ref_on("click", {
            let rel = rel.clone();
            let eg = pc.eg();
            move |_| eg.event(|pc| {
                rel.0.delete.set(pc, !rel.0.delete.get());
            }).unwrap()
        });
        button_delete.ref_own(|out| link!((_pc = pc), (deleted = rel.0.delete.clone()), (), (out = out.weak()), {
            let out = out.upgrade()?;
            out.ref_modify_classes(&[(&style_export::class_state_pressed().value, deleted.get())]);
        }));
        right.push(button_delete);
        style_export::cont_node_toolbar(style_export::ContNodeToolbarArgs {
            left: left,
            right: right,
        }).root
    };
    let node_el =
        build_edit_node(
            pc,
            &rel.0.node_type,
            &rel.0.node_value,
            BuildEditInitialNode::Rel(rel.0.initial_pred_node.clone()),
        );
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
            link!((pc = pc), (predicate_value = rel.0.predicate.clone()), (input_value = input_value.clone()), (out = out.weak()), {
                let input_el = out.upgrade()?;
                input_value.set(pc, predicate_value.borrow().clone());
                input_el.ref_text(predicate_value.borrow().as_str());
            }),
            link!((pc = pc), (input_value = input_value.clone()), (predicate_value = rel.0.predicate.clone()), (), {
                predicate_value.set(pc, input_value.borrow().clone());
            }),
            link!(
                (_pc = pc),
                (pred = rel.0.predicate.clone(), initial_pred_node = rel.0.initial_pred_node.clone()),
                (),
                (out = out.weak()) {
                    let out = out.upgrade()?;
                    out.ref_modify_classes(
                        &[
                            (
                                &style_export::class_state_modified().value,
                                if let Some(initial_pred_node) = &*initial_pred_node.borrow() {
                                    *pred.borrow() != initial_pred_node.predicate
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
    if rel.0.incoming {
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

pub struct BuildNodeEditContentsRes {
    pub children: Vec<El>,
    pub bar_children: Vec<El>,
}

pub async fn build_node_edit_contents(
    eg: EventGraph,
    title: String,
    nodes: Vec<Node>,
) -> Result<BuildNodeEditContentsRes, String> {
    let mut rels = req_post_json(ReqGetTriplesAround { nodes: nodes.clone() }).await?;
    return eg.event(|pc| {
        let pivot_state;
        let draft_data;
        let delete_all;
        if nodes.len() == 1 {
            let draft_data1 = restore_draft(nodes[0].clone());
            pivot_state = new_pivot_state(pc, PivotStateArgs::Single(PivotStateArgsSingle {
                save_data: &draft_data1,
                source_node: nodes.iter().next().unwrap().clone(),
            }));
            delete_all = HistPrim::new(pc, draft_data1.0.delete_all.get());
            draft_data = Some(draft_data1);
        } else {
            draft_data = None;
            delete_all = HistPrim::new(pc, false);
            pivot_state =
                new_pivot_state(pc, PivotStateArgs::Multi(PivotStateArgsMulti { source_nodes: nodes.clone() }));
        }
        let error_slot = style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root;
        let mut out = vec![error_slot.clone()];
        let mut bar_out = vec![];
        let pivot_lookup = nodes.iter().collect::<HashSet<_>>();
        let rel_states = Rc::new(RefCell::new(vec![] as Vec<RelState>));

        // Incoming relations
        {
            let rels_box =
                style_export::cont_page_node_section_rel(
                    style_export::ContPageNodeSectionRelArgs { children: vec![] },
                ).root;
            let mut incoming_rels = HashMap::<(String, Node), Vec<Node>>::new();
            for t in rels.extract_if(.., |x| pivot_lookup.contains(&x.object)) {
                incoming_rels.entry((t.predicate.clone(), t.subject.clone())).or_default().push(t.object.clone());
            }
            let mut incoming_rels = incoming_rels.into_iter().collect::<Vec<_>>();
            incoming_rels.sort_by_cached_key(|r| (r.0.0.clone(), r.0.1.clone()));
            for ((pred, subj), objs) in incoming_rels {
                let full_pivot = objs.len() == pivot_lookup.len();
                let rel = new_rel_state(pc, RelStateArgs {
                    draft_data: draft_data.as_ref(),
                    draft_entry: if let Some(draft_data) = &draft_data {
                        Some(
                            draft_data
                                .0
                                .incoming
                                .borrow_mut()
                                .entry((pred.clone(), subj.clone()))
                                .or_insert(Default::default())
                                .clone(),
                        )
                    } else {
                        None
                    },
                    initial_fill: full_pivot,
                    initial_pivot: if nodes.len() == 1 {
                        RelInitialPivot::Single
                    } else {
                        RelInitialPivot::Multi(Prim::new(objs))
                    },
                    initial_pred_node: Some((pred.clone(), subj)),
                    new: false,
                    incoming: true,
                });
                rels_box.ref_push(build_edit_rel(pc, nodes.len(), &rel, false));
                rel_states.borrow_mut().push(rel);
            }
            if let Some(draft_data) = draft_data.as_ref() {
                let draft_new = draft_data.0.new_incoming.borrow().clone();
                for draft_value in draft_new {
                    let rel_state = new_rel_state(pc, RelStateArgs {
                        draft_data: Some(draft_data),
                        draft_entry: Some(draft_value),
                        initial_fill: true,
                        initial_pivot: RelInitialPivot::Single,
                        initial_pred_node: None,
                        new: true,
                        incoming: true,
                    });
                    rels_box.ref_push(build_edit_rel(pc, nodes.len(), &rel_state, true));
                    rel_states.borrow_mut().push(rel_state);
                }
            }
            let add_row_res =
                style_export::cont_node_row_incoming_add(
                    style_export::ContNodeRowIncomingAddArgs { hint: format!("Add incoming") },
                );
            add_row_res.button.ref_on("click", {
                let eg = pc.eg();
                let rel_states = rel_states.clone();
                let incoming_rels_box = rels_box.clone();
                let save_data = draft_data.clone();
                let nodes = nodes.clone();
                move |_| eg.event(|pc| {
                    let rel = new_rel_state(pc, RelStateArgs {
                        draft_data: save_data.as_ref(),
                        draft_entry: if let Some(draft_data) = &save_data {
                            let draft_entry = MutDraftRel::default();
                            draft_data.0.new_incoming.borrow_mut().insert(draft_entry.clone());
                            Some(draft_entry)
                        } else {
                            None
                        },
                        initial_fill: true,
                        initial_pivot: if nodes.len() == 1 {
                            RelInitialPivot::Single
                        } else {
                            RelInitialPivot::Multi(Prim::new(nodes.clone()))
                        },
                        initial_pred_node: None,
                        new: true,
                        incoming: true,
                    });
                    incoming_rels_box.ref_splice(0, 0, vec![build_edit_rel(pc, nodes.len(), &rel, true)]);
                    rel_states.borrow_mut().push(rel);
                }).unwrap()
            });
            out.push(add_row_res.root);
            out.push(rels_box);
        }

        // Pivot
        {
            let buttons_el = {
                let mut right = vec![];
                if let PivotState_::Single(pivot) = pivot_state.0.as_ref() {
                    let button_revert = style_export::leaf_node_edit_toolbar_revert_button().root;
                    button_revert.ref_on("click", {
                        let pivot_original = pivot.initial.clone();
                        let pivot_type = pivot.type_.clone();
                        let pivot_node = pivot.value.clone();
                        let eg = pc.eg();
                        move |_| eg.event(|pc| {
                            let pivot_original = pivot_original.borrow();
                            pivot_type.set(pc, pivot_original.node_type_.clone());
                            pivot_node.set(pc, pivot_original.node_value.clone());
                        }).unwrap()
                    });
                    right.push(button_revert);
                    let link =
                        style_export::leaf_node_edit_toolbar_view_link_button(
                            style_export::LeafNodeEditToolbarViewLinkButtonArgs {
                                link: ministate_octothorpe(&super::ministate::Ministate::NodeView(MinistateNodeView {
                                    title: format!("View node"),
                                    node: {
                                        let pivot_initial = pivot.initial.borrow();
                                        initial_type_value_to_node(
                                            &pivot_initial.node_type_,
                                            &pivot_initial.node_value,
                                        )
                                    },
                                })),
                            },
                        ).root;
                    right.push(link);
                }
                style_export::cont_node_toolbar(style_export::ContNodeToolbarArgs {
                    left: vec![],
                    right: right,
                }).root
            };
            let mut vert_children = vec![buttons_el];
            vert_children.push(match pivot_state.0.as_ref() {
                PivotState_::Single(pivot) => {
                    build_edit_node(
                        pc,
                        &pivot.type_,
                        &pivot.value,
                        BuildEditInitialNode::Pivot(pivot.initial.clone()),
                    )
                },
                PivotState_::Multi(_) => {
                    style_export::leaf_node_edit_number_text_center(
                        style_export::LeafNodeEditNumberTextCenterArgs { total: nodes.len() },
                    ).root
                },
            });
            out.push(
                style_export::cont_node_section_center(
                    style_export::ContNodeSectionCenterArgs { children: vert_children },
                ).root,
            );
        }

        // Outgoing relations
        {
            let rels_box =
                style_export::cont_page_node_section_rel(
                    style_export::ContPageNodeSectionRelArgs { children: vec![] },
                ).root;
            let mut outgoing_rels = HashMap::<(String, Node), Vec<Node>>::new();
            for t in rels.extract_if(.., |x| pivot_lookup.contains(&x.subject)) {
                outgoing_rels.entry((t.predicate.clone(), t.object.clone())).or_default().push(t.subject.clone());
            }
            let mut outgoing_rels = outgoing_rels.into_iter().collect::<Vec<_>>();
            outgoing_rels.sort_by_cached_key(|r| (r.0.0.clone(), r.0.1.clone()));
            for ((pred, obj), subjs) in outgoing_rels {
                let full_pivot = subjs.len() == pivot_lookup.len();
                let rel = new_rel_state(pc, RelStateArgs {
                    draft_data: draft_data.as_ref(),
                    draft_entry: if let Some(draft_data) = &draft_data {
                        Some(
                            draft_data
                                .0
                                .outgoing
                                .borrow_mut()
                                .entry((pred.clone(), obj.clone()))
                                .or_insert(Default::default())
                                .clone(),
                        )
                    } else {
                        None
                    },
                    initial_fill: full_pivot,
                    initial_pivot: if nodes.len() == 1 {
                        RelInitialPivot::Single
                    } else {
                        RelInitialPivot::Multi(Prim::new(subjs))
                    },
                    initial_pred_node: Some((pred.clone(), obj)),
                    new: false,
                    incoming: false,
                });
                rels_box.ref_push(build_edit_rel(pc, nodes.len(), &rel, false));
                rel_states.borrow_mut().push(rel);
            }
            if let Some(draft_data) = draft_data.as_ref() {
                let draft_new = draft_data.0.new_outgoing.borrow().clone();
                for draft_value in draft_new {
                    let rel_state = new_rel_state(pc, RelStateArgs {
                        draft_data: Some(draft_data),
                        draft_entry: Some(draft_value),
                        initial_fill: true,
                        initial_pivot: RelInitialPivot::Single,
                        initial_pred_node: None,
                        new: true,
                        incoming: false,
                    });
                    rels_box.ref_push(build_edit_rel(pc, nodes.len(), &rel_state, true));
                    rel_states.borrow_mut().push(rel_state);
                }
            }
            out.push(rels_box.clone());
            let add_row_res =
                style_export::cont_node_row_outgoing_add(
                    style_export::ContNodeRowOutgoingAddArgs { hint: "Add outgoing".to_string() },
                );
            add_row_res.button.ref_on("click", {
                let eg = pc.eg();
                let rel_states = rel_states.clone();
                let outgoing_rels_box = rels_box;
                let save_data = draft_data.clone();
                let total_nodes = nodes.len();
                move |_| eg.event(|pc| {
                    let rel = new_rel_state(pc, RelStateArgs {
                        draft_data: save_data.as_ref(),
                        draft_entry: if let Some(draft_data) = &save_data {
                            let draft_entry = MutDraftRel::default();
                            draft_data.0.new_outgoing.borrow_mut().insert(draft_entry.clone());
                            Some(draft_entry)
                        } else {
                            None
                        },
                        initial_fill: true,
                        initial_pivot: if nodes.len() == 1 {
                            RelInitialPivot::Single
                        } else {
                            RelInitialPivot::Multi(Prim::new(nodes.clone()))
                        },
                        initial_pred_node: None,
                        new: true,
                        incoming: false,
                    });
                    outgoing_rels_box.ref_push(build_edit_rel(pc, total_nodes, &rel, true));
                    rel_states.borrow_mut().push(rel);
                }).unwrap()
            });
            out.push(add_row_res.root);
        }

        // Edit form controls
        let button_delete = style_export::leaf_button_big_delete().root;
        button_delete.ref_on("click", {
            let delete_all = delete_all.clone();
            let eg = pc.eg();
            move |_| eg.event(|pc| {
                delete_all.set(pc, !delete_all.get());
            }).unwrap()
        });
        button_delete.ref_own(|ele| (link!((_pc = pc), (deleted = delete_all.clone()), (), (ele = ele.weak()), {
            let ele = ele.upgrade()?;
            ele.ref_modify_classes(&[(&style_export::class_state_pressed().value, deleted.get())]);
        }),));
        bar_out.push(button_delete);
        let button_commit = style_export::leaf_button_big_commit().root;
        button_commit.ref_on("click", {
            let rel_states = rel_states.clone();
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
                    let rel_states = rel_states.clone();
                    let pivot_state = pivot_state.clone();
                    let error_slot = error_slot.clone();
                    let draft_data = draft_data.clone();
                    let title = title.clone();
                    let eg = eg.clone();
                    let delete_all = delete_all.clone();
                    async move {
                        let mut rel_nodes_predicates = vec![];
                        let mut file_unique = 0usize;
                        let old_pivot;
                        let new_pivot;
                        let pivot_changed;
                        match &*pivot_state.0 {
                            PivotState_::Single(p) => {
                                let old_pivot0 = initial_type_value_to_node(&*p.type_.borrow(), &*p.value.borrow());
                                let new_pivot0 = type_value_to_node({
                                    file_unique += 1;
                                    file_unique
                                }, &*p.type_.borrow(), &*p.value.borrow());
                                pivot_changed = Some(&old_pivot0) != exenum!(&new_pivot0, CommitNode:: Node(x) => x);
                                old_pivot = vec![old_pivot0];
                                new_pivot = vec![new_pivot0];
                            },
                            PivotState_::Multi(p) => {
                                old_pivot = p.initial.borrow().clone();
                                new_pivot =
                                    old_pivot.iter().map(|n| CommitNode::Node(n.clone())).collect::<Vec<_>>();
                                pivot_changed = false;
                            },
                        }
                        let res = async {
                            ta_return!(HashMap < usize, FileHash >, String);
                            let mut add = vec![];
                            let mut remove = vec![];
                            let delete_all = delete_all.get();
                            for rel in &*RefCell::borrow(&rel_states) {
                                // Get current values
                                let rel_node = type_value_to_node({
                                    file_unique += 1;
                                    file_unique
                                }, &*rel.0.node_type.borrow(), &*rel.0.node_value.borrow());
                                let rel_pred = rel.0.predicate.borrow().clone();
                                rel_nodes_predicates.push((rel_pred.clone(), rel_node.clone()));

                                // Get original values for comparison
                                let rel_old_pivot;
                                let rel_new_pivot;
                                match &rel.0.initial_pivot {
                                    RelInitialPivot::Single => {
                                        rel_old_pivot = old_pivot.clone();
                                        rel_new_pivot = new_pivot.clone();
                                    },
                                    RelInitialPivot::Multi(initial_pivot) => {
                                        rel_old_pivot = initial_pivot.borrow().clone();
                                        rel_new_pivot =
                                            rel_old_pivot
                                                .iter()
                                                .map(|x| CommitNode::Node(x.clone()))
                                                .collect::<Vec<_>>();
                                    },
                                }
                                let rel_initial = match &*rel.0.initial_pred_node.borrow() {
                                    Some(initial_pred_node) => Some(
                                        (
                                            initial_pred_node.predicate.clone(),
                                            initial_type_value_to_node(
                                                &initial_pred_node.node_type_,
                                                &initial_pred_node.node_value,
                                            ),
                                        ),
                                    ),
                                    None => None,
                                };

                                // Classify if changed/deleted
                                let changed;
                                let new;
                                if let Some((rel_predicate_initial, rel_node_initial)) = &rel_initial {
                                    new = false;
                                    changed =
                                        pivot_changed ||
                                            exenum!(&rel_node, CommitNode:: Node(x) => x) != Some(rel_node_initial) ||
                                            rel.0.predicate.borrow().as_str() != rel_predicate_initial.as_str();
                                } else {
                                    new = true;
                                    changed = false;
                                }

                                // If not new but deleted or changed, delete first
                                if let Some((rel_predicate_initial, rel_node_initial)) = rel_initial {
                                    if delete_all || rel.0.delete.get() || changed {
                                        for initial_pivot in &rel_old_pivot {
                                            let old_subject;
                                            let old_object;
                                            if rel.0.incoming {
                                                old_subject = rel_node_initial.clone();
                                                old_object = initial_pivot.clone();
                                            } else {
                                                old_subject = initial_pivot.clone();
                                                old_object = rel_node_initial.clone();
                                            }
                                            remove.push(Triple {
                                                subject: old_subject,
                                                predicate: rel_predicate_initial.clone(),
                                                object: old_object,
                                            });
                                        }
                                    }
                                }

                                // If new or changed, write the new relation
                                if new || changed {
                                    for new_pivot in &rel_new_pivot {
                                        let subject;
                                        let object;
                                        if rel.0.incoming {
                                            subject = rel_node.clone();
                                            object = new_pivot.clone();
                                        } else {
                                            subject = new_pivot.clone();
                                            object = rel_node.clone();
                                        }
                                        add.push(CommitTriple {
                                            subject: subject,
                                            predicate: rel.0.predicate.borrow().clone(),
                                            object: object,
                                        });
                                    }
                                }
                            }

                            // # Send compiled changes
                            //
                            // Preprocess
                            let mut add1 = vec![];
                            let mut files_to_return = HashMap::new();
                            let mut files_to_commit = vec![];
                            let mut files_to_upload = vec![];
                            for rel in add {
                                let Some(subject) =
                                    commit::prep_node(
                                        &mut files_to_return,
                                        &mut files_to_commit,
                                        &mut files_to_upload,
                                        rel.subject,
                                    ).await else {
                                        continue;
                                    };
                                let Some(object) =
                                    commit::prep_node(
                                        &mut files_to_return,
                                        &mut files_to_commit,
                                        &mut files_to_upload,
                                        rel.object,
                                    ).await else {
                                        continue;
                                    };
                                add1.push(Triple {
                                    subject: subject,
                                    predicate: rel.predicate,
                                    object: object,
                                });
                            }

                            // Write commit
                            req_post_json(ReqCommit {
                                comment: format!("Edit node [{}]", title),
                                add: add1,
                                remove: remove,
                                files: files_to_commit,
                            }).await;

                            // Upload files
                            commit::upload_files(files_to_upload).await?;
                            return Ok(files_to_return);
                        }.await;
                        button.class_list().remove_1(&style_export::class_state_thinking().value).unwrap();
                        match res {
                            Ok(mut file_lookup) => {
                                eg.event(|pc| {
                                    delete_all.set(pc, false);

                                    // Get committed nodes (files replaced with hashes) and update initial values,
                                    // clear draft
                                    match &*pivot_state.0 {
                                        PivotState_::Single(p) => {
                                            // Replace files with file nodes
                                            let pivot_node = match new_pivot.into_iter().next().unwrap() {
                                                CommitNode::Node(n) => n,
                                                CommitNode::File(unique, _) => Node::File(
                                                    file_lookup.remove(&unique).unwrap(),
                                                ),
                                                CommitNode::DatetimeNow => unreachable!(),
                                            };

                                            // Sync initial
                                            let (t, v) = node_to_type_value(&pivot_node);
                                            p.initial.set(pc, InitialNode {
                                                node_type_: t.clone(),
                                                node_value: v.clone(),
                                            });

                                            // (in case file, change current value to "file" and not "new file" too)
                                            p.type_.set(pc, t);
                                            p.value.set(pc, v);

                                            // Clear draft
                                            if let Some(draft_data) = draft_data {
                                                clear_draft(&draft_data, &pivot_node);
                                            }
                                        },
                                        PivotState_::Multi(_) => {
                                            // nop
                                        },
                                    }
                                    for (
                                        rel,
                                        (sent_pred, sent_node),
                                    ) in Iterator::zip(
                                        RefCell::borrow(&rel_states).iter(),
                                        rel_nodes_predicates.into_iter(),
                                    ) {
                                        // Replace files with file nodes
                                        let (t, v) = node_to_type_value(&match sent_node {
                                            CommitNode::Node(n) => n,
                                            CommitNode::File(unique, _) => Node::File(
                                                file_lookup.remove(&unique).unwrap(),
                                            ),
                                            CommitNode::DatetimeNow => unreachable!(),
                                        });

                                        // Sync initial
                                        rel.0.initial_fill.set(pc, rel.0.fill.get());
                                        rel.0.initial_pred_node.set(pc, Some(RelInitialPredNode {
                                            predicate: sent_pred,
                                            node_type_: t.clone(),
                                            node_value: v.clone(),
                                        }));

                                        // (in case file, change current value to "file" and not "new file" too)
                                        rel.0.node_type.set(pc, t);
                                        rel.0.node_value.set(pc, v);
                                    }
                                }).unwrap();
                            },
                            Err(e) => {
                                let Some(error_slot) = error_slot.upgrade() else {
                                    return;
                                };
                                error_slot.ref_push(style_export::leaf_err_block(style_export::LeafErrBlockArgs {
                                    in_root: false,
                                    data: e,
                                }).root);
                            },
                        }
                    }
                }));
            }
        });
        bar_out.push(button_commit);
        return Ok(BuildNodeEditContentsRes {
            children: out,
            bar_children: bar_out,
        });
    }).unwrap();
}
