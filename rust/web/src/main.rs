use std::{
    cell::RefCell,
    collections::HashMap,
    panic,
    rc::Rc,
    str::FromStr,
};
use gloo::{
    console::{
        log,
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
enum Subdir {
    Up,
    Down,
    Left,
    Right,
}

impl Subdir {
    fn css_con(self) -> &'static str {
        match self {
            Subdir::Up => return "converse_up",
            Subdir::Down => return "converse_down",
            Subdir::Left => return "converse_left",
            Subdir::Right => return "converse_right",
        }
    }

    fn css_trans(self) -> &'static str {
        match self {
            Subdir::Up => return "transverse_up",
            Subdir::Down => return "transverse_down",
            Subdir::Left => return "transverse_left",
            Subdir::Right => return "transverse_right",
        }
    }
}

#[derive(Clone, Copy)]
enum Dir {
    UpLeft,
    UpRight,
    DownLeft,
    DownRight,
    LeftUp,
    LeftDown,
    RightUp,
    RightDown,
}

impl Dir {
    fn css(self) -> [&'static str; 3] {
        return [match self {
            Dir::UpLeft => "direction_up_left",
            Dir::UpRight => "direction_up_right",
            Dir::DownLeft => "direction_down_left",
            Dir::DownRight => "direction_down_right",
            Dir::LeftUp => "direction_left_up",
            Dir::LeftDown => "direction_left_down",
            Dir::RightUp => "direction_right_up",
            Dir::RightDown => "direction_right_down",
        }, self.con().css_con(), self.trans().css_trans()];
    }

    fn con(self) -> Subdir {
        match self {
            Dir::UpLeft | Dir::UpRight => return Subdir::Up,
            Dir::DownLeft | Dir::DownRight => return Subdir::Down,
            Dir::LeftUp | Dir::LeftDown => return Subdir::Left,
            Dir::RightUp | Dir::RightDown => return Subdir::Right,
        }
    }

    fn trans(self) -> Subdir {
        match self {
            Dir::UpLeft | Dir::DownLeft => return Subdir::Left,
            Dir::UpRight | Dir::DownRight => return Subdir::Right,
            Dir::LeftUp | Dir::RightUp => return Subdir::Up,
            Dir::LeftDown | Dir::RightDown => return Subdir::Down,
        }
    }
}

#[derive(Clone, Copy)]
enum Align {
    Start,
    Middle,
    End,
}

#[derive(Clone)]
struct WidgetNest {
    direction: Dir,
    align: Align,
    children: Vec<Widget>,
}

#[derive(Clone)]
struct LayoutIndividual {
    direction: Dir,
    align: Align,
    nest: WidgetNest,
}

#[derive(Clone)]
struct LayoutTable {
    direction: Dir,
    align: Align,
    children: Vec<Widget>,
}

#[derive(Clone)]
enum LineSizeMode {
    Full,
    Ellipsize,
    Wrap,
    Scroll,
}

#[derive(Clone)]
struct WidgetTextLineStyle {
    prefix: String,
    suffix: String,
    size: String,
    size_mode: LineSizeMode,
    direction: Dir,
    align: Align,
}

#[derive(Clone)]
struct WidgetConstTextLine {
    text: String,
    style: WidgetTextLineStyle,
}

#[derive(Clone)]
struct WidgetTextLine {
    field: String,
    style: WidgetTextLineStyle,
}

#[derive(Clone)]
enum BlockSizeMode {
    Stretch,
    Cover,
    Contain,
}

#[derive(Clone)]
struct WidgetImage {
    field: String,
    size_mode: BlockSizeMode,
    width: Option<String>,
    height: Option<String>,
    align: Align,
}

#[derive(Clone)]
struct WidgetAudio {
    field: String,
    name_field: Option<String>,
    album_field: Option<String>,
    artist_field: Option<String>,
    thumbnail_field: Option<String>,
    align: Align,
}

#[derive(Clone)]
struct WidgetVideo {
    field: String,
    name_field: Option<String>,
    album_field: Option<String>,
    artist_field: Option<String>,
    thumbnail_field: Option<String>,
    align: Align,
}

#[derive(Clone)]
struct WidgetQuery {
    query: String,
    layout: Layout,
}

#[derive(Clone)]
enum Layout {
    Individual(LayoutIndividual),
    Table(LayoutTable),
}

