use {
    super::{
        infinite::build_infinite,
        page_node_view::build_node_el,
        state::set_page,
    },
    crate::libnonlink::{
        api::req_post_json,
        infinite::InfPageRes,
        ministate::{
            MinistateHistory,
            MinistateHistoryPredicate,
        },
        state::{
            state,
        },
    },
    lunk::ProcessingContext,
    rooting::{
        El,
        WeakEl,
        spawn_rooted,
    },
    shared::interface::wire::{
        ReqCommit,
        ReqHistory,
        ReqHistoryFilter,
        ReqHistoryFilterPredicate,
        Triple,
    },
    std::{
        cell::{
            Cell,
            RefCell,
        },
        collections::HashSet,
        rc::Rc,
    },
    wasm::js::style_export,
};

struct HistState {
    revert_was_deleted: RefCell<HashSet<Triple>>,
    revert_was_added: RefCell<HashSet<Triple>>,
    save: WeakEl,
    ministate: MinistateHistory,
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

pub fn build_page_history(pc: &mut ProcessingContext, ministate: &MinistateHistory) {
    let error_slot = style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root;
    let button_commit = style_export::leaf_button_big_commit().root;
    button_commit.ref_classes(&[&style_export::class_state_disabled().value]);
    let page_res = style_export::cont_page_history(style_export::ContPageHistoryArgs {
        bar_children: vec![button_commit.clone()],
        children: vec![error_slot.clone()],
    });
    let hist_state = Rc::new(HistState {
        revert_was_deleted: Default::default(),
        revert_was_added: Default::default(),
        save: button_commit.weak(),
        ministate: ministate.clone(),
    });
    page_res.body.ref_push(build_infinite(&state().log, None, {
        let hist_state = hist_state.clone();
        move |page_key| {
            let hist_state = hist_state.clone();
            async move {
                let hist_res = req_post_json(ReqHistory {
                    page_key: page_key.clone(),
                    filter: match &hist_state.ministate.filter {
                        Some(f) => Some(ReqHistoryFilter {
                            node: f.node.clone(),
                            predicate: match &f.predicate {
                                Some(p) => match p {
                                    MinistateHistoryPredicate::Incoming(p) => Some(
                                        ReqHistoryFilterPredicate::Incoming(p.clone()),
                                    ),
                                    MinistateHistoryPredicate::Outgoing(p) => Some(
                                        ReqHistoryFilterPredicate::Outgoing(p.clone()),
                                    ),
                                },
                                None => None,
                            },
                        }),
                        None => None,
                    },
                }).await?;
                if hist_res.events.is_empty() {
                    return Ok(InfPageRes {
                        next_key: None,
                        page_els: vec![],
                        immediate_advance: false,
                    });
                }
                let page_key_next = hist_res.events.last().as_ref().map(|x| (x.commit, x.triple.clone()));
                let mut prev_commit = page_key.as_ref().map(|x| x.0);
                let mut prev_subject = page_key.map(|x| x.1.subject);
                let mut out = vec![];
                for event in hist_res.events {
                    let mut commit_changed = false;
                    if Some(event.commit) != prev_commit {
                        prev_commit = Some(event.commit);
                        commit_changed = true;
                        out.push(style_export::cont_history_commit(style_export::ContHistoryCommitArgs {
                            stamp: event.commit.to_rfc3339(),
                            desc: hist_res.commit_descriptions.get(&event.commit).cloned().unwrap_or_default(),
                        }).root);
                    }
                    if commit_changed || Some(&event.triple.subject) != prev_subject.as_ref() {
                        prev_subject = Some(event.triple.subject.clone());
                        out.push(
                            style_export::cont_history_subject(
                                style_export::ContHistorySubjectArgs {
                                    center: vec![build_node_el(&event.triple.subject)],
                                },
                            ).root,
                        );
                    }
                    out.push(if event.delete {
                        let row_res =
                            style_export::cont_history_predicate_object_remove(
                                style_export::ContHistoryPredicateObjectRemoveArgs {
                                    children: vec![
                                        style_export::leaf_node_view_predicate(
                                            style_export::LeafNodeViewPredicateArgs {
                                                value: event.triple.predicate.clone(),
                                            },
                                        ).root,
                                        build_node_el(&event.triple.object),
                                    ],
                                },
                            );
                        setup_revert_button(&row_res.button, hist_state.clone(), true, event.triple);
                        row_res.root
                    } else {
                        let row_res =
                            style_export::cont_history_predicate_object_add(
                                style_export::ContHistoryPredicateObjectAddArgs {
                                    children: vec![
                                        style_export::leaf_node_view_predicate(
                                            style_export::LeafNodeViewPredicateArgs {
                                                value: event.triple.predicate.clone(),
                                            },
                                        ).root,
                                        build_node_el(&event.triple.object),
                                    ],
                                },
                            );
                        setup_revert_button(&row_res.button, hist_state.clone(), false, event.triple);
                        row_res.root
                    });
                }
                return Ok(InfPageRes {
                    next_key: Some(page_key_next),
                    page_els: out,
                    immediate_advance: false,
                });
            }
        }
    }));
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
                let button = button.weak();
                let eg = eg.clone();
                async move {
                    req_post_json(ReqCommit {
                        comment: format!("History restore"),
                        add: hist_state.revert_was_deleted.borrow().iter().cloned().collect(),
                        remove: hist_state.revert_was_added.borrow().iter().cloned().collect(),
                        files: vec![],
                    }).await;
                    let Some(button) = button.upgrade() else {
                        return;
                    };
                    button.ref_remove_classes(&[&style_export::class_state_thinking().value]);
                    eg.event(|pc| {
                        build_page_history(pc, &hist_state.ministate);
                    });
                }
            }));
        }
    });
    set_page(pc, "History", page_res.root);
}
