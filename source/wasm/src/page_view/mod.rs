use {
    self::elements::{
        el_image_err,
        el_media_button,
        el_media_button_err,
        style_tree,
        CSS_TREE_IMAGE,
        CSS_TREE_LAYOUT_INDIVIDUAL,
        CSS_TREE_LAYOUT_TABLE,
        CSS_TREE_MEDIA_BUTTON,
        CSS_TREE_NEST,
        CSS_TREE_TEXT,
    },
    super::{
        ministate::{
            PlaylistEntryPath,
            PlaylistPos,
        },
        playlist::{
            ImagePlaylistMedia,
            PlaylistEntryMediaType,
        },
    },
    crate::{
        constants::LINK_HASH_PREFIX,
        el_general::{
            el_async,
            el_audio,
            el_button_icon,
            el_button_icon_switch,
            el_button_icon_text,
            el_err_block,
            el_err_span,
            el_hbox,
            el_hscroll,
            el_icon,
            el_modal,
            el_stack,
            el_vbox,
            el_video,
            log,
            CSS_STATE_GROW,
            ICON_NOSHARE,
            ICON_SHARE,
            ICON_TRANSPORT_NEXT,
            ICON_TRANSPORT_PAUSE,
            ICON_TRANSPORT_PLAY,
            ICON_TRANSPORT_PREVIOUS,
            ICON_VOLUME,
        },
        ont::{
            ROOT_AUDIO_VALUE,
            ROOT_IMAGE_VALUE,
            ROOT_VIDEO_VALUE,
        },
        playlist::{
            playlist_clear,
            playlist_len,
            playlist_next,
            playlist_previous,
            playlist_push,
            playlist_seek,
            playlist_toggle_play,
            AudioPlaylistMedia,
            PlaylistEntry,
            VideoPlaylistMedia,
        },
        state::State,
        util::OptString,
        websocket::Ws,
        world::{
            file_url,
            generated_file_url,
            req_post_json,
        },
    },
    chrono::{
        Duration,
        Utc,
    },
    flowcontrol::{
        shed,
        superif,
    },
    gloo::{
        timers::future::TimeoutFuture,
        utils::window,
    },
    lunk::{
        link,
        Prim,
        ProcessingContext,
    },
    qrcode::QrCode,
    rooting::{
        el,
        el_from_raw,
        spawn_rooted,
        El,
        ScopeValue,
    },
    shared::interface::{
        config::view::{
            Direction,
            FieldOrLiteral,
            Layout,
            LineSizeMode,
            QueryDefParameter,
            QueryOrField,
            View,
            ViewPartList,
            Widget,
            WidgetNest,
        },
        query::Query,
        triple::{
            FileHash,
            Node,
        },
        wire::{
            ReqQuery,
            TreeNode,
        },
    },
    std::{
        cell::RefCell,
        collections::{
            BTreeMap,
            HashMap,
        },
        rc::Rc,
        str::FromStr,
    },
    uuid::Uuid,
    wasm_bindgen::JsCast,
    web_sys::{
        DomParser,
        Element,
        Event,
        HtmlInputElement,
        HtmlMediaElement,
        MouseEvent,
        SupportedType,
    },
};

pub mod elements;

fn query_res_as_file(r: &TreeNode) -> Result<FileHash, String> {
    let TreeNode::Scalar(Node::File(n)) = r else {
        return Err(format!("Field contents wasn't a string: {:?}", r));
    };
    return Ok(n.clone());
}

pub fn build_widget_query(
    pc: &mut ProcessingContext,
    state: &State,
    queries: &Rc<BTreeMap<String, Query>>,
    depth: usize,
    def: &ViewPartList,
    data: &BTreeMap<String, TreeNode>,
    build_playlist_pos: &BuildPlaylistPos,
    restore_playlist_pos: &Option<PlaylistPos>,
) -> El {
    return el_async().own(|e| spawn_rooted({
        let def = def.clone();
        let source_data = data.clone();
        let queries = queries.clone();
        let state = state.clone();
        let eg = pc.eg();
        let e = e.weak();
        let build_playlist_pos = build_playlist_pos.clone();
        let restore_playlist_pos = restore_playlist_pos.clone();
        async move {
            let Some(placeholder) = e.upgrade() else {
                return;
            };
            let rows;
            match &def.data {
                QueryOrField::Field(f) => {
                    let Some(key_value) = source_data.get(f) else {
                        placeholder.ref_replace(vec![
                            //. .
                            el_err_span("Specified field for list doesn't exist in parent row/parameter data".to_string())
                        ]);
                        return;
                    };
                    let TreeNode::Array(v) = key_value else {
                        placeholder.ref_replace(vec![
                            //. .
                            el_err_span(format!("Specified field for list is a scalar, not an array"))
                        ]);
                        return;
                    };
                    rows = v.clone();
                },
                QueryOrField::Query(query) => {
                    let mut parameters = HashMap::new();
                    for (k, v) in source_data.iter() {
                        let TreeNode::Scalar(n) = v else {
                            placeholder.ref_push(el_err_span(format!("Parameter for query is not a scalar")));
                            return;
                        };
                        parameters.insert(k.clone(), n.clone());
                    }
                    let Some(query) = queries.get(query) else {
                        placeholder.ref_push(el_err_span(format!("No query with id {}", query)));
                        return;
                    };
                    let res = req_post_json(&state.base_url, ReqQuery {
                        query: query.clone(),
                        parameters: parameters,
                    }).await;
                    placeholder.ref_clear();
                    let res = match res {
                        Ok(res) => res,
                        Err(e) => {
                            placeholder.ref_push(el_err_span(e));
                            return;
                        },
                    };
                    let TreeNode::Array(res) = res.records else {
                        placeholder.ref_push(el_err_span(format!("Query response is not an array (likely bug)")));
                        return;
                    };
                    rows = res;
                },
            }
            eg.event(|pc| {
                placeholder.ref_replace(vec![
                    //. .
                    build_layout(
                        pc,
                        &state,
                        &queries,
                        depth,
                        &def.layout,
                        &rows,
                        &def.key_field,
                        &build_playlist_pos,
                        &restore_playlist_pos,
                    )
                ]);
            });
        }
    }));
}

