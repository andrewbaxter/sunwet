use {
    super::{
        api::req_post_json,
        ministate::MinistateNodeView,
        state::set_page,
    },
    crate::libnonlink::{
        ministate::{
            ministate_octothorpe,
            record_replace_ministate,
            Ministate,
            MinistateNodeEdit,
        },
        state::state,
    },
    flowcontrol::ta_return,
    lunk::ProcessingContext,
    rooting::{
        el_from_raw,
        El,
    },
    shared::interface::{
        triple::Node,
        wire::ReqGetTriplesAround,
    },
    wasm::{
        js::{
            el_async_,
            style_export,
        },
        ont,
    },
    wasm_bindgen::JsCast,
};

fn node_to_text(node: &Node) -> String {
    match node {
        Node::File(node) => return node.to_string(),
        Node::Value(node) => match node {
            serde_json::Value::String(v) => return v.clone(),
            node => return serde_json::to_string(node).unwrap(),
        },
    };
}

fn build_node_el(node: &Node, link: bool) -> El {
    let text = node_to_text(node);
    return el_from_raw(style_export::leaf_node_view_node_text(style_export::LeafNodeViewNodeTextArgs {
        value: text.clone(),
        link: if link {
            Some(ministate_octothorpe(&super::ministate::Ministate::NodeView(MinistateNodeView {
                title: text,
                node: node.clone(),
            })))
        } else {
            None
        },
    }).root.into());
}

pub fn build_page_node_view(pc: &mut ProcessingContext, title: &str, node: &Node) {
    set_page(pc, title, el_async_(true, {
        let eg = pc.eg();
        let title = title.to_string();
        let node = node.clone();
        async move {
            ta_return!(El, String);
            let triples = req_post_json(&state().env.base_url, ReqGetTriplesAround { node: node.clone() }).await?;
            return eg.event(|_pc| {
                let mut out = vec![];

                // Top buttons
                let mut buttons_out = vec![];
                {
                    let style_res =
                        style_export::leaf_button_small_edit(
                            style_export::LeafButtonSmallEditArgs {
                                link: ministate_octothorpe(&Ministate::NodeEdit(MinistateNodeEdit {
                                    title: title.clone(),
                                    node: node.clone(),
                                })),
                            },
                        );
                    buttons_out.push(el_from_raw(style_res.root.into()));
                }

                // Incoming triples
                {
                    let mut triples_els = vec![];
                    for t in triples.incoming {
                        let triple_els =
                            vec![
                                build_node_el(&t.subject, true),
                                el_from_raw(
                                    style_export::leaf_node_view_predicate(
                                        style_export::LeafNodeViewPredicateArgs { value: t.predicate.clone() },
                                    )
                                        .root
                                        .into(),
                                ),
                            ];
                        triples_els.push(
                            el_from_raw(style_export::cont_node_row_incoming(style_export::ContNodeRowIncomingArgs {
                                children: triple_els.iter().map(|x| x.raw().dyn_into().unwrap()).collect(),
                                new: false,
                            }).root.into()).own(|_| triple_els),
                        );
                    }
                    out.push(
                        el_from_raw(
                            style_export::cont_page_node_section_rel(
                                style_export::ContPageNodeSectionRelArgs {
                                    children: triples_els.iter().map(|x| x.raw()).collect(),
                                },
                            )
                                .root
                                .into(),
                        ).own(|_| triples_els),
                    );
                }

                // Pivot
                {
                    let children = [build_node_el(&node, false)];
                    out.push(
                        el_from_raw(
                            style_export::cont_node_section_center(
                                style_export::ContNodeSectionCenterArgs {
                                    children: children.iter().map(|x| x.raw().dyn_into().unwrap()).collect(),
                                },
                            )
                                .root
                                .into(),
                        ).own(|_| (children)),
                    );
                }

                // Outgoing triples
                {
                    let mut triples_els = vec![];
                    for t in triples.outgoing {
                        if t.predicate == ont::PREDICATE_NAME {
                            let name = node_to_text(&t.object);
                            state().main_title.ref_text(&name);
                            record_replace_ministate(&Ministate::NodeView(MinistateNodeView {
                                title: name,
                                node: t.subject.clone(),
                            }));
                        }
                        let triple_els =
                            vec![
                                el_from_raw(
                                    style_export::leaf_node_view_predicate(
                                        style_export::LeafNodeViewPredicateArgs { value: t.predicate.clone() },
                                    )
                                        .root
                                        .into(),
                                ),
                                build_node_el(&t.object, true),
                            ];
                        triples_els.push(
                            el_from_raw(style_export::cont_node_row_outgoing(style_export::ContNodeRowOutgoingArgs {
                                children: triple_els.iter().map(|x| x.raw().dyn_into().unwrap()).collect(),
                                new: false,
                            }).root.into()).own(|_| triple_els),
                        );
                    }
                    out.push(
                        el_from_raw(
                            style_export::cont_page_node_section_rel(
                                style_export::ContPageNodeSectionRelArgs {
                                    children: triples_els.iter().map(|x| x.raw()).collect(),
                                },
                            )
                                .root
                                .into(),
                        ).own(|_| triples_els),
                    );
                }
                return Ok(el_from_raw(style_export::cont_page_node_view(style_export::ContPageNodeViewArgs {
                    page_button_children: buttons_out.iter().map(|x| x.raw().dyn_into().unwrap()).collect(),
                    children: out.iter().map(|x| x.raw().dyn_into().unwrap()).collect(),
                }).root.into()).own(|_| (out, buttons_out)));
            }).unwrap();
        }
    }));
}
