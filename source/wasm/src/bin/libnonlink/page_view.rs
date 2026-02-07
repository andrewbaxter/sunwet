use {
    super::{
        infinite::build_infinite,
        ministate::{
            MinistateNodeView,
            PlaylistRestorePos,
        },
        playlist::playlist_clear,
        state::{
            MinistateViewState,
            MinistateViewState_,
            state,
        },
    },
    crate::libnonlink::{
        api::req_post_json,
        infinite::InfPageRes,
        ministate::{
            Ministate,
            MinistateForm,
            MinistateView,
            ministate_octothorpe,
        },
        node_button::setup_node_button,
        offline::{
            ensure_offline,
            remove_offline,
            retrieve_offline_query,
        },
        playlist::{
            PlaylistPushArg,
            categorize_mime_media,
            playlist_extend,
            playlist_next,
            playlist_previous,
            playlist_set_link,
            playlist_toggle_play,
        },
        seekbar::setup_seekbar,
        state::goto_replace_ministate,
        viewutil::{
            DataStackLevel,
            maybe_get_field,
            maybe_get_field_or_literal,
            maybe_get_field_or_literal_string,
            maybe_get_meta,
            tree_node_to_text,
            unwrap_value_media_url,
        },
    },
    flowcontrol::{
        shed,
        ta_return,
    },
    gloo::{
        storage::{
            LocalStorage,
            Storage,
        },
        timers::callback::Timeout,
    },
    js_sys::Math::random,
    lunk::{
        EventGraph,
        Prim,
        ProcessingContext,
        link,
    },
    qrcode::{
        QrCode,
        render::svg::Color,
    },
    rooting::{
        El,
        WeakEl,
        el,
        el_from_raw,
    },
    shared::interface::{
        config::view::{
            ClientView,
            ClientViewParam,
            Direction,
            Link,
            LinkDest,
            Orientation,
            QueryOrField,
            ViewId,
            Widget,
            WidgetColor,
            WidgetDataRows,
            WidgetDate,
            WidgetDatetime,
            WidgetIcon,
            WidgetLayout,
            WidgetMedia,
            WidgetNode,
            WidgetPlayButton,
            WidgetRootDataRows,
            WidgetText,
            WidgetTime,
        },
        triple::Node,
        wire::{
            NodeMeta,
            Pagination,
            ReqViewQuery,
            RespQueryRows,
            TreeNode,
        },
    },
    std::{
        cell::{
            Cell,
            RefCell,
        },
        collections::{
            BTreeMap,
            HashMap,
        },
        rc::Rc,
        u64,
    },
    uuid::Uuid,
    wasm::{
        constants::LINK_HASH_PREFIX,
        js::{
            LogJsErr,
            el_async,
            env_preferred_audio_url,
            env_preferred_video_url,
            on_thinking,
            style_export::{
                self,
                OrientationType,
            },
        },
        world::file_url,
    },
    wasm_bindgen::JsCast,
    web_sys::{
        DomParser,
        HtmlElement,
    },
};

pub const LOCALSTORAGE_SHARE_SESSION_ID: &str = "share_session_id";

fn unwrap_value_move_url(data_stack: &Vec<Rc<DataStackLevel>>, link: &Link) -> Result<Option<String>, String> {
    let title = match maybe_get_field_or_literal(&link.title, data_stack) {
        Some(x) => tree_node_to_text(&x),
        None => format!("(unknown name)"),
    };
    match &link.dest {
        LinkDest::Plain(d) => {
            let Some(TreeNode::Scalar(data_at)) = maybe_get_field_or_literal(d, data_stack) else {
                return Ok(None);
            };
            match data_at {
                Node::File(data_at) => {
                    return Ok(Some(file_url(&state().env, &data_at)));
                },
                Node::Value(serde_json::Value::String(data_at)) => {
                    return Ok(Some(data_at));
                },
                _ => {
                    return Ok(None);
                },
            }
        },
        LinkDest::View(d) => {
            let mut params = HashMap::new();
            for (k, v) in &d.parameters {
                let Some(TreeNode::Scalar(v)) = maybe_get_field_or_literal(v, data_stack) else {
                    continue;
                };
                params.insert(k.clone(), v);
            }
            return Ok(Some(ministate_octothorpe(&Ministate::View(MinistateView {
                id: d.id.clone(),
                title: title,
                pos: None,
                params: params,
            }))));
        },
        LinkDest::Form(d) => {
            let mut params = HashMap::new();
            for (k, v) in &d.parameters {
                let Some(TreeNode::Scalar(v)) = maybe_get_field_or_literal(v, data_stack) else {
                    continue;
                };
                params.insert(k.clone(), v);
            }
            return Ok(Some(ministate_octothorpe(&Ministate::Form(MinistateForm {
                id: d.id.clone(),
                title: title,
                params: params,
            }))));
        },
        LinkDest::Node(d) => {
            let Some(TreeNode::Scalar(data_at)) = maybe_get_field_or_literal(d, data_stack) else {
                return Ok(None);
            };
            return Ok(Some(ministate_octothorpe(&Ministate::NodeView(MinistateNodeView {
                title: title,
                node: data_at,
            }))));
        },
    }
}

struct Build {
    view_id: ViewId,
    param_data: HashMap<String, Node>,
    restore_playlist_pos: Option<PlaylistRestorePos>,
    playlist_add: Vec<PlaylistPushArg>,
    have_media: Rc<Cell<bool>>,
    want_media: bool,
    transport_slot: El,
    vs: MinistateViewState,
    seed: u64,
    offline: Option<String>,
}

impl Build {
    fn build_widget_layout(
        &mut self,
        pc: &mut ProcessingContext,
        parent_orientation: Orientation,
        parent_orientation_type: OrientationType,
        config_at: &WidgetLayout,
        config_query_params: &BTreeMap<String, Vec<String>>,
        data_id: &Vec<usize>,
        data_at: &Vec<Rc<DataStackLevel>>,
    ) -> El {
        let mut children = vec![];
        for child_config_at in &config_at.elements {
            children.push(
                self.build_widget(
                    pc,
                    config_at.orientation,
                    OrientationType::Flex,
                    child_config_at,
                    config_query_params,
                    data_id,
                    data_at,
                ),
            );
        }
        return style_export::cont_view_list(style_export::ContViewListArgs {
            parent_orientation: parent_orientation,
            parent_orientation_type: parent_orientation_type,
            orientation: config_at.orientation,
            trans_align: config_at.trans_align,
            conv_scroll: config_at.conv_scroll,
            conv_size_max: config_at.conv_size_max.clone(),
            trans_size_max: config_at.trans_size_max.clone(),
            children: children,
            gap: config_at.gap.clone(),
            wrap: config_at.wrap,
        }).root;
    }

