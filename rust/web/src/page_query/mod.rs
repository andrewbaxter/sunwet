use std::{
    any::Any,
    cell::{
        Cell,
        RefCell,
    },
    collections::HashMap,
    panic,
    rc::Rc,
    str::FromStr,
};
use gloo::{
    console::{
        warn,
    },
    utils::{
        document,
        window,
    },
};
use js_sys::Function;
use lunk::{
    link,
    EventGraph,
    HistPrim,
    Prim,
    ProcessingContext,
};
use reqwasm::http::Request;
use rooting::{
    el,
    set_root,
    spawn_rooted,
    El,
};
use rooting_forms::{
    BigString,
    Form,
    FormState,
};
use serde::de::DeserializeOwned;
use shared::{
    model::{
        C2SReq,
        FileHash,
        Node,
        Query,
    },
    unenum,
};
use wasm_bindgen::{
    closure::Closure,
    JsCast,
    JsValue,
    UnwrapThrowExt,
};
use web_sys::{
    Element,
    Event,
    HtmlAudioElement,
    HtmlElement,
    HtmlMediaElement,
    MediaMetadata,
    MediaSession,
    MouseEvent,
};
use crate::{
    el_general::{
        el_button_icon,
        el_button_icon_switch,
        el_button_icon_switch_auto,
        el_button_text,
        el_group,
        el_hbox,
        el_stack,
        el_vbox,
        log,
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
    state::{
        self,
        State,
        View,
    },
    util::{
        OptString,
        CSS_GROW,
        ICON_EDIT,
        ICON_NOEDIT,
        ICON_TRANSPORT_NEXT,
        ICON_TRANSPORT_PAUSE,
        ICON_TRANSPORT_PLAY,
        ICON_TRANSPORT_PREVIOUS,
    },
    world::{
        file_url,
        req_post_json,
    },
};
use self::{
    definition::{
        BlockSizeMode,
        Direction,
        FieldOrLiteral,
        Layout,
        LineSizeMode,
        QueryOrField,
        Widget,
        WidgetList,
        WidgetNest,
    },
    elements::{
        el_err,
        el_image_err,
        el_media_button,
        el_media_button_err,
        el_text_err,
        style_tree,
        CSS_TREE_IMAGE,
        CSS_TREE_LAYOUT_INDIVIDUAL,
        CSS_TREE_LAYOUT_TABLE,
        CSS_TREE_MEDIA_BUTTON,
        CSS_TREE_NEST,
        CSS_TREE_TEXT,
    },
};

pub mod definition;
pub mod elements;

fn extract_node_value(v: &serde_json::Value) -> Option<serde_json::Value> {
    let n = extract_node(v)?;
    let Node:: Value(v) = n else {
        return None;
    };
    return Some(v);
}

fn extract_node_text(v: &serde_json::Value) -> Option<String> {
    let v = extract_node_value(v)?;
    return unenum!(v, serde_json:: Value:: String(v) => v);
}

fn extract_node_file(v: &serde_json::Value) -> Option<FileHash> {
    let n = extract_node(v)?;
    let Node:: File(v) = n else {
        return None;
    };
    return Some(v);
}

fn json_value_type(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "bool",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

pub fn extract_node(v: &serde_json::Value) -> Option<Node> {
    let serde_json:: Value:: Array(v) = v else {
        return None;
    };
    let mut v = v.iter();
    let Some(serde_json::Value::String(key)) = v.next() else {
        return None;
    };
    match key.as_str() {
        "id" => {
            let Some(serde_json::Value::String(v)) = v.next() else {
                return None;
            };
            return Some(Node::Id(v.clone()));
        },
        "file" => {
            let Some(serde_json::Value::String(v)) = v.next() else {
                return None;
            };
            let Ok(v) = FileHash:: from_str(&v) else {
                return None;
            };
            return Some(Node::File(v));
        },
        "value" => {
            let Some(v) = v.next() else {
                return None;
            };
            return Some(Node::Value(v.clone()));
        },
        _ => return None,
    }
}

pub fn build_widget_query(
    pc: &mut ProcessingContext,
    state: &State,
    depth: usize,
    def: &WidgetList,
    data: &Rc<HashMap<String, serde_json::Value>>,
) -> El {
    return el_group().own(|e| spawn_rooted({
        let def = def.clone();
        let source_data = data.clone();
        let state = state.clone();
        let eg = pc.eg();
        let e = e.weak();
        async move {
            let Some(ele) = e.upgrade() else {
                return;
            };
            let rows;
            match &def.data {
                QueryOrField::Field(f) => {
                    match source_data.get(f) {
                        Some(f) => match f {
                            serde_json::Value::Array(v) => {
                                let mut out = vec![];
                                for i in v {
                                    match i {
                                        serde_json::Value::Object(v) => {
                                            out.push(
                                                Rc::new(
                                                    v
                                                        .iter()
                                                        .map(|(k, v)| (k.clone(), v.clone()))
                                                        .collect::<HashMap<_, _>>(),
                                                ),
                                            );
                                        },
                                        _ => {
                                            ele.ref_push(
                                                el_err(
                                                    format!(
                                                        "Specified field for list contains {} element, not object",
                                                        json_value_type(&i)
                                                    ),
                                                ),
                                            );
                                            return;
                                        },
                                    }
                                }
                                rows = out;
                            },
                            _ => {
                                ele.ref_push(
                                    el_err(
                                        format!(
                                            "Specified field for list is a {}, not a list type",
                                            json_value_type(&f)
                                        ),
                                    ),
                                );
                                return;
                            },
                        },
                        None => {
                            ele.ref_push(
                                el_err(
                                    "Specified field for list doesn't exist in parent row/parameter data".to_string(),
                                ),
                            );
                            return;
                        },
                    }
                },
                QueryOrField::Query(query) => {
                    let res =
                        req_post_json::<Vec<HashMap<String, serde_json::Value>>>(
                            &state.origin,
                            C2SReq::Query(Query {
                                query: query.0.clone(),
                                parameters: source_data.as_ref().to_owned(),
                            }),
                        ).await;
                    ele.ref_clear();
                    rows = match res {
                        Ok(rows) => rows.into_iter().map(|x| Rc::new(x)).collect::<Vec<_>>(),
                        Err(e) => {
                            ele.ref_push(el_err(e));
                            return;
                        },
                    };
                },
            }
            eg.event(|pc| {
                ele.ref_push(build_layout(pc, &state, depth, &def.layout, &rows));
            });
        }
    }));
}

fn build_widget(
    pc: &mut ProcessingContext,
    state: &State,
    depth: usize,
    def: &Widget,
    data: &Rc<HashMap<String, serde_json::Value>>,
) -> El {
    match def {
        Widget::Nest(d) => return build_nest(pc, state, depth, d, data),
        Widget::TextLine(d) => {
            let text;
            match &d.data {
                FieldOrLiteral::Field(field) => {
                    let Some(v) = data.get(field) else {
                        return el_text_err(format!("Missing field {}", field));
                    };
                    let mut v = v.clone();
                    if let Some(v1) = extract_node_value(&v) {
                        v = v1;
                    }
                    text = match v {
                        serde_json::Value::Null => "-".to_string(),
                        serde_json::Value::Bool(v) => match v {
                            true => "yes".to_string(),
                            false => "no".to_string(),
                        },
                        serde_json::Value::Number(v) => v.to_string(),
                        serde_json::Value::String(v) => v.clone(),
                        serde_json::Value::Array(v) => serde_json::to_string(&v).unwrap(),
                        serde_json::Value::Object(v) => serde_json::to_string(&v).unwrap(),
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
                LineSizeMode::Full => { },
                LineSizeMode::Ellipsize => style.push(format!("text-overflow: ellipsis")),
                LineSizeMode::Wrap => style.push(format!("overflow-wrap: break-word")),
                LineSizeMode::Scroll => match d.orientation.con() {
                    Direction::Up | Direction::Down => style.push(format!("overflow-x: auto")),
                    Direction::Left | Direction::Right => style.push(format!("overflow-y: auto")),
                },
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
                    if let Some(n) = extract_node_file(v) {
                        url = file_url(&state.origin, &n);
                    } else if let serde_json::Value::String(v) = v {
                        url = v.clone();
                    } else {
                        return el_image_err(format!("Field contents wasn't string value node or string: {:?}", v));
                    }
                },
                FieldOrLiteral::Literal(data) => {
                    url = data.clone();
                },
            }
            let out = el("img").attr("src", &url);
            style_tree(CSS_TREE_IMAGE, depth, d.align, &out);
            let mut style = vec![];
            match d.size_mode {
                BlockSizeMode::Cover => style.push(format!("object-fit: cover")),
                BlockSizeMode::Contain => style.push(format!("object-fit: contain")),
            }
            if let Some(width) = (&d.width).if_some() {
                style.push(format!("width: {}", width));
            }
            if let Some(height) = (&d.height).if_some() {
                style.push(format!("height: {}", height));
            }
            out.ref_attr("style", &style.join("; "));
            return out;
        },
        Widget::Audio(d) => {
            let Some(v) = data.get(&d.field) else {
                return el_media_button_err(format!("Missing field {}", d.field));
            };
            let i = playlist_len(&state.playlist);
            let audio = el("audio").attr("preload", "metadata").on("ended", {
                let state = state.clone();
                let eg = pc.eg();
                move |_| eg.event(|pc| {
                    playlist_next(pc, &state.playlist, Some(i));
                })
            });
            if let Some(n) = extract_node_file(v) {
                audio.ref_attr("src", &file_url(&state.origin, &n));
            } else if let serde_json::Value::String(v) = v {
                audio.ref_attr("src", &v);
            } else {
                return el_media_button_err(format!("Field contents wasn't string value node or string: {:?}", v));
            }
            playlist_push(&state.playlist, Rc::new(PlaylistEntry {
                name: (&d.name_field).if_some().and_then(|v| data.get(v)).and_then(|v| extract_node_text(v)),
                album: (&d.album_field).if_some().and_then(|v| data.get(v)).and_then(|v| extract_node_text(v)),
                artist: (&d.artist_field).if_some().and_then(|v| data.get(v)).and_then(|v| extract_node_text(v)),
                thumbnail: (&d.thumbnail_field)
                    .if_some()
                    .and_then(|v| data.get(v))
                    .and_then(|v| extract_node_file(v))
                    .map(|h| file_url(&state.origin, &h)),
                media: Box::new(AudioPlaylistMedia(audio)),
            }));
            let out = el_media_button(pc, &state.playlist, i);
            style_tree(CSS_TREE_MEDIA_BUTTON, depth, d.align, &out);
            return out;
        },
        Widget::Video(d) => {
            let Some(v) = data.get(&d.field) else {
                return el_image_err(format!("Missing field {}", d.field));
            };
            let i = playlist_len(&state.playlist);
            let source = el("source");
            if let Some(n) = extract_node_file(v) {
                source.ref_attr("src", &file_url(&state.origin, &n));
            } else if let serde_json::Value::String(v) = v {
                source.ref_attr("src", &v);
            } else {
                return el_media_button_err(format!("Field contents wasn't string value node or string: {:?}", v));
            }
            let video = el("video").attr("preload", "metadata").push(source).on("ended", {
                let state = state.clone();
                let eg = pc.eg();
                move |_| eg.event(|pc| {
                    playlist_next(pc, &state.playlist, Some(i));
                })
            });
            playlist_push(&state.playlist, Rc::new(PlaylistEntry {
                name: (&d.name_field).if_some().and_then(|v| data.get(v)).and_then(|v| extract_node_text(v)),
                album: (&d.album_field).if_some().and_then(|v| data.get(v)).and_then(|v| extract_node_text(v)),
                artist: (&d.artist_field).if_some().and_then(|v| data.get(v)).and_then(|v| extract_node_text(v)),
                thumbnail: (&d.thumbnail_field)
                    .if_some()
                    .and_then(|v| data.get(v))
                    .and_then(|v| extract_node_file(v))
                    .map(|h| file_url(&state.origin, &h)),
                media: Box::new(VideoPlaylistMedia(video)),
            }));
            let out = el_media_button(pc, &state.playlist, i);
            style_tree(CSS_TREE_MEDIA_BUTTON, depth, d.align, &out);
            return out;
        },
        Widget::Sublist(d) => return build_widget_query(pc, state, depth, d, data),
    }
}

fn build_nest(
    pc: &mut ProcessingContext,
    state: &State,
    depth: usize,
    def: &WidgetNest,
    data: &Rc<HashMap<String, serde_json::Value>>,
) -> El {
    let out = el("div").classes(&def.orientation.css());
    style_tree(CSS_TREE_NEST, depth, def.align, &out);
    for col_def in &def.children {
        out.ref_push(build_widget(pc, state, depth + 1, col_def, data));
    }
    return out;
}

fn build_layout(
    pc: &mut ProcessingContext,
    state: &State,
    depth: usize,
    def: &Layout,
    data: &Vec<Rc<HashMap<String, serde_json::Value>>>,
) -> El {
    match def {
        Layout::Individual(d) => {
            let out = el("div");
            style_tree(CSS_TREE_LAYOUT_INDIVIDUAL, depth, d.align, &out);
            out.ref_classes(&d.orientation.css());
            for row_data in data {
                out.ref_push(build_nest(pc, state, depth, &d.item, &row_data));
            }
            return out;
        },
        Layout::Table(d) => {
            let out = el("div");
            style_tree(CSS_TREE_LAYOUT_TABLE, depth, d.align, &out);
            out.ref_classes(&d.orientation.css());
            for (trans_i, trans_data) in data.iter().enumerate() {
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
                    cell_out.ref_push(build_widget(pc, state, depth, &cell_def, &trans_data));
                    out.ref_push(cell_out);
                }
            }
            return out;
        },
    }
}

struct ViewState_ {
    state: State,
    committed_def: RefCell<Rc<state::View>>,
    working_def: Prim<Rc<state::View>>,
    edit_state: RefCell<Option<(Box<dyn FormState<String>>, Box<dyn FormState<WidgetList>>)>>,
}

type ViewState = Rc<ViewState_>;

pub fn build_page_view_by_id(
    pc: &mut ProcessingContext,
    outer_state: &State,
    view_title: &str,
    view_id: usize,
    initial_play_entry: Vec<HashMap<String, serde_json::Value>>,
    initial_play_time: f64,
) {
    outer_state.mobile_vert_title_group.upgrade().unwrap().ref_clear().ref_push(el("h1").text(view_title));
    outer_state.title_group.upgrade().unwrap().ref_clear().ref_push(el("h1").text(view_title));
    outer_state.body_group.upgrade().unwrap().ref_push(el_group().own(|e| {
        let e = e.weak();
        let eg = pc.eg();
        let outer_state = outer_state.clone();
        spawn_rooted(async move {
            let Some(e) = e.upgrade() else {
                return;
            };
            let views = outer_state.views.get().await;
            let views = views.borrow();
            eg.event(|pc| {
                match views.get(&view_id) {
                    Some(v) => {
                        e.ref_replace(vec![]);
                        build_page_view(pc, &outer_state, v.clone(), initial_play_entry, initial_play_time);
                    },
                    None => {
                        e.ref_replace(vec![el("span").text("Unknown view")]);
                    },
                }
            });
        })
    }));
}

pub fn build_page_view(
    pc: &mut ProcessingContext,
    outer_state: &State,
    original_def: state::View,
    initial_play_entry: Vec<HashMap<String, serde_json::Value>>,
    initial_play_time: f64,
) {
    let original_def = Rc::new(original_def);
    let state = ViewState::new(ViewState_ {
        state: outer_state.clone(),
        committed_def: RefCell::new(original_def.clone()),
        working_def: Prim::new(pc, original_def),
        edit_state: RefCell::new(None),
    });
    let edit = Prim::new(pc, false);
    let title_group = el_group();
    state.state.title_group.upgrade().unwrap().ref_clear().ref_push(el_hbox().classes(&[CSS_GROW]).extend(vec![
        //. .
        title_group.clone(),
        el_button_icon_switch_auto(pc, ICON_EDIT, "Edit", ICON_NOEDIT, "View", &edit)
    ]).own(|_| link!(
        //. .
        (pc = pc),
        (edit = edit, working_def = state.working_def.clone()),
        (),
        (title_group = title_group.weak(), state = state.clone()) {
            playlist_clear(pc, &state.state.playlist);
            let title_group = title_group.upgrade()?;
            title_group.ref_clear();
            let body_group = state.state.body_group.upgrade()?;
            body_group.ref_clear();
            let mobile_vert_title_group = state.state.mobile_vert_title_group.upgrade()?;
            mobile_vert_title_group.ref_clear();
            if *edit.borrow() {
                // Edit
                let working_def = working_def.borrow();
                mobile_vert_title_group.ref_push(el("h1").text(&format!("Edit: {}", working_def.name)));
                let (title_state_elements, title_state) = String::new_form("", Some(&working_def.name));
                title_group.ref_push(el("h1").extend(title_state_elements.elements));
                let (form_state_elements, form_state) = WidgetList::new_form("", Some(&working_def.def));
                let mut form_elements = vec![];
                if let Some(error) = form_state_elements.error {
                    form_elements.push(error);
                }
                form_elements.extend(form_state_elements.elements);
                body_group.ref_extend(vec![el_hbox().classes(&["s_navigation"]).extend(vec![
                    //. .
                    el_button_text(pc, "Replace", {
                        let state = state.clone();
                        let edit = edit.clone();
                        move |pc| {
                            *state.committed_def.borrow_mut() = state.working_def.borrow().clone();
                            edit.set(pc, false);
                        }
                    }),
                    el_button_text(pc, "Branch", {
                        let state = state.clone();
                        let edit = edit.clone();
                        move |pc| {
                            *state.committed_def.borrow_mut() = state.working_def.borrow().clone();
                            edit.set(pc, false);
                        }
                    }),
                    el_button_text(pc, "Discard", {
                        let state = state.clone();
                        let edit = edit.clone();
                        move |pc| {
                            let committed_def = state.committed_def.borrow();
                            state.working_def.set(pc, committed_def.clone());
                            edit.set(pc, false);
                        }
                    })
                ])]).ref_push(el("div").classes(&["s_edit_view_body"]).extend(form_elements));
                *state.edit_state.borrow_mut() = Some((title_state, form_state));
            } else {
                // View
                if let Some((title_state, form_state)) = state.edit_state.borrow_mut().take() {
                    let title = title_state.parse().unwrap();
                    let form = form_state.parse().unwrap();
                    state.working_def.set(pc, Rc::new(View {
                        name: title,
                        def: form,
                    }));
                }
                let working_def = working_def.borrow();
                mobile_vert_title_group.ref_push(el("h1").text(&working_def.name));
                title_group.ref_push(el("h1").text(&working_def.name));

                fn get_mouse_time(ev: &Event, state: &ViewState) -> Option<f64> {
                    let Some(max_time) =* state.state.playlist.0.playing_max_time.borrow() else {
                        return None;
                    };
                    let bar = ev.target().unwrap().dyn_into::<Element>().unwrap();
                    let ev = ev.dyn_ref::<MouseEvent>().unwrap();
                    let bar_rect = bar.get_bounding_client_rect();
                    let percent =
                        ((ev.client_x() as f64 - bar_rect.x()) / bar_rect.width().max(0.001)).clamp(0., 1.);
                    return Some(max_time * percent);
                }

                let hover_time = Prim::new(pc, None);
                body_group.ref_extend(
                    vec![
                        el("div").classes(&["s_transport"]).extend(vec![
                            //. .
                            el_hbox().classes(&["left"]),
                            el_hbox().classes(&["middle"]).extend(vec![
                                //. .
                                el_button_icon(pc, ICON_TRANSPORT_PREVIOUS, "Previous", {
                                    let state = state.clone();
                                    move |pc| {
                                        playlist_previous(pc, &state.state.playlist, None);
                                    }
                                }),
                                el_stack().classes(&["s_seekbar"]).extend(vec![
                                    //. .
                                    el("div").classes(&["gutter"]).push(el("div").classes(&["fill"]).own(|fill| link!(
                                        //. .
                                        (_pc = pc),
                                        (
                                            time = state.state.playlist.0.playing_time.clone(),
                                            max_time = state.state.playlist.0.playing_max_time.clone(),
                                        ),
                                        (),
                                        (fill = fill.weak()) {
                                            let Some(max_time) =* max_time.borrow() else {
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
                                        (
                                            playing_time = state.state.playlist.0.playing_time.clone(),
                                            hover_time = hover_time.clone(),
                                        ),
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
                                                label.text(
                                                    &format!("{:02}:{:02}:{:02}:{:02}", days, hours, minutes, seconds),
                                                );
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
                                        playlist_seek(pc, &state.state.playlist, time);
                                    })
                                }),
                                el_button_icon(pc, ICON_TRANSPORT_NEXT, "Next", {
                                    let state = state.clone();
                                    move |pc| {
                                        playlist_next(pc, &state.state.playlist, None);
                                    }
                                }),
                                el_button_icon_switch(
                                    pc,
                                    ICON_TRANSPORT_PLAY,
                                    "Play",
                                    ICON_TRANSPORT_PAUSE,
                                    "Pause",
                                    &state.state.playlist.0.playing,
                                ).on("click", {
                                    let eg = pc.eg();
                                    let state = state.clone();
                                    move |_| eg.event(|pc| {
                                        playlist_toggle_play(pc, &state.state.playlist, None);
                                    })
                                })
                            ]),
                            el_hbox().classes(&["right"])
                        ]),
                        el("div")
                            .classes(&["s_view_body"])
                            .push(
                                build_widget_query(pc, &state.state, 0, &working_def.def, &Rc::new(HashMap::new())),
                            )
                    ],
                );
            }
        }
    )));
}
