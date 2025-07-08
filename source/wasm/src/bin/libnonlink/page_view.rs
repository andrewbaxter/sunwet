use {
    super::{
        infinite::build_infinite,
        ministate::{
            MinistateNodeView,
            PlaylistRestorePos,
        },
        playlist::{
            playlist_clear,
            PlaylistIndex,
        },
        state::{
            state,
            MinistateViewState,
            MinistateViewState_,
        },
    },
    crate::libnonlink::{
        api::req_post_json,
        ministate::{
            ministate_octothorpe,
            Ministate,
            MinistateForm,
            MinistateView,
        },
        playlist::{
            playlist_extend,
            playlist_next,
            playlist_previous,
            playlist_seek,
            playlist_set_link,
            playlist_toggle_play,
            PlaylistEntryMediaType,
            PlaylistPushArg,
        },
    },
    flowcontrol::{
        exenum,
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
        link,
        EventGraph,
        Prim,
        ProcessingContext,
    },
    qrcode::{
        render::svg::Color,
        QrCode,
    },
    rooting::{
        el,
        el_from_raw,
        El,
        WeakEl,
    },
    shared::interface::{
        config::view::{
            ClientView,
            Direction,
            FieldOrLiteral,
            FieldOrLiteralString,
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
            WidgetLayout,
            WidgetMedia,
            WidgetPlayButton,
            WidgetRootDataRows,
            WidgetText,
            WidgetTime,
        },
        triple::Node,
        wire::{
            link::SourceUrl,
            NodeMeta,
            Pagination,
            ReqViewQuery,
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
            el_async,
            style_export::{
                self,
            },
            LogJsErr,
        },
        world::file_url,
    },
    wasm_bindgen::JsCast,
    web_sys::{
        DomParser,
        Element,
        Event,
        MouseEvent,
    },
};

pub const LOCALSTORAGE_SHARE_SESSION_ID: &str = "share_session_id";

#[derive(Clone)]
struct DataStackLevel {
    data: TreeNode,
    node_meta: Rc<HashMap<Node, NodeMeta>>,
}

fn maybe_get_meta<'a>(data_stack: &'a Vec<DataStackLevel>, node: &Node) -> Option<&'a NodeMeta> {
    for data_at in data_stack.iter().rev() {
        if let Some(meta) = data_at.node_meta.get(node) {
            return Some(meta);
        }
    }
    return None;
}

fn maybe_get_field(config_at: &String, data_stack: &Vec<DataStackLevel>) -> Option<TreeNode> {
    for data_at in data_stack.iter().rev() {
        let TreeNode::Record(data_at) = &data_at.data else {
            continue;
        };
        let Some(data_at) = data_at.get(config_at) else {
            continue;
        };
        if exenum!(data_at, TreeNode:: Scalar(Node::Value(serde_json::Value::Null)) =>()).is_some() {
            continue;
        }
        return Some(data_at.clone());
    }
    return None;
}

fn maybe_get_field_or_literal(
    config_at: &FieldOrLiteral,
    data_stack: &Vec<DataStackLevel>,
) -> Result<Option<TreeNode>, String> {
    match config_at {
        FieldOrLiteral::Field(config_at) => return Ok(maybe_get_field(config_at, data_stack)),
        FieldOrLiteral::Literal(config_at) => return Ok(Some(TreeNode::Scalar(config_at.clone()))),
    }
}

fn maybe_get_field_or_literal_string(
    config_at: &FieldOrLiteralString,
    data_stack: &Vec<DataStackLevel>,
) -> Result<Option<TreeNode>, String> {
    match config_at {
        FieldOrLiteralString::Field(config_at) => return Ok(maybe_get_field(config_at, data_stack)),
        FieldOrLiteralString::Literal(config_at) => return Ok(
            Some(TreeNode::Scalar(Node::Value(serde_json::Value::String(config_at.clone())))),
        ),
    }
}