    fn build_widget_data_rows(
        &mut self,
        pc: &mut ProcessingContext,
        parent_orientation: Orientation,
        parent_orientation_type: OrientationType,
        config_at: &WidgetDataRows,
        config_query_params: &BTreeMap<String, Vec<String>>,
        data_id: &Vec<usize>,
        data_at: &Vec<Rc<DataStackLevel>>,
    ) -> El {
        return el_async({
            let view_id = self.view_id.clone();
            let param_data = self.param_data.clone();
            let restore_playlist_pos = self.restore_playlist_pos.clone();
            let eg = pc.eg();
            let transport_slot = self.transport_slot.clone();
            let have_media = self.have_media.clone();
            let config_at = config_at.clone();
            let config_query_params = config_query_params.clone();
            let vs = self.vs.clone();
            let data_id = data_id.clone();
            let data_at = data_at.clone();
            let seed = self.seed;
            let offline = self.offline.clone();
            async move {
                let node_meta;
                let new_data_at_tops;
                match config_at.data {
                    QueryOrField::Field(config_at) => {
                        let Some(TreeNode::Array(res)) = maybe_get_field(&config_at, &data_at) else {
                            return Err(
                                format!(
                                    "Data rows field [{}] must be an array, but it is missing or some other type",
                                    config_at
                                ),
                            );
                        };
                        new_data_at_tops = res;
                        node_meta = Default::default();
                    },
                    QueryOrField::Query(query_id) => {
                        let mut params = HashMap::new();
                        if let Some(query_params) = config_query_params.get(&query_id) {
                            for k in query_params {
                                let Some(TreeNode::Scalar(v)) = maybe_get_field(k, &data_at) else {
                                    return Err(
                                        format!(
                                            "Parameters must be scalars, but query paramter [{}] is missing or not a scalar",
                                            k
                                        ),
                                    );
                                };
                                params.insert(k.clone(), v);
                            }
                        }
                        let res = if let Some(key) = &offline {
                            retrieve_offline_query(key, &query_id, &params).await?
                        } else {
                            req_post_json(ReqViewQuery {
                                view_id: view_id.clone(),
                                query: query_id.clone(),
                                parameters: params.clone(),
                                pagination: None,
                            }).await?
                        };
                        let mut out = vec![];
                        match res.rows {
                            RespQueryRows::Scalar(rows) => {
                                for v in rows {
                                    out.push(TreeNode::Scalar(v));
                                }
                            },
                            RespQueryRows::Record(rows) => {
                                for v in rows {
                                    out.push(TreeNode::Record(v));
                                }
                            },
                        }
                        new_data_at_tops = out;
                        node_meta = Rc::new(res.meta.into_iter().collect::<HashMap<_, _>>());
                    },
                };
                return eg.event(move |pc| {
                    let mut build = Build {
                        view_id: view_id.clone(),
                        vs: vs.clone(),
                        param_data: param_data.clone(),
                        restore_playlist_pos: restore_playlist_pos.clone(),
                        playlist_add: Default::default(),
                        have_media: have_media,
                        want_media: false,
                        transport_slot: transport_slot,
                        seed: seed,
                        offline: offline.clone(),
                    };
                    let out;
                    match &config_at.row_widget {
                        shared::interface::config::view::DataRowsLayout::Unaligned(row_widget) => {
                            let orientation = row_widget.orientation.unwrap_or(parent_orientation);
                            let mut children = vec![];
                            for (i, new_data_at_top) in new_data_at_tops.into_iter().enumerate() {
                                let mut data_at = data_at.clone();
                                data_at.push(Rc::new(DataStackLevel {
                                    data: new_data_at_top,
                                    node_meta: node_meta.clone(),
                                }));
                                let mut data_id = data_id.clone();
                                data_id.push(i);
                                children.push(
                                    build.build_widget(
                                        pc,
                                        orientation,
                                        OrientationType::Flex,
                                        &row_widget.widget,
                                        &config_query_params,
                                        &data_id,
                                        &data_at,
                                    ),
                                );
                            }
                            out = style_export::cont_view_list(style_export::ContViewListArgs {
                                parent_orientation: parent_orientation,
                                parent_orientation_type: parent_orientation_type,
                                orientation: orientation,
                                trans_align: config_at.trans_align,
                                conv_scroll: row_widget.conv_scroll,
                                conv_size_max: row_widget.conv_size_max.clone(),
                                trans_size_max: row_widget.trans_size_max.clone(),
                                wrap: row_widget.wrap,
                                children: children,
                                gap: row_widget.gap.clone(),
                            }).root;
                        },
                        shared::interface::config::view::DataRowsLayout::Table(row_widget) => {
                            let mut rows = vec![];
                            for (i, new_data_at_top) in new_data_at_tops.into_iter().enumerate() {
                                let mut data_at = data_at.clone();
                                data_at.push(Rc::new(DataStackLevel {
                                    data: new_data_at_top,
                                    node_meta: node_meta.clone(),
                                }));
                                let mut data_id = data_id.clone();
                                data_id.push(i);
                                let mut columns = vec![];
                                let mut columns_raw = vec![];
                                for config_at in &row_widget.elements {
                                    let column =
                                        build.build_widget(
                                            pc,
                                            row_widget.orientation,
                                            OrientationType::Grid,
                                            config_at,
                                            &config_query_params,
                                            &data_id,
                                            &data_at,
                                        );
                                    columns_raw.push(column.raw());
                                    columns.push(column);
                                }
                                rows.push(columns);
                            }
                            out = style_export::cont_view_table(style_export::ContViewTableArgs {
                                orientation: row_widget.orientation,
                                trans_scroll: row_widget.trans_scroll,
                                conv_size_max: row_widget.conv_size_max.clone(),
                                trans_size_max: row_widget.trans_size_max.clone(),
                                children: rows,
                                gap: row_widget.gap.clone(),
                            }).root;
                        },
                    }
                    playlist_extend(pc, &state().playlist, vs.clone(), build.playlist_add, &restore_playlist_pos);
                    if !build.have_media.get() && build.want_media {
                        build.transport_slot.ref_push(build_transport(pc));
                        build.have_media.set(true);
                    }
                    return Ok(vec![out]);
                }).unwrap();
            }
        });
    }

