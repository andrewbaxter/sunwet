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
            MinistateHistory,
            MinistateHistoryFilter,
            MinistateNodeEdit,
        },
        playlist::{
            categorize_mime_media,
            PlaylistEntryMediaType,
        },
        state::state,
    },
    flowcontrol::ta_return,
    lunk::ProcessingContext,
    rooting::El,
    shared::interface::{
        ont::PREDICATE_NAME,
        triple::Node,
        wire::{
            ReqGetNodeMeta,
            ReqGetTriplesAround,
        },
    },
    wasm::{
        js::{
            el_async,
            el_async_,
            style_export,
        },
        world::file_url,
    },
};

pub fn node_to_text(node: &Node) -> String {
    match node {
        Node::File(node) => return node.to_string(),
        Node::Value(node) => match node {
            serde_json::Value::String(v) => return v.clone(),
            node => return serde_json::to_string(node).unwrap(),
        },
    };
}

pub fn build_node_media_el(node: &Node) -> Option<El> {
    let Node::File(h) = node else {
        return None;
    };
    let h = h.clone();
    let src_url = file_url(&state().env, &h);
    return Some(el_async(async move {
        ta_return!(Vec < El >, String);
        let meta = req_post_json(&state().env.base_url, ReqGetNodeMeta { node: Node::File(h.clone()) }).await?;
        match meta {
            Some(meta) => {
                match categorize_mime_media(meta.mime.as_ref().map(|x| x.as_str()).unwrap_or("")) {
                    Some(PlaylistEntryMediaType::Audio) => {
                        return Ok(
                            vec![
                                style_export::leaf_media_audio(style_export::LeafMediaAudioArgs { src: src_url }).root
                            ],
                        );
                    },
                    Some(PlaylistEntryMediaType::Video) => {
                        return Ok(
                            vec![
                                style_export::leaf_media_video(style_export::LeafMediaVideoArgs { src: src_url }).root
                            ],
                        );
                    },
                    Some(PlaylistEntryMediaType::Image) => {
                        return Ok(
                            vec![style_export::leaf_media_img(style_export::LeafMediaImgArgs { src: src_url }).root],
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

pub fn build_node_el(node: &Node, link: bool) -> El {
    let text = node_to_text(node);
    return style_export::leaf_node_view_node_text(style_export::LeafNodeViewNodeTextArgs {
        value: text.clone(),
        link: if link {
            Some(ministate_octothorpe(&super::ministate::Ministate::NodeView(MinistateNodeView {
                title: text,
                node: node.clone(),
            })))
        } else {
            None
        },
    }).root;
}

pub fn build_page_node_view(pc: &mut ProcessingContext, title: &str, node: &Node) {
    set_page(pc, title, el_async_(true, {
        let eg = pc.eg();
        let title = title.to_string();
        let node = node.clone();
        async move {
            ta_return!(Vec < El >, String);
            let triples = req_post_json(&state().env.base_url, ReqGetTriplesAround { node: node.clone() }).await?;
            return eg.event(|_pc| {
                let mut out = vec![];

                // Incoming triples
                {
                    let mut triples_els = vec![];
                    for t in triples.incoming {
                        let mut triple_els = vec![];
                        triple_els.push(build_node_el(&t.subject, true));
                        triple_els.push(
                            style_export::leaf_node_view_predicate(
                                style_export::LeafNodeViewPredicateArgs { value: t.predicate.clone() },
                            ).root,
                        );
                        if let Some(ele) = build_node_media_el(&t.subject) {
                            triple_els.push(ele);
                        };
                        triple_els.push(
                            style_export::leaf_node_view_node_buttons(style_export::LeafNodeViewNodeButtonsArgs {
                                edit: None,
                                download: match &t.subject {
                                    Node::File(n) => Some(file_url(&state().env, n)),
                                    _ => None,
                                },
                                history: Some(
                                    ministate_octothorpe(
                                        &Ministate::History(MinistateHistory { filter: Some(MinistateHistoryFilter {
                                            predicate: Some(
                                                crate::libnonlink::ministate::MinistateHistoryPredicate::Incoming(
                                                    t.predicate.clone(),
                                                ),
                                            ),
                                            node: node.clone(),
                                        }) }),
                                    ),
                                ),
                            }).root,
                        );
                        triples_els.push(style_export::cont_node_row_incoming(style_export::ContNodeRowIncomingArgs {
                            children: triple_els,
                            new: false,
                        }).root);
                    }
                    out.push(
                        style_export::cont_page_node_section_rel(
                            style_export::ContPageNodeSectionRelArgs { children: triples_els },
                        ).root,
                    );
                }

                // Pivot
                {
                    let mut children = vec![
                        //. .
                        build_node_el(&node, false),
                        style_export::leaf_node_view_node_buttons(style_export::LeafNodeViewNodeButtonsArgs {
                            download: None,
                            edit: Some(ministate_octothorpe(&Ministate::NodeEdit(MinistateNodeEdit {
                                title: title.clone(),
                                node: node.clone(),
                            }))),
                            history: Some(
                                ministate_octothorpe(
                                    &Ministate::History(MinistateHistory { filter: Some(MinistateHistoryFilter {
                                        node: node.clone(),
                                        predicate: None,
                                    }) }),
                                ),
                            ),
                        }).root,
                    ];
                    if let Some(ele) = build_node_media_el(&node) {
                        children.push(ele);
                    };
                    out.push(
                        style_export::cont_node_section_center(
                            style_export::ContNodeSectionCenterArgs { children: children },
                        ).root,
                    );
                }

                // Outgoing triples
                {
                    let mut triples_els = vec![];
                    for t in triples.outgoing {
                        if t.predicate == PREDICATE_NAME {
                            let name = node_to_text(&t.object);
                            state().main_title.ref_text(&name);
                            record_replace_ministate(&Ministate::NodeView(MinistateNodeView {
                                title: name,
                                node: t.subject.clone(),
                            }));
                        }
                        let mut triple_els = vec![];
                        triple_els.push(
                            style_export::leaf_node_view_predicate(
                                style_export::LeafNodeViewPredicateArgs { value: t.predicate.clone() },
                            ).root,
                        );
                        triple_els.push(build_node_el(&t.object, true));
                        if let Some(ele) = build_node_media_el(&t.object) {
                            triple_els.push(ele);
                        }
                        triple_els.push({
                            style_export::leaf_node_view_node_buttons(style_export::LeafNodeViewNodeButtonsArgs {
                                download: match &t.object {
                                    Node::File(n) => Some(file_url(&state().env, n)),
                                    _ => None,
                                },
                                edit: None,
                                history: Some(
                                    ministate_octothorpe(
                                        &Ministate::History(MinistateHistory { filter: Some(MinistateHistoryFilter {
                                            node: node.clone(),
                                            predicate: Some(
                                                crate::libnonlink::ministate::MinistateHistoryPredicate::Outgoing(
                                                    t.predicate.clone(),
                                                ),
                                            ),
                                        }) }),
                                    ),
                                ),
                            }).root
                        });
                        triples_els.push(style_export::cont_node_row_outgoing(style_export::ContNodeRowOutgoingArgs {
                            children: triple_els,
                            new: false,
                        }).root);
                    }
                    out.push(
                        style_export::cont_page_node_section_rel(
                            style_export::ContPageNodeSectionRelArgs { children: triples_els },
                        ).root,
                    );
                }
                return Ok(vec![style_export::cont_page_node(style_export::ContPageNodeArgs {
                    bar_children: vec![],
                    children: out,
                }).root]);
            }).unwrap();
        }
    }));
}
