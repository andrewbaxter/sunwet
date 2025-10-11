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
            MinistateListEdit,
            MinistateNodeEdit,
        },
        node_button::setup_node_button,
        playlist::{
            categorize_mime_media,
            PlaylistEntryMediaType,
        },
        state::state,
    },
    flowcontrol::ta_return,
    lunk::ProcessingContext,
    rooting::El,
    shared::{
        interface::{
            ont::PREDICATE_NAME,
            triple::{
                FileHash,
                Node,
            },
            wire::{
                ReqGetNodeMeta,
                ReqGetTriplesAround,
            },
        },
        stringpattern::node_to_text,
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

pub fn build_node_el(node: &Node) -> El {
    let text = node_to_text(node);
    return style_export::leaf_node_view_node_text(style_export::LeafNodeViewNodeTextArgs {
        value: text.clone(),
        link: None,
    }).root;
}

fn build_node_rel_buttons(
    pc: &mut ProcessingContext,
    node: &Node,
    download: Option<&FileHash>,
    history: String,
) -> El {
    let mut right = vec![];
    if let Some(download) = download {
        right.push(
            style_export::leaf_node_view_toolbar_download_link_button(
                style_export::LeafNodeViewToolbarDownloadLinkButtonArgs { link: file_url(&state().env, download) },
            ).root,
        );
    }
    right.push(
        style_export::leaf_node_view_toolbar_history_link_button(
            style_export::LeafNodeViewToolbarHistoryLinkButtonArgs { link: history },
        ).root,
    );
    let node_button = style_export::leaf_node_view_toolbar_node_button().root;
    setup_node_button(pc, &node_button, node_to_text(&node), node.clone());
    right.push(node_button);
    return style_export::cont_node_toolbar(style_export::ContNodeToolbarArgs {
        left: vec![],
        right: right,
    }).root;
}

pub fn build_page_node_view(pc: &mut ProcessingContext, title: &str, node: &Node) {
    set_page(pc, title, el_async_(true, {
        let eg = pc.eg();
        let title = title.to_string();
        let node = node.clone();
        async move {
            ta_return!(Vec < El >, String);
            let mut triples =
                req_post_json(&state().env.base_url, ReqGetTriplesAround { nodes: vec![node.clone()] }).await?;
            return eg.event(|pc| {
                let mut out = vec![];

                // Incoming triples
                {
                    let mut triples_els = vec![];
                    let mut incoming_rels = triples.extract_if(.., |x| x.object == node).collect::<Vec<_>>();
                    incoming_rels.sort_by_cached_key(|r| (r.predicate.clone(), r.subject.clone()));
                    for t in incoming_rels {
                        let mut triple_els = vec![];
                        triple_els.push(build_node_el(&t.subject));
                        triple_els.push(
                            style_export::leaf_node_view_predicate(
                                style_export::LeafNodeViewPredicateArgs { value: t.predicate.clone() },
                            ).root,
                        );
                        if let Some(ele) = build_node_media_el(&t.subject) {
                            triple_els.push(ele);
                        };
                        triple_els.push(build_node_rel_buttons(
                            pc,
                            &t.subject,
                            match &t.subject {
                                Node::File(n) => Some(n),
                                _ => None,
                            },
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
                        ));
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
                    let node_button = style_export::leaf_node_view_toolbar_node_button().root;
                    setup_node_button(pc, &node_button, title.clone(), node.clone());
                    let mut children = vec![
                        //. .
                        build_node_el(&node),
                        style_export::cont_node_toolbar(style_export::ContNodeToolbarArgs {
                            left: vec![],
                            right: vec![
                                style_export::leaf_node_view_toolbar_history_link_button(
                                    style_export::LeafNodeViewToolbarHistoryLinkButtonArgs {
                                        link: ministate_octothorpe(
                                            &Ministate::History(
                                                MinistateHistory { filter: Some(MinistateHistoryFilter {
                                                    node: node.clone(),
                                                    predicate: None,
                                                }) },
                                            ),
                                        ),
                                    },
                                ).root,
                                style_export::leaf_node_view_toolbar_edit_link_button(
                                    style_export::LeafNodeViewToolbarEditLinkButtonArgs {
                                        link: ministate_octothorpe(&Ministate::NodeEdit(MinistateNodeEdit {
                                            title: title.clone(),
                                            nodes: vec![node.clone()],
                                        })),
                                    },
                                ).root,
                                style_export::leaf_node_view_toolbar_edit_list_link_button(
                                    style_export::LeafNodeViewToolbarEditListLinkButtonArgs {
                                        link: ministate_octothorpe(&Ministate::ListEdit(MinistateListEdit {
                                            title: title.clone(),
                                            node: node.clone(),
                                        })),
                                    },
                                ).root,
                                node_button,
                            ],
                        }).root
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
                    triples.sort_by_cached_key(|r| (r.predicate.clone(), r.object.clone()));
                    for t in triples {
                        if t.predicate == PREDICATE_NAME {
                            let name = node_to_text(&t.object);
                            state().main_title.ref_text(&name);
                            record_replace_ministate(&state().log, &Ministate::NodeView(MinistateNodeView {
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
                        triple_els.push(build_node_el(&t.object));
                        if let Some(ele) = build_node_media_el(&t.object) {
                            triple_els.push(ele);
                        }
                        triple_els.push({
                            build_node_rel_buttons(
                                pc,
                                &t.object,
                                match &t.object {
                                    Node::File(n) => Some(n),
                                    _ => None,
                                },
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
                            )
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
                return Ok(
                    vec![
                        style_export::cont_page_node_view(style_export::ContPageNodeViewArgs { children: out }).root
                    ],
                );
            }).unwrap();
        }
    }));
}