    fn build_widget_root_data_rows(
        &mut self,
        pc: &mut ProcessingContext,
        config_at: &WidgetRootDataRows,
        config_query_params: &BTreeMap<String, Vec<String>>,
        data_at: &Vec<Rc<DataStackLevel>>,
    ) -> El {
        let build_infinite_page = {
            let view_id = self.view_id.clone();
            let vs = self.vs.clone();
            let param_data = self.param_data.clone();
            let restore_playlist_pos = self.restore_playlist_pos.clone();
            let eg = pc.eg();
            let transport_slot = self.transport_slot.clone();
            let have_media = self.have_media.clone();
            let config_at = config_at.clone();
            let config_query_params = config_query_params.clone();
            let data_at = data_at.clone();
            let seed = self.seed;
            let offline = self.offline.clone();
            move |chunk: Vec<(usize, TreeNode)>, node_meta: Rc<HashMap<Node, NodeMeta>>| -> Vec<El> {
                return eg.event(|pc| {
                    let mut build = Build {
                        view_id: view_id.clone(),
                        param_data: param_data.clone(),
                        restore_playlist_pos: restore_playlist_pos.clone(),
                        playlist_add: Default::default(),
                        want_media: false,
                        have_media: have_media.clone(),
                        transport_slot: transport_slot.clone(),
                        vs: vs.clone(),
                        seed: seed,
                        offline: offline.clone(),
                    };
                    let mut children = vec![];
                    for (i, new_data_at_top) in chunk {
                        let mut data_at = data_at.clone();
                        data_at.push(Rc::new(DataStackLevel {
                            data: new_data_at_top,
                            node_meta: node_meta.clone(),
                        }));
                        children.push(style_export::cont_view_element(style_export::ContViewElementArgs {
                            body: build.build_widget(
                                pc,
                                Orientation::RightDown,
                                OrientationType::Flex,
                                &config_at.element_body,
                                &config_query_params,
                                &vec![i],
                                &data_at,
                            ),
                            height: config_at.element_height.clone(),
                            expand: match &config_at.element_expansion {
                                None => None,
                                Some(exp) => Some(
                                    build.build_widget(
                                        pc,
                                        Orientation::DownRight,
                                        OrientationType::Grid,
                                        &exp,
                                        &config_query_params,
                                        &vec![i],
                                        &data_at,
                                    ),
                                ),
                            },
                        }).root);
                    }
                    playlist_extend(pc, &state().playlist, vs.clone(), build.playlist_add, &restore_playlist_pos);
                    if !build.have_media.get() && build.want_media {
                        build.transport_slot.ref_push(build_transport(pc));
                        build.have_media.set(true);
                    }
                    return children;
                }).unwrap();
            }
        };
        let restore = self.restore_playlist_pos.as_ref().and_then(|x| x.index.first().copied());
        return el_async({
            let config_at = config_at.clone();
            let view_id = self.view_id.clone();
            let config_query_params = config_query_params.clone();
            let data_at = data_at.clone();
            let seed = self.seed;
            let offline = self.offline.clone();
            async move {
                let body = style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root;
                match config_at.data {
                    QueryOrField::Field(data_field) => {
                        let Some(TreeNode::Array(res)) = maybe_get_field(&data_field, &data_at) else {
                            return Err(
                                format!(
                                    "Data rows field [{}] must be an array, but it is missing or not an array",
                                    data_field
                                ),
                            );
                        };
                        let new_data_at_tops = res;
                        let mut chunked_data = vec![];
                        let mut chunk_top = vec![];
                        for e in new_data_at_tops.into_iter().enumerate() {
                            chunk_top.push(e);
                            if chunk_top.len() > 20 {
                                chunked_data.push(chunk_top.split_off(0));
                            }
                        }
                        if !chunk_top.is_empty() {
                            chunked_data.push(chunk_top);
                        }
                        if chunked_data.is_empty() {
                            chunked_data.push(Default::default());
                        }
                        let mut chunked_data = chunked_data.into_iter();
                        body.ref_push(build_infinite(&state().log, chunked_data.next().unwrap(), {
                            let build_infinite_page = build_infinite_page.clone();
                            move |chunk| {
                                let immediate_advance =
                                    Option::zip(chunk.last(), restore)
                                        .map(|(last, restore)| restore > last.0)
                                        .unwrap_or(false);
                                let children = build_infinite_page(chunk, Default::default());
                                let next_key = chunked_data.next();
                                async move {
                                    Ok(InfPageRes {
                                        next_key: next_key,
                                        page_els: children,
                                        immediate_advance: immediate_advance,
                                    })
                                }
                            }
                        }));
                    },
                    QueryOrField::Query(query_id) => {
                        let mut params = HashMap::new();
                        if let Some(query_params) = config_query_params.get(&query_id) {
                            for k in query_params {
                                let Some(TreeNode::Scalar(v)) = maybe_get_field(k, &data_at) else {
                                    return Err(
                                        format!(
                                            "Parameters must be scalars, but query paramter [{}] is missing or not a scalar",
                                            k
                                        ),
                                    );
                                };
                                params.insert(k.clone(), v);
                            }
                        }
                        if let Some(key) = &offline {
                            let res = retrieve_offline_query(key, &query_id, &params).await?;
                            let mut rows = vec![];
                            let mut count = 0;
                            match res.rows {
                                RespQueryRows::Scalar(rows1) => {
                                    for v in rows1 {
                                        rows.push((count, TreeNode::Scalar(v)));
                                        count += 1;
                                    }
                                },
                                RespQueryRows::Record(rows1) => {
                                    for v in rows1 {
                                        rows.push((count, TreeNode::Record(v)));
                                        count += 1;
                                    }
                                },
                            }
                            body.ref_push(
                                style_export::cont_group(
                                    style_export::ContGroupArgs {
                                        children: build_infinite_page(
                                            rows,
                                            Rc::new(res.meta.into_iter().collect::<HashMap<_, _>>()),
                                        ),
                                    },
                                ).root,
                            );
                        } else {
                            body.ref_push(build_infinite(&state().log, None, {
                                let view_id = view_id.clone();
                                let query_id = query_id.clone();
                                let count = Rc::new(Cell::new(0usize));
                                move |key| {
                                    let view_id = view_id.clone();
                                    let query_id = query_id.clone();
                                    let params = params.clone();
                                    let build_infinite_page = build_infinite_page.clone();
                                    let count = count.clone();
                                    async move {
                                        let res = req_post_json(ReqViewQuery {
                                            view_id: view_id.clone(),
                                            query: query_id.clone(),
                                            parameters: params.clone(),
                                            pagination: Some(Pagination {
                                                count: 10,
                                                seed: Some(seed),
                                                key: key.clone(),
                                            }),
                                        }).await?;
                                        let mut chunk = vec![];
                                        match res.rows {
                                            RespQueryRows::Scalar(rows) => {
                                                for v in rows {
                                                    chunk.push((count.get(), TreeNode::Scalar(v)));
                                                    count.set(count.get() + 1);
                                                }
                                            },
                                            RespQueryRows::Record(rows) => {
                                                for v in rows {
                                                    chunk.push((count.get(), TreeNode::Record(v)));
                                                    count.set(count.get() + 1);
                                                }
                                            },
                                        }
                                        Ok(InfPageRes {
                                            immediate_advance: restore
                                                .as_ref()
                                                .map(|restore| *restore >= count.get())
                                                .unwrap_or(false),
                                            next_key: res.next_page_key.map(|x| Some(x)),
                                            page_els: build_infinite_page(
                                                chunk,
                                                Rc::new(res.meta.into_iter().collect::<HashMap<_, _>>()),
                                            ),
                                        })
                                    }
                                }
                            }));
                        }
                    },
                }
                return Ok(vec![body]);
            }
        });
    }

