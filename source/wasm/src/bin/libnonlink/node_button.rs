use {
    crate::libnonlink::{
        api::req_post_json,
        ministate::{
            Ministate,
            MinistateNodeView,
            ministate_octothorpe,
        },
        state::{
            CurrentList,
            state,
        },
        viewutil::tree_node_to_text,
    },
    flowcontrol::{
        exenum,
        ta_return,
    },
    gloo::storage::{
        LocalStorage,
        SessionStorage,
        Storage,
    },
    lunk::{
        EventGraph,
        link,
    },
    rooting::El,
    shared::{
        interface::{
            ont::{
                PREDICATE_INDEX,
                PREDICATE_NAME,
                PREDICATE_TRACK,
                PREDICATE_VALUE,
            },
            query::{
                Chain,
                ChainHead,
                ChainRoot,
                ChainTail,
                MoveDirection,
                Query,
                QuerySuffix,
                Step,
                StepMove,
                StepRecurse,
                StepSpecific,
                StrValue,
            },
            triple::Node,
            wire::{
                ReqCommit,
                ReqCommitFree,
                ReqQuery,
                RespQueryRows,
                TreeNode,
                Triple,
            },
        },
        stringpattern::node_to_text,
    },
    wasm::js::{
        LogJsErr,
        on_thinking,
        style_export,
    },
};

pub const STORAGE_CURRENT_LIST: &str = "current_list";

pub struct ReqListResEntry {
    pub node: Node,
    pub index: Option<f64>,
    pub name: Option<String>,
}

pub async fn req_list(node: &Node) -> Result<Vec<ReqListResEntry>, String> {
    pub const KEY_INDEX: &str = "index";
    pub const KEY_NODE: &str = "node";
    pub const KEY_NAME: &str = "name";
    let existing = req_post_json(ReqQuery {
        query: Query {
            chain_head: ChainHead {
                root: Some(ChainRoot::Value(shared::interface::query::Value::Literal(node.clone()))),
                steps: vec![Step {
                    specific: StepSpecific::Move(StepMove {
                        dir: MoveDirection::Forward,
                        predicate: StrValue::Literal(PREDICATE_TRACK.to_string()),
                        filter: None,
                    }),
                    first: false,
                    sort: None,
                }],
            },
            suffix: Some(QuerySuffix {
                chain_tail: ChainTail {
                    bind: Some(KEY_NODE.to_string()),
                    subchains: vec![
                        //. .
                        Chain {
                            head: ChainHead {
                                root: None,
                                steps: vec![Step {
                                    specific: StepSpecific::Move(StepMove {
                                        dir: MoveDirection::Forward,
                                        predicate: StrValue::Literal(PREDICATE_INDEX.to_string()),
                                        filter: None,
                                    }),
                                    sort: None,
                                    first: true,
                                }],
                            },
                            tail: ChainTail {
                                bind: Some(KEY_INDEX.to_string()),
                                subchains: vec![],
                            },
                        },
                        Chain {
                            head: ChainHead {
                                root: None,
                                steps: vec![
                                    //. .
                                    Step {
                                        specific: StepSpecific::Recurse(StepRecurse { subchain: ChainHead {
                                            root: None,
                                            steps: vec![Step {
                                                specific: StepSpecific::Move(StepMove {
                                                    dir: MoveDirection::Forward,
                                                    predicate: StrValue::Literal(PREDICATE_VALUE.to_string()),
                                                    filter: None,
                                                }),
                                                sort: None,
                                                first: false,
                                            }],
                                        } }),
                                        sort: None,
                                        first: false,
                                    },
                                    Step {
                                        specific: StepSpecific::Move(StepMove {
                                            dir: MoveDirection::Forward,
                                            predicate: StrValue::Literal(PREDICATE_NAME.to_string()),
                                            filter: None,
                                        }),
                                        sort: None,
                                        first: true,
                                    }
                                ],
                            },
                            tail: ChainTail {
                                bind: Some(KEY_NAME.to_string()),
                                subchains: vec![],
                            },
                        }
                    ],
                },
                sort: None,
            }),
        },
        parameters: Default::default(),
        pagination: None,
    }).await?;
    let RespQueryRows::Record(rows) = existing.rows else {
        return Err(format!("Add item to list failed; resp returned non-record rows"));
    };
    let mut out = vec![];
    for mut x in rows {
        out.push(ReqListResEntry {
            name: x.remove(KEY_NAME).map(|x| tree_node_to_text(&x)),
            index: exenum!(
                x.remove(KEY_INDEX),
                Some(TreeNode::Scalar(Node::Value(serde_json::Value::Number(last)))) => last.as_f64()
            ).flatten(),
            node: {
                let Some(TreeNode::Scalar(n)) = x.remove(KEY_NODE) else {
                    return Err(
                        format!(
                            "Unexpectedly received row missing node or non-scalar: {}",
                            serde_json::to_string(&x).unwrap()
                        ),
                    );
                };
                n.clone()
            },
        });
    }
    out.sort_by(|x, y| {
        match (x.index, y.index) {
            (None, None) => std::cmp::Ordering::Equal,
            (None, Some(_)) => std::cmp::Ordering::Less,
            (Some(_), None) => std::cmp::Ordering::Greater,
            (Some(x), Some(y)) => x.total_cmp(&y),
        }
    });
    return Ok(out);
}

