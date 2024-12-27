use {
    super::state::State,
    flowcontrol::shed,
    lunk::{
        link,
        Prim,
        ProcessingContext,
    },
    rooting::{
        el,
        spawn_rooted,
        El,
    },
    serde_json::Number,
    shared::interface::{
        triple::{
            FileHash,
            Node,
        },
        wire::{
            ReqCommit,
            ReqGetTriplesAround,
        },
    },
    std::{
        cell::{
            Cell,
            RefCell,
        },
        rc::Rc,
        str::FromStr,
    },
    wasm_bindgen::JsCast,
    web::{
        el_general::{
            el_async,
            el_button_icon,
            el_button_icon_text,
            el_err_block,
            el_hbox,
            el_vbox,
            CSS_FORM_BUTTONBOX,
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
    web_sys::{
        HtmlInputElement,
        HtmlSelectElement,
    },
};

pub fn build_page_edit(pc: &mut ProcessingContext, outer_state: &State, edit_title: &str, node: &Node) {
    outer_state.page_title.upgrade().unwrap().ref_text(&edit_title);
    outer_state.page_body.upgrade().unwrap().ref_push(el_async().own(|async_el| {
        let async_el = async_el.weak();
        let eg = pc.eg();
        let outer_state = outer_state.clone();
        let node = node.clone();
        spawn_rooted(async move {
            let triples =
                match req_post_json(&outer_state.origin, ReqGetTriplesAround { node: node.clone() }).await {
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
                    fn build_cont_incoming() -> El {
                        return el("div").classes(&["g_edit_row_incoming"]);
                    }

                    fn build_cont_outgoing() -> El {
                        return el("div").classes(&["g_edit_row_outgoing"]);
                    }

                    #[derive(Clone)]
                    enum EditNodeType {
                        Id,
                        File,
                        Str,
                        Num,
                        Bool,
                        Json,
                    }

                    enum EditNodeValue {
                        String(String),
                        Bool(bool),
                    }

                    #[derive(Clone)]
                    struct NodeState {
                        type_: Prim<EditNodeType>,
                        value: Prim<EditNodeValue>,
                    }

                    struct BuildEditNodeRes {
                        type_select: El,
                        input: El,
                    }

                    fn build_edit_node(pc: &mut ProcessingContext, node: &NodeState) -> BuildEditNodeRes {
                        let type_select = el("select");
                        const OPT_ID: &str = "id";
                        let opt_id = el("option").attr("value", OPT_ID).text("Id");
                        type_select.ref_push(opt_id.clone());
                        const OPT_FILE: &str = "file";
                        let opt_file = el("option").attr("value", OPT_FILE).text("File");
                        type_select.ref_push(opt_file.clone());
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
                        match &*node.type_.borrow() {
                            EditNodeType::Id => {
                                opt_id.attr("selected", "selected");
                            },
                            EditNodeType::File => {
                                opt_file.attr("selected", "selected");
                            },
                            EditNodeType::Bool => {
                                opt_val_bool.attr("selected", "selected");
                            },
                            EditNodeType::Num => {
                                opt_val_num.attr("selected", "selected");
                            },
                            EditNodeType::Str => {
                                opt_val_str.attr("selected", "selected");
                            },
                            EditNodeType::Json => {
                                opt_val_json.attr("selected", "selected");
                            },
                        }
                        let old_input = el_async();
                        type_select.on("change", {
                            let node = node.clone();
                            let eg = pc.eg();
                            move |ev| eg.event(|pc| {
                                // TODO select as prim w/ enum, do handling here in link to address cycles
                                let type_select = ev.target().unwrap().dyn_into::<HtmlSelectElement>().unwrap();
                                let new_type_raw = type_select.value();
                                let new_type;
                                let new_value;
                                shed!{
                                    match new_type_raw.as_str() {
                                        OPT_FILE => {
                                            node.type_.set(pc, EditNodeType::File);
                                            new_type = EditNodeType::File;
                                            new_value = EditNodeValue::String(as_str());
                                        },
                                        OPT_ID => {
                                            break Node::Id(as_str());
                                        },
                                        OPT_VAL_BOOL => {
                                            let old_value = as_str();
                                            if old_value == "true" {
                                                break Node::Value(serde_json::Value::Bool(true));
                                            } else if old_value == "false" {
                                                break Node::Value(serde_json::Value::Bool(false));
                                            }
                                        },
                                        OPT_VAL_NUM => {
                                            if let Ok(v) = serde_json::from_str::<serde_json::Number>(&as_str()) {
                                                break Node::Value(serde_json::Value::Number(v));
                                            }
                                        },
                                        OPT_VAL_STR => {
                                            break Node::Value(serde_json::Value::String(as_str()));
                                        },
                                        OPT_VAL_JSON => {
                                            // fall through
                                        },
                                        _ => {
                                            unreachable!();
                                        },
                                    }
                                    break Node::Value(serde_json::Value::String(as_str()));
                                };
                                node.0.set(pc, new_value);
                            })
                        });
                        type_select.ref_own(|_| {
                            link!((_pc = pc), (node = node.0.clone()), (), (old_input = RefCell::new(old_input)) {
                                let new_input;
                                match &*node.borrow() {
                                    Node::Id(v) => {
                                        new_input = el("input").attr("type", "text").attr("value", &v);
                                    },
                                    Node::File(v) => {
                                        new_input = el("input").attr("type", "text").attr("value", &v.to_string());
                                    },
                                    Node::Value(v) => match v {
                                        serde_json::Value::Bool(v) => {
                                            new_input = el("input").attr("type", "checkbox");
                                            if *v {
                                                new_input.ref_attr("checked", "checked");
                                            }
                                        },
                                        serde_json::Value::Number(v) => {
                                            new_input =
                                                el("input").attr("type", "number").attr("value", &v.to_string());
                                        },
                                        serde_json::Value::String(v) => {
                                            new_input = el("input").attr("type", "text").attr("value", &v);
                                        },
                                        _ => {
                                            new_input =
                                                el("input")
                                                    .attr("type", "text")
                                                    .attr("value", &serde_json::to_string(&v).unwrap());
                                        },
                                    },
                                }
                                let mut old_input = old_input.borrow_mut();
                                old_input.ref_replace(vec![new_input.clone()]);
                                *old_input = new_input;
                            })
                        });
                        return BuildEditNodeRes {
                            type_select: type_select,
                            input: old_input,
                        };
                    }

                    let pivot = NodeState(Prim::new(pc, node));

                    struct TripleState_ {
                        incoming: bool,
                        initial: (String, Node),
                        add: bool,
                        delete: Prim<bool>,
                        predicate: Prim<String>,
                        node: NodeState,
                    }

                    struct TripleState(Rc<TripleState_>);

                    fn build_edit_triple(pc: &mut ProcessingContext, triple: &TripleState) -> El {
                        let out = el_vbox();
                        let node_row = el_hbox();
                        node_row.ref_push(el_button_icon(pc, ICON_RESET, "Restore original value", {
                            let out = out.weak();
                            let triple = triple.clone();
                            |pc| {
                                let Some(out) = out.upgrade() else {
                                    return;
                                };
                                triple.0.predicate.set(pc, triple.0.initial.0.clone());
                                *triple.0.node.0.set(pc, triple.0.initial.1.clone());
                            }
                        }));
                        node_row.ref_push(build_edit_node(&triple.0.node));
                        node_row.ref_push(el_button_icon_toggle(pc, ICON_REMOVE, "Delete triple", {
                            let triple = triple.clone();
                            move |pc, on| {
                                triple.0.delete.set(!on);
                                out.ref_modify_classes(([(CSS_EDIT_DELETED, !on)]));
                            }
                        }));
                        node_row.ref_push(el_button_icon(pc, ICON_COPY, "Copy node", {
                            || { }
                        }));
                        node_row.ref_push(el_button_icon(pc, ICON_PASTE, "Paste node", {
                            || { }
                        }));
                        let predicate_row =
                            el("input").attr("type", "text").attr("value", triple.0.predicate.get_mut());
                        if triple.0.incoming {
                            return el_vbox().push(node_row).push(predicate_row);
                        } else {
                            return el_vbox().push(predicate_row).push(node_row);
                        }
                    }

                    let mut out = vec![];
                    out.push(build_cont_incoming().push(el_button_icon(pc, ICON_ADD, "Add incoming", || { })));
                    for t in triples.incoming {
                        let triple = TripleState(Rc::new(TripleState_ {
                            incoming: true,
                            initial: (t.predicate.clone(), t.subject.clone()),
                            add: false,
                            delete: Cell::new(false),
                            predicate: Cell::new(t.predicate),
                            node: NodeState(Rc::new(RefCell::new(t.subject))),
                        }));
                        out.push(
                            build_cont_incoming().push(
                                el_vbox().extend(
                                    vec![
                                        build_incoming_triple(
                                            build_triple_state(
                                                build_node_state(&t.subject),
                                                t.predicate,
                                                pivot.clone(),
                                                t.iam_target,
                                            ),
                                        )
                                    ],
                                ),
                            ),
                        );
                    }
                    out.push(build_edit_node(pivot.clone()));
                    for t in triples.outgoing {
                        out.push(
                            build_outgoing_triple(
                                build_triple_state(
                                    pivot.clone(),
                                    t.predicate,
                                    build_node_state(&t.object),
                                    t.iam_target,
                                ),
                            ),
                        );
                    }
                    out.push(build_cont_outgoing().push(el_button_icon(pc, ICON_ADD, "Add outgoing", || { })));
                    let error_el = el("div").classes(&[CSS_FORM_ERROR]);
                    out.push(error_el.clone());
                    out.push(
                        el("div").classes(&[CSS_FORM_BUTTONBOX]).push(el_button_icon_text(pc, ICON_SAVE, "Save", {
                            let outer_state = outer_state.clone();
                            let bg = Rc::new(RefCell::new(None));
                            move |pc| {
                                *bg.borrow_mut() = Some(spawn_rooted(async move {
                                    match async {
                                        let data = fs.0.data.borrow();
                                        let mut add = vec![];
                                        let mut remove = vec![];
                                        for triple in new_triples {
                                            if triple.new == triple.old {
                                                continue;
                                            }
                                            add.push(triple.new);
                                            remove.push(triple.old);
                                        }
                                        drop(data);
                                        world::req_post_json(&outer_state.origin, ReqCommit {
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
                                            let Some(async_el) = async_el.upgrade() else {
                                                return;
                                            };
                                            async_el.ref_replace(vec![el_err_block(e)]);
                                            return;
                                        },
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