#[derive(Clone)]
pub struct BuildPlaylistPos {
    pub list_id: String,
    pub list_title: String,
    pub entry_path: Option<PlaylistEntryPath>,
}

impl BuildPlaylistPos {
    pub fn add(&self, a: Option<Node>) -> Self {
        return Self {
            list_id: self.list_id.clone(),
            list_title: self.list_title.clone(),
            entry_path: match (&self.entry_path, a) {
                (Some(ep), Some(a)) => {
                    let mut out = ep.0.clone();
                    out.push(a);
                    Some(PlaylistEntryPath(out))
                },
                _ => None,
            },
        };
    }
}

fn build_widget(
    pc: &mut ProcessingContext,
    state: &State,
    queries: &Rc<BTreeMap<String, Query>>,
    depth: usize,
    def: &Widget,
    data: &BTreeMap<String, TreeNode>,
    build_playlist_pos: &BuildPlaylistPos,
    restore_playlist_pos: &Option<PlaylistPos>,
) -> El {
    match def {
        Widget::Nest(d) => return build_nest(
            pc,
            state,
            queries,
            depth,
            d,
            data,
            build_playlist_pos,
            restore_playlist_pos,
        ),
        Widget::TextLine(d) => {
            let text;
            match &d.data {
                FieldOrLiteral::Field(field) => {
                    let Some(v) = data.get(field) else {
                        let out = el_err_span(format!("Missing field {}", field));
                        style_tree(CSS_TREE_TEXT, depth, d.align, &out);
                        return out;
                    };
                    text = match v {
                        TreeNode::Array(v) => serde_json::to_string(v).unwrap(),
                        TreeNode::Record(v) => serde_json::to_string(v).unwrap(),
                        TreeNode::Scalar(v) => match v {
                            Node::File(v) => {
                                v.to_string()
                            },
                            Node::Value(v) => match v {
                                serde_json::Value::Null => "-".to_string(),
                                serde_json::Value::Bool(v) => match v {
                                    true => "yes".to_string(),
                                    false => "no".to_string(),
                                },
                                serde_json::Value::Number(v) => v.to_string(),
                                serde_json::Value::String(v) => v.clone(),
                                serde_json::Value::Array(v) => serde_json::to_string(&v).unwrap(),
                                serde_json::Value::Object(v) => serde_json::to_string(&v).unwrap(),
                            },
                        },
                    };
                },
                FieldOrLiteral::Literal(v) => {
                    text = v.clone();
                },
            }
            let out = el("span").text(&format!("{}{}{}", d.prefix, text, d.suffix));
            style_tree(CSS_TREE_TEXT, depth, d.align, &out);
            out.ref_classes(&d.orientation.css());
            let mut style = vec![];
            style.push(format!("font-size: {}", d.size));
            match d.size_mode {
                LineSizeMode::Ellipsize => style.push(format!("text-overflow: ellipsis")),
                LineSizeMode::Wrap => style.push(format!("overflow-wrap: break-word")),
            }
            if let Some(size_max) = (&d.size_max).if_some() {
                match d.orientation.con() {
                    Direction::Up | Direction::Down => style.push(format!("max-height: {}", size_max)),
                    Direction::Left | Direction::Right => style.push(format!("max-width: {}", size_max)),
                }
            }
            out.ref_attr("style", &style.join("; "));
            return out;
        },
        Widget::Image(d) => {
            let url;
            match &d.data {
                FieldOrLiteral::Field(field) => {
                    let Some(v) = data.get(field) else {
                        return el_image_err(format!("Missing field {}", field));
                    };
                    superif!({
                        let TreeNode::Scalar(Node::Value(serde_json::Value::String(n))) = v else {
                            break 'bad;
                        };
                        let Ok(n) = FileHash::from_str(&n) else {
                            break 'bad;
                        };
                        url = file_url(&state.base_url, &n);
                    } 'bad {
                        return el_image_err(format!("Field contents wasn't file node: {:?}", v));
                    });
                },
                FieldOrLiteral::Literal(data) => {
                    url = data.clone();
                },
            }
            let out = el("img").attr("src", &url);
            style_tree(CSS_TREE_IMAGE, depth, d.align, &out);
            let mut style = vec![];
            if let Some(width) = (&d.width).if_some() {
                style.push(format!("width: {}", width));
            }
            if let Some(height) = (&d.height).if_some() {
                style.push(format!("height: {}", height));
            }
            out.ref_attr("style", &style.join("; "));
            return out;
        },
        Widget::MediaButton(d) => {
            let Some(v) = data.get(&d.field) else {
                return el_image_err(format!("Missing field {}", d.field));
            };
            let media_type;
            let media;
            match &d.media_field {
                FieldOrLiteral::Field(field) => {
                    let Some(v) = data.get(field) else {
                        return el_image_err(format!("Missing field media {}", d.field));
                    };
                    if let TreeNode::Scalar(Node::Value(serde_json::Value::String(m))) = v {
                        media_type = m.clone();
                    } else {
                        return el_image_err("Media field value not string".to_string());
                    }
                },
                FieldOrLiteral::Literal(v) => {
                    media_type = v.clone();
                },
            }
            let setup_media_element = |pc: &mut ProcessingContext, i: usize, media: &El| {
                media.ref_on("ended", {
                    let state = state.clone();
                    let eg = pc.eg();
                    move |_| eg.event(|pc| {
                        playlist_next(pc, &state.playlist, Some(i));
                    })
                }).ref_on("pause", {
                    let eg = pc.eg();
                    let state = state.playlist.weak();
                    move |_| eg.event(|pc| {
                        let Some(state) = state.upgrade() else {
                            return;
                        };
                        state.0.playing.set(pc, false);
                    })
                }).ref_on("play", {
                    let eg = pc.eg();
                    let state = state.playlist.weak();
                    move |_| eg.event(|pc| {
                        let Some(state) = state.upgrade() else {
                            return;
                        };
                        state.0.playing.set(pc, true);
                    })
                }).ref_on("volumechange", {
                    let eg = pc.eg();
                    let volume = state.playlist.0.volume.clone();
                    let debounce = state.playlist.0.volume_debounce.clone();
                    move |ev| {
                        if Utc::now().signed_duration_since(debounce.get()) < Duration::milliseconds(200) {
                            return;
                        }
                        eg.event(|pc| {
                            let v = ev.target().unwrap().dyn_ref::<HtmlMediaElement>().unwrap().volume();
                            volume.set(pc, (v / 2., v / 2.));
                        })
                    }
                });
                let restore_pos = shed!{
                    'restore_pos _;
                    shed!{
                        let Some(init) = restore_playlist_pos else {
                            break;
                        };
                        media.ref_on("loadedmetadata", {
                            let time = init.time;
                            move |e| {
                                e.target().unwrap().dyn_into::<HtmlMediaElement>().unwrap().set_current_time(time);
                            }
                        });
                        break 'restore_pos true;
                    };
                    break 'restore_pos false;
                };
                if restore_pos {
                    state.playlist.0.playing_i.set(pc, Some(i));
                }
            };
            let i = playlist_len(&state.playlist);
            match media_type.as_str() {
                ROOT_AUDIO_VALUE => {
                    let source;
                    let Ok(n) = query_res_as_file(v) else {
                        return el_media_button_err(
                            format!("Field contents wasn't string value node or string: {:?}", v),
                        );
                    };
                    source = n;
                    media = el_audio(&file_url(&state.base_url, &source)).attr("controls", "true");
                    setup_media_element(pc, i, &media);
                    playlist_push(&state.playlist, Rc::new(PlaylistEntry {
                        name: shed!{
                            let Some(v) = &d.name_field else {
                                break None;
                            };
                            let Some(v) = data.get(v) else {
                                break None;
                            };
                            let TreeNode::Scalar(Node::Value(serde_json::Value::String(v))) = v else {
                                break None;
                            };
                            Some(v.clone())
                        },
                        album: shed!{
                            let Some(v) = &d.album_field else {
                                break None;
                            };
                            let Some(v) = data.get(v) else {
                                break None;
                            };
                            let TreeNode::Scalar(Node::Value(serde_json::Value::String(v))) = v else {
                                break None;
                            };
                            Some(v.clone())
                        },
                        artist: shed!{
                            let Some(v) = &d.artist_field else {
                                break None;
                            };
                            let Some(v) = data.get(v) else {
                                break None;
                            };
                            let TreeNode::Scalar(Node::Value(serde_json::Value::String(v))) = v else {
                                break None;
                            };
                            Some(v.clone())
                        },
                        cover: shed!{
                            let Some(v) = &d.cover_field else {
                                break None;
                            };
                            let Some(v) = data.get(v) else {
                                break None;
                            };
                            let Ok(v) = query_res_as_file(v) else {
                                break None;
                            };
                            Some(v.clone())
                        },
                        file: source.clone(),
                        media_type: PlaylistEntryMediaType::Audio,
                        media: Box::new(AudioPlaylistMedia {
                            element: media.clone(),
                            ministate_id: build_playlist_pos.list_id.clone(),
                            ministate_title: build_playlist_pos.list_title.clone(),
                            ministate_path: build_playlist_pos.entry_path.clone(),
                        }),
                    }));
                },
                ROOT_VIDEO_VALUE => {
                    let mut sub_tracks = vec![];
                    let source;
                    let Ok(n) = query_res_as_file(v) else {
                        return el_media_button_err(
                            format!("Field contents wasn't string value node or string: {:?}", v),
                        );
                    };
                    source = n;
                    for lang in window().navigator().languages() {
                        let lang = lang.as_string().unwrap();
                        sub_tracks.push((generated_file_url(&state.base_url, &source, &format!("webvtt_{}", {
                            let lang = if let Some((lang, _)) = lang.split_once("-") {
                                lang
                            } else {
                                &lang
                            };
                            match lang {
                                "en" => "eng",
                                "jp" => "jpn",
                                _ => {
                                    log(format!("Unhandled subtitle translation for language {}", lang));
                                    continue;
                                },
                            }
                        }), "text/vtt"), lang));
                    }
                    media =
                        el_video(
                            &generated_file_url(&state.base_url, &source, "", "video/webm"),
                        ).attr("controls", "true");
                    setup_media_element(pc, i, &media);
                    for (i, (url, lang)) in sub_tracks.iter().enumerate() {
                        let track = el("track").attr("kind", "subtitles").attr("src", url).attr("srclang", lang);
                        if i == 0 {
                            track.ref_attr("default", "default");
                        }
                        media.ref_push(track);
                    }
                    playlist_push(&state.playlist, Rc::new(PlaylistEntry {
                        name: shed!{
                            let Some(field) = &d.name_field else {
                                break None;
                            };
                            let Some(TreeNode::Scalar(Node::Value(serde_json::Value::String(v)))) =
                                data.get(field) else {
                                    break None;
                                };
                            Some(v.clone())
                        },
                        album: shed!{
                            let Some(field) = &d.album_field else {
                                break None;
                            };
                            let Some(TreeNode::Scalar(Node::Value(serde_json::Value::String(v)))) =
                                data.get(field) else {
                                    break None;
                                };
                            Some(v.clone())
                        },
                        artist: shed!{
                            let Some(field) = &d.artist_field else {
                                break None;
                            };
                            let Some(TreeNode::Scalar(Node::Value(serde_json::Value::String(v)))) =
                                data.get(field) else {
                                    break None;
                                };
                            Some(v.clone())
                        },
                        cover: shed!{
                            let Some(field) = &d.cover_field else {
                                break None;
                            };
                            let Some(v) = data.get(field) else {
                                break None;
                            };
                            let Ok(v) = query_res_as_file(v) else {
                                break None;
                            };
                            Some(v.clone())
                        },
                        file: source.clone(),
                        media_type: PlaylistEntryMediaType::Video,
                        media: Box::new(VideoPlaylistMedia {
                            element: media.clone(),
                            ministate_id: build_playlist_pos.list_id.clone(),
                            ministate_title: build_playlist_pos.list_title.clone(),
                            ministate_path: build_playlist_pos.entry_path.clone(),
                        }),
                    }));
                },
                ROOT_IMAGE_VALUE => {
                    let source;
                    let Ok(n) = query_res_as_file(v) else {
                        return el_media_button_err(
                            format!("Field contents wasn't string value node or string: {:?}", v),
                        );
                    };
                    source = n;
                    media = el("img").attr("src", &file_url(&state.base_url, &source)).attr("loading", "lazy");
                    playlist_push(&state.playlist, Rc::new(PlaylistEntry {
                        name: shed!{
                            let Some(field) = &d.name_field else {
                                break None;
                            };
                            let Some(TreeNode::Scalar(Node::Value(serde_json::Value::String(v)))) =
                                data.get(field) else {
                                    break None;
                                };
                            Some(v.clone())
                        },
                        album: shed!{
                            let Some(field) = &d.album_field else {
                                break None;
                            };
                            let Some(TreeNode::Scalar(Node::Value(serde_json::Value::String(v)))) =
                                data.get(field) else {
                                    break None;
                                };
                            Some(v.clone())
                        },
                        artist: shed!{
                            let Some(field) = &d.artist_field else {
                                break None;
                            };
                            let Some(TreeNode::Scalar(Node::Value(serde_json::Value::String(v)))) =
                                data.get(field) else {
                                    break None;
                                };
                            Some(v.clone())
                        },
                        cover: shed!{
                            let Some(field) = &d.cover_field else {
                                break None;
                            };
                            let Some(v) = data.get(field) else {
                                break None;
                            };
                            let Ok(v) = query_res_as_file(v) else {
                                break None;
                            };
                            Some(v.clone())
                        },
                        file: source.clone(),
                        media_type: PlaylistEntryMediaType::Image,
                        media: Box::new(ImagePlaylistMedia {
                            element: media.clone(),
                            ministate_id: build_playlist_pos.list_id.clone(),
                            ministate_title: build_playlist_pos.list_title.clone(),
                            ministate_path: build_playlist_pos.entry_path.clone(),
                        }),
                    }));
                },
                _ => {
                    return el_image_err(format!("Unknown media type {}", media_type));
                },
            }
            let out = el_media_button(pc, &state.playlist, i);
            style_tree(CSS_TREE_MEDIA_BUTTON, depth, d.align, &out);
            return out;
        },
        Widget::Sublist(d) => return build_widget_query(
            pc,
            state,
            queries,
            depth,
            d,
            data,
            build_playlist_pos,
            restore_playlist_pos,
        ),
    }
}

