use {
    super::{
        infinite::build_infinite,
        page_node_view::build_node_el,
        state::set_page,
    },
    crate::libnonlink::{
        api::req_post_json,
        state::state,
    },
    chrono::Utc,
    lunk::ProcessingContext,
    rooting::{
        spawn_rooted,
        El,
        WeakEl,
    },
    shared::interface::{
        triple::Node,
        wire::{
            ReqCommit,
            ReqHistoryCommitCount,
            Triple,
        },
    },
    std::{
        cell::{
            Cell,
            RefCell,
        },
        cmp::Reverse,
        collections::{
            BTreeMap,
            HashSet,
        },
        rc::Rc,
    },
    wasm::js::style_export,
};

struct HistState {
    revert_was_deleted: RefCell<HashSet<Triple>>,
    revert_was_added: RefCell<HashSet<Triple>>,
    save: WeakEl,
}

fn setup_revert_button<'a>(button: &El, hist_state: Rc<HistState>, was_deleted: bool, row: Triple) {
    button.ref_on("click", {
        let button = button.weak();
        let reverted = Cell::new(false);
        move |_| {
            let Some(button) = button.upgrade() else {
                return;
            };
            let on = !reverted.get();
            reverted.set(on);
            button.ref_modify_classes(&[(&style_export::class_state_pressed().value, on)]);
            let mut revert_was_deleted = hist_state.revert_was_deleted.borrow_mut();
            let mut revert_was_added = hist_state.revert_was_added.borrow_mut();
            let revert_set = if was_deleted {
                &mut *revert_was_deleted
            } else {
                &mut *revert_was_added
            };
            if on {
                revert_set.insert(row.clone());
            } else {
                revert_set.remove(&row);
            }
            if let Some(save) = hist_state.save.upgrade() {
                save.ref_modify_classes(
                    &[
                        (
                            &style_export::class_state_disabled().value,
                            revert_was_deleted.is_empty() && revert_was_added.is_empty(),
                        ),
                    ],
                );
            }
        }
    });
}

pub fn build_page_history(pc: &mut ProcessingContext) {
    let error_slot = style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root;
    let button_commit = style_export::leaf_button_big_commit().root;
    let page_res = style_export::cont_page_node(style_export::ContPageNodeArgs {
        page_button_children: vec![],
        bar_children: vec![button_commit.clone()],
        children: vec![error_slot.clone()],
    });
    let hist_state = Rc::new(HistState {
        revert_was_deleted: Default::default(),
        revert_was_added: Default::default(),
        save: button_commit.weak(),
    });
    build_infinite(page_res.body.clone(), Utc::now(), {
        let hist_state = hist_state.clone();
        move |key| {
            let hist_state = hist_state.clone();
            async move {
                let mut commits =
                    req_post_json(&state().env.base_url, ReqHistoryCommitCount { end_excl: key }).await?;
                commits.sort_by_cached_key(|c| Reverse(c.timestamp));
                let mut out = vec![];
                let next_key = commits.last().map(|x| x.timestamp);
                for commit in commits {
                    #[derive(Default)]
                    struct Pred {
                        remove: Vec<Triple>,
                        add: Vec<Triple>,
                    }

                    let mut subjects: BTreeMap<Node, BTreeMap<String, Pred>> = Default::default();
                    for triple in commit.remove {
                        subjects
                            .entry(triple.subject.clone())
                            .or_default()
                            .entry(triple.predicate.clone())
                            .or_default()
                            .remove
                            .push(triple);
                    }
                    for triple in commit.add {
                        subjects
                            .entry(triple.subject.clone())
                            .or_default()
                            .entry(triple.predicate.clone())
                            .or_default()
                            .add
                            .push(triple);
                    }
                    let mut commit_children = vec![];
                    for (subject, preds) in subjects {
                        let mut subject_rows = vec![];
                        for (_, pred) in preds {
                            for row in pred.remove {
                                let row_res =
                                    style_export::cont_history_predicate_object_remove(
                                        style_export::ContHistoryPredicateObjectRemoveArgs {
                                            children: vec![
                                                style_export::leaf_node_view_predicate(
                                                    style_export::LeafNodeViewPredicateArgs {
                                                        value: row.predicate.clone(),
                                                    },
                                                ).root,
                                                build_node_el(&row.object, true),
                                            ],
                                        },
                                    );
                                setup_revert_button(&row_res.button, hist_state.clone(), true, row);
                                subject_rows.push(row_res.root);
                            }
                            for row in pred.add {
                                let row_res =
                                    style_export::cont_history_predicate_object_add(
                                        style_export::ContHistoryPredicateObjectAddArgs {
                                            children: vec![
                                                style_export::leaf_node_view_predicate(
                                                    style_export::LeafNodeViewPredicateArgs {
                                                        value: row.predicate.clone(),
                                                    },
                                                ).root,
                                                build_node_el(&row.object, true),
                                            ],
                                        },
                                    );
                                setup_revert_button(&row_res.button, hist_state.clone(), false, row);
                                subject_rows.push(row_res.root);
                            }
                        }
                        commit_children.push(style_export::cont_history_subject(style_export::ContHistorySubjectArgs {
                            center: vec![build_node_el(&subject, true)],
                            rows: subject_rows,
                        }).root);
                    }
                    out.push(style_export::cont_history_commit(style_export::ContHistoryCommitArgs {
                        stamp: commit.timestamp.to_rfc3339(),
                        desc: commit.desc,
                        children: commit_children,
                    }).root);
                }
                return Ok((next_key, out));
            }
        }
    });
    button_commit.ref_on("click", {
        let hist_state = hist_state.clone();
        let error_slot = error_slot.weak();
        let button = button_commit.weak();
        let eg = pc.eg();
        let save_thinking = Rc::new(RefCell::new(None));
        move |_ev| {
            if save_thinking.borrow().is_some() {
                return;
            }
            {
                let Some(error_slot) = error_slot.upgrade() else {
                    return;
                };
                error_slot.ref_clear();
            }
            let Some(button) = button.upgrade() else {
                return;
            };
            button.ref_classes(&[&style_export::class_state_thinking().value]);
            *save_thinking.borrow_mut() = Some(spawn_rooted({
                let hist_state = hist_state.clone();
                let error_slot = error_slot.clone();
                let button = button.weak();
                let eg = eg.clone();
                async move {
                    let res = req_post_json(&state().env.base_url, ReqCommit {
                        comment: format!("History restore"),
                        add: hist_state.revert_was_deleted.borrow().iter().cloned().collect(),
                        remove: hist_state.revert_was_added.borrow().iter().cloned().collect(),
                        files: vec![],
                    }).await;
                    let Some(button) = button.upgrade() else {
                        return;
                    };
                    button.ref_remove_classes(&[&style_export::class_state_thinking().value]);
                    match res {
                        Ok(_) => {
                            eg.event(|pc| {
                                build_page_history(pc);
                            });
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
    set_page(pc, "History", page_res.root);
}