    fn build_widget_text(
        &mut self,
        parent_orientation: Orientation,
        parent_orientation_type: OrientationType,
        config_at: &WidgetText,
        data_stack: &Vec<Rc<DataStackLevel>>,
    ) -> El {
        match (|| {
            ta_return!(El, String);
            return Ok(style_export::leaf_view_text(style_export::LeafViewTextArgs {
                parent_orientation: parent_orientation,
                parent_orientation_type: parent_orientation_type,
                trans_align: config_at.trans_align,
                orientation: config_at.orientation,
                text: format!(
                    "{}{}{}",
                    config_at.prefix,
                    tree_node_to_text(&match maybe_get_field_or_literal_string(&config_at.data, data_stack) {
                        Some(x) => x,
                        None => return Ok(el("div")),
                    }),
                    config_at.suffix
                ),
                font_size: config_at.font_size.clone(),
                color: config_at.color.clone(),
                conv_size_max: config_at.conv_size_max.clone(),
                conv_size_mode: Some(config_at.conv_size_mode.clone()),
                link: shed!{
                    let Some(link) = config_at.link.as_ref() else {
                        break None;
                    };
                    break unwrap_value_move_url(data_stack, &link)?;
                },
            }).root);
        })() {
            Ok(e) => return e,
            Err(e) => return style_export::leaf_err_block(style_export::LeafErrBlockArgs {
                in_root: false,
                data: e,
            }).root,
        }
    }

    fn build_widget_media(
        &mut self,
        parent_orientation: Orientation,
        parent_orientation_type: OrientationType,
        config_at: &WidgetMedia,
        data_stack: &Vec<Rc<DataStackLevel>>,
    ) -> El {
        match (|| {
            ta_return!(El, String);
            let standin = || -> Result<El, String> {
                return Ok(style_export::leaf_view_image(style_export::LeafViewImageArgs {
                    parent_orientation: parent_orientation,
                    parent_orientation_type: parent_orientation_type,
                    trans_align: config_at.trans_align,
                    src: "".to_string(),
                    link: None,
                    text: None,
                    width: config_at.width.clone(),
                    height: config_at.height.clone(),
                    aspect: config_at.aspect.clone(),
                }).root)
            };
            let Some(src) = maybe_get_field_or_literal(&config_at.data, &data_stack) else {
                return standin();
            };
            let TreeNode::Scalar(src) = src else {
                return standin();
            };
            let Some(meta) = maybe_get_meta(data_stack, &src) else {
                return standin();
            };
            match meta.mime.as_ref().map(|x| x.as_str()).unwrap_or("").split("/").next().unwrap() {
                "image" => {
                    return Ok(style_export::leaf_view_image(style_export::LeafViewImageArgs {
                        parent_orientation: parent_orientation,
                        parent_orientation_type: parent_orientation_type,
                        trans_align: config_at.trans_align,
                        src: file_url(&state().env, &unwrap_value_media_url(&src)?),
                        link: shed!{
                            let Some(link) = config_at.link.as_ref() else {
                                break None;
                            };
                            break unwrap_value_move_url(data_stack, &link)?;
                        },
                        text: shed!{
                            let Some(v) = &config_at.alt else {
                                break None;
                            };
                            let Some(d) = maybe_get_field_or_literal(v, data_stack) else {
                                break None;
                            };
                            break Some(tree_node_to_text(&d));
                        },
                        width: config_at.width.clone(),
                        height: config_at.height.clone(),
                        aspect: config_at.aspect.clone(),
                    }).root);
                },
                "video" => {
                    return Ok(style_export::leaf_view_video(style_export::LeafViewVideoArgs {
                        parent_orientation: parent_orientation,
                        parent_orientation_type: parent_orientation_type,
                        trans_align: config_at.trans_align,
                        src: env_preferred_video_url(&state().env, &unwrap_value_media_url(&src)?),
                        link: shed!{
                            let Some(link) = config_at.link.as_ref() else {
                                break None;
                            };
                            break unwrap_value_move_url(data_stack, &link)?;
                        },
                        text: shed!{
                            let Some(v) = &config_at.alt else {
                                break None;
                            };
                            let Some(d) = maybe_get_field_or_literal(v, data_stack) else {
                                break None;
                            };
                            break Some(tree_node_to_text(&d));
                        },
                        width: config_at.width.clone(),
                        height: config_at.height.clone(),
                        aspect: config_at.aspect.clone(),
                    }).root);
                },
                "audio" => {
                    return Ok(style_export::leaf_view_audio(style_export::LeafViewAudioArgs {
                        parent_orientation: parent_orientation,
                        parent_orientation_type: parent_orientation_type,
                        direction: config_at.audio_direction.unwrap_or(Direction::Right),
                        trans_align: config_at.trans_align,
                        src: env_preferred_audio_url(&state().env, &unwrap_value_media_url(&src)?),
                        link: shed!{
                            let Some(link) = config_at.link.as_ref() else {
                                break None;
                            };
                            break unwrap_value_move_url(data_stack, &link)?;
                        },
                        text: shed!{
                            let Some(v) = &config_at.alt else {
                                break None;
                            };
                            let Some(d) = maybe_get_field_or_literal(v, data_stack) else {
                                break None;
                            };
                            break Some(tree_node_to_text(&d));
                        },
                        length: config_at.width.clone(),
                    }).root);
                },
                _ => {
                    return Ok(el("div"));
                },
            }
        })() {
            Ok(e) => return e,
            Err(e) => return style_export::leaf_err_block(style_export::LeafErrBlockArgs {
                in_root: false,
                data: e,
            }).root,
        }
    }

