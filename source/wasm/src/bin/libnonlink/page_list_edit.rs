use {
    super::{
        api::req_post_json,
        ministate::MinistateNodeView,
        state::set_page,
    },
    crate::libnonlink::{
        ministate::{
            ministate_octothorpe,
            Ministate,
        },
        node_button::req_list,
        state::state,
    },
    flowcontrol::ta_return,
    lunk::{
        link,
        Prim,
        ProcessingContext,
    },
    rooting::El,
    shared::{
        interface::{
            ont::PREDICATE_INDEX,
            triple::Node,
            wire::{
                ReqCommit,
                ReqGetTriplesAround,
                Triple,
            },
        },
        stringpattern::node_to_text,
    },
    std::{
        cell::RefCell,
        rc::Rc,
    },
    wasm::js::{
        el_async_,
        style_export,
        LogJsErr,
    },
    wasm_bindgen::JsCast,
    wasm_bindgen_futures::spawn_local,
    web_sys::{
        Event,
        HtmlElement,
        HtmlInputElement,
    },
};

struct EntryState {
    node: Node,
    index: Prim<f64>,
    initial_index: Prim<f64>,
    delete: Prim<bool>,
    initial_delete: Prim<bool>,
    checkbox: El,
}

pub fn build_page_list_edit(pc: &mut ProcessingContext, title: &str, node: &Node) {
    set_page(pc, &format!("Edit list: {}", title), el_async_(true, {
        let eg = pc.eg();
        let title = title.to_string();
        let node = node.clone();
        async move {
            ta_return!(Vec < El >, String);
            let entries = req_list(&node).await?;
            return eg.event(|pc| {
                let enable_numbers = Prim::new(entries.iter().any(|x| x.index.is_some()) || entries.is_empty());
                let initial_enable_numbers = Prim::new(*enable_numbers.borrow());
                let states: Rc<RefCell<Vec<EntryState>>> = Rc::new(RefCell::new(vec![]));
                let renumber = {
                    let states = states.clone();
                    move |pc: &mut ProcessingContext| {
                        for (i, s) in states.borrow().iter().filter(|x| !*x.delete.borrow()).enumerate() {
                            s.index.set(pc, (i + 1) as f64);
                        }
                    }
                };

                // Build entries
                let mut el_out = vec![];
                let mut state_out = vec![];
                for (i, entry) in entries.into_iter().enumerate() {
                    let index = Prim::new(entry.index.unwrap_or(i as f64));
                    let initial_index = Prim::new(*index.borrow());
                    let delete = Prim::new(false);
                    let name = entry.name.unwrap_or_else(|| node_to_text(&entry.node));
                    let entry_el = style_export::leaf_page_list_edit_entry(style_export::LeafPageListEditEntryArgs {
                        id: node_to_text(&entry.node),
                        id_link: ministate_octothorpe(&Ministate::NodeView(MinistateNodeView {
                            title: name.clone(),
                            node: entry.node.clone(),
                        })),
                        name: name,
                    });
                    entry_el
                        .root
                        .ref_own(
                            |root| link!(
                                (_pc = pc),
                                (index = index.clone()),
                                (),
                                (root_el = root.weak(), number_el = entry_el.number.clone()) {
                                    let root_el = root_el.upgrade()?;
                                    let n = (*index.borrow()).to_string();
                                    root_el
                                        .raw()
                                        .dyn_into::<HtmlElement>()
                                        .unwrap()
                                        .style()
                                        .set_property("order", &n)
                                        .log(&state().log, "Error setting order on list edit entry element");
                                    number_el.ref_text(&format!("{}.", n));
                                }
                            ),
                        );
                    entry_el.number.ref_own(|num_el| (
                        //. .
                        link!((_pc = pc), (enable_numbers = enable_numbers.clone()), (), (num_el = num_el.weak()) {
                            let num_el = num_el.upgrade()?;
                            num_el.ref_modify_classes(
                                &[(&style_export::class_state_hide().value, !*enable_numbers.borrow())],
                            );
                        }),
                        link!(
                            (_pc = pc),
                            (initial = initial_index.clone(), index = index.clone()),
                            (),
                            (num_el = num_el.weak()) {
                                let num_el = num_el.upgrade()?;
                                num_el.ref_modify_classes(
                                    &[
                                        (
                                            &style_export::class_state_modified().value,
                                            *initial.borrow() != *index.borrow(),
                                        ),
                                    ],
                                );
                            }
                        ),
                    ));
                    entry_el
                        .delete_button
                        .ref_own(
                            |delete_el| link!(
                                (_pc = pc),
                                (delete = delete.clone()),
                                (),
                                (delete_el = delete_el.weak()) {
                                    let delete_el = delete_el.upgrade()?;
                                    delete_el.ref_modify_classes(
                                        &[(&style_export::class_state_modified().value, *delete.borrow())],
                                    );
                                }
                            ),
                        );
                    entry_el.delete_button.ref_on("click", {
                        let delete = delete.clone();
                        let eg = pc.eg();
                        let renumber = renumber.clone();
                        move |_| eg.event(|pc| {
                            let old_delete = *delete.borrow();
                            delete.set(pc, !old_delete);
                            renumber(pc);
                        }).unwrap()
                    });
                    let initial_deleted = Prim::new(false);
                    entry_el
                        .root
                        .ref_own(
                            |root| link!(
                                (_pc = pc),
                                (initial_deleted = initial_deleted.clone()),
                                (),
                                (root = root.weak()) {
                                    let root = root.upgrade()?;
                                    root.ref_modify_classes(
                                        &[(&style_export::class_state_hide().value, *initial_deleted.borrow())],
                                    );
                                }
                            ),
                        );
                    el_out.push(entry_el.root);
                    state_out.push(EntryState {
                        node: entry.node,
                        index: index,
                        initial_index: initial_index,
                        delete: delete,
                        initial_delete: initial_deleted,
                        checkbox: entry_el.checkbox,
                    });
                }
                *states.borrow_mut() = state_out;
                let out = style_export::cont_page_list_edit(style_export::ContPageListEditArgs {
                    back_to_view_link: ministate_octothorpe(&Ministate::NodeView(MinistateNodeView {
                        title: title.clone(),
                        node: node.clone(),
                    })),
                    children: el_out,
                });
                let selected_indices = {
                    let states = states.clone();
                    move || {
                        let mut indices = vec![];
                        for (i, e) in states.borrow().iter().enumerate() {
                            if e.checkbox.raw().dyn_into::<HtmlInputElement>().unwrap().checked() {
                                indices.push(i);
                            }
                        }
                        return indices;
                    }
                };

                // Toggle delete
                out.button_delete.ref_on("click", {
                    let selected_indices = selected_indices.clone();
                    let eg = pc.eg();
                    let states = states.clone();
                    let renumber = renumber.clone();
                    move |_| eg.event(|pc| {
                        let selected = selected_indices();
                        let delete = {
                            let s = states.borrow();
                            !selected.iter().map(|x| &(*s)[*x]).all(|x| *x.delete.borrow())
                        };
                        for index in selected {
                            states.borrow_mut()[index].delete.set(pc, delete);
                        }
                        renumber(pc);
                    }).unwrap()
                });

                // Deselect
                out.button_deselect.ref_on("click", {
                    let selected_indices = selected_indices.clone();
                    let eg = pc.eg();
                    let states = states.clone();
                    move |_| eg.event(|_pc| {
                        let selected = selected_indices();
                        for index in selected {
                            states.borrow_mut()[index]
                                .checkbox
                                .raw()
                                .dyn_into::<HtmlInputElement>()
                                .unwrap()
                                .set_checked(false);
                        }
                    }).unwrap()
                });

                // Move up/down
                let remove_selected = {
                    let states = states.clone();
                    move || {
                        let mut removed = vec![];
                        let states_len = states.borrow().len();
                        for i in (0 .. states_len).rev() {
                            let mut states = states.borrow_mut();
                            let checked =
                                (*states)[i].checkbox.raw().dyn_into::<HtmlInputElement>().unwrap().checked();
                            if checked {
                                removed.push((i, states.remove(i)));
                            }
                        }
                        removed.reverse();
                        return removed;
                    }
                };
                out.button_move_down.ref_own({
                    |self0| link!((_pc = pc), (enable_numbers = enable_numbers.clone()), (), (self0 = self0.weak()) {
                        let self0 = self0.upgrade()?;
                        self0.ref_modify_classes(
                            &[(&style_export::class_state_hide().value, !*enable_numbers.borrow())],
                        );
                    })
                });
                out.button_move_down.ref_on("click", {
                    let remove_selected = remove_selected.clone();
                    let eg = pc.eg();
                    let states = states.clone();
                    let renumber = renumber.clone();
                    move |_| eg.event(|pc| {
                        let removed = remove_selected();
                        let Some(last_orig_index) = removed.last().map(|x| x.0) else {
                            return;
                        };
                        let place_index = (last_orig_index - (removed.len() - 1) + 1).min(states.borrow().len());
                        states
                            .borrow_mut()
                            .splice(
                                place_index .. place_index,
                                removed.into_iter().map(|x| x.1).collect::<Vec<_>>(),
                            );
                        renumber(pc);
                    }).unwrap()
                });
                out.button_move_up.ref_own({
                    |self0| link!((_pc = pc), (enable_numbers = enable_numbers.clone()), (), (self0 = self0.weak()) {
                        let self0 = self0.upgrade()?;
                        self0.ref_modify_classes(
                            &[(&style_export::class_state_hide().value, !*enable_numbers.borrow())],
                        );
                    })
                });
                out.button_move_up.ref_on("click", {
                    let remove_selected = remove_selected.clone();
                    let eg = pc.eg();
                    let states = states.clone();
                    let renumber = renumber.clone();
                    move |_| eg.event(|pc| {
                        let removed = remove_selected();
                        let Some(first_orig_index) = removed.first().map(|x| x.0) else {
                            return;
                        };
                        let place_index = if first_orig_index > 0 {
                            first_orig_index - 1
                        } else {
                            0
                        };
                        states
                            .borrow_mut()
                            .splice(
                                place_index .. place_index,
                                removed.into_iter().map(|x| x.1).collect::<Vec<_>>(),
                            );
                        renumber(pc);
                    }).unwrap()
                });

                // Number toggle
                if *enable_numbers.borrow() {
                    out.numbered_toggle.raw().dyn_into::<HtmlInputElement>().unwrap().set_checked(true);
                }
                out
                    .numbered_outer
                    .ref_own(
                        |t| link!(
                            (_pc = pc),
                            (initial_numbered = initial_enable_numbers.clone(), numbered = enable_numbers.clone()),
                            (),
                            (t = t.weak()) {
                                let t = t.upgrade()?;
                                t.ref_modify_classes(&[
                                    //. .
                                    (&style_export::class_state_modified().value, *initial_numbered.borrow() != *numbered.borrow()),
                                ]);
                            }
                        ),
                    );
                out.numbered_toggle.ref_on("input", {
                    let enable_numbers = enable_numbers.clone();
                    let eg = pc.eg();
                    let renumber = renumber.clone();
                    move |ev| eg.event(|pc| {
                        enable_numbers.set(
                            pc,
                            ev
                                .dyn_ref::<Event>()
                                .unwrap()
                                .target()
                                .unwrap()
                                .dyn_into::<HtmlInputElement>()
                                .unwrap()
                                .checked(),
                        );
                        renumber(pc);
                    }).unwrap()
                });

                // Commit
                let commit_thinking = Prim::new(false);
                out
                    .button_commit
                    .ref_own(|b| link!((_pc = pc), (thinking = commit_thinking.clone()), (), (b = b.weak()) {
                        let b = b.upgrade()?;
                        b.ref_modify_classes(&[(&style_export::class_state_thinking().value, *thinking.borrow())]);
                    }));
                out.button_commit.ref_on("click", {
                    let thinking = commit_thinking.clone();
                    let states = states.clone();
                    let eg = pc.eg();
                    let initial_enable_numbers = initial_enable_numbers.clone();
                    let enable_numbers = enable_numbers.clone();
                    let title = title.clone();
                    move |_| eg.event(|pc| {
                        if *thinking.borrow() {
                            return;
                        }
                        thinking.set(pc, true);
                        spawn_local({
                            let states = states.clone();
                            let eg = pc.eg();
                            let initial_enable_numbers = initial_enable_numbers.clone();
                            let enable_numbers = enable_numbers.clone();
                            let thinking = thinking.clone();
                            let title = title.clone();
                            let selected_indices = selected_indices.clone();
                            async move {
                                let res = async {
                                    ta_return!((), String);
                                    let mut add = vec![];
                                    let mut remove = vec![];
                                    let mut delete_nodes = vec![];
                                    for el in states.borrow().iter() {
                                        if *el.delete.borrow() && !*el.initial_delete.borrow() {
                                            delete_nodes.push(el.node.clone());
                                        } else {
                                            let new_index = *el.index.borrow();
                                            let old_index = *el.initial_index.borrow();
                                            if new_index != old_index ||
                                                (!*enable_numbers.borrow() && *initial_enable_numbers.borrow()) {
                                                remove.push(Triple {
                                                    subject: el.node.clone(),
                                                    predicate: PREDICATE_INDEX.to_string(),
                                                    object: Node::Value(
                                                        serde_json::Value::Number(
                                                            serde_json::Number::from_f64(old_index).unwrap(),
                                                        ),
                                                    ),
                                                });
                                            }
                                            if new_index != old_index ||
                                                (*enable_numbers.borrow() && !*initial_enable_numbers.borrow()) {
                                                add.push(Triple {
                                                    subject: el.node.clone(),
                                                    predicate: PREDICATE_INDEX.to_string(),
                                                    object: Node::Value(
                                                        serde_json::Value::Number(
                                                            serde_json::Number::from_f64(new_index).unwrap(),
                                                        ),
                                                    ),
                                                });
                                            }
                                        }
                                    }
                                    remove.extend(
                                        req_post_json(
                                            &state().env.base_url,
                                            ReqGetTriplesAround { nodes: delete_nodes },
                                        ).await?,
                                    );
                                    req_post_json(&state().env.base_url, ReqCommit {
                                        comment: format!("Editing list {}", title),
                                        add: add,
                                        remove: remove,
                                        files: vec![],
                                    }).await?;
                                    eg.event(|pc| {
                                        initial_enable_numbers.set(pc, *enable_numbers.borrow());
                                        for state in states.borrow().iter() {
                                            state.initial_index.set(pc, *state.index.borrow());
                                            state.initial_delete.set(pc, *state.delete.borrow());
                                        }
                                        let selected = selected_indices();
                                        for index in selected {
                                            states.borrow_mut()[index]
                                                .checkbox
                                                .raw()
                                                .dyn_into::<HtmlInputElement>()
                                                .unwrap()
                                                .set_checked(false);
                                        }
                                    }).unwrap();
                                    return Ok(());
                                }.await;
                                eg.event(|pc| {
                                    thinking.set(pc, false);
                                }).unwrap();
                                match res {
                                    Ok(_) => { },
                                    Err(e) => {
                                        state().log.log(&format!("Committing list changes failed: {}", e));
                                    },
                                }
                            }
                        });
                    }).unwrap()
                });

                // Out
                return Ok(vec![out.root]);
            }).unwrap();
        }
    }));
}