fn build_nest(
    pc: &mut ProcessingContext,
    state: &State,
    queries: &Rc<BTreeMap<String, Query>>,
    depth: usize,
    def: &WidgetNest,
    data: &BTreeMap<String, TreeNode>,
    build_playlist_pos: &BuildPlaylistPos,
    restore_playlist_pos: &Option<PlaylistPos>,
) -> El {
    let out = el("div").classes(&def.orientation.css());
    style_tree(CSS_TREE_NEST, depth, def.align, &out);
    for col_def in &def.children {
        out.ref_push(
            build_widget(pc, state, queries, depth + 1, col_def, data, build_playlist_pos, restore_playlist_pos),
        );
    }
    return out;
}

fn build_layout(
    pc: &mut ProcessingContext,
    state: &State,
    queries: &Rc<BTreeMap<String, Query>>,
    depth: usize,
    def: &Layout,
    data: &Vec<TreeNode>,
    key: &str,
    build_playlist_pos: &BuildPlaylistPos,
    restore_playlist_pos: &Option<PlaylistPos>,
) -> El {
    match def {
        Layout::Individual(d) => {
            let out = el("div");
            style_tree(CSS_TREE_LAYOUT_INDIVIDUAL, depth, d.align, &out);
            out.ref_classes(&d.orientation.css());
            for trans_data in data {
                let TreeNode::Record(trans_data) = trans_data else {
                    let out = el_err_block(format!("Value list contains non-record elements"));
                    style_tree(CSS_TREE_TEXT, depth, d.align, &out);
                    return out;
                };
                let key_value = match trans_data.get(key) {
                    Some(TreeNode::Scalar(v)) => v,
                    _ => continue,
                };
                let subrestore_playlist_pos = shed!{
                    'found_pos _;
                    shed!{
                        let Some(init) = &restore_playlist_pos else {
                            break;
                        };
                        let Some((path_first, path_remainder)) = init.entry_path.0.split_first() else {
                            break;
                        };
                        if key_value != path_first {
                            break;
                        };
                        break 'found_pos Some(PlaylistPos {
                            entry_path: PlaylistEntryPath(path_remainder.to_vec()),
                            time: init.time,
                        });
                    }
                    break 'found_pos None;
                };
                out.ref_push(
                    build_nest(
                        pc,
                        state,
                        queries,
                        depth,
                        &d.item,
                        &trans_data,
                        &build_playlist_pos.add(Some(key_value.clone())),
                        &subrestore_playlist_pos,
                    ),
                );
            }
            if d.x_scroll {
                return el_hscroll(out);
            } else {
                return out;
            }
        },
        Layout::Table(d) => {
            let out = el("div");
            style_tree(CSS_TREE_LAYOUT_TABLE, depth, d.align, &out);
            out.ref_classes(&d.orientation.css());
            for (trans_i, trans_data) in data.iter().enumerate() {
                let TreeNode::Record(trans_data) = trans_data else {
                    let out = el_err_block(format!("Value list contains non-record elements"));
                    style_tree(CSS_TREE_TEXT, depth, d.align, &out);
                    return out;
                };
                let key_value = match trans_data.get(key) {
                    Some(TreeNode::Scalar(v)) => v,
                    _ => continue,
                };
                let subrestore_playlist_pos = shed!{
                    'found_pos _;
                    shed!{
                        let Some(init) = restore_playlist_pos else {
                            break;
                        };
                        let Some((path_first, path_remainder)) = init.entry_path.0.split_first() else {
                            break;
                        };
                        if key_value != path_first {
                            break;
                        };
                        break 'found_pos Some(PlaylistPos {
                            entry_path: PlaylistEntryPath(path_remainder.to_vec()),
                            time: init.time,
                        });
                    }
                    break 'found_pos None;
                };
                let subbuild_playlist_pos = build_playlist_pos.add(Some(key_value.clone()));
                let rev_trans_i = data.len() - trans_i - 1;
                for (con_i, cell_def) in d.columns.iter().enumerate() {
                    let rev_con_i = d.columns.len() - con_i - 1;
                    let cell_out = el("span");
                    let mut row = None;
                    let mut col = None;
                    match d.orientation.con() {
                        Direction::Up => row = Some(rev_con_i),
                        Direction::Down => row = Some(con_i),
                        Direction::Left => col = Some(rev_con_i),
                        Direction::Right => col = Some(con_i),
                    }
                    match d.orientation.trans() {
                        Direction::Up => row = Some(rev_trans_i),
                        Direction::Down => row = Some(trans_i),
                        Direction::Left => col = Some(rev_trans_i),
                        Direction::Right => col = Some(trans_i),
                    }
                    let mut style = vec![];
                    style.push(format!("grid-row: {}", row.unwrap() + 1));
                    style.push(format!("grid-column: {}", col.unwrap() + 1));
                    cell_out.ref_attr("style", &style.join("; "));
                    cell_out.ref_push(
                        build_widget(
                            pc,
                            state,
                            queries,
                            depth,
                            &cell_def,
                            &trans_data,
                            &subbuild_playlist_pos,
                            &subrestore_playlist_pos,
                        ),
                    );
                    out.ref_push(cell_out);
                }
            }
            if d.x_scroll {
                return el_hscroll(out);
            } else {
                return out;
            }
        },
    }
}

