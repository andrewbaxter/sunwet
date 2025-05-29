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
        state::state,
    },
    crate::libnonlink::{
        api::req_post_json,
        ministate::{
            ministate_octothorpe,
            Ministate,
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
            Orientation,
            QueryOrField,
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
            ReqViewQuery,
            TreeNode,
        },
    },
    std::{
        cell::{
            Cell,
            RefCell,
        },
        collections::HashMap,
        rc::Rc,
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
    wasm_bindgen::{
        JsCast,
        JsValue,
    },
    web_sys::{
        console::log_1,
        DomParser,
        Element,
        Event,
        HtmlInputElement,
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

fn get_field(config_at: &String, data_stack: &Vec<DataStackLevel>) -> Result<TreeNode, String> {
    let Some(data_at) = maybe_get_field(config_at, data_stack) else {
        return Err(format!("No data in scope is a record or field `{}` didn't exist at any level", config_at));
    };
    return Ok(data_at);
}

fn get_field_or_literal(config_at: &FieldOrLiteral, data_stack: &Vec<DataStackLevel>) -> Result<TreeNode, String> {
    match config_at {
        FieldOrLiteral::Field(config_at) => return Ok(get_field(config_at, data_stack)?),
        FieldOrLiteral::Literal(config_at) => return Ok(TreeNode::Scalar(config_at.clone())),
    }
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

fn get_field_or_literal_string(
    config_at: &FieldOrLiteralString,
    data_stack: &Vec<DataStackLevel>,
) -> Result<TreeNode, String> {
    match config_at {
        FieldOrLiteralString::Field(config_at) => return Ok(get_field(config_at, data_stack)?),
        FieldOrLiteralString::Literal(config_at) => return Ok(
            TreeNode::Scalar(Node::Value(serde_json::Value::String(config_at.clone()))),
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

fn unwrap_value_move_url(
    title: &FieldOrLiteral,
    data_at: &TreeNode,
    data_stack: &Vec<DataStackLevel>,
    to_node: bool,
) -> Result<String, String> {
    match data_at {
        TreeNode::Array(v) => return Ok(serde_json::to_string(v).unwrap()),
        TreeNode::Record(v) => return Ok(serde_json::to_string(v).unwrap()),
        TreeNode::Scalar(v) => {
            if to_node {
                return Ok(ministate_octothorpe(&Ministate::NodeView(MinistateNodeView {
                    title: unwrap_value_string(&get_field_or_literal(title, data_stack)?),
                    node: v.clone(),
                })));
            }
            match v {
                Node::File(v) => return Ok(file_url(&state().env, v)),
                Node::Value(v) => match v {
                    serde_json::Value::String(v) => return Ok(v.clone()),
                    _ => return Ok(serde_json::to_string(v).unwrap()),
                },
            }
        },
    }
}

struct Build {
    menu_item_id: String,
    menu_item_title: String,
    param_data: HashMap<String, Node>,
    restore_playlist_pos: Option<PlaylistRestorePos>,
    playlist_add: Vec<(PlaylistIndex, PlaylistPushArg)>,
    have_media: Rc<Cell<bool>>,
    want_media: bool,
    transport_slot: El,
}

impl Build {
    fn build_widget_layout(
        &mut self,
        pc: &mut ProcessingContext,
        config_at: &WidgetLayout,
        data_id: &Vec<usize>,
        data_at: &Vec<DataStackLevel>,
    ) -> El {
        let mut children = vec![];
        for config_at in &config_at.elements {
            children.push(self.build_widget(pc, config_at, data_id, data_at));
        }
        return style_export::cont_view_list(style_export::ContViewListArgs {
            direction: config_at.direction,
            x_scroll: config_at.x_scroll,
            children: children,
            gap: config_at.gap.clone(),
        }).root;
    }

    fn build_widget_data_rows(
        &mut self,
        pc: &mut ProcessingContext,
        config_at: &WidgetDataRows,
        data_id: &Vec<usize>,
        data_at: &Vec<DataStackLevel>,
    ) -> El {
        return el_async({
            let menu_item_id = self.menu_item_id.clone();
            let menu_item_title = self.menu_item_title.clone();
            let param_data = self.param_data.clone();
            let restore_playlist_pos = self.restore_playlist_pos.clone();
            let eg = pc.eg();
            let transport_slot = self.transport_slot.clone();
            let have_media = self.have_media.clone();
            let config_at = config_at.clone();
            let data_id = data_id.clone();
            let data_at = data_at.clone();
            async move {
                let node_meta;
                let new_data_at_tops;
                match config_at.data {
                    QueryOrField::Field(config_at) => {
                        let TreeNode::Array(res) = get_field(&config_at, &data_at)? else {
                            return Err(format!("Data rows field [{}] must be an array, but it is not", config_at));
                        };
                        new_data_at_tops = res;
                        node_meta = Default::default();
                    },
                    QueryOrField::Query(config_at) => {
                        let mut params = HashMap::new();
                        for (k, config_at) in &config_at.params {
                            let TreeNode::Scalar(v) = get_field_or_literal(config_at, &data_at)? else {
                                return Err(
                                    format!(
                                        "Parameters must be scalars, but query paramter [{}] is not a scalar",
                                        serde_json::to_string(&config_at).unwrap()
                                    ),
                                );
                            };
                            params.insert(k.clone(), v);
                        }
                        let res = req_post_json(&state().env.base_url, ReqViewQuery {
                            menu_item_id: menu_item_id.clone(),
                            query: config_at.query.clone(),
                            parameters: params,
                        }).await?;
                        let mut out = vec![];
                        for v in res.records {
                            out.push(TreeNode::Record(v));
                        }
                        new_data_at_tops = out;
                        node_meta = Rc::new(res.meta);
                    },
                };
                return eg.event(move |pc| {
                    let mut build = Build {
                        menu_item_id: menu_item_id.clone(),
                        menu_item_title: menu_item_title.clone(),
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
                                children.push(build.build_widget(pc, &row_widget.widget, &data_id, &data_at));
                            }
                            out = style_export::cont_view_list(style_export::ContViewListArgs {
                                direction: row_widget.direction.unwrap_or(Direction::Down),
                                x_scroll: row_widget.x_scroll,
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
                                    let column = build.build_widget(pc, config_at, &data_id, &data_at);
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
                    playlist_extend(
                        pc,
                        &state().playlist,
                        &menu_item_id,
                        &menu_item_title,
                        &param_data,
                        build.playlist_add,
                        &restore_playlist_pos,
                    );
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
        data_id: &Vec<usize>,
        data_at: &Vec<DataStackLevel>,
    ) -> El {
        return el_async({
            let menu_item_id = self.menu_item_id.clone();
            let menu_item_title = self.menu_item_title.clone();
            let param_data = self.param_data.clone();
            let restore_playlist_pos = self.restore_playlist_pos.clone();
            let eg = pc.eg();
            let transport_slot = self.transport_slot.clone();
            let have_media = self.have_media.clone();
            let config_at = config_at.clone();
            let data_id = data_id.clone();
            let data_at = data_at.clone();
            async move {
                let node_meta;
                let new_data_at_tops;
                match config_at.data {
                    QueryOrField::Field(config_at) => {
                        let TreeNode::Array(res) = get_field(&config_at, &data_at)? else {
                            return Err(format!("Data rows field [{}] must be an array, but it is not", config_at));
                        };
                        new_data_at_tops = res;
                        node_meta = Default::default();
                    },
                    QueryOrField::Query(config_at) => {
                        let mut params = HashMap::new();
                        for (k, config_at) in &config_at.params {
                            let TreeNode::Scalar(v) = get_field_or_literal(config_at, &data_at)? else {
                                return Err(
                                    format!(
                                        "Parameters must be scalars, but query paramter [{}] is not a scalar",
                                        serde_json::to_string(&config_at).unwrap()
                                    ),
                                );
                            };
                            params.insert(k.clone(), v);
                        }
                        let res = req_post_json(&state().env.base_url, ReqViewQuery {
                            menu_item_id: menu_item_id.clone(),
                            query: config_at.query.clone(),
                            parameters: params,
                        }).await?;
                        let mut out = vec![];
                        for v in res.records {
                            out.push(TreeNode::Record(v));
                        }
                        new_data_at_tops = out;
                        node_meta = Rc::new(res.meta);
                    },
                }
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
                let mut chunked_data = chunked_data.into_iter();
                let body = style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root;
                build_infinite(body.clone(), chunked_data.next().unwrap(), {
                    move |chunk| {
                        let children = eg.event(|pc| {
                            let mut build = Build {
                                menu_item_id: menu_item_id.clone(),
                                menu_item_title: menu_item_title.clone(),
                                param_data: param_data.clone(),
                                restore_playlist_pos: restore_playlist_pos.clone(),
                                playlist_add: Default::default(),
                                want_media: false,
                                have_media: have_media.clone(),
                                transport_slot: transport_slot.clone(),
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
                                        build.build_widget(pc, &config_at.widget, &data_id, &data_at);
                                    blocks.push(style_export::cont_view_block(style_export::ContViewBlockArgs {
                                        children: vec![block_contents],
                                        width: config_at.width.clone(),
                                    }).root);
                                }
                                children.push(
                                    style_export::cont_view_row(
                                        style_export::ContViewRowArgs { blocks: blocks },
                                    ).root,
                                );
                            }
                            playlist_extend(
                                pc,
                                &state().playlist,
                                &menu_item_id,
                                &menu_item_title,
                                &param_data,
                                build.playlist_add,
                                &restore_playlist_pos,
                            );
                            if !build.have_media.get() && build.want_media {
                                build.transport_slot.ref_push(build_transport(pc));
                                build.have_media.set(true);
                            }
                            return children;
                        }).unwrap();
                        let next_key = chunked_data.next();
                        async move {
                            Ok((next_key, children))
                        }
                    }
                });
                return Ok(vec![body]);
            }
        });
    }

    fn build_widget_text(&mut self, config_at: &WidgetText, data_at: &Vec<DataStackLevel>) -> El {
        match (|| {
            ta_return!(El, String);
            return Ok(style_export::leaf_view_text(style_export::LeafViewTextArgs {
                trans_align: config_at.trans_align,
                orientation: config_at.orientation,
                text: format!(
                    "{}{}{}",
                    config_at.prefix,
                    unwrap_value_string(&get_field_or_literal_string(&config_at.data, data_at)?),
                    config_at.suffix
                ),
                font_size: config_at.font_size.clone(),
                max_size: config_at.cons_size_max.clone(),
                link: config_at
                    .link
                    .as_ref()
                    .map(
                        |l| Ok(
                            unwrap_value_move_url(
                                &l.title,
                                &get_field_or_literal(&l.value, data_at)?,
                                data_at,
                                l.to_node,
                            )?,
                        ) as
                            Result<_, String>,
                    )
                    .transpose()?,
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
            match meta.mime.split("/").next().unwrap() {
                "image" => {
                    return Ok(style_export::leaf_view_image(style_export::LeafViewImageArgs {
                        trans_align: config_at.trans_align,
                        src: match unwrap_value_media_url(&src)? {
                            SourceUrl::Url(v) => v,
                            SourceUrl::File(v) => file_url(&state().env, &v),
                        },
                        link: shed!{
                            let Some(l) = &config_at.link else {
                                break None;
                            };
                            let Some(d) = maybe_get_field_or_literal(&l.value, &data_stack)? else {
                                break None;
                            };
                            break Some(unwrap_value_move_url(&l.title, &d, data_stack, l.to_node)?);
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
                            let Some(l) = &config_at.link else {
                                break None;
                            };
                            let Some(d) = maybe_get_field_or_literal(&l.value, &data_stack)? else {
                                break None;
                            };
                            break Some(unwrap_value_move_url(&l.title, &d, data_stack, l.to_node)?);
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
                            let Some(l) = &config_at.link else {
                                break None;
                            };
                            let Some(d) = maybe_get_field_or_literal(&l.value, &data_stack)? else {
                                break None;
                            };
                            break Some(unwrap_value_move_url(&l.title, &d, data_stack, l.to_node)?);
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
            let TreeNode::Scalar(Node::Value(serde_json::Value::String(src))) =
                get_field_or_literal_string(&config_at.data, &data_stack)? else {
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
            let TreeNode::Scalar(Node::Value(serde_json::Value::String(src))) =
                get_field_or_literal_string(&config_at.data, &data_stack)? else {
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
            let TreeNode::Scalar(Node::Value(serde_json::Value::String(src))) =
                get_field_or_literal_string(&config_at.data, &data_stack)? else {
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
            let TreeNode::Scalar(Node::Value(serde_json::Value::String(src))) =
                get_field_or_literal_string(&config_at.data, &data_stack)? else {
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
            let src = get_field(&config_at.media_file_field, data_stack)?;
            let media_type;
            let TreeNode::Scalar(src) = &src else {
                return Ok(el("div"));
            };
            let Some(meta) = maybe_get_meta(data_stack, src) else {
                return Ok(el("div"));
            };
            match meta.mime.split("/").next().unwrap() {
                "image" => {
                    media_type = PlaylistEntryMediaType::Image;
                },
                "video" => {
                    media_type = PlaylistEntryMediaType::Video;
                },
                "audio" => {
                    media_type = PlaylistEntryMediaType::Audio;
                },
                _ => {
                    return Ok(el("div"));
                },
            }
            self.want_media = true;
            let src_url = unwrap_value_media_url(&src)?;
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
                cover_source_url: shed!{
                    let Some(config_at) = &config_at.cover_field else {
                        break None;
                    };
                    log_1(&JsValue::from(format!("got cover field config: {}", config_at)));
                    let Some(d) = maybe_get_field(config_at, data_stack) else {
                        break None;
                    };
                    let TreeNode::Scalar(d) = d else {
                        break None;
                    };
                    log_1(&JsValue::from(format!("got cover field data: {:?}", d)));
                    break Some(unwrap_value_media_url(&d).map_err(|e| format!("Building cover url: {}", e))?);
                },
                source_url: src_url,
                media_type: media_type,
            }));
            let out = style_export::leaf_view_play_button(style_export::LeafViewPlayButtonArgs {
                trans_align: config_at.trans_align,
                orientation: config_at.orientation.unwrap_or(Orientation::RightDown),
            }).root;
            out.ref_on("click", {
                let data_id = data_id.clone();
                let eg = pc.eg();
                move |_| eg.event(|pc| {
                    log_1(&JsValue::from("Press play button"));
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
                        out.ref_modify_classes(
                            &[
                                (
                                    style_export::class_state_playing().value.as_ref(),
                                    playing.get() && playing_i.get().as_ref() == Some(index),
                                ),
                            ],
                        );
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
        data_id: &Vec<usize>,
        data_stack: &Vec<DataStackLevel>,
    ) -> El {
        match config_at {
            Widget::Layout(config_at) => return self.build_widget_layout(pc, config_at, data_id, data_stack),
            Widget::DataRows(config_at) => return self.build_widget_data_rows(pc, config_at, data_id, data_stack),
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
    let button_share = transport_res.button_share;
    button_share.ref_on("click", {
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
            let bg_el = modal_res.bg;
            let button_close_el = modal_res.button_close;
            let button_unshare_el = modal_res.button_unshare;
            let modal_el = modal_res.root;
            button_close_el.ref_on("click", {
                let modal_el = modal_el.weak();
                let eg = pc.eg();
                move |_| eg.event(|_pc| {
                    let Some(modal_el) = modal_el.upgrade() else {
                        return;
                    };
                    modal_el.ref_replace(vec![]);
                }).unwrap()
            });
            bg_el.ref_on("click", {
                let modal_el = modal_el.weak();
                let eg = pc.eg();
                move |_| eg.event(|_pc| {
                    let Some(modal_el) = modal_el.upgrade() else {
                        return;
                    };
                    modal_el.ref_replace(vec![]);
                }).unwrap()
            });
            button_unshare_el.ref_on("click", {
                let modal_el = modal_el.weak();
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
            state().modal_stack.ref_push(modal_el.clone());
        }).unwrap()
    });
    button_share.ref_own(|b| link!((_pc = pc), (sharing = state().playlist.0.share.clone()), (), (ele = b.weak()), {
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
            out.ref_modify_classes(&[(style_export::class_state_playing().value.as_ref(), playing.get())]);
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
            let time = time as u64;
            let seconds = time % 60;
            let time = time / 60;
            let minutes = time % 60;
            let time = time / 60;
            let hours = time % 24;
            let days = time / 24;
            if days > 0 {
                label.text(&format!("{:02}:{:02}:{:02}:{:02}", days, hours, minutes, seconds));
            } else if hours > 0 {
                label.text(&format!("{:02}:{:02}:{:02}", hours, minutes, seconds));
            } else {
                label.text(&format!("{:02}:{:02}", minutes, seconds));
            }
        }
    ));

    // Assemble
    return transport_res.root;
}

#[derive(Clone)]
struct BuildViewBodyCommon {
    id: String,
    title: String,
    config_at: WidgetRootDataRows,
    body: WeakEl,
    transport_slot: WeakEl,
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
        menu_item_id: common.id.clone(),
        menu_item_title: common.title.clone(),
        param_data: param_data.clone(),
        restore_playlist_pos: restore_playlist_pos.clone(),
        playlist_add: Default::default(),
        want_media: false,
        have_media: Rc::new(Cell::new(false)),
        transport_slot: transport_slot,
    };
    body.ref_push(build.build_widget_root_data_rows(pc, &common.config_at, &vec![], &vec![DataStackLevel {
        data: TreeNode::Record(
            param_data.iter().map(|(k, v)| (k.clone(), TreeNode::Scalar(v.clone()))).collect(),
        ),
        node_meta: Default::default(),
    }]));
    playlist_extend(
        pc,
        &state().playlist,
        &common.id,
        &common.title,
        param_data,
        build.playlist_add,
        &restore_playlist_pos,
    );
}

pub fn build_page_view(
    eg: EventGraph,
    menu_item_title: String,
    view: ClientView,
    restore_playlist_pos: Option<PlaylistRestorePos>,
) -> Result<El, String> {
    return eg.event(|pc| {
        let common = Rc::new(BuildViewBodyCommon {
            id: view.id.clone(),
            title: menu_item_title.clone(),
            transport_slot: style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root.weak(),
            config_at: view.root,
            body: style_export::cont_view_root_rows(style_export::ContViewRootRowsArgs { rows: vec![] })
                .root
                .weak(),
        });
        let params_debounce = Rc::new(RefCell::new(None));
        let param_data;
        match &restore_playlist_pos {
            Some(p) => {
                param_data = Rc::new(RefCell::new(p.params.clone()));
            },
            None => {
                param_data = Rc::new(RefCell::new(HashMap::new()));
            },
        }
        let mut params = vec![];
        for (k, v) in view.parameters {
            match v {
                shared::interface::config::view::ClientViewParam::Text => {
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
                                param_data
                                    .borrow_mut()
                                    .insert(
                                        k,
                                        Node::Value(
                                            serde_json::Value::String(
                                                input.raw().dyn_into::<HtmlInputElement>().unwrap().value(),
                                            ),
                                        ),
                                    );
                                eg.event(|pc| {
                                    build_page_view_body(pc, &common, &*param_data.borrow(), None);
                                }).unwrap();
                            }
                        }))
                    });
                    params.push(pair.root);
                },
            }
        }
        build_page_view_body(pc, &common, &*param_data.borrow(), restore_playlist_pos);
        return Ok(style_export::cont_page_view(style_export::ContPageViewArgs {
            transport: Some(common.transport_slot.upgrade().unwrap()),
            params: params,
            rows: common.body.upgrade().unwrap(),
        }).root);
    }).unwrap();
}