#[derive(Clone)]
enum Widget {
    Nest(WidgetNest),
    ConstTextLine(WidgetConstTextLine),
    TextLine(WidgetTextLine),
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

struct AudioPlaylistMedia(El);

impl AudioPlaylistMedia {
    fn audio(&self) -> HtmlAudioElement {
        return self.0.raw().dyn_ref::<HtmlAudioElement>().unwrap().to_owned();
    }
}

impl PlaylistMedia for AudioPlaylistMedia {
    fn pm_play(&self) {
        let audio = self.audio();
        _ = audio.play().unwrap();
    }

    fn pm_stop(&self) {
        let audio = self.audio();
        audio.pause().unwrap();
    }

    fn pm_seek_forward(&self, offset_seconds: f64) {
        let audio = self.audio();
        audio.set_current_time(audio.current_time() + offset_seconds);
    }

    fn pm_seek_backwards(&self, offset_seconds: f64) {
        let audio = self.audio();
        audio.set_current_time(audio.current_time() - offset_seconds);
    }

    fn pm_seek_to(&self, time_seconds: f64) {
        let audio = self.audio();
        audio.set_current_time(time_seconds);
    }
}

struct VideoPlaylistMedia(El);

impl VideoPlaylistMedia {
    fn media(&self) -> HtmlMediaElement {
        return self.0.raw().dyn_ref::<HtmlMediaElement>().unwrap().to_owned();
    }
}

impl PlaylistMedia for VideoPlaylistMedia {
    fn pm_play(&self) {
        let s = self.media();
        match s.request_fullscreen() {
            Err(e) => {
                warn!("Failed to fullscreen video: {}", e);
            },
            _ => { },
        }
        _ = s.play().unwrap();
    }

    fn pm_stop(&self) {
        let s = self.media();
        s.pause().unwrap();
        document().exit_fullscreen();
    }

    fn pm_seek_forward(&self, offset_seconds: f64) {
        let s = self.media();
        s.set_current_time(s.current_time() + offset_seconds);
    }

    fn pm_seek_backwards(&self, offset_seconds: f64) {
        let s = self.media();
        s.set_current_time(s.current_time() - offset_seconds);
    }

    fn pm_seek_to(&self, time_seconds: f64) {
        let s = self.media();
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
    playlist: RefCell<Vec<Rc<PlaylistEntry>>>,
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

fn el_stack() -> El {
    return el("div").classes(&["stack"]);
}

fn build_widget_query(
    pc: &mut ProcessingContext,
    state: &State,
    depth: usize,
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
                ele.ref_push(build_layout(pc, &state, depth, &def.layout, &rows));
            });
        }
    }));
}

fn playlist_next(pc: &mut ProcessingContext, state: &State, basis: Option<usize>) {
    let Some(i) = basis else {
        return;
    };
    if i + 1 < state.playlist.borrow().len() {
        state.playing_i.set(pc, Some(i + 1));
    } else {
        state.playing_i.set(pc, None);
        state.playing.set(pc, false);
    }
}

const CSS_TREE: &'static str = "tree";
const CSS_TREE_NEST: &'static str = "tree_nest";
const CSS_TREE_LAYOUT_INDIVIDUAL: &'static str = "tree_layout_individual";
const CSS_TREE_LAYOUT_TABLE: &'static str = "tree_layout_table";
const CSS_TREE_TEXT: &'static str = "tree_text";
const CSS_TREE_IMAGE: &'static str = "tree_image";
const CSS_TREE_MEDIA_BUTTON: &'static str = "tree_media_button";

fn style_tree(w: &El, depth: usize, type_: &str, align: Align) {
    w.ref_classes(&[
        //. .
        CSS_TREE,
        type_,
        &format!("tree_depth_{}", depth),
        if depth % 2 == 0 {
            "tree_depth_even"
        } else {
            "tree_depth_odd"
        },
        match align {
            Align::Start => "align_start",
            Align::Middle => "align_middle",
            Align::End => "align_end",
        },
    ]);
}

