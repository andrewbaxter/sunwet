use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    str::FromStr,
};
use gloo::utils::{
    document,
    window,
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
    HtmlAudioElement,
    HtmlMediaElement,
    HtmlVideoElement,
    MediaMetadata,
    MediaSession,
};

async fn send_req(req: Request) -> Result<Vec<u8>, String> {
    let resp = match req.send().await {
        Ok(r) => r,
        Err(e) => {
            return Err(format!("Failed to send request: {}", e));
        },
    };
    let status = resp.status();
    let body = match resp.binary().await {
        Err(e) => {
            return Err(format!("Got error response, got additional error trying to read body [{}]: {}", status, e));
        },
        Ok(r) => r,
    };
    if status >= 400 {
        return Err(format!("Got error response [{}]: [{}]", status, String::from_utf8_lossy(&body)));
    }
    return Ok(body);
}

pub async fn req_post(origin: &str, req: C2SReq) -> Result<Vec<u8>, String> {
    let res =
        send_req(
            Request::post(&format!("{}/api", origin)).body(serde_json::to_string(&req).unwrap_throw()),
        ).await?;
    return Ok(res);
}

pub async fn req_post_json<R: DeserializeOwned>(origin: &str, req: C2SReq) -> Result<R, String> {
    let res =
        send_req(
            Request::post(&format!("{}/api", origin))
                .header("Content-type", "application/json")
                .body(serde_json::to_string(&req).unwrap_throw()),
        ).await?;
    return Ok(
        serde_json::from_slice::<R>(
            &res,
        ).map_err(
            |e| format!("Error parsing JSON response from server: {}\nBody: {}", e, String::from_utf8_lossy(&res)),
        )?,
    );
}

#[derive(Clone, Copy)]
enum Dir {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone)]
struct LayoutSingle {
    con: Dir,
    trans: Dir,
    children: Vec<Widget>,
}

#[derive(Clone)]
struct LayoutTable {
    con: Dir,
    trans: Dir,
    children: Vec<Widget>,
}

#[derive(Clone)]
struct WidgetText {
    field: String,
    size: String,
}

#[derive(Clone)]
struct WidgetImage {
    field: String,
}

#[derive(Clone)]
struct WidgetAudio {
    field: String,
    name_field: Option<String>,
    album_field: Option<String>,
    artist_field: Option<String>,
    thumbnail_field: Option<String>,
}

#[derive(Clone)]
struct WidgetVideo {
    field: String,
    name_field: Option<String>,
    album_field: Option<String>,
    artist_field: Option<String>,
    thumbnail_field: Option<String>,
}

#[derive(Clone)]
struct WidgetQuery {
    query: String,
    layout: Layout,
}

#[derive(Clone)]
enum Layout {
    Single(LayoutSingle),
    Table(LayoutTable),
}

#[derive(Clone)]
enum Widget {
    Nest(LayoutSingle),
    Text(WidgetText),
    Image(WidgetImage),
    Audio(WidgetAudio),
    Video(WidgetVideo),
    Subquery(WidgetQuery),
}