fn unwrap_value_string(data_at: &TreeNode) -> String {
    match data_at {
        TreeNode::Array(v) => return serde_json::to_string(v).unwrap(),
        TreeNode::Record(v) => return serde_json::to_string(v).unwrap(),
        TreeNode::Scalar(v) => match v {
            Node::File(v) => return v.to_string(),
            Node::Value(v) => match v {
                serde_json::Value::String(v) => return v.clone(),
                serde_json::Value::Number(v) => {
                    if let Some(v) = v.as_i64() {
                        return v.to_string();
                    } else if let Some(v) = v.as_u64() {
                        return v.to_string();
                    } else if let Some(v) = v.as_f64() {
                        return v.to_string();
                    } else {
                        return v.to_string();
                    }
                },
                _ => return serde_json::to_string(v).unwrap(),
            },
        },
    }
}

fn unwrap_value_media_url(data_at: &Node) -> Result<SourceUrl, String> {
    match data_at {
        Node::File(v) => return Ok(SourceUrl::File(v.clone())),
        Node::Value(v) => match v {
            serde_json::Value::String(v) => return Ok(SourceUrl::Url(v.clone())),
            _ => return Err(format!("Url is not a string: {}", serde_json::to_string(v).unwrap())),
        },
    }
}

fn unwrap_value_move_url(data_stack: &Vec<DataStackLevel>, link: &Link) -> Result<Option<String>, String> {
    let title = match maybe_get_field_or_literal(&link.title, data_stack)? {
        Some(x) => unwrap_value_string(&x),
        None => format!("(unknown name)"),
    };
    match &link.dest {
        LinkDest::Plain(d) => {
            let Some(TreeNode::Scalar(data_at)) = maybe_get_field_or_literal(d, data_stack)? else {
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
                let Some(TreeNode::Scalar(v)) = maybe_get_field_or_literal(v, data_stack)? else {
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
                let Some(TreeNode::Scalar(v)) = maybe_get_field_or_literal(v, data_stack)? else {
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
            let Some(TreeNode::Scalar(data_at)) = maybe_get_field_or_literal(d, data_stack)? else {
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
    playlist_add: Vec<(PlaylistIndex, PlaylistPushArg)>,
    have_media: Rc<Cell<bool>>,
    want_media: bool,
    transport_slot: El,
    vs: MinistateViewState,
}

impl Build {
    fn build_widget_layout(
        &mut self,
        pc: &mut ProcessingContext,
        config_at: &WidgetLayout,
        config_query_params: &BTreeMap<String, Vec<String>>,
        data_id: &Vec<usize>,
        data_at: &Vec<DataStackLevel>,
    ) -> El {
        let mut children = vec![];
        for config_at in &config_at.elements {
            children.push(self.build_widget(pc, config_at, config_query_params, data_id, data_at));
        }
        return style_export::cont_view_list(style_export::ContViewListArgs {
            direction: config_at.direction,
            trans_align: config_at.trans_align,
            x_scroll: config_at.x_scroll,
            children: children,
            gap: config_at.gap.clone(),
            wrap: config_at.wrap,
        }).root;
    }

    fn build_widget_data_rows(
        &mut self,
        pc: &mut ProcessingContext,
        config_at: &WidgetDataRows,
        config_query_params: &BTreeMap<String, Vec<String>>,
        data_id: &Vec<usize>,
        data_at: &Vec<DataStackLevel>,
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
                        let res = req_post_json(&state().env.base_url, ReqViewQuery {
                            view_id: view_id.clone(),
                            query: query_id.clone(),
                            parameters: params,
                            pagination: None,
                        }).await?;
                        let mut out = vec![];
                        for v in res.records {
                            out.push(TreeNode::Record(v));
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
                    };
                    let out;
                    match &config_at.row_widget {
                        shared::interface::config::view::DataRowsLayout::Unaligned(row_widget) => {
                            let mut children = vec![];
                            for (i, new_data_at_top) in new_data_at_tops.into_iter().enumerate() {
                                let mut data_at = data_at.clone();
                                data_at.push(DataStackLevel {
                                    data: new_data_at_top,
                                    node_meta: node_meta.clone(),
                                });
                                let mut data_id = data_id.clone();
                                data_id.push(i);
                                children.push(
                                    build.build_widget(
                                        pc,
                                        &row_widget.widget,
                                        &config_query_params,
                                        &data_id,
                                        &data_at,
                                    ),
                                );
                            }
                            out = style_export::cont_view_list(style_export::ContViewListArgs {
                                direction: row_widget.direction.unwrap_or(Direction::Down),
                                trans_align: row_widget.trans_align,
                                x_scroll: row_widget.x_scroll,
                                wrap: row_widget.wrap,
                                children: children,
                                gap: row_widget.gap.clone(),
                            }).root;
                        },
                        shared::interface::config::view::DataRowsLayout::Table(row_widget) => {
                            let mut rows = vec![];
                            for (i, new_data_at_top) in new_data_at_tops.into_iter().enumerate() {
                                let mut data_at = data_at.clone();
                                data_at.push(DataStackLevel {
                                    data: new_data_at_top,
                                    node_meta: node_meta.clone(),
                                });
                                let mut data_id = data_id.clone();
                                data_id.push(i);
                                let mut columns = vec![];
                                let mut columns_raw = vec![];
                                for config_at in &row_widget.elements {
                                    let column =
                                        build.build_widget(pc, config_at, &config_query_params, &data_id, &data_at);
                                    columns_raw.push(column.raw());
                                    columns.push(column);
                                }
                                rows.push(columns);
                            }
                            out = style_export::cont_view_table(style_export::ContViewTableArgs {
                                orientation: row_widget.orientation,
                                x_scroll: row_widget.x_scroll,
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
        data_id: &Vec<usize>,
        data_at: &Vec<DataStackLevel>,
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
            let data_id = data_id.clone();
            let data_at = data_at.clone();
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
                    };
                    let mut children = vec![];
                    for (i, new_data_at_top) in chunk {
                        let mut data_at = data_at.clone();
                        data_at.push(DataStackLevel {
                            data: new_data_at_top,
                            node_meta: node_meta.clone(),
                        });
                        let mut data_id = data_id.clone();
                        data_id.push(i);
                        let mut blocks = vec![];
                        for config_at in &config_at.row_blocks {
                            let block_contents =
                                build.build_widget(pc, &config_at.widget, &config_query_params, &data_id, &data_at);
                            blocks.push(style_export::cont_view_block(style_export::ContViewBlockArgs {
                                children: vec![block_contents],
                                width: config_at.width.clone(),
                            }).root);
                        }
                        children.push(
                            style_export::cont_view_row(style_export::ContViewRowArgs { blocks: blocks }).root,
                        );
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
        return el_async({
            let config_at = config_at.clone();
            let view_id = self.view_id.clone();
            let config_query_params = config_query_params.clone();
            let data_at = data_at.clone();
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
                        body.ref_push(build_infinite(chunked_data.next().unwrap(), {
                            let build_infinite_page = build_infinite_page.clone();
                            move |chunk| {
                                let children = build_infinite_page(chunk, Default::default());
                                let next_key = chunked_data.next();
                                async move {
                                    Ok((next_key, children))
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
                        body.ref_push(build_infinite(None, {
                            let seed = (random() * u64::MAX as f64) as u64;
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
                                    let res = req_post_json(&state().env.base_url, ReqViewQuery {
                                        view_id: view_id.clone(),
                                        query: query_id.clone(),
                                        parameters: params,
                                        pagination: Some(Pagination {
                                            count: 10,
                                            seed: Some(seed),
                                            key: key,
                                        }),
                                    }).await?;
                                    let mut chunk = vec![];
                                    for v in res.records {
                                        chunk.push((count.get(), TreeNode::Record(v)));
                                        count.set(count.get() + 1);
                                    }
                                    Ok(
                                        (
                                            res.next_page_key.map(|x| Some(x)),
                                            build_infinite_page(
                                                chunk,
                                                Rc::new(res.meta.into_iter().collect::<HashMap<_, _>>()),
                                            ),
                                        ),
                                    )
                                }
                            }
                        }));
                    },
                }
                return Ok(vec![body]);
            }
        });
    }

    fn build_widget_text(&mut self, config_at: &WidgetText, data_stack: &Vec<DataStackLevel>) -> El {
        match (|| {
            ta_return!(El, String);
            return Ok(style_export::leaf_view_text(style_export::LeafViewTextArgs {
                trans_align: config_at.trans_align,
                orientation: config_at.orientation,
                text: format!(
                    "{}{}{}",
                    config_at.prefix,
                    unwrap_value_string(&match maybe_get_field_or_literal_string(&config_at.data, data_stack)? {
                        Some(x) => x,
                        None => return Ok(el("div")),
                    }),
                    config_at.suffix
                ),
                font_size: config_at.font_size.clone(),
                color: config_at.color.clone(),
                max_size: config_at.cons_size_max.clone(),
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

    fn build_widget_media(&mut self, config_at: &WidgetMedia, data_stack: &Vec<DataStackLevel>) -> El {
        match (|| {
            ta_return!(El, String);
            let Some(src) = maybe_get_field_or_literal(&config_at.data, &data_stack)? else {
                return Ok(el("div"));
            };
            let TreeNode::Scalar(src) = src else {
                return Ok(el("div"));
            };
            let Some(meta) = maybe_get_meta(data_stack, &src) else {
                return Ok(el("div"));
            };
            match meta.mime.as_ref().map(|x| x.as_str()).unwrap_or("").split("/").next().unwrap() {
                "image" => {
                    return Ok(style_export::leaf_view_image(style_export::LeafViewImageArgs {
                        trans_align: config_at.trans_align,
                        src: match unwrap_value_media_url(&src)? {
                            SourceUrl::Url(v) => v,
                            SourceUrl::File(v) => file_url(&state().env, &v),
                        },
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
                            let Some(d) = maybe_get_field_or_literal(v, data_stack)? else {
                                break None;
                            };
                            break Some(unwrap_value_string(&d));
                        },
                        width: config_at.width.clone(),
                        height: config_at.height.clone(),
                    }).root);
                },
                "video" => {
                    return Ok(style_export::leaf_view_video(style_export::LeafViewVideoArgs {
                        trans_align: config_at.trans_align,
                        src: match unwrap_value_media_url(&src)? {
                            SourceUrl::Url(v) => v,
                            SourceUrl::File(v) => file_url(&state().env, &v),
                        },
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
                            let Some(d) = maybe_get_field_or_literal(v, data_stack)? else {
                                break None;
                            };
                            break Some(unwrap_value_string(&d));
                        },
                        width: config_at.width.clone(),
                        height: config_at.height.clone(),
                    }).root);
                },
                "audio" => {
                    return Ok(style_export::leaf_view_audio(style_export::LeafViewAudioArgs {
                        direction: config_at.direction.unwrap_or(Direction::Right),
                        trans_align: config_at.trans_align,
                        src: match unwrap_value_media_url(&src)? {
                            SourceUrl::Url(v) => v,
                            SourceUrl::File(v) => file_url(&state().env, &v),
                        },
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
                            let Some(d) = maybe_get_field_or_literal(v, data_stack)? else {
                                break None;
                            };
                            break Some(unwrap_value_string(&d));
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

    fn build_widget_color(&mut self, config_at: &WidgetColor, data_stack: &Vec<DataStackLevel>) -> El {
        match (|| {
            ta_return!(El, String);
            let Some(TreeNode::Scalar(Node::Value(serde_json::Value::String(src)))) =
                maybe_get_field_or_literal_string(&config_at.data, &data_stack)? else {
                    return Ok(el("div"));
                };
            return Ok(style_export::leaf_view_color(style_export::LeafViewColorArgs {
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

    fn build_widget_datetime(&mut self, config_at: &WidgetDatetime, data_stack: &Vec<DataStackLevel>) -> El {
        match (|| {
            ta_return!(El, String);
            let Some(TreeNode::Scalar(Node::Value(serde_json::Value::String(src)))) =
                maybe_get_field_or_literal_string(&config_at.data, &data_stack)? else {
                    return Ok(el("div"));
                };
            return Ok(style_export::leaf_view_datetime(style_export::LeafViewDatetimeArgs {
                trans_align: config_at.trans_align,
                orientation: config_at.orientation,
                value: src,
                font_size: config_at.font_size.clone(),
            }).root);
        })() {
            Ok(e) => return e,
            Err(e) => return style_export::leaf_err_block(style_export::LeafErrBlockArgs {
                in_root: false,
                data: e,
            }).root,
        }
    }

    fn build_widget_date(&mut self, config_at: &WidgetDate, data_stack: &Vec<DataStackLevel>) -> El {
        match (|| {
            ta_return!(El, String);
            let Some(TreeNode::Scalar(Node::Value(serde_json::Value::String(src)))) =
                maybe_get_field_or_literal_string(&config_at.data, &data_stack)? else {
                    return Ok(el("div"));
                };
            return Ok(style_export::leaf_view_date(style_export::LeafViewDateArgs {
                trans_align: config_at.trans_align,
                orientation: config_at.orientation,
                value: src,
                font_size: config_at.font_size.clone(),
            }).root);
        })() {
            Ok(e) => return e,
            Err(e) => return style_export::leaf_err_block(style_export::LeafErrBlockArgs {
                in_root: false,
                data: e,
            }).root,
        }
    }

    fn build_widget_time(&mut self, config_at: &WidgetTime, data_stack: &Vec<DataStackLevel>) -> El {
        match (|| {
            ta_return!(El, String);
            let Some(TreeNode::Scalar(Node::Value(serde_json::Value::String(src)))) =
                maybe_get_field_or_literal_string(&config_at.data, &data_stack)? else {
                    return Ok(el("div"));
                };
            return Ok(style_export::leaf_view_time(style_export::LeafViewTimeArgs {
                trans_align: config_at.trans_align,
                orientation: config_at.orientation,
                value: src,
                font_size: config_at.font_size.clone(),
            }).root);
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
        config_at: &WidgetPlayButton,
        data_id: &Vec<usize>,
        data_stack: &Vec<DataStackLevel>,
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
            let mime = meta.mime.as_ref().map(|x| x.as_str()).unwrap_or("");
            let mime = mime.split_once("/").unwrap_or((mime, ""));
            match mime {
                ("image", _) => {
                    media_type = PlaylistEntryMediaType::Image;
                },
                ("video", _) => {
                    media_type = PlaylistEntryMediaType::Video;
                },
                ("audio", _) => {
                    media_type = PlaylistEntryMediaType::Audio;
                },
                ("application", "epub+zip") => {
                    media_type = PlaylistEntryMediaType::Book;
                },
                ("application", "x-cbr") | ("application", "x-cbz") | ("application", "x-cb7") => {
                    media_type = PlaylistEntryMediaType::Comic;
                },
                _ => {
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
            self.playlist_add.push((data_id.clone(), PlaylistPushArg {
                name: shed!{
                    let Some(config_at) = &config_at.name_field else {
                        break None;
                    };
                    let Some(d) = maybe_get_field(config_at, data_stack) else {
                        break None;
                    };
                    break Some(unwrap_value_string(&d));
                },
                album: shed!{
                    let Some(config_at) = &config_at.album_field else {
                        break None;
                    };
                    let Some(d) = maybe_get_field(config_at, data_stack) else {
                        break None;
                    };
                    break Some(unwrap_value_string(&d));
                },
                artist: shed!{
                    let Some(config_at) = &config_at.artist_field else {
                        break None;
                    };
                    let Some(d) = maybe_get_field(config_at, data_stack) else {
                        break None;
                    };
                    break Some(unwrap_value_string(&d));
                },
                cover_source_url: cover_source_url.clone(),
                source_url: src_url,
                media_type: media_type,
            }));
            let out = style_export::leaf_view_play_button(style_export::LeafViewPlayButtonArgs {
                image: if config_at.show_image {
                    cover_source_url.map(|x| match x {
                        SourceUrl::Url(u) => u.clone(),
                        SourceUrl::File(f) => file_url(&state().env, &f),
                    })
                } else {
                    None
                },
                width: config_at.width.clone(),
                height: config_at.height.clone(),
                trans_align: config_at.trans_align,
                orientation: config_at.orientation.unwrap_or(Orientation::RightDown),
            }).root;
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
        config_at: &Widget,
        config_query_params: &BTreeMap<String, Vec<String>>,
        data_id: &Vec<usize>,
        data_stack: &Vec<DataStackLevel>,
    ) -> El {
        match config_at {
            Widget::Layout(config_at) => return self.build_widget_layout(
                pc,
                config_at,
                config_query_params,
                data_id,
                data_stack,
            ),
            Widget::DataRows(config_at) => return self.build_widget_data_rows(
                pc,
                config_at,
                config_query_params,
                data_id,
                data_stack,
            ),
            Widget::Text(config_at) => return self.build_widget_text(config_at, data_stack),
            Widget::Media(config_at) => return self.build_widget_media(config_at, data_stack),
            Widget::PlayButton(config_at) => return self.build_widget_play_button(
                pc,
                config_at,
                data_id,
                data_stack,
            ),
            Widget::Color(config_at) => return self.build_widget_color(config_at, data_stack),
            Widget::Date(config_at) => return self.build_widget_date(config_at, data_stack),
            Widget::Datetime(config_at) => return self.build_widget_datetime(config_at, data_stack),
            Widget::Time(config_at) => return self.build_widget_time(config_at, data_stack),
            Widget::Space => return style_export::leaf_space().root,
        }
    }
}

fn build_transport(pc: &mut ProcessingContext) -> El {
    let hover_time = Prim::new(None);

    fn get_mouse_pct(ev: &Event) -> (f64, f64, MouseEvent) {
        let element = ev.target().unwrap().dyn_into::<Element>().unwrap();
        let ev = ev.dyn_ref::<MouseEvent>().unwrap();
        let element_rect = element.get_bounding_client_rect();
        let percent_x = ((ev.client_x() as f64 - element_rect.x()) / element_rect.width().max(0.001)).clamp(0., 1.);
        let percent_y = ((ev.client_y() as f64 - element_rect.y()) / element_rect.width().max(0.001)).clamp(0., 1.);
        return (percent_x, percent_y, ev.clone());
    }

    fn get_mouse_time(ev: &Event) -> Option<f64> {
        let Some(max_time) = *state().playlist.0.media_max_time.borrow() else {
            return None;
        };
        let percent = get_mouse_pct(ev).0;
        return Some(max_time * percent);
    }

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
                        ).log("Error persisting session id");
                        sess_id
                    };
                    playlist_set_link(pc, &state().playlist, &sess_id);
                    sess_id
                },
            };
            let link = format!("{}link.html#{}{}", state().env.base_url, LINK_HASH_PREFIX, sess_id);
            let modal_res = style_export::cont_modal_view_share(style_export::ContModalViewShareArgs {
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
            modal_res.bg.ref_on("click", {
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
    button_play.ref_own(
        |out| link!((_pc = pc), (playing = state().playlist.0.playing.clone()), (), (out = out.weak()) {
            let out = out.upgrade()?;
            if playing.get() {
                out.ref_attr(&style_export::attr_state().value, &style_export::attr_state_playing().value);
            } else {
                out.ref_attr(&style_export::attr_state().value, "");
            }
        }),
    );

    // Seekbar
    let seekbar = transport_res.seekbar;
    seekbar.ref_on("mousemove", {
        let eg = pc.eg();
        let hover_time = hover_time.clone();
        move |ev| eg.event(|pc| {
            hover_time.set(pc, get_mouse_time(ev));
        }).unwrap()
    });
    seekbar.ref_on("mouseleave", {
        let eg = pc.eg();
        let hover_time = hover_time.clone();
        move |_| eg.event(|pc| {
            hover_time.set(pc, None);
        }).unwrap()
    });
    seekbar.ref_on("click", {
        let eg = pc.eg();
        move |ev| eg.event(|pc| {
            let Some(time) = get_mouse_time(ev) else {
                return;
            };
            playlist_seek(pc, &state().playlist, time);
        }).unwrap()
    });
    let seekbar_fill = transport_res.seekbar_fill;
    seekbar_fill.ref_attr("style", &format!("width: 0%;"));
    seekbar_fill.ref_own(|fill| link!(
        //. .
        (_pc = pc),
        (time = state().playlist.0.playing_time.clone(), max_time = state().playlist.0.media_max_time.clone()),
        (),
        (fill = fill.weak()) {
            let Some(max_time) = *max_time.borrow() else {
                return None;
            };
            let fill = fill.upgrade()?;
            fill.ref_attr("style", &format!("width: {}%;", *time.borrow() / max_time.max(0.0001) * 100.));
        }
    ));
    let seekbar_label = transport_res.seekbar_label;
    seekbar_label.ref_own(|label| link!(
        //. .
        (_pc = pc),
        (playing_time = state().playlist.0.playing_time.clone(), hover_time = hover_time.clone()),
        (),
        (label = label.weak()) {
            let label = label.upgrade()?;
            let time: f64;
            if let Some(t) = *hover_time.borrow() {
                time = t;
            } else {
                time = *playing_time.borrow();
            }
            label.text(&state().playlist.format_time(time));
        }
    ));

    // Assemble
    return transport_res.root;
}

#[derive(Clone)]
struct BuildViewBodyCommon {
    id: ViewId,
    config_at: WidgetRootDataRows,
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
) {
    let Some(body) = common.body.upgrade() else {
        return;
    };
    let Some(transport_slot) = common.transport_slot.upgrade() else {
        return;
    };
    body.ref_clear();
    playlist_clear(pc, &state().playlist);
    let mut build = Build {
        view_id: common.id.clone(),
        vs: common.view_ministate_state.clone(),
        param_data: param_data.clone(),
        restore_playlist_pos: restore_playlist_pos.clone(),
        playlist_add: Default::default(),
        want_media: false,
        have_media: common.have_media.clone(),
        transport_slot: transport_slot,
    };
    body.ref_push(
        build.build_widget_root_data_rows(
            pc,
            &common.config_at,
            &common.config_query_params,
            &vec![],
            &vec![DataStackLevel {
                data: TreeNode::Record(
                    param_data.iter().map(|(k, v)| (k.clone(), TreeNode::Scalar(v.clone()))).collect(),
                ),
                node_meta: Default::default(),
            }],
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
) -> Result<El, String> {
    return eg.event(|pc| {
        let vs = MinistateViewState(Rc::new(RefCell::new(MinistateViewState_ {
            view_id: id.clone(),
            title: title.clone(),
            pos: restore_playlist_pos.clone(),
            params: params.clone(),
        })));
        let transport_slot = style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root;
        let body = style_export::cont_view_root_rows(style_export::ContViewRootRowsArgs { rows: vec![] }).root;
        let common = Rc::new(BuildViewBodyCommon {
            id: id.clone(),
            view_ministate_state: vs.clone(),
            transport_slot: transport_slot.weak(),
            config_at: view.root,
            config_query_params: view.query_parameters,
            body: body.weak(),
            have_media: Rc::new(Cell::new(false)),
        });
        let params_debounce = Rc::new(RefCell::new(None));
        let param_data = Rc::new(RefCell::new(params));
        let mut param_els = vec![];
        for (k, v) in view.parameters {
            match v {
                shared::interface::config::view::ClientViewParam::Text => {
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
                        let eg = pc.eg();
                        let k = k.clone();
                        let input = pair.input.weak();
                        let common = common.clone();
                        let params_debounce = params_debounce.clone();
                        let param_data = param_data.clone();
                        move |_| *params_debounce.borrow_mut() = Some(Timeout::new(500, {
                            let input = input.clone();
                            let common = common.clone();
                            let param_data = param_data.clone();
                            let eg = eg.clone();
                            let k = k.clone();
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
                                    build_page_view_body(pc, &common, &*param_data.borrow(), None);
                                }).unwrap();
                            }
                        }))
                    });
                    param_els.push(pair.root);
                },
            }
        }
        build_page_view_body(pc, &common, &*param_data.borrow(), restore_playlist_pos);
        return Ok(style_export::cont_page_view(style_export::ContPageViewArgs {
            transport: Some(transport_slot),
            params: param_els,
            rows: body,
        }).root);
    }).unwrap();
}