fn build_widget_text_line(text: &String, depth: usize, s: &WidgetTextLineStyle) -> El {
    let out = el("span").text(&format!("{}{}{}", s.prefix, text, s.suffix));
    style_tree(&out, depth, CSS_TREE_TEXT, s.align);
    out.ref_classes(&s.direction.css());
    let mut style = vec![];
    style.push(format!("font-size: {}", s.size));
    match s.size_mode {
        LineSizeMode::Full => { },
        LineSizeMode::Ellipsize => style.push(format!("text-overflow: ellipsis")),
        LineSizeMode::Wrap => style.push(format!("overflow-wrap: break-word")),
        LineSizeMode::Scroll => match s.direction.con() {
            Subdir::Up | Subdir::Down => style.push(format!("overflow-x: auto")),
            Subdir::Left | Subdir::Right => style.push(format!("overflow-y: auto")),
        },
    }
    out.ref_attr("style", &style.join("; "));
    return out;
}

fn build_widget(
    pc: &mut ProcessingContext,
    state: &State,
    depth: usize,
    def: &Widget,
    data: &HashMap<String, serde_json::Value>,
) -> El {
    match def {
        Widget::Nest(d) => return build_nest(pc, state, depth, &d, data),
        Widget::ConstTextLine(d) => {
            return build_widget_text_line(&d.text, depth, &d.style);
        },
        Widget::TextLine(d) => {
            let Some(v) = data.get(&d.field) else {
                return el_text_err(format!("Missing field {}", d.field));
            };
            let mut v = v.clone();
            if let Some(v1) = extract_node_value(&v) {
                v = v1;
            }
            let text = match v {
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
            return build_widget_text_line(&text, depth, &d.style);
        },
        Widget::Image(d) => {
            let Some(v) = data.get(&d.field) else {
                return el_image_err(format!("Missing field {}", d.field));
            };
            let out = el("img");
            style_tree(&out, depth, CSS_TREE_IMAGE, d.align);
            if let Some(n) = extract_node_file(v) {
                out.ref_attr("src", &file_url(&state.origin, &n));
            } else if let serde_json::Value::String(v) = v {
                out.ref_attr("src", &v);
            } else {
                return el_image_err(format!("Field contents wasn't string value node or string: {:?}", v));
            }
            let mut style = vec![];
            match d.size_mode {
                BlockSizeMode::Stretch => style.push(format!("object-fit: stretch")),
                BlockSizeMode::Cover => style.push(format!("object-fit: cover")),
                BlockSizeMode::Contain => style.push(format!("object-fit: contain")),
            }
            if let Some(width) = &d.width {
                style.push(format!("width: {}", width));
            }
            if let Some(height) = &d.height {
                style.push(format!("height: {}", height));
            }
            out.ref_attr("style", &style.join("; "));
            return out;
        },
        Widget::Audio(d) => {
            let Some(v) = data.get(&d.field) else {
                return el_media_button_err(format!("Missing field {}", d.field));
            };
            let i = state.playlist.borrow().len();
            let audio = el("audio").on("ended", {
                let state = state.clone();
                let eg = pc.eg();
                move |_| eg.event(|pc| {
                    playlist_next(pc, &state, Some(i));
                })
            });
            if let Some(n) = extract_node_file(v) {
                audio.ref_attr("src", &file_url(&state.origin, &n));
            } else if let serde_json::Value::String(v) = v {
                audio.ref_attr("src", &v);
            } else {
                return el_media_button_err(format!("Field contents wasn't string value node or string: {:?}", v));
            }
            state.playlist.borrow_mut().push(Rc::new(PlaylistEntry {
                name: d.name_field.as_ref().and_then(|v| data.get(v)).and_then(|v| extract_node_text(v)),
                album: d.album_field.as_ref().and_then(|v| data.get(v)).and_then(|v| extract_node_text(v)),
                artist: d.artist_field.as_ref().and_then(|v| data.get(v)).and_then(|v| extract_node_text(v)),
                thumbnail: d
                    .thumbnail_field
                    .as_ref()
                    .and_then(|v| data.get(v))
                    .and_then(|v| extract_node_file(v)),
                media: Box::new(AudioPlaylistMedia(audio)),
            }));
            let out = el_media_button(pc, state, i);
            style_tree(&out, depth, CSS_TREE_MEDIA_BUTTON, d.align);
            return out;
        },
        Widget::Video(d) => {
            let Some(v) = data.get(&d.field) else {
                return el_image_err(format!("Missing field {}", d.field));
            };
            let i = state.playlist.borrow().len();
            let source = el("source");
            if let Some(n) = extract_node_file(v) {
                source.ref_attr("src", &file_url(&state.origin, &n));
            } else if let serde_json::Value::String(v) = v {
                source.ref_attr("src", &v);
            } else {
                return el_media_button_err(format!("Field contents wasn't string value node or string: {:?}", v));
            }
            let video = el("video").push(source).on("ended", {
                let state = state.clone();
                let eg = pc.eg();
                move |_| eg.event(|pc| {
                    playlist_next(pc, &state, Some(i));
                })
            });
            state.playlist.borrow_mut().push(Rc::new(PlaylistEntry {
                name: d.name_field.as_ref().and_then(|v| data.get(v)).and_then(|v| extract_node_text(v)),
                album: d.album_field.as_ref().and_then(|v| data.get(v)).and_then(|v| extract_node_text(v)),
                artist: d.artist_field.as_ref().and_then(|v| data.get(v)).and_then(|v| extract_node_text(v)),
                thumbnail: d
                    .thumbnail_field
                    .as_ref()
                    .and_then(|v| data.get(v))
                    .and_then(|v| extract_node_file(v)),
                media: Box::new(VideoPlaylistMedia(video)),
            }));
            let out = el_media_button(pc, state, i);
            style_tree(&out, depth, CSS_TREE_MEDIA_BUTTON, d.align);
            return out;
        },
        Widget::Subquery(d) => return build_widget_query(pc, state, depth, d, data),
    }
}

fn build_nest(
    pc: &mut ProcessingContext,
    state: &State,
    depth: usize,
    def: &WidgetNest,
    data: &HashMap<String, serde_json::Value>,
) -> El {
    let out = el("div").classes(&def.direction.css());
    style_tree(&out, depth, CSS_TREE_NEST, def.align);
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
    data: &Vec<HashMap<String, serde_json::Value>>,
) -> El {
    match def {
        Layout::Individual(d) => {
            let out = el("div");
            style_tree(&out, depth, CSS_TREE_LAYOUT_INDIVIDUAL, d.align);
            out.ref_classes(&d.direction.css());
            for row_data in data {
                out.ref_push(build_nest(pc, state, depth, &d.nest, &row_data));
            }
            return out;
        },
        Layout::Table(d) => {
            let out = el("div");
            style_tree(&out, depth, CSS_TREE_LAYOUT_TABLE, d.align);
            for (trans_i, trans_data) in data.iter().enumerate() {
                let rev_trans_i = data.len() - trans_i - 1;
                for (con_i, cell_def) in d.children.iter().enumerate() {
                    let rev_con_i = d.children.len() - con_i - 1;
                    let cell_out = el("div");
                    let mut row = None;
                    let mut col = None;
                    match d.direction.con() {
                        Subdir::Up => row = Some(rev_con_i),
                        Subdir::Down => row = Some(con_i),
                        Subdir::Left => col = Some(rev_con_i),
                        Subdir::Right => col = Some(con_i),
                    }
                    match d.direction.trans() {
                        Subdir::Up => row = Some(rev_trans_i),
                        Subdir::Down => row = Some(trans_i),
                        Subdir::Left => col = Some(rev_trans_i),
                        Subdir::Right => col = Some(trans_i),
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

fn main() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
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
                            if let Some(e) = state.playlist.borrow().get(i).cloned() {
                                e.media.pm_stop();
                            }
                        }
                    } else {
                        // Stop previous if it changed
                        if let Some(i) = playing_i.get_old() {
                            if Some(i) != playing_i.get() {
                                let e = state.playlist.borrow().get(i).cloned().unwrap();
                                e.media.pm_stop();
                                e.media.pm_seek_to(0.);
                            }
                        }

                        // Start next/current
                        let i = match playing_i.get() {
                            Some(i) => i,
                            None => {
                                0
                            },
                        };
                        let e = state.playlist.borrow().get(i).cloned().unwrap();
                        e.media.pm_play();
                    }
                    match state.playing_i.get() {
                        Some(i) => {
                            let e = state.playlist.borrow().get(i).cloned().unwrap();
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
                        },
                        None => {
                            state.media_session.set_metadata(None);
                        },
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
                playlist_next(pc, &state, state.playing_i.get());
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
                let offset =
                    js_sys::Reflect::get(&args, &JsValue::from("seekOffset")).unwrap().as_f64().unwrap_or(5.);
                let pm = state.playlist.borrow().get(i).cloned().unwrap();
                pm.media.pm_seek_forward(offset);
            }
        })));
        state.media_session.set_action_handler(web_sys::MediaSessionAction::Seekbackward, Some(&media_fn(pc, {
            let state = state.clone();
            move |_pc, args| {
                let Some(i) = state.playing_i.get() else {
                    return;
                };
                let offset =
                    js_sys::Reflect::get(&args, &JsValue::from("seekOffset")).unwrap().as_f64().unwrap_or(5.);
                let pm = state.playlist.borrow().get(i).cloned().unwrap();
                pm.media.pm_seek_backwards(offset);
            }
        })));
        state.media_session.set_action_handler(web_sys::MediaSessionAction::Seekto, Some(&media_fn(pc, {
            let state = state.clone();
            move |_pc, args| {
                let Some(i) = state.playing_i.get() else {
                    return;
                };
                let time = js_sys::Reflect::get(&args, &JsValue::from("seekTime")).unwrap().as_f64().unwrap();
                let pm = state.playlist.borrow().get(i).cloned().unwrap();
                pm.media.pm_seek_to(time);
            }
        })));

        // # UI
        state.playing.set(pc, false);
        state.playing_i.set(pc, None);
        let query = WidgetQuery {
            query: include_str!("query_albums.datalog").to_string(),
            layout: Layout::Individual(LayoutIndividual {
                direction: Dir::DownRight,
                align: Align::Start,
                nest: WidgetNest {
                    direction: Dir::RightDown,
                    align: Align::Start,
                    children: vec![Widget::Image(WidgetImage {
                        field: "cover".to_string(),
                        size_mode: BlockSizeMode::Cover,
                        width: Some("5cm".to_string()),
                        height: Some("5cm".to_string()),
                        align: Align::Start,
                    }), Widget::Nest(WidgetNest {
                        direction: Dir::DownRight,
                        align: Align::Start,
                        children: vec![Widget::TextLine(WidgetTextLine {
                            field: "album".to_string(),
                            style: WidgetTextLineStyle {
                                prefix: "".to_string(),
                                suffix: "".to_string(),
                                size: "14pt".to_string(),
                                size_mode: LineSizeMode::Ellipsize,
                                direction: Dir::RightDown,
                                align: Align::Start,
                            },
                        }), Widget::Subquery(WidgetQuery {
                            query: include_str!("query_tracks.datalog").to_string(),
                            layout: Layout::Table(LayoutTable {
                                direction: Dir::DownRight,
                                align: Align::Start,
                                children: vec![Widget::Audio(WidgetAudio {
                                    field: "file".to_string(),
                                    name_field: Some("name".to_string()),
                                    album_field: Some("album".to_string()),
                                    artist_field: Some("artist".to_string()),
                                    thumbnail_field: Some("cover".to_string()),
                                    align: Align::Start,
                                }), Widget::TextLine(WidgetTextLine {
                                    field: "index".to_string(),
                                    style: WidgetTextLineStyle {
                                        prefix: "".to_string(),
                                        suffix: ".".to_string(),
                                        size: "12pt".to_string(),
                                        size_mode: LineSizeMode::Full,
                                        direction: Dir::DownRight,
                                        align: Align::End,
                                    },
                                }), Widget::TextLine(WidgetTextLine {
                                    field: "artist".to_string(),
                                    style: WidgetTextLineStyle {
                                        prefix: "".to_string(),
                                        suffix: "".to_string(),
                                        size: "12pt".to_string(),
                                        size_mode: LineSizeMode::Full,
                                        direction: Dir::DownRight,
                                        align: Align::Start,
                                    },
                                }), Widget::ConstTextLine(WidgetConstTextLine {
                                    text: " - ".to_string(),
                                    style: WidgetTextLineStyle {
                                        prefix: "".to_string(),
                                        suffix: "".to_string(),
                                        size: "12pt".to_string(),
                                        size_mode: LineSizeMode::Full,
                                        direction: Dir::DownRight,
                                        align: Align::Start,
                                    },
                                }), Widget::TextLine(WidgetTextLine {
                                    field: "name".to_string(),
                                    style: WidgetTextLineStyle {
                                        prefix: "".to_string(),
                                        suffix: "".to_string(),
                                        size: "12pt".to_string(),
                                        size_mode: LineSizeMode::Full,
                                        direction: Dir::DownRight,
                                        align: Align::Start,
                                    },
                                })],
                            }),
                        })],
                    })],
                },
            }),
        };
        root.ref_push(build_widget_query(pc, &state, 0, &query, &HashMap::new()));
        set_root(vec![root]);
    });
}
