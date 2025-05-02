use {
    super::{
        ministate::PlaylistRestorePos,
        playlist::PlaylistIndex,
        state::{
            set_page,
            state,
        },
    },
    crate::libnonlink::{
        api::req_post_json,
        ministate::{
            ministate_octothorpe,
            Ministate,
            MinistateEdit,
        },
        playlist::{
            playlist_extend,
            playlist_next,
            playlist_previous,
            playlist_seek,
            playlist_toggle_play,
            PlaylistEntryMediaType,
            PlaylistPushArg,
        },
    },
    flowcontrol::ta_return,
    lunk::{
        link,
        Prim,
        ProcessingContext,
    },
    qrcode::QrCode,
    rooting::{
        el_from_raw,
        El,
    },
    shared::interface::{
        config::view::{
            Direction,
            FieldOrLiteral,
            FieldOrLiteralString,
            QueryOrField,
            Widget,
            WidgetDataRows,
            WidgetImage,
            WidgetLayout,
            WidgetPlayButton,
            WidgetText,
        },
        triple::Node,
        wire::{
            link::SourceUrl,
            ReqViewQuery,
            TreeNode,
        },
    },
    std::collections::HashMap,
    uuid::Uuid,
    wasm::{
        constants::LINK_HASH_PREFIX,
        js::{
            el_async,
            style_export::{
                self,
            },
        },
        ont::{
            ROOT_AUDIO_VALUE,
            ROOT_IMAGE_VALUE,
            ROOT_VIDEO_VALUE,
        },
        websocket::Ws,
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
        HtmlElement,
        MouseEvent,
    },
};

fn maybe<I, O>(v: &Option<I>, f: impl FnOnce(&I) -> Result<O, String>) -> Result<Option<O>, String> {
    match v {
        Some(v) => match f(v) {
            Ok(v) => Ok(Some(v)),
            Err(v) => Err(v),
        },
        None => return Ok(None),
    }
}

fn get_field(config_at: &String, data_stack: &Vec<TreeNode>) -> Result<TreeNode, String> {
    for data_at in data_stack.iter().rev() {
        let TreeNode::Record(data_at) = data_at else {
            continue;
        };
        let Some(data_at) = data_at.get(config_at) else {
            continue;
        };
        return Ok(data_at.clone());
    }
    return Err(format!("No data in scope is a record or field `{}` didn't exist at any level", config_at));
}

fn get_field_or_literal(config_at: &FieldOrLiteral, data_stack: &Vec<TreeNode>) -> Result<TreeNode, String> {
    match config_at {
        FieldOrLiteral::Field(config_at) => return Ok(get_field(config_at, data_stack)?),
        FieldOrLiteral::Literal(config_at) => return Ok(TreeNode::Scalar(config_at.clone())),
    }
}