    fn build_widget_icon(
        &mut self,
        parent_orientation: Orientation,
        parent_orientation_type: OrientationType,
        config_at: &WidgetIcon,
        data_stack: &Vec<Rc<DataStackLevel>>,
    ) -> El {
        match (|| {
            ta_return!(El, String);
            return Ok(style_export::leaf_view_icon(style_export::LeafViewIconArgs {
                parent_orientation: parent_orientation,
                parent_orientation_type: parent_orientation_type,
                icon: config_at.data.clone(),
                link: shed!{
                    let Some(link) = config_at.link.as_ref() else {
                        break None;
                    };
                    break unwrap_value_move_url(data_stack, &link)?;
                },
                width: config_at.width.clone(),
                height: config_at.height.clone(),
                color: config_at.color.clone(),
                orientation: config_at.orientation.unwrap_or_default(),
                trans_align: config_at.trans_align.clone(),
            }).root);
        })() {
            Ok(e) => return e,
            Err(e) => return style_export::leaf_err_block(style_export::LeafErrBlockArgs {
                in_root: false,
                data: e,
            }).root,
        }
    }

    fn build_widget_color(
        &mut self,
        parent_orientation: Orientation,
        parent_orientation_type: OrientationType,
        config_at: &WidgetColor,
        data_stack: &Vec<Rc<DataStackLevel>>,
    ) -> El {
        match (|| {
            ta_return!(El, String);
            let Some(TreeNode::Scalar(Node::Value(serde_json::Value::String(src)))) =
                maybe_get_field_or_literal_string(&config_at.data, &data_stack) else {
                    return Ok(el("div"));
                };
            return Ok(style_export::leaf_view_color(style_export::LeafViewColorArgs {
                parent_orientation: parent_orientation,
                parent_orientation_type: parent_orientation_type,
                trans_align: config_at.trans_align,
                color: src,
                width: config_at.width.clone(),
                height: config_at.height.clone(),
            }).root);
        })() {
            Ok(e) => return e,
            Err(e) => return style_export::leaf_err_block(style_export::LeafErrBlockArgs {
                in_root: false,
                data: e,
            }).root,
        }
    }

    fn build_widget_datetime(
        &mut self,
        parent_orientation: Orientation,
        parent_orientation_type: OrientationType,
        config_at: &WidgetDatetime,
        data_stack: &Vec<Rc<DataStackLevel>>,
    ) -> El {
        match (|| {
            ta_return!(El, String);
            let Some(TreeNode::Scalar(Node::Value(serde_json::Value::String(src)))) =
                maybe_get_field_or_literal_string(&config_at.data, &data_stack) else {
                    return Ok(el("div"));
                };
            return Ok(style_export::leaf_view_datetime(style_export::LeafViewDatetimeArgs {
                parent_orientation: parent_orientation,
                parent_orientation_type: parent_orientation_type,
                trans_align: config_at.trans_align,
                orientation: config_at.orientation,
                value: src,
                font_size: config_at.font_size.clone(),
                color: config_at.color.clone(),
            }).root);
        })() {
            Ok(e) => return e,
            Err(e) => return style_export::leaf_err_block(style_export::LeafErrBlockArgs {
                in_root: false,
                data: e,
            }).root,
        }
    }

    fn build_widget_date(
        &mut self,
        parent_orientation: Orientation,
        parent_orientation_type: OrientationType,
        config_at: &WidgetDate,
        data_stack: &Vec<Rc<DataStackLevel>>,
    ) -> El {
        match (|| {
            ta_return!(El, String);
            let Some(TreeNode::Scalar(Node::Value(serde_json::Value::String(src)))) =
                maybe_get_field_or_literal_string(&config_at.data, &data_stack) else {
                    return Ok(el("div"));
                };
            return Ok(style_export::leaf_view_date(style_export::LeafViewDateArgs {
                parent_orientation: parent_orientation,
                parent_orientation_type: parent_orientation_type,
                trans_align: config_at.trans_align,
                orientation: config_at.orientation,
                value: src,
                font_size: config_at.font_size.clone(),
                color: config_at.color.clone(),
            }).root);
        })() {
            Ok(e) => return e,
            Err(e) => return style_export::leaf_err_block(style_export::LeafErrBlockArgs {
                in_root: false,
                data: e,
            }).root,
        }
    }

    fn build_widget_time(
        &mut self,
        parent_orientation: Orientation,
        parent_orientation_type: OrientationType,
        config_at: &WidgetTime,
        data_stack: &Vec<Rc<DataStackLevel>>,
    ) -> El {
        match (|| {
            ta_return!(El, String);
            let Some(TreeNode::Scalar(Node::Value(serde_json::Value::String(src)))) =
                maybe_get_field_or_literal_string(&config_at.data, &data_stack) else {
                    return Ok(el("div"));
                };
            return Ok(style_export::leaf_view_time(style_export::LeafViewTimeArgs {
                parent_orientation: parent_orientation,
                parent_orientation_type: parent_orientation_type,
                trans_align: config_at.trans_align,
                orientation: config_at.orientation,
                value: src,
                font_size: config_at.font_size.clone(),
                color: config_at.color.clone(),
            }).root);
        })() {
            Ok(e) => return e,
            Err(e) => return style_export::leaf_err_block(style_export::LeafErrBlockArgs {
                in_root: false,
                data: e,
            }).root,
        }
    }

    fn build_widget_node(
        &mut self,
        pc: &mut ProcessingContext,
        parent_orientation: Orientation,
        parent_orientation_type: OrientationType,
        config_at: &WidgetNode,
        data_stack: &Vec<Rc<DataStackLevel>>,
    ) -> El {
        match (|| {
            ta_return!(El, String);
            let Some(TreeNode::Scalar(Node::Value(serde_json::Value::String(name)))) =
                maybe_get_field_or_literal_string(&config_at.name, &data_stack) else {
                    return Ok(el("div"));
                };
            let Some(TreeNode::Scalar(node)) = maybe_get_field_or_literal(&config_at.node, &data_stack) else {
                return Ok(el("div"));
            };
            let out = style_export::leaf_view_node_button(style_export::LeafViewNodeButtonArgs {
                parent_orientation: parent_orientation,
                parent_orientation_type: parent_orientation_type,
                trans_align: config_at.trans_align,
                orientation: config_at.orientation,
            }).root;
            setup_node_button(pc, &out, name, node);
            return Ok(out);
        })() {
            Ok(e) => return e,
            Err(e) => return style_export::leaf_err_block(style_export::LeafErrBlockArgs {
                in_root: false,
                data: e,
            }).root,
        }
    }