pub fn setup_node_button(eg: &EventGraph, out: &El, name: String, node: Node) {
    out.ref_on("click", {
        let eg = eg.clone();
        let name = name.clone();
        let node = node.clone();
        move |_| eg.event(|pc| {
            let current_list = state().current_list.borrow().clone();
            let modal_res = style_export::cont_modal_node(style_export::ContModalNodeArgs {
                current_list_name: current_list.as_ref().map(|x| x.name.clone()),
                current_list_id: current_list.as_ref().map(|x| node_to_text(&x.node)),
                current_list_link: current_list
                    .as_ref()
                    .map(|x| ministate_octothorpe(&Ministate::NodeView(MinistateNodeView {
                        title: x.name.clone(),
                        node: x.node.clone(),
                    }))),
                node_link: ministate_octothorpe(&Ministate::NodeView(MinistateNodeView {
                    title: name.clone(),
                    node: node.clone(),
                })),
            });

            // Modal boilerplate
            modal_res.button_close.ref_on("click", {
                let modal_el = modal_res.root.weak();
                let eg = pc.eg();
                move |_| eg.event(|_pc| {
                    let Some(modal_el) = modal_el.upgrade() else {
                        return;
                    };
                    modal_el.ref_replace(vec![]);
                }).unwrap()
            });
            modal_res.root.ref_on("click", {
                let modal_el = modal_res.root.weak();
                let eg = pc.eg();
                move |_| eg.event(|_pc| {
                    let Some(modal_el) = modal_el.upgrade() else {
                        return;
                    };
                    modal_el.ref_replace(vec![]);
                }).unwrap()
            });

            // Add to list
            modal_res.button_add_to_list.ref_own({
                |self0| (
                    //. .
                    link!((_pc = pc), (current_list = state().current_list.clone()), (), (self0 = self0.weak()) {
                        let self0 = self0.upgrade()?;
                        self0.ref_modify_classes(
                            &[(&style_export::class_state_disabled().value, current_list.borrow().is_none())],
                        );
                    }),
                )
            });
            on_thinking(&modal_res.button_add_to_list, {
                let node = node.clone();
                let modal_el = modal_res.root.weak();
                let modal_errs = modal_res.errors.weak();
                async move || {
                    let res = async {
                        ta_return!((), String);
                        let rows = req_list(&node).await?;
                        let Some(current_list) = state().current_list.borrow().clone() else {
                            return Ok(());
                        };
                        let middle = Node::Value(serde_json::Value::String(uuid::Uuid::new_v4().to_string()));
                        let mut add = vec![
                            //. .
                            Triple {
                                subject: current_list.node.clone(),
                                predicate: PREDICATE_TRACK.to_string(),
                                object: middle.clone(),
                            },
                            Triple {
                                subject: middle.clone(),
                                predicate: PREDICATE_VALUE.to_string(),
                                object: node.clone(),
                            },
                        ];
                        if rows.is_empty() || rows.iter().any(|x| x.index.is_some()) {
                            let last_index =
                                if let Some(last) = rows.iter().flat_map(|x| x.index).max_by(f64::total_cmp) {
                                    serde_json::Number::from_f64(last).unwrap_or(serde_json::Number::from(0))
                                } else {
                                    serde_json::Number::from(0)
                                };
                            add.push(Triple {
                                subject: middle.clone(),
                                predicate: PREDICATE_INDEX.to_string(),
                                object: Node::Value(
                                    serde_json::Value::Number(
                                        serde_json::Number::from(last_index.as_i64().unwrap_or(0) + 1),
                                    ),
                                ),
                            });
                        }
                        req_post_json(ReqCommit::Free(ReqCommitFree {
                            add: add,
                            comment: "Add node to list via UI".to_string(),
                            remove: vec![],
                            files: vec![],
                        })).await?;
                        return Ok(());
                    }.await;
                    let Some(modal_errs) = modal_errs.upgrade() else {
                        return;
                    };
                    modal_errs.ref_clear();
                    match res {
                        Ok(_) => {
                            let Some(modal_el) = modal_el.upgrade() else {
                                return;
                            };
                            modal_el.ref_replace(vec![]);
                        },
                        Err(e) => {
                            modal_errs.ref_push(style_export::leaf_err_block(style_export::LeafErrBlockArgs {
                                data: e,
                                in_root: false,
                            }).root);
                        },
                    }
                }
            });

            // Set list
            modal_res.button_set_list.ref_on("click", {
                let modal_el = modal_res.root.weak();
                let name = name.clone();
                let node = node.clone();
                let eg = pc.eg();
                move |_| eg.event(|pc| {
                    let Some(modal_el) = modal_el.upgrade() else {
                        return;
                    };
                    modal_el.ref_replace(vec![]);
                    let new_list = CurrentList {
                        name: name.clone(),
                        node: node.clone(),
                    };
                    SessionStorage::set(
                        STORAGE_CURRENT_LIST,
                        &new_list,
                    ).log(&state().log, "Error storing list state in session storage");
                    LocalStorage::set(
                        STORAGE_CURRENT_LIST,
                        &new_list,
                    ).log(&state().log, "Error storing list state in local storage");
                    state().current_list.set(pc, Some(new_list));
                }).unwrap()
            });

            // Finish up
            state().modal_stack.ref_push(modal_res.root.clone());
        }).unwrap()
    });
}