pub fn build_page_list_by_id(
    pc: &mut ProcessingContext,
    outer_state: &State,
    list_title: &str,
    list_id: &str,
    build_playlist_pos: &BuildPlaylistPos,
    restore_playlist_pos: &Option<PlaylistPos>,
) {
    outer_state.page_title.upgrade().unwrap().ref_clear().ref_push(el("h1").text(list_title));
    outer_state.page_body.upgrade().unwrap().ref_push(el_async().own(|async_el| {
        let async_el = async_el.weak();
        let eg = pc.eg();
        let outer_state = outer_state.clone();
        let build_playlist_pos = build_playlist_pos.clone();
        let restore_playlist_pos = restore_playlist_pos.clone();
        let list_id = list_id.to_string();
        spawn_rooted(async move {
            let Some(async_el) = async_el.upgrade() else {
                return;
            };
            let menu = match outer_state.menu.get().await {
                Ok(m) => m,
                Err(e) => {
                    async_el.ref_replace(vec![el_err_block(format!("Error retrieving menu: {}", e))]);
                    return;
                },
            };
            eg.event(|pc| {
                match menu.views.get(&list_id) {
                    Some(v) => {
                        async_el.ref_replace(vec![]);
                        build_page_view(pc, &outer_state, v.clone(), &build_playlist_pos, &restore_playlist_pos);
                    },
                    None => {
                        async_el.ref_replace(vec![el_err_block("Unknown view".to_string())]);
                    },
                }
            });
        })
    }));
}