    fn build_widget_play_button(
        &mut self,
        pc: &mut ProcessingContext,
        parent_orientation: Orientation,
        parent_orientation_type: OrientationType,
        config_at: &WidgetPlayButton,
        data_id: &Vec<usize>,
        data_stack: &Vec<Rc<DataStackLevel>>,
    ) -> El {
        match (|| {
            ta_return!(El, String);
            let Some(src) = maybe_get_field(&config_at.media_file_field, data_stack) else {
                return Ok(el("div"));
            };
            let media_type;
            let TreeNode::Scalar(src) = &src else {
                return Ok(el("div"));
            };
            let Some(meta) = maybe_get_meta(data_stack, src) else {
                return Ok(el("div"));
            };
            match categorize_mime_media(meta.mime.as_ref().map(|x| x.as_str()).unwrap_or("")) {
                Some(m) => {
                    media_type = m;
                },
                None => {
                    return Ok(el("div"));
                },
            }
            self.want_media = true;
            let src_url = unwrap_value_media_url(&src)?;
            let cover_source_url = shed!{
                let Some(config_at) = &config_at.cover_field else {
                    break None;
                };
                let Some(d) = maybe_get_field(config_at, data_stack) else {
                    break None;
                };
                let TreeNode::Scalar(d) = d else {
                    break None;
                };
                break Some(unwrap_value_media_url(&d).map_err(|e| format!("Building cover url: {}", e))?);
            };
            let out = style_export::leaf_view_play_button(style_export::LeafViewPlayButtonArgs {
                parent_orientation: parent_orientation,
                parent_orientation_type: parent_orientation_type,
                trans_align: config_at.trans_align,
                orientation: config_at.orientation.unwrap_or_default(),
            }).root;
            self.playlist_add.push(PlaylistPushArg {
                index: data_id.clone(),
                name: shed!{
                    let Some(config_at) = &config_at.name_field else {
                        break None;
                    };
                    let Some(d) = maybe_get_field(config_at, data_stack) else {
                        break None;
                    };
                    break Some(tree_node_to_text(&d));
                },
                album: shed!{
                    let Some(config_at) = &config_at.album_field else {
                        break None;
                    };
                    let Some(d) = maybe_get_field(config_at, data_stack) else {
                        break None;
                    };
                    break Some(tree_node_to_text(&d));
                },
                artist: shed!{
                    let Some(config_at) = &config_at.artist_field else {
                        break None;
                    };
                    let Some(d) = maybe_get_field(config_at, data_stack) else {
                        break None;
                    };
                    break Some(tree_node_to_text(&d));
                },
                cover_source_url: cover_source_url,
                source_url: src_url,
                media_type: media_type,
                play_buttons: vec![out.raw().dyn_into().unwrap()],
            });
            out.ref_on("click", {
                let data_id = data_id.clone();
                let eg = pc.eg();
                move |_| eg.event(|pc| {
                    playlist_toggle_play(pc, &state().playlist, Some(data_id.clone()));
                }).unwrap()
            });
            out.ref_own(
                |out| link!(
                    (_pc = pc),
                    (playing = state().playlist.0.playing.clone(), playing_i = state().playlist.0.playing_i.clone()),
                    (),
                    (index = data_id.clone(), out = out.weak()) {
                        let out = out.upgrade()?;
                        if playing.get() && playing_i.get().as_ref() == Some(index) {
                            out.ref_attr(
                                &style_export::attr_state().value,
                                &style_export::attr_state_playing().value,
                            );
                            out
                                .raw()
                                .dyn_into::<HtmlElement>()
                                .unwrap()
                                .focus()
                                .log(&state().log, "Error focusing media button");
                        } else {
                            out.ref_attr(&style_export::attr_state().value, "");
                        }
                    }
                ),
            );
            return Ok(out);
        })() {
            Ok(e) => return e,
            Err(e) => return style_export::leaf_err_block(style_export::LeafErrBlockArgs {
                in_root: false,
                data: e,
            }).root,
        }
    }

    fn build_widget(
        &mut self,
        pc: &mut ProcessingContext,
        parent_orientation: Orientation,
        parent_orientation_type: OrientationType,
        config_at: &Widget,
        config_query_params: &BTreeMap<String, Vec<String>>,
        data_id: &Vec<usize>,
        data_stack: &Vec<Rc<DataStackLevel>>,
    ) -> El {
        match config_at {
            Widget::Layout(config_at) => return self.build_widget_layout(
                pc,
                parent_orientation,
                parent_orientation_type,
                config_at,
                config_query_params,
                data_id,
                data_stack,
            ),
            Widget::DataRows(config_at) => return self.build_widget_data_rows(
                pc,
                parent_orientation,
                parent_orientation_type,
                config_at,
                config_query_params,
                data_id,
                data_stack,
            ),
            Widget::Text(config_at) => return self.build_widget_text(
                parent_orientation,
                parent_orientation_type,
                config_at,
                data_stack,
            ),
            Widget::Media(config_at) => return self.build_widget_media(
                parent_orientation,
                parent_orientation_type,
                config_at,
                data_stack,
            ),
            Widget::Icon(config_at) => return self.build_widget_icon(
                parent_orientation,
                parent_orientation_type,
                config_at,
                data_stack,
            ),
            Widget::PlayButton(config_at) => return self.build_widget_play_button(
                pc,
                parent_orientation,
                parent_orientation_type,
                config_at,
                data_id,
                data_stack,
            ),
            Widget::Color(config_at) => return self.build_widget_color(
                parent_orientation,
                parent_orientation_type,
                config_at,
                data_stack,
            ),
            Widget::Date(config_at) => return self.build_widget_date(
                parent_orientation,
                parent_orientation_type,
                config_at,
                data_stack,
            ),
            Widget::Datetime(config_at) => return self.build_widget_datetime(
                parent_orientation,
                parent_orientation_type,
                config_at,
                data_stack,
            ),
            Widget::Time(config_at) => return self.build_widget_time(
                parent_orientation,
                parent_orientation_type,
                config_at,
                data_stack,
            ),
            Widget::Space => return style_export::leaf_space().root,
            Widget::Node(config_at) => return self.build_widget_node(
                pc,
                parent_orientation,
                parent_orientation_type,
                config_at,
                data_stack,
            ),
        }
    }
}