fn get_field_or_literal_string(
    config_at: &FieldOrLiteralString,
    data_stack: &Vec<TreeNode>,
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

fn unwrap_value_media_url(data_at: &TreeNode) -> Result<SourceUrl, String> {
    match data_at {
        TreeNode::Array(v) => return Err(
            format!("Url value is an array, not a string: {}", serde_json::to_string(v).unwrap()),
        ),
        TreeNode::Record(v) => return Err(
            format!("Url value is a record, not a string: {}", serde_json::to_string(v).unwrap()),
        ),
        TreeNode::Scalar(v) => {
            match v {
                Node::File(v) => return Ok(SourceUrl {
                    url: file_url(&state().base_url, v),
                    file: Some(v.clone()),
                }),
                Node::Value(v) => match v {
                    serde_json::Value::String(v) => return Ok(SourceUrl {
                        url: v.clone(),
                        file: None,
                    }),
                    _ => return Err(format!("Url is not a string: {}", serde_json::to_string(v).unwrap())),
                },
            }
        },
    }
}

fn unwrap_value_move_url(
    title: &FieldOrLiteral,
    data_at: &TreeNode,
    data_stack: &Vec<TreeNode>,
    to_node: bool,
) -> Result<String, String> {
    match data_at {
        TreeNode::Array(v) => return Ok(serde_json::to_string(v).unwrap()),
        TreeNode::Record(v) => return Ok(serde_json::to_string(v).unwrap()),
        TreeNode::Scalar(v) => {
            if to_node {
                return Ok(ministate_octothorpe(&Ministate::Edit(MinistateEdit {
                    title: unwrap_value_string(&get_field_or_literal(title, data_stack)?),
                    node: v.clone(),
                })));
            }
            match v {
                Node::File(v) => return Ok(file_url(&state().base_url, v)),
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
    restore_playlist_pos: Option<PlaylistRestorePos>,
    playlist_add: Vec<(PlaylistIndex, PlaylistPushArg)>,
    have_media: bool,
    transport_slot: El,
}

impl Build {
    fn build_widget_layout(
        &mut self,
        pc: &mut ProcessingContext,
        config_at: &WidgetLayout,
        data_id: &Vec<usize>,
        data_at: &Vec<TreeNode>,
    ) -> El {
        let mut children_raw = vec![];
        let mut children = vec![];
        for config_at in &config_at.elements {
            let child_el = self.build_widget(pc, config_at, data_id, data_at);
            children_raw.push(child_el.raw().dyn_into::<HtmlElement>().unwrap());
            children.push(child_el);
        }
        return el_from_raw(style_export::cont_view_list(style_export::ContViewListArgs {
            direction: config_at.direction,
            x_scroll: config_at.x_scroll,
            children: children_raw,
            gap: config_at.gap.clone(),
        }).root.into()).own(|_| children);
    }

    fn build_widget_data_rows(
        &mut self,
        pc: &mut ProcessingContext,
        config_at: &WidgetDataRows,
        data_id: &Vec<usize>,
        data_at: &Vec<TreeNode>,
    ) -> El {
        return el_async({
            let menu_item_id = self.menu_item_id.clone();
            let menu_item_title = self.menu_item_title.clone();
            let restore_playlist_pos = self.restore_playlist_pos.clone();
            let eg = pc.eg();
            let transport_slot = self.transport_slot.clone();
            let old_have_media = self.have_media;
            let config_at = config_at.clone();
            let data_id = data_id.clone();
            let data_at = data_at.clone();
            async move {
                let new_data_at_tops = match config_at.data {
                    QueryOrField::Field(config_at) => {
                        let TreeNode::Array(res) = get_field(&config_at, &data_at)? else {
                            return Err(format!("Data rows field [{}] must be an array, but it is not", config_at));
                        };
                        res
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
                        let res = req_post_json(&state().base_url, ReqViewQuery {
                            menu_item_id: menu_item_id.clone(),
                            query: config_at.query.clone(),
                            parameters: params,
                        }).await?;
                        let mut out = vec![];
                        for v in res.records {
                            out.push(TreeNode::Record(v));
                        }
                        out
                    },
                };
                return eg.event(move |pc| {
                    let mut build = Build {
                        menu_item_id: menu_item_id.clone(),
                        menu_item_title: menu_item_title.clone(),
                        restore_playlist_pos: restore_playlist_pos.clone(),
                        playlist_add: Default::default(),
                        have_media: false,
                        transport_slot: transport_slot,
                    };
                    let out;
                    match &config_at.row_widget {
                        shared::interface::config::view::DataRowsLayout::Unaligned(row_widget) => {
                            let mut children = vec![];
                            let mut children_raw = vec![];
                            for (i, new_data_at_top) in new_data_at_tops.into_iter().enumerate() {
                                let mut data_at = data_at.clone();
                                data_at.push(new_data_at_top);
                                let mut data_id = data_id.clone();
                                data_id.push(i);
                                let child = build.build_widget(pc, &row_widget.widget, &data_id, &data_at);
                                children_raw.push(child.raw().dyn_into::<HtmlElement>().unwrap());
                                children.push(child);
                            }
                            out = el_from_raw(style_export::cont_view_list(style_export::ContViewListArgs {
                                direction: row_widget.direction.unwrap_or(Direction::Down),
                                x_scroll: row_widget.x_scroll,
                                children: children_raw,
                                gap: row_widget.gap.clone(),
                            }).root.into()).own(|_| children);
                        },
                        shared::interface::config::view::DataRowsLayout::Table(row_widget) => {
                            let mut rows = vec![];
                            let mut rows_raw = vec![];
                            for (i, new_data_at_top) in new_data_at_tops.into_iter().enumerate() {
                                let mut data_at = data_at.clone();
                                data_at.push(new_data_at_top);
                                let mut data_id = data_id.clone();
                                data_id.push(i);
                                let mut columns = vec![];
                                let mut columns_raw = vec![];
                                for config_at in &row_widget.elements {
                                    let column = build.build_widget(pc, config_at, &data_id, &data_at);
                                    columns_raw.push(column.raw().dyn_into::<HtmlElement>().unwrap());
                                    columns.push(column);
                                }
                                rows.push(columns);
                                rows_raw.push(columns_raw);
                            }
                            out = el_from_raw(style_export::cont_view_table(style_export::ContViewTableArgs {
                                orientation: row_widget.orientation,
                                x_scroll: row_widget.x_scroll,
                                children: rows_raw,
                                gap: row_widget.gap.clone(),
                            }).root.into()).own(|_| rows);
                        },
                    }
                    playlist_extend(
                        pc,
                        &state().playlist,
                        &menu_item_id,
                        &menu_item_title,
                        build.playlist_add,
                        &restore_playlist_pos,
                    );
                    if build.have_media && !old_have_media {
                        build.transport_slot.ref_push(build_transport(pc));
                    }
                    return Ok(out);
                }).unwrap();
            }
        });
    }

    fn build_widget_text(&mut self, config_at: &WidgetText, data_at: &Vec<TreeNode>) -> El {
        match (|| {
            ta_return!(El, String);
            return Ok(el_from_raw(style_export::leaf_view_text(style_export::LeafViewTextArgs {
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
            }).root.into()));
        })() {
            Ok(e) => return e,
            Err(e) => return el_from_raw(
                style_export::leaf_err_block(style_export::LeafErrBlockArgs { data: e }).root.into(),
            ),
        }
    }

    fn build_widget_image(&mut self, config_at: &WidgetImage, data_stack: &Vec<TreeNode>) -> El {
        match (|| {
            ta_return!(El, String);
            return Ok(el_from_raw(style_export::leaf_view_image(style_export::LeafViewImageArgs {
                trans_align: config_at.trans_align,
                src: unwrap_value_media_url(&get_field_or_literal(&config_at.data, &data_stack)?)?.url,
                link: maybe(
                    &config_at.link,
                    |l| Ok(
                        unwrap_value_move_url(
                            &l.title,
                            &get_field_or_literal(&l.value, &data_stack)?,
                            data_stack,
                            l.to_node,
                        )?,
                    ),
                )?,
                text: maybe(
                    &config_at.alt,
                    |v| Ok(unwrap_value_string(&get_field_or_literal(v, data_stack)?)) as Result<_, String>,
                )?,
                width: config_at.width.clone(),
                height: config_at.height.clone(),
            }).root.into()));
        })() {
            Ok(e) => return e,
            Err(e) => return el_from_raw(
                style_export::leaf_err_block(style_export::LeafErrBlockArgs { data: e }).root.into(),
            ),
        }
    }

    fn build_widget_play_button(
        &mut self,
        pc: &mut ProcessingContext,
        config_at: &WidgetPlayButton,
        data_id: &Vec<usize>,
        data_stack: &Vec<TreeNode>,
    ) -> El {
        match (|| {
            ta_return!(El, String);
            self.have_media = true;
            let media_type =
                match unwrap_value_string(
                    &get_field_or_literal(&config_at.media_type_field, data_stack)?,
                ).as_str() {
                    ROOT_AUDIO_VALUE => PlaylistEntryMediaType::Audio,
                    ROOT_IMAGE_VALUE => PlaylistEntryMediaType::Image,
                    ROOT_VIDEO_VALUE => PlaylistEntryMediaType::Video,
                    t => {
                        return Err(format!("Invalid media type: {}", t));
                    },
                };
            let src_url = unwrap_value_media_url(&get_field(&config_at.media_file_field, data_stack)?)?;
            self.playlist_add.push((data_id.clone(), PlaylistPushArg {
                name: maybe(
                    &config_at.name_field,
                    |config_at| Ok(unwrap_value_string(&get_field(config_at, data_stack)?)),
                )?,
                album: maybe(
                    &config_at.album_field,
                    |config_at| Ok(unwrap_value_string(&get_field(config_at, data_stack)?)),
                )?,
                artist: maybe(
                    &config_at.artist_field,
                    |config_at| Ok(unwrap_value_string(&get_field(config_at, data_stack)?)),
                )?,
                cover_source_url: maybe(
                    &config_at.cover_field,
                    |config_at| Ok(unwrap_value_media_url(&get_field(config_at, data_stack)?)?),
                )?,
                source_url: src_url,
                media_type: media_type,
            }));
            let out = el_from_raw(style_export::leaf_view_play_button(style_export::LeafViewPlayButtonArgs {
                trans_align: config_at.trans_align,
                direction: config_at.direction.unwrap_or(Direction::Right),
            }).root.into());
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
            Err(e) => return el_from_raw(
                style_export::leaf_err_block(style_export::LeafErrBlockArgs { data: e }).root.into(),
            ),
        }
    }

    fn build_widget(
        &mut self,
        pc: &mut ProcessingContext,
        config_at: &Widget,
        data_id: &Vec<usize>,
        data_stack: &Vec<TreeNode>,
    ) -> El {
        match config_at {
            Widget::Layout(config_at) => return self.build_widget_layout(pc, config_at, data_id, data_stack),
            Widget::DataRows(config_at) => return self.build_widget_data_rows(pc, config_at, data_id, data_stack),
            Widget::Text(config_at) => return self.build_widget_text(config_at, data_stack),
            Widget::Image(config_at) => return self.build_widget_image(config_at, data_stack),
            Widget::PlayButton(config_at) => return self.build_widget_play_button(
                pc,
                config_at,
                data_id,
                data_stack,
            ),
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
        let Some(max_time) = *state().playlist.0.playing_max_time.borrow() else {
            return None;
        };
        let percent = get_mouse_pct(ev).0;
        return Some(max_time * percent);
    }

    let transport_res = style_export::cont_bar_view_transport();
    let button_share = el_from_raw(transport_res.button_share.into());
    button_share.ref_on("click", {
        let eg = pc.eg();
        move |_| eg.event(|pc| {
            let sess_id = state().playlist.0.share.borrow().as_ref().map(|x| x.0.clone());
            let sess_id = match sess_id {
                Some(sess_id) => {
                    sess_id.clone()
                },
                None => {
                    let id = Uuid::new_v4().to_string();
                    state()
                        .playlist
                        .0
                        .share
                        .set(pc, Some((id.clone(), Ws::new(format!("main/{}", id), |_, _| unreachable!()))));
                    id
                },
            };
            let link = format!("link.html{}#{}{}", state().base_url, LINK_HASH_PREFIX, sess_id);
            let modal_res = style_export::cont_modal_view_share(style_export::ContModalViewShareArgs {
                qr: DomParser::new()
                    .unwrap()
                    .parse_from_string(
                        &QrCode::new(&link)
                            .unwrap()
                            .render::<qrcode::render::svg::Color>()
                            .quiet_zone(false)
                            .build(),
                        web_sys::SupportedType::ImageSvgXml,
                    )
                    .unwrap()
                    .first_element_child()
                    .unwrap()
                    .dyn_into()
                    .unwrap(),
                link: link,
            });
            let bg_el = el_from_raw(modal_res.bg.into());
            let button_close_el = el_from_raw(modal_res.button_close.into());
            let button_unshare_el = el_from_raw(modal_res.button_unshare.into());
            let modal_el =
                el_from_raw(
                    modal_res.root.into(),
                ).own(|_| (bg_el.clone(), button_close_el.clone(), button_unshare_el.clone()));
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
                }).unwrap()
            });
            state().modal_stack.ref_push(modal_el.clone());
        }).unwrap()
    });

    // Prev
    let button_prev = el_from_raw(transport_res.button_prev.into());
    button_prev.ref_on("click", {
        let eg = pc.eg();
        move |_| eg.event(|pc| {
            playlist_previous(pc, &state().playlist, None);
        }).unwrap()
    });

    // Next
    let button_next = el_from_raw(transport_res.button_next.into());
    button_next.ref_on("click", {
        let eg = pc.eg();
        move |_| eg.event(|pc| {
            playlist_next(pc, &state().playlist, None);
        }).unwrap()
    });

    // Play
    let button_play = el_from_raw(transport_res.button_play.into());
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
    let seekbar = el_from_raw(transport_res.seekbar.into());
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
    let seekbar_fill = el_from_raw(transport_res.seekbar_fill.into());
    seekbar_fill.ref_own(|fill| link!(
        //. .
        (_pc = pc),
        (time = state().playlist.0.playing_time.clone(), max_time = state().playlist.0.playing_max_time.clone()),
        (),
        (fill = fill.weak()) {
            let Some(max_time) = *max_time.borrow() else {
                return None;
            };
            let fill = fill.upgrade()?;
            fill.ref_attr("style", &format!("width: {}%;", *time.borrow() / max_time.max(0.0001) * 100.));
        }
    ));
    let seekbar_label = el_from_raw(transport_res.seekbar_label.into());
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
    return el_from_raw(
        transport_res.root.into(),
    ).own(|_| (button_share, button_prev, button_next, button_play, seekbar, seekbar_fill));
}

pub fn build_page_view(
    pc: &mut ProcessingContext,
    menu_item_title: &str,
    menu_item_id: &str,
    restore_playlist_pos: Option<PlaylistRestorePos>,
) {
    set_page(pc, menu_item_title, el_async({
        let menu_item_id = menu_item_id.to_string();
        let menu_item_title = menu_item_title.to_string();
        let eg = pc.eg();
        async move {
            // # Async
            let client_config = state().client_config.get().await?;
            let Some(view) = client_config.views.get(&menu_item_id) else {
                return Err(format!("No view with id [{}] in config", menu_item_id));
            };

            // # Content
            return eg.event(|pc| {
                let mut build = Build {
                    menu_item_id: menu_item_id.clone(),
                    menu_item_title: menu_item_title.clone(),
                    restore_playlist_pos: restore_playlist_pos.clone(),
                    playlist_add: Default::default(),
                    have_media: false,
                    transport_slot: el_from_raw(
                        style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root.into(),
                    ),
                };
                let data_rows_res =
                    build.build_widget_data_rows(
                        pc,
                        &view.config,
                        &vec![],
                        &vec![TreeNode::Record(Default::default())],
                    );
                playlist_extend(
                    pc,
                    &state().playlist,
                    &menu_item_id,
                    &menu_item_title,
                    build.playlist_add,
                    &restore_playlist_pos,
                );
                return Ok(el_from_raw(style_export::cont_page_view_list(style_export::ContPageViewListArgs {
                    transport: Some(build.transport_slot.raw().dyn_into().unwrap()),
                    rows: data_rows_res.raw().dyn_into().unwrap(),
                }).root.into()).own(|_| (build.transport_slot, data_rows_res)));
            }).unwrap();
        }
    }));
}