pub fn build_page_view(
    pc: &mut ProcessingContext,
    state: &State,
    def: View,
    build_playlist_pos: &BuildPlaylistPos,
    restore_playlist_pos: &Option<PlaylistPos>,
) {
    playlist_clear(pc, &state.playlist);
    let Some(page_title) = state.page_title.upgrade() else {
        return;
    };
    page_title.ref_text(&def.name);

    fn get_mouse_pct(ev: &Event) -> (f64, f64, MouseEvent) {
        let element = ev.target().unwrap().dyn_into::<Element>().unwrap();
        let ev = ev.dyn_ref::<MouseEvent>().unwrap();
        let element_rect = element.get_bounding_client_rect();
        let percent_x = ((ev.client_x() as f64 - element_rect.x()) / element_rect.width().max(0.001)).clamp(0., 1.);
        let percent_y = ((ev.client_y() as f64 - element_rect.y()) / element_rect.width().max(0.001)).clamp(0., 1.);
        return (percent_x, percent_y, ev.clone());
    }

    fn get_mouse_time(ev: &Event, state: &State) -> Option<f64> {
        let Some(max_time) = *state.playlist.0.playing_max_time.borrow() else {
            return None;
        };
        let percent = get_mouse_pct(ev).0;
        return Some(max_time * percent);
    }

    let queries = Rc::new(def.queries.clone());
    let body =
        el("div")
            .classes(&["s_listview_body"])
            .push(
                build_widget_query(
                    pc,
                    &state,
                    &queries,
                    0,
                    &def.display,
                    &Default::default(),
                    build_playlist_pos,
                    restore_playlist_pos,
                ),
            );
    let hover_time = Prim::new(None);
    state.page_body.upgrade().unwrap().ref_clear().ref_extend(vec![
        //. .
        el("div").classes(&["s_transport"]).extend(vec![
            //. .
            el_hbox().classes(&["left"]).extend(vec![
                //. .
                el_button_icon(pc, el_icon(ICON_SHARE), "Share stream", {
                    let state = state.clone();
                    move |pc| {
                        let Some(stack) = state.stack.upgrade() else {
                            return;
                        };
                        let sess_id = state.playlist.0.share.borrow().as_ref().map(|x| x.0.clone());
                        let sess_id = match sess_id {
                            Some(sess_id) => {
                                sess_id.clone()
                            },
                            None => {
                                let id = Uuid::new_v4().to_string();
                                state
                                    .playlist
                                    .0
                                    .share
                                    .set(
                                        pc,
                                        Some((id.clone(), Ws::new(format!("main/{}", id), |_, _| unreachable!()))),
                                    );
                                id
                            },
                        };
                        let link = format!("{}#{}{}", state.base_url, LINK_HASH_PREFIX, sess_id);
                        stack.ref_push(el_modal(pc, "Share", |pc, root| {
                            return vec![
                                //. .
                                el("a").classes(&["g_qr"]).attr("href", &link).push(el_from_raw(
                                    //. .
                                    DomParser::new()
                                        .unwrap()
                                        .parse_from_string(
                                            &QrCode::new(&link)
                                                .unwrap()
                                                .render::<qrcode::render::svg::Color>()
                                                .quiet_zone(false)
                                                .build(),
                                            SupportedType::ImageSvgXml,
                                        )
                                        .unwrap()
                                        .first_element_child()
                                        .unwrap(),
                                )),
                                el_button_icon_text(pc, ICON_NOSHARE, "Stop sharing", {
                                    let state = state.clone();
                                    let root = root.clone();
                                    move |pc| {
                                        let Some(root) = root.upgrade() else {
                                            return;
                                        };
                                        state.playlist.0.share.set(pc, None);
                                        root.ref_replace(vec![]);
                                    }
                                })
                            ];
                        }));
                    }
                }).own(|e| link!((_pc = pc), (share = state.playlist.0.share.clone()), (), (e = e.weak()) {
                    let e = e.upgrade()?;
                    e.ref_modify_classes(&[("on", share.borrow().is_some())]);
                }))
            ]),
            el_hbox().classes(&["middle", CSS_STATE_GROW]).extend(vec![
                //. .
                el_button_icon(pc, el_icon(ICON_TRANSPORT_PREVIOUS), "Previous", {
                    let state = state.clone();
                    move |pc| {
                        playlist_previous(pc, &state.playlist, None);
                    }
                }),
                el_stack().classes(&["s_seekbar"]).extend(vec![
                    //. .
                    el("div").classes(&["gutter"]).push(el("div").classes(&["fill"]).own(|fill| link!(
                        //. .
                        (_pc = pc),
                        (
                            time = state.playlist.0.playing_time.clone(),
                            max_time = state.playlist.0.playing_max_time.clone(),
                        ),
                        (),
                        (fill = fill.weak()) {
                            let Some(max_time) = *max_time.borrow() else {
                                return None;
                            };
                            let fill = fill.upgrade()?;
                            fill.ref_attr(
                                "style",
                                &format!("width: {}%;", *time.borrow() / max_time.max(0.0001) * 100.),
                            );
                        }
                    ))),
                    el("span").classes(&["label"]).own(|label| link!(
                        //. .
                        (_pc = pc),
                        (playing_time = state.playlist.0.playing_time.clone(), hover_time = hover_time.clone()),
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
                    ))
                ]).on("mousemove", {
                    let eg = pc.eg();
                    let state = state.clone();
                    let hover_time = hover_time.clone();
                    move |ev| eg.event(|pc| {
                        hover_time.set(pc, get_mouse_time(ev, &state));
                    })
                }).on("mouseleave", {
                    let eg = pc.eg();
                    let hover_time = hover_time.clone();
                    move |_| eg.event(|pc| {
                        hover_time.set(pc, None);
                    })
                }).on("click", {
                    let state = state.clone();
                    let eg = pc.eg();
                    move |ev| eg.event(|pc| {
                        let Some(time) = get_mouse_time(ev, &state) else {
                            return;
                        };
                        playlist_seek(pc, &state.playlist, time);
                    })
                }),
                el_button_icon(pc, el_icon(ICON_TRANSPORT_NEXT), "Next", {
                    let state = state.clone();
                    move |pc| {
                        playlist_next(pc, &state.playlist, None);
                    }
                }),
                el_button_icon_switch(
                    pc,
                    ICON_TRANSPORT_PLAY,
                    "Play",
                    ICON_TRANSPORT_PAUSE,
                    "Pause",
                    &state.playlist.0.playing,
                ).on("click", {
                    let eg = pc.eg();
                    let state = state.clone();
                    move |_| eg.event(|pc| {
                        playlist_toggle_play(pc, &state.playlist, None);
                    })
                })
            ]),
            el_hbox().classes(&["right"]).extend(vec![
                //. .
                el_button_icon(pc, el_icon(ICON_VOLUME), "Set volume", {
                    let state = state.clone();
                    move |pc| {
                        let Some(stack) = state.stack.upgrade() else {
                            return;
                        };
                        stack.ref_push(
                            el_modal(pc, "Volume", |pc, _root| vec![el_vbox().classes(&["s_volume"]).extend(vec![
                                //. .
                                el_stack().extend(vec![
                                    //. .
                                    el_stack().classes(&["s_vol_bg"]).extend(vec![el("div"), el("div")]),
                                    el("div")
                                        .classes(&["puck"])
                                        .own(
                                            |e| link!(
                                                (_pc = pc),
                                                (vol = state.playlist.0.volume.clone()),
                                                (),
                                                (e = e.weak()) {
                                                    let e = e.upgrade()?;
                                                    let (x, y) = &*vol.borrow();
                                                    e.ref_attr(
                                                        "style",
                                                        &format!("left: {}%; bottom: {}%;", x * 200., y * 200.),
                                                    );
                                                }
                                            ),
                                        )
                                ]).on("mousemove", {
                                    let state = state.playlist.clone();
                                    let eg = pc.eg();
                                    move |ev| {
                                        let (x, y, ev) = get_mouse_pct(ev);
                                        if ev.buttons() != 1 {
                                            return;
                                        }
                                        let vol = (x / 2., (1. - y) / 2.);
                                        eg.event(|pc| {
                                            state.0.volume.set(pc, vol);
                                            state.0.volume_debounce.set(Utc::now());
                                        });
                                    }
                                }).on("click", {
                                    let state = state.playlist.clone();
                                    let eg = pc.eg();
                                    move |ev| {
                                        let (x, y, ev) = get_mouse_pct(ev);
                                        if ev.button() != 0 {
                                            return;
                                        }
                                        let vol = (x / 2., (1. - y) / 2.);
                                        eg.event(|pc| {
                                            state.0.volume.set(pc, vol);
                                            state.0.volume_debounce.set(Utc::now());
                                        });
                                    }
                                }),
                                el(
                                    "span",
                                ).own(
                                    |e| link!((_pc = pc), (vol = state.playlist.0.volume.clone()), (), (e = e.weak()) {
                                        let e = e.upgrade()?;
                                        let (x, y) = &*vol.borrow();
                                        e.ref_text(&format!("{}%", ((x + y) * 100.) as i32));
                                    }),
                                )
                            ])]),
                        );
                    }
                })
            ])
        ]),
        {
            let parameters = el_vbox().classes(&["s_parameters"]);
            if !def.parameters.is_empty() {
                let bg = Rc::new(RefCell::new(None));

                #[derive(Clone)]
                struct DelayRebuild {
                    bg: Rc<RefCell<Option<ScopeValue>>>,
                    state: State,
                    queries: Rc<BTreeMap<String, Query>>,
                    def: View,
                    parameter_values: Rc<RefCell<BTreeMap<String, TreeNode>>>,
                    build_playlist_pos: BuildPlaylistPos,
                    body: El,
                }

                impl DelayRebuild {
                    fn call(&self, pc: &mut ProcessingContext) {
                        *self.bg.borrow_mut() = Some(spawn_rooted({
                            let eg = pc.eg();
                            let state = self.state.clone();
                            let queries = self.queries.clone();
                            let def = self.def.clone();
                            let parameter_values = self.parameter_values.clone();
                            let build_playlist_pos = self.build_playlist_pos.clone();
                            let body = self.body.clone();
                            async move {
                                TimeoutFuture::new(1000).await;
                                eg.event(|pc| {
                                    body
                                        .ref_clear()
                                        .ref_push(
                                            build_widget_query(
                                                pc,
                                                &state,
                                                &queries,
                                                0,
                                                &def.display,
                                                &*parameter_values.borrow(),
                                                &build_playlist_pos,
                                                &None,
                                            ),
                                        );
                                });
                            }
                        }));
                    }
                }

                let delay_rebuild = DelayRebuild {
                    bg: bg.clone(),
                    state: state.clone(),
                    queries: queries.clone(),
                    def: def.clone(),
                    parameter_values: Default::default(),
                    build_playlist_pos: build_playlist_pos.clone(),
                    body: body.clone(),
                };
                parameters.ref_own(|_| bg.clone());
                for (k, v) in def.parameters {
                    parameters.ref_push(el("label").extend(vec![el("span").text(&k), match v {
                        QueryDefParameter::Text => {
                            delay_rebuild
                                .parameter_values
                                .borrow_mut()
                                .insert(
                                    k.clone(),
                                    TreeNode::Scalar(Node::Value(serde_json::Value::String("".to_string()))),
                                );
                            el("input").attr("type", "text").on("input", {
                                let eg = pc.eg();
                                let delay_rebuild = delay_rebuild.clone();
                                let parameter_values = delay_rebuild.parameter_values.clone();
                                move |ev| eg.event(|pc| {
                                    let e = ev.target().unwrap().dyn_into::<HtmlInputElement>().unwrap();
                                    parameter_values
                                        .borrow_mut()
                                        .insert(
                                            k.clone(),
                                            TreeNode::Scalar(Node::Value(serde_json::Value::String(e.value()))),
                                        );
                                    delay_rebuild.call(pc);
                                })
                            })
                        },
                        QueryDefParameter::Number => {
                            delay_rebuild
                                .parameter_values
                                .borrow_mut()
                                .insert(
                                    k.clone(),
                                    TreeNode::Scalar(
                                        Node::Value(serde_json::Value::Number(serde_json::Number::from(0))),
                                    ),
                                );
                            el("input").attr("type", "number").on("input", {
                                let eg = pc.eg();
                                let delay_rebuild = delay_rebuild.clone();
                                move |ev| eg.event(|pc| {
                                    let e = ev.target().unwrap().dyn_into::<HtmlInputElement>().unwrap();
                                    if !e.value().is_empty() {
                                        let n = serde_json::Number::from_str(&e.value()).unwrap();
                                        delay_rebuild
                                            .parameter_values
                                            .borrow_mut()
                                            .insert(
                                                k.clone(),
                                                TreeNode::Scalar(Node::Value(serde_json::Value::Number(n))),
                                            );
                                        delay_rebuild.call(pc);
                                    }
                                })
                            })
                        },
                        QueryDefParameter::Bool => {
                            delay_rebuild
                                .parameter_values
                                .borrow_mut()
                                .insert(k.clone(), TreeNode::Scalar(Node::Value(serde_json::Value::Bool(false))));
                            el("input").attr("type", "checkbox").on("input", {
                                let eg = pc.eg();
                                let delay_rebuild = delay_rebuild.clone();
                                move |ev| eg.event(|pc| {
                                    let e = ev.target().unwrap().dyn_into::<HtmlInputElement>().unwrap();
                                    delay_rebuild
                                        .parameter_values
                                        .borrow_mut()
                                        .insert(
                                            k.clone(),
                                            TreeNode::Scalar(Node::Value(serde_json::Value::Bool(e.checked()))),
                                        );
                                    delay_rebuild.call(pc);
                                })
                            })
                        },
                        QueryDefParameter::Datetime => {
                            delay_rebuild
                                .parameter_values
                                .borrow_mut()
                                .insert(
                                    k.clone(),
                                    TreeNode::Scalar(
                                        Node::Value(serde_json::Value::String(Utc::now().to_rfc3339())),
                                    ),
                                );
                            el("input").attr("type", "datetime-local").on("input", {
                                let eg = pc.eg();
                                let delay_rebuild = delay_rebuild.clone();
                                move |ev| eg.event(|pc| {
                                    let e = ev.target().unwrap().dyn_into::<HtmlInputElement>().unwrap();
                                    delay_rebuild
                                        .parameter_values
                                        .borrow_mut()
                                        .insert(
                                            k.clone(),
                                            TreeNode::Scalar(Node::Value(serde_json::Value::String(e.value()))),
                                        );
                                    delay_rebuild.call(pc);
                                })
                            })
                        },
                    }]));
                }
            }
            parameters
        },
        body
    ]);
}