fn build_transport(pc: &mut ProcessingContext) -> El {
    let transport_res = style_export::cont_bar_view_transport();
    transport_res.button_share.ref_on("click", {
        let eg = pc.eg();
        move |_| eg.event(|pc| {
            let sess_id = state().playlist.0.share.borrow().as_ref().map(|x| x.0.clone());
            let sess_id = match sess_id {
                Some(sess_id) => {
                    sess_id.clone()
                },
                None => {
                    let sess_id = if let Ok(id) = LocalStorage::get::<String>(LOCALSTORAGE_SHARE_SESSION_ID) {
                        id
                    } else {
                        let sess_id = Uuid::new_v4().to_string();
                        LocalStorage::set(
                            LOCALSTORAGE_SHARE_SESSION_ID,
                            &sess_id,
                        ).log(&state().log, "Error persisting session id");
                        sess_id
                    };
                    playlist_set_link(pc, &state().playlist, &sess_id);
                    sess_id
                },
            };
            let link = format!("{}link.html#{}{}", state().env.base_url, LINK_HASH_PREFIX, sess_id);
            let modal_res = style_export::cont_view_modal_share(style_export::ContViewModalShareArgs {
                qr: el_from_raw(
                    DomParser::new()
                        .unwrap()
                        .parse_from_string(
                            &QrCode::new(&link)
                                .unwrap()
                                .render::<qrcode::render::svg::Color>()
                                .dark_color(Color("currentColor"))
                                .light_color(Color("transparent"))
                                .quiet_zone(false)
                                .build(),
                            web_sys::SupportedType::ImageSvgXml,
                        )
                        .unwrap()
                        .first_element_child()
                        .unwrap()
                        .dyn_into()
                        .unwrap(),
                ),
                link: link,
            });
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
            modal_res.button_unshare.ref_on("click", {
                let modal_el = modal_res.root.weak();
                let eg = pc.eg();
                move |_| eg.event(|pc| {
                    let Some(modal_el) = modal_el.upgrade() else {
                        return;
                    };
                    modal_el.ref_replace(vec![]);
                    state().playlist.0.share.set(pc, None);
                    LocalStorage::delete(LOCALSTORAGE_SHARE_SESSION_ID);
                }).unwrap()
            });
            state().modal_stack.ref_push(modal_res.root.clone());
        }).unwrap()
    });
    transport_res
        .button_share
        .ref_own(|b| link!((_pc = pc), (sharing = state().playlist.0.share.clone()), (), (ele = b.weak()), {
            let ele = ele.upgrade()?;
            ele.ref_modify_classes(&[(&style_export::class_state_sharing().value, sharing.borrow().is_some())]);
        }));

    // Prev
    let button_prev = transport_res.button_prev;
    button_prev.ref_on("click", {
        let eg = pc.eg();
        move |_| eg.event(|pc| {
            playlist_previous(pc, &state().playlist, None);
        }).unwrap()
    });

    // Next
    let button_next = transport_res.button_next;
    button_next.ref_on("click", {
        let eg = pc.eg();
        move |_| eg.event(|pc| {
            playlist_next(pc, &state().playlist, None);
        }).unwrap()
    });

    // Play
    let button_play = transport_res.button_play;
    button_play.ref_on("click", {
        let eg = pc.eg();
        move |_| eg.event(|pc| {
            playlist_toggle_play(pc, &state().playlist, None);
        }).unwrap()
    });
    button_play.ref_own(|out| (
        //. .
        link!((_pc = pc), (playing = state().playlist.0.playing.clone()), (), (out = out.weak()) {
            let out = out.upgrade()?;
            if playing.get() {
                out.ref_attr(&style_export::attr_state().value, &style_export::attr_state_playing().value);
            } else {
                out.ref_attr(&style_export::attr_state().value, "");
            }
        }),
        link!((_pc = pc), (active = state().playlist.0.playing_i.clone()), (), (out = out.weak()) {
            let out = out.upgrade()?;
            out.ref_modify_classes(&[(&style_export::class_state_selected().value, active.get().is_some())]);
        }),
    ));
    setup_seekbar(pc, transport_res.seekbar, transport_res.seekbar_fill, transport_res.seekbar_label);

    // Assemble
    return transport_res.root;
}

#[derive(Clone)]
struct BuildViewBodyCommon {
    id: ViewId,
    config_at: WidgetRootDataRows,
    shuffle: bool,
    config_query_params: BTreeMap<String, Vec<String>>,
    body: WeakEl,
    transport_slot: WeakEl,
    have_media: Rc<Cell<bool>>,
    view_ministate_state: MinistateViewState,
}

fn build_page_view_body(
    pc: &mut ProcessingContext,
    common: &BuildViewBodyCommon,
    param_data: &HashMap<String, Node>,
    restore_playlist_pos: Option<PlaylistRestorePos>,
    offline: Option<String>,
) {
    let Some(body) = common.body.upgrade() else {
        return;
    };
    let Some(transport_slot) = common.transport_slot.upgrade() else {
        return;
    };
    body.ref_clear();
    playlist_clear(pc, &state().playlist, common.shuffle);
    let mut build = Build {
        view_id: common.id.clone(),
        vs: common.view_ministate_state.clone(),
        param_data: param_data.clone(),
        restore_playlist_pos: restore_playlist_pos.clone(),
        playlist_add: Default::default(),
        want_media: false,
        have_media: common.have_media.clone(),
        transport_slot: transport_slot,
        seed: (random() * u64::MAX as f64) as u64,
        offline: offline,
    };
    body.ref_push(
        build.build_widget_root_data_rows(
            pc,
            &common.config_at,
            &common.config_query_params,
            &vec![Rc::new(DataStackLevel {
                data: TreeNode::Record(
                    param_data.iter().map(|(k, v)| (k.clone(), TreeNode::Scalar(v.clone()))).collect(),
                ),
                node_meta: Default::default(),
            })],
        ),
    );
    playlist_extend(
        pc,
        &state().playlist,
        common.view_ministate_state.clone(),
        build.playlist_add,
        &restore_playlist_pos,
    );
}

