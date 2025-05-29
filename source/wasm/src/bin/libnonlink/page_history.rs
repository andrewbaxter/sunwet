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
    chrono::{
        Utc,
    },
    lunk::ProcessingContext,
    rooting::{
        spawn_rooted,
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
    wasm::js::{
        style_export,
    },
};

struct HistState {
    revert: RefCell<HashSet<Triple>>,
    save: WeakEl,
}

pub fn build_page_history(pc: &mut ProcessingContext) {
    let error_slot = style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root;
    let button_save = style_export::leaf_button_big_save().root;
    let page_res = style_export::cont_page_node_view_and_history(style_export::ContPageNodeViewAndHistoryArgs {
        page_button_children: vec![button_save.clone()],
        children: vec![error_slot.clone()],
    });
    let hist_state = Rc::new(HistState {
        revert: Default::default(),
        save: button_save.weak(),
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
                                row_res.button.ref_on("click", {
                                    let button = row_res.button.weak();
                                    let reverted = Cell::new(false);
                                    let hist_state = hist_state.clone();
                                    move |_| {
                                        let Some(button) = button.upgrade() else {
                                            return;
                                        };
                                        let new_reverted = !reverted.get();
                                        reverted.set(new_reverted);
                                        button.ref_modify_classes(
                                            &[(&style_export::class_state_disabled().value, new_reverted)],
                                        );
                                        if new_reverted {
                                            hist_state.revert.borrow_mut().insert(row.clone());
                                        } else {
                                            hist_state.revert.borrow_mut().remove(&row);
                                        }
                                        if let Some(save) = hist_state.save.upgrade() {
                                            save.ref_modify_classes(
                                                &[
                                                    (
                                                        &style_export::class_state_disabled().value,
                                                        hist_state.revert.borrow().is_empty(),
                                                    ),
                                                ],
                                            );
                                        }
                                    }
                                });
                                subject_rows.push(row_res.root);
                            }
                            for row in pred.add {
                                subject_rows.push(
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
                                    ).root,
                                );
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
    button_save.ref_on("click", {
        let hist_state = hist_state.clone();
        let error_slot = error_slot.weak();
        let button = button_save.weak();
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
                        add: hist_state.revert.borrow().iter().cloned().collect(),
                        remove: vec![],
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