fn extract_node(v: &serde_json::Value) -> Option<Node> {
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

trait PlaylistMedia {
    fn pm_play(&self);
    fn pm_stop(&self);
    fn pm_seek_forward(&self, offset_seconds: f64);
    fn pm_seek_backwards(&self, offset_seconds: f64);
    fn pm_seek_to(&self, time_seconds: f64);
}

impl PlaylistMedia for HtmlAudioElement {
    fn pm_play(&self) {
        _ = self.play().unwrap();
    }

    fn pm_stop(&self) {
        self.pause().unwrap();
    }

    fn pm_seek_forward(&self, offset_seconds: f64) {
        self.set_current_time(self.current_time() + offset_seconds);
    }

    fn pm_seek_backwards(&self, offset_seconds: f64) {
        self.set_current_time(self.current_time() - offset_seconds);
    }

    fn pm_seek_to(&self, time_seconds: f64) {
        self.set_current_time(time_seconds);
    }
}

impl PlaylistMedia for HtmlVideoElement {
    fn pm_play(&self) {
        _ = self.dyn_ref::<HtmlMediaElement>().unwrap().play().unwrap();
    }

    fn pm_stop(&self) {
        self.dyn_ref::<HtmlMediaElement>().unwrap().pause().unwrap();
    }

    fn pm_seek_forward(&self, offset_seconds: f64) {
        let s = self.dyn_ref::<HtmlMediaElement>().unwrap();
        s.set_current_time(s.current_time() + offset_seconds);
    }

    fn pm_seek_backwards(&self, offset_seconds: f64) {
        let s = self.dyn_ref::<HtmlMediaElement>().unwrap();
        s.set_current_time(s.current_time() - offset_seconds);
    }

    fn pm_seek_to(&self, time_seconds: f64) {
        let s = self.dyn_ref::<HtmlMediaElement>().unwrap();
        s.set_current_time(time_seconds);
    }
}

struct PlaylistEntry {
    name: Option<String>,
    album: Option<String>,
    artist: Option<String>,
    thumbnail: Option<FileHash>,
    media: Box<dyn PlaylistMedia>,
}

struct State_ {
    origin: String,
    playlist: RefCell<Vec<PlaylistEntry>>,
    playing: Prim<bool>,
    // Must be Some if playing, otherwise may be Some.
    playing_i: HistPrim<Option<usize>>,
    media_session: MediaSession,
}

type State = Rc<State_>;

fn file_url(origin: &String, hash: &FileHash) -> String {
    return format!("{}/file/{}", origin, hash.to_string());
}

fn el_text_err(text: String) -> El {
    return el("span").classes(&["error"]).text(&text);
}

fn el_image_err(text: String) -> El {
    return el("img").attr("src", &text);
}

fn el_media_button(pc: &mut ProcessingContext, state: &State, entry: usize) -> El {
    return el("button").on("click", {
        let state = state.clone();
        let eg = pc.eg();
        move |_| eg.event(|pc| {
            if *state.playing.borrow() {
                let i = state.playing_i.get().unwrap();
                if i == entry {
                    state.playing.set(pc, false);
                } else {
                    state.playing_i.set(pc, Some(entry));
                }
            } else {
                if state.playlist.borrow().is_empty() {
                    return;
                }
                state.playing_i.set(pc, Some(entry));
                state.playing.set(pc, true);
            }
        })
    }).text("play");
}

fn el_media_button_err(text: String) -> El {
    return el("div").classes(&["error"]).text(&text);
}

fn el_err(text: String) -> El {
    return el("span").classes(&["error"]).text(&text);
}

fn el_group() -> El {
    return el("div").classes(&["group"]);
}

fn build_widget_query(
    pc: &mut ProcessingContext,
    state: &State,
    def: &WidgetQuery,
    data: &HashMap<String, serde_json::Value>,
) -> El {
    return el_group().own(|e| spawn_rooted({
        let def = def.clone();
        let params = data.clone();
        let state = state.clone();
        let eg = pc.eg();
        let e = e.weak();
        async move {
            let Some(ele) = e.upgrade() else {
                return;
            };
            let res = req_post_json::<Vec<HashMap<String, serde_json::Value>>>(&state.origin, C2SReq::Query(Query {
                query: def.query,
                parameters: params,
            })).await;
            ele.ref_clear();
            let rows = match res {
                Ok(rows) => rows,
                Err(e) => {
                    ele.ref_push(el_err(e));
                    return;
                },
            };
            eg.event(|pc| {
                ele.ref_push(build_layout(pc, &state, &def.layout, &rows));
            });
        }
    }));
}

fn build_widget(
    pc: &mut ProcessingContext,
    state: &State,
    def: &Widget,
    data: &HashMap<String, serde_json::Value>,
) -> El {
    match def {
        Widget::Nest(d) => return build_layout_single(pc, state, &d, data),
        Widget::Text(d) => {
            let Some(v) = data.get(&d.field) else {
                return el_text_err(format!("Missing field {}", d.field));
            };
            let mut v = v.clone();
            if let Some(v1) = extract_node_value(&v) {
                v = v1;
            }
            let text = match v {
                serde_json::Value::Null => "".to_string(),
                serde_json::Value::Bool(v) => match v {
                    true => "yes".to_string(),
                    false => "no".to_string(),
                },
                serde_json::Value::Number(v) => v.to_string(),
                serde_json::Value::String(v) => v.clone(),
                serde_json::Value::Array(v) => serde_json::to_string(&v).unwrap(),
                serde_json::Value::Object(v) => serde_json::to_string(&v).unwrap(),
            };
            return el("span").text(&text);
        },
        Widget::Image(d) => {
            let Some(v) = data.get(&d.field) else {
                return el_image_err(format!("Missing field {}", d.field));
            };
            if let Some(n) = extract_node_file(v) {
                return el("img").attr("src", &file_url(&state.origin, &n));
            } else if let serde_json::Value::String(v) = v {
                return el("img").attr("src", &v);
            } else {
                return el_image_err(format!("Field contents wasn't string value node or string: {:?}", v));
            }
        },
        Widget::Audio(d) => {
            let Some(v) = data.get(&d.field) else {
                return el_media_button_err(format!("Missing field {}", d.field));
            };
            let mut playlist = state.playlist.borrow_mut();
            let i = playlist.len();
            let media;
            if let Some(n) = extract_node_file(v) {
                media = Box::new(HtmlAudioElement::new_with_src(&file_url(&state.origin, &n)).unwrap());
            } else if let serde_json::Value::String(v) = v {
                media = Box::new(HtmlAudioElement::new_with_src(&v).unwrap());
            } else {
                return el_media_button_err(format!("Field contents wasn't string value node or string: {:?}", v));
            }
            playlist.push(PlaylistEntry {
                name: d.name_field.as_ref().and_then(|v| data.get(v)).and_then(|v| extract_node_text(v)),
                album: d.album_field.as_ref().and_then(|v| data.get(v)).and_then(|v| extract_node_text(v)),
                artist: d.artist_field.as_ref().and_then(|v| data.get(v)).and_then(|v| extract_node_text(v)),
                thumbnail: d
                    .thumbnail_field
                    .as_ref()
                    .and_then(|v| data.get(v))
                    .and_then(|v| extract_node_file(v)),
                media: media,
            });
            return el_media_button(pc, state, i);
        },
        Widget::Video(d) => {
            let Some(v) = data.get(&d.field) else {
                return el_image_err(format!("Missing field {}", d.field));
            };
            let mut playlist = state.playlist.borrow_mut();
            let i = playlist.len();
            let source = document().create_element("source").unwrap();
            if let Some(n) = extract_node_file(v) {
                source.set_attribute("src", &file_url(&state.origin, &n)).unwrap();
            } else if let serde_json::Value::String(v) = v {
                source.set_attribute("src", &v).unwrap();
            } else {
                return el_media_button_err(format!("Field contents wasn't string value node or string: {:?}", v));
            }
            let video = document().create_element("video").unwrap();
            playlist.push(PlaylistEntry {
                name: d.name_field.as_ref().and_then(|v| data.get(v)).and_then(|v| extract_node_text(v)),
                album: d.album_field.as_ref().and_then(|v| data.get(v)).and_then(|v| extract_node_text(v)),
                artist: d.artist_field.as_ref().and_then(|v| data.get(v)).and_then(|v| extract_node_text(v)),
                thumbnail: d
                    .thumbnail_field
                    .as_ref()
                    .and_then(|v| data.get(v))
                    .and_then(|v| extract_node_file(v)),
                media: Box::new(video.dyn_into::<HtmlVideoElement>().unwrap()),
            });
            return el_media_button(pc, state, i);
        },
        Widget::Subquery(d) => return build_widget_query(pc, state, d, data),
    }
}

fn build_layout_single(
    pc: &mut ProcessingContext,
    state: &State,
    def: &LayoutSingle,
    data: &HashMap<String, serde_json::Value>,
) -> El {
    let row_el = el("div");
    for col_def in &def.children {
        row_el.ref_push(build_widget(pc, state, col_def, data));
    }
    return row_el;
}

fn build_layout(
    pc: &mut ProcessingContext,
    state: &State,
    def: &Layout,
    data: &Vec<HashMap<String, serde_json::Value>>,
) -> El {
    match def {
        Layout::Single(d) => {
            let out = el("div");
            for row_data in data {
                out.ref_push(build_layout_single(pc, state, d, &row_data));
            }
            return out;
        },
        Layout::Table(d) => {
            let out = el("div");
            for (row_i, row_data) in data.iter().enumerate() {
                for (col_i, col_def) in d.children.iter().enumerate() {
                    out.ref_push(
                        el("div")
                            .attr("style", &format!("row: {}; column: {};", row_i + 1, col_i + 1))
                            .push(build_widget(pc, state, &col_def, &row_data)),
                    );
                }
            }
            return out;
        },
    }
}

fn main() {
    let eg = EventGraph::new();
    eg.event(|pc| {
        let origin = window().location().origin().unwrap_throw();
        let media_session = window().navigator().media_session();
        let state = State::new(State_ {
            origin: origin,
            playlist: RefCell::new(vec![]),
            playing: Prim::new(pc, false),
            playing_i: HistPrim::new(pc, None),
            media_session: media_session,
        });
        let root = el("div").own(|_| {
            link!(
                (_pc = pc),
                (playing = state.playing.clone(), playing_i = state.playing_i.clone()),
                (),
                (state = state.clone()) {
                    if !*playing.borrow() {
                        // Stop previous
                        if let Some(i) = playing_i.get_old() {
                            if let Some(e) = state.playlist.borrow().get(i) {
                                e.media.pm_stop();
                            }
                        }
                        state.media_session.set_metadata(None);
                    } else {
                        let mut changed = false;

                        // Stop previous if it changed
                        if let Some(i) = playing_i.get_old() {
                            if Some(i) != playing_i.get() {
                                changed = true;
                                let playlist = state.playlist.borrow();
                                let e = playlist.get(i).unwrap();
                                e.media.pm_stop();
                                e.media.pm_seek_to(0.);
                            }
                        }

                        // Start next/current
                        let i = match playing_i.get() {
                            Some(i) => i,
                            None => {
                                changed = true;
                                0
                            },
                        };
                        let playlist = state.playlist.borrow();
                        let e = playlist.get(i).unwrap();
                        e.media.pm_play();

                        // Update view
                        if changed {
                            state.media_session.set_metadata(Some(&{
                                let m = MediaMetadata::new().unwrap();
                                if let Some(name) = &e.name {
                                    m.set_title(name);
                                }
                                if let Some(album) = &e.album {
                                    m.set_album(album);
                                }
                                if let Some(artist) = &e.artist {
                                    m.set_artist(artist);
                                }
                                if let Some(thumbnail) = &e.thumbnail {
                                    let arr = js_sys::Array::new();
                                    let e = js_sys::Object::new();
                                    js_sys::Reflect::set(
                                        &e,
                                        &JsValue::from("src"),
                                        &JsValue::from(file_url(&state.origin, &thumbnail)),
                                    ).unwrap();
                                    arr.push(e.dyn_ref().unwrap());
                                    m.set_artwork(&arr.dyn_into().unwrap());
                                }
                                m
                            }));
                        }
                    }
                }
            )
        });

        // # Media control
        fn media_fn(pc: &mut ProcessingContext, f: impl 'static + Fn(&mut ProcessingContext, JsValue) -> ()) -> Function {
            let eg = pc.eg();
            let fn1 = Closure::<dyn Fn(JsValue) -> ()>::wrap(Box::new(move |args| eg.event(|pc| f(pc, args))));
            let fn2: Function = fn1.as_ref().unchecked_ref::<Function>().to_owned();
            fn1.forget();
            return fn2;
        }

        state.media_session.set_action_handler(web_sys::MediaSessionAction::Play, Some(&media_fn(pc, {
            let state = state.clone();
            move |pc, _args| {
                if state.playlist.borrow().is_empty() {
                    return;
                }
                if *state.playing.borrow() {
                    return;
                }
                state.playing.set(pc, true);
            }
        })));
        state.media_session.set_action_handler(web_sys::MediaSessionAction::Pause, Some(&media_fn(pc, {
            let state = state.clone();
            move |pc, _args| {
                state.playing.set(pc, false);
            }
        })));
        state.media_session.set_action_handler(web_sys::MediaSessionAction::Stop, Some(&media_fn(pc, {
            let state = state.clone();
            move |pc, _args| {
                state.playing.set(pc, false);
                state.playing_i.set(pc, None);
            }
        })));
        state.media_session.set_action_handler(web_sys::MediaSessionAction::Nexttrack, Some(&media_fn(pc, {
            let state = state.clone();
            move |pc, _args| {
                let Some(i) = state.playing_i.get() else {
                    return;
                };
                if i + 1 < state.playlist.borrow().len() {
                    state.playing_i.set(pc, Some(i + 1));
                } else {
                    state.playing_i.set(pc, None);
                    state.playing.set(pc, false);
                }
            }
        })));
        state.media_session.set_action_handler(web_sys::MediaSessionAction::Previoustrack, Some(&media_fn(pc, {
            let state = state.clone();
            move |pc, _args| {
                let Some(i) = state.playing_i.get() else {
                    return;
                };
                if i > 0 {
                    state.playing_i.set(pc, Some(i - 1));
                } else {
                    state.playing_i.set(pc, None);
                    state.playing.set(pc, false);
                }
            }
        })));
        state.media_session.set_action_handler(web_sys::MediaSessionAction::Seekforward, Some(&media_fn(pc, {
            let state = state.clone();
            move |_pc, args| {
                let Some(i) = state.playing_i.get() else {
                    return;
                };
                let offset = js_sys::Reflect::get(&args, &JsValue::from("seekOffset")).unwrap().as_f64().unwrap();
                state.playlist.borrow().get(i).unwrap().media.pm_seek_forward(offset);
            }
        })));
        state.media_session.set_action_handler(web_sys::MediaSessionAction::Seekbackward, Some(&media_fn(pc, {
            let state = state.clone();
            move |_pc, args| {
                let Some(i) = state.playing_i.get() else {
                    return;
                };
                let offset = js_sys::Reflect::get(&args, &JsValue::from("seekOffset")).unwrap().as_f64().unwrap();
                state.playlist.borrow().get(i).unwrap().media.pm_seek_backwards(offset);
            }
        })));
        state.media_session.set_action_handler(web_sys::MediaSessionAction::Seekto, Some(&media_fn(pc, {
            let state = state.clone();
            move |_pc, args| {
                let Some(i) = state.playing_i.get() else {
                    return;
                };
                let time = js_sys::Reflect::get(&args, &JsValue::from("seekTime")).unwrap().as_f64().unwrap();
                state.playlist.borrow().get(i).unwrap().media.pm_seek_to(time);
            }
        })));

        // # UI
        state.playing.set(pc, false);
        state.playing_i.set(pc, None);
        let query = WidgetQuery {
            query: include_str!("query_albums.datalog").to_string(),
            layout: Layout::Single(LayoutSingle {
                con: Dir::Left,
                trans: Dir::Down,
                children: vec![
                    Widget::Image(WidgetImage { field: "cover".to_string() }),
                    Widget::Nest(LayoutSingle {
                        con: Dir::Down,
                        trans: Dir::Right,
                        children: vec![Widget::Text(WidgetText {
                            field: "album".to_string(),
                            size: "14pt".to_string(),
                        }), Widget::Subquery(WidgetQuery {
                            query: include_str!("query_tracks.datalog").to_string(),
                            layout: Layout::Table(LayoutTable {
                                con: Dir::Down,
                                trans: Dir::Right,
                                children: vec![Widget::Audio(WidgetAudio {
                                    field: "file".to_string(),
                                    name_field: Some("name".to_string()),
                                    album_field: Some("album".to_string()),
                                    artist_field: Some("artist".to_string()),
                                    thumbnail_field: Some("cover".to_string()),
                                }), Widget::Text(WidgetText {
                                    field: "index".to_string(),
                                    size: "12pt".to_string(),
                                }), Widget::Text(WidgetText {
                                    field: "artist".to_string(),
                                    size: "12pt".to_string(),
                                }), Widget::Text(WidgetText {
                                    field: "name".to_string(),
                                    size: "12pt".to_string(),
                                })],
                            }),
                        })],
                    })
                ],
            }),
        };
        root.ref_push(build_widget_query(pc, &state, &query, &HashMap::new()));
        set_root(vec![root]);
    });
}