pub fn build_page_view(
    eg: EventGraph,
    id: ViewId,
    title: String,
    view: ClientView,
    params: HashMap<String, Node>,
    restore_playlist_pos: Option<PlaylistRestorePos>,
    offline: Option<String>,
) -> Result<El, String> {
    return eg.event(|pc| {
        let vs = MinistateViewState(Rc::new(RefCell::new(MinistateViewState_ {
            view_id: id.clone(),
            title: title.clone(),
            pos: restore_playlist_pos.clone(),
            params: params.clone(),
            offline: offline.clone(),
        })));
        let transport_slot = style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root;
        let body = style_export::cont_view_root(style_export::ContViewRootArgs {
            elements: vec![],
            element_width: view.root.element_width.clone(),
        }).root;
        let common = Rc::new(BuildViewBodyCommon {
            id: id.clone(),
            view_ministate_state: vs.clone(),
            transport_slot: transport_slot.weak(),
            config_at: view.root,
            shuffle: view.shuffle,
            config_query_params: view.query_parameter_keys,
            body: body.weak(),
            have_media: Rc::new(Cell::new(false)),
        });
        let param_data = Rc::new(RefCell::new(params));
        let mut param_els = vec![];
        if let Some(key) = &offline {
            for (k, v) in view.parameter_specs {
                match v {
                    ClientViewParam::Text => {
                        param_data
                            .borrow_mut()
                            .entry(k.clone())
                            .or_insert_with(|| Node::Value(serde_json::Value::String(format!(""))));
                        let pair = style_export::leaf_input_pair_text_fixed(style_export::LeafInputPairTextFixedArgs {
                            id: k.clone(),
                            title: k.clone(),
                            value: match param_data.borrow().get(&k) {
                                Some(Node::Value(serde_json::Value::String(v))) => v.clone(),
                                _ => format!(""),
                            },
                        });
                        param_els.push(pair.root);
                    },
                }
            }
            let unoffline_button = style_export::leaf_view_title_button_unoffline().root;
            unoffline_button.ref_on("click", {
                let eg = pc.eg();
                let key = key.clone();
                move |_| {
                    let modal_res = style_export::cont_view_modal_confirm_unoffline();
                    modal_res.button_close.ref_on("click", {
                        let modal_el = modal_res.root.weak();
                        let eg = eg.clone();
                        move |_| eg.event(|_pc| {
                            let Some(modal_el) = modal_el.upgrade() else {
                                return;
                            };
                            modal_el.ref_replace(vec![]);
                        }).unwrap()
                    });
                    modal_res.root.ref_on("click", {
                        let modal_el = modal_res.root.weak();
                        let eg = eg.clone();
                        move |_| eg.event(|_pc| {
                            let Some(modal_el) = modal_el.upgrade() else {
                                return;
                            };
                            modal_el.ref_replace(vec![]);
                        }).unwrap()
                    });
                    on_thinking(&modal_res.button_ok, {
                        let key = key.clone();
                        let eg = eg.clone();
                        move || {
                            let key = key.clone();
                            let eg = eg.clone();
                            async move {
                                if let Err(e) = remove_offline(eg.clone(), &key).await {
                                    state().log.log(&format!("Error removing offline for view: {}", e));
                                } else {
                                    eg.event(|pc| {
                                        goto_replace_ministate(pc, &state().log, &Ministate::Home);
                                    }).unwrap();
                                }
                            }
                        }
                    });
                    state().modal_stack.ref_push(modal_res.root.clone());
                }
            });
            state().main_title_right.ref_push(unoffline_button);
        } else {
            let offline_view = Prim::new(MinistateView {
                id: id.clone(),
                title: title.clone(),
                pos: None,
                params: param_data.borrow().clone(),
            });
            let params_debounce = Rc::new(RefCell::new(None));
            for (k, v) in view.parameter_specs {
                match v {
                    ClientViewParam::Text => {
                        param_data
                            .borrow_mut()
                            .entry(k.clone())
                            .or_insert_with(|| Node::Value(serde_json::Value::String(format!(""))));
                        let pair = style_export::leaf_input_pair_text(style_export::LeafInputPairTextArgs {
                            id: k.clone(),
                            title: k.clone(),
                            value: match param_data.borrow().get(&k) {
                                Some(Node::Value(serde_json::Value::String(v))) => v.clone(),
                                _ => format!(""),
                            },
                        });
                        pair.input.ref_on("input", {
                            let id = id.clone();
                            let title = title.clone();
                            let eg = pc.eg();
                            let k = k.clone();
                            let input = pair.input.weak();
                            let common = common.clone();
                            let params_debounce = params_debounce.clone();
                            let param_data = param_data.clone();
                            let offline_view = offline_view.clone();
                            move |_| *params_debounce.borrow_mut() = Some(Timeout::new(500, {
                                let id = id.clone();
                                let title = title.clone();
                                let input = input.clone();
                                let common = common.clone();
                                let param_data = param_data.clone();
                                let eg = eg.clone();
                                let k = k.clone();
                                let offline_view = offline_view.clone();
                                move || {
                                    let Some(input) = input.upgrade() else {
                                        return;
                                    };
                                    let v =
                                        Node::Value(
                                            serde_json::Value::String(input.raw().text_content().unwrap_or_default()),
                                        );
                                    common.view_ministate_state.set_param(k.clone(), v.clone());
                                    param_data.borrow_mut().insert(k, v);
                                    eg.event(|pc| {
                                        offline_view.set(pc, MinistateView {
                                            id: id.clone(),
                                            title: title.clone(),
                                            pos: None,
                                            params: param_data.borrow().clone(),
                                        });
                                        build_page_view_body(pc, &common, &*param_data.borrow(), None, None);
                                    }).unwrap();
                                }
                            }))
                        });
                        param_els.push(pair.root);
                    },
                }
            }
            let offline_button = style_export::leaf_view_title_button_offline().root;
            on_thinking(&offline_button, {
                let view = offline_view.clone();
                let eg = pc.eg();
                move || {
                    let view = view.clone();
                    let eg = eg.clone();
                    async move {
                        if let Err(e) = ensure_offline(eg.clone(), view.borrow().clone()).await {
                            state().log.log(&format!("Error triggering offline for view: {}", e));
                        }
                    }
                }
            });
            offline_button.ref_own(
                |b: &El| link!(
                    (_pc = pc),
                    (offline_view = offline_view.clone(), offline_views_list = state().offline_list.clone()),
                    (),
                    (b = b.weak()) {
                        let b = b.upgrade()?;
                        let offline_view = offline_view.borrow();
                        b.ref_modify_classes(
                            &[
                                (
                                    &style_export::class_state_disabled().value,
                                    offline_views_list.borrow_values().iter().any(|(_, v1)| v1 == &*offline_view),
                                )
                            ]
                        );
                    }
                ),
            );
            state().main_title_right.ref_push(offline_button);
        }
        build_page_view_body(pc, &common, &*param_data.borrow(), restore_playlist_pos, offline);
        return Ok(style_export::cont_page_view(style_export::ContPageViewArgs {
            transport: Some(transport_slot),
            params: param_els,
            elements: body,
        }).root);
    }).unwrap();
}
