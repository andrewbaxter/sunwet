use std::{
    any::Any,
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
use rooting_forms::{
    BigString,
    Form,
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

#[derive(Clone, Copy, rooting_forms::Form)]
enum Direction {
    #[title("Up")]
    Up,
    #[title("Down")]
    Down,
    #[title("Left")]
    Left,
    #[title("Right")]
    Right,
}

impl Direction {
    fn css_con(self) -> &'static str {
        match self {
            Direction::Up => return "converse_up",
            Direction::Down => return "converse_down",
            Direction::Left => return "converse_left",
            Direction::Right => return "converse_right",
        }
    }

    fn css_trans(self) -> &'static str {
        match self {
            Direction::Up => return "transverse_up",
            Direction::Down => return "transverse_down",
            Direction::Left => return "transverse_left",
            Direction::Right => return "transverse_right",
        }
    }
}

#[derive(Clone, Copy, rooting_forms::Form)]
enum Orientation {
    #[title("Bottom-top, right-left")]
    UpLeft,
    #[title("Bottom-top, left-right")]
    UpRight,
    #[title("Top-bottom, right-left")]
    DownLeft,
    #[title("Top-bottom, left-right")]
    DownRight,
    #[title("Right-left, bottom-top")]
    LeftUp,
    #[title("Right-left, top-bottom")]
    LeftDown,
    #[title("Left-right, bottom-top")]
    RightUp,
    #[title("Left-right, top-bottom")]
    RightDown,
}

impl Orientation {
    fn css(self) -> [&'static str; 3] {
        return [match self {
            Orientation::UpLeft => "orientation_up_left",
            Orientation::UpRight => "orientation_up_right",
            Orientation::DownLeft => "orientation_down_left",
            Orientation::DownRight => "orientation_down_right",
            Orientation::LeftUp => "orientation_left_up",
            Orientation::LeftDown => "orientation_left_down",
            Orientation::RightUp => "orientation_right_up",
            Orientation::RightDown => "orientation_right_down",
        }, self.con().css_con(), self.trans().css_trans()];
    }

    fn con(self) -> Direction {
        match self {
            Orientation::UpLeft | Orientation::UpRight => return Direction::Up,
            Orientation::DownLeft | Orientation::DownRight => return Direction::Down,
            Orientation::LeftUp | Orientation::LeftDown => return Direction::Left,
            Orientation::RightUp | Orientation::RightDown => return Direction::Right,
        }
    }

    fn trans(self) -> Direction {
        match self {
            Orientation::UpLeft | Orientation::DownLeft => return Direction::Left,
            Orientation::UpRight | Orientation::DownRight => return Direction::Right,
            Orientation::LeftUp | Orientation::RightUp => return Direction::Up,
            Orientation::LeftDown | Orientation::RightDown => return Direction::Down,
        }
    }
}

#[derive(Clone, Copy, rooting_forms::Form)]
enum Align {
    #[title("Start")]
    Start,
    #[title("Middle")]
    Middle,
    #[title("End")]
    End,
}

#[derive(Clone, rooting_forms::Form)]
struct WidgetNest {
    #[title("Orientation")]
    orientation: Orientation,
    #[title("Alignment")]
    align: Align,
    #[title("Elements")]
    children: Vec<Widget>,
}

#[derive(Clone, rooting_forms::Form)]
struct LayoutIndividual {
    #[title("Orientation")]
    orientation: Orientation,
    #[title("Alignment")]
    align: Align,
    #[title("Item settings")]
    item: WidgetNest,
}

#[derive(Clone, rooting_forms::Form)]
struct LayoutTable {
    #[title("Orientation")]
    orientation: Orientation,
    #[title("Alignment")]
    align: Align,
    #[title("Columns")]
    columns: Vec<Widget>,
}

#[derive(Clone, Copy, rooting_forms::Form)]
enum LineSizeMode {
    #[title("Expand to show everything")]
    Full,
    #[title("Ellipsize")]
    Ellipsize,
    #[title("Wrap")]
    Wrap,
    #[title("Scroll")]
    Scroll,
}

#[derive(Clone, rooting_forms::Form)]
enum FieldOrLiteral {
    #[title("Field")]
    Field(String),
    #[title("Literal")]
    Literal(String),
}

#[derive(Clone, rooting_forms::Form)]
enum QueryOrField {
    #[title("Field/parameter")]
    Field(String),
    #[title("Query")]
    Query(BigString),
}

#[derive(Clone, rooting_forms::Form)]
struct WidgetTextLine {
    #[title("Data source")]
    data: FieldOrLiteral,
    #[title("Prefix text")]
    prefix: String,
    #[title("Suffix text")]
    suffix: String,
    #[title("Font size")]
    size: String,
    #[title("Line sizing")]
    size_mode: LineSizeMode,
    #[title("Orientation")]
    orientation: Orientation,
    #[title("Alignment")]
    align: Align,
}

#[derive(Clone, rooting_forms::Form)]
enum BlockSizeMode {
    #[title("Cover area")]
    Cover,
    #[title("Fit into area")]
    Contain,
}

#[derive(Clone, rooting_forms::Form)]
struct WidgetImage {
    #[title("Data source")]
    data: FieldOrLiteral,
    #[title("How to size imge")]
    size_mode: BlockSizeMode,
    #[title("Set image width (any valid css measurement)")]
    width: Option<String>,
    #[title("Set image height (any valid css measurement)")]
    height: Option<String>,
    #[title("Alignment in parent")]
    align: Align,
}

#[derive(Clone, rooting_forms::Form)]
struct WidgetAudio {
    #[title("Name of field containing audio file node")]
    field: String,
    #[title("Name of field containing video name value node")]
    name_field: Option<String>,
    #[title("Name of field containing album name value node")]
    album_field: Option<String>,
    #[title("Name of field containing artist name value node")]
    artist_field: Option<String>,
    #[title("Name of field containing thumbnail image file node")]
    thumbnail_field: Option<String>,
    #[title("Alignment in parent")]
    align: Align,
}

#[derive(Clone, rooting_forms::Form)]
struct WidgetVideo {
    #[title("Name of field containing video file node")]
    field: String,
    #[title("Name of field containing video name value node")]
    name_field: Option<String>,
    #[title("Name of field containing album name value node")]
    album_field: Option<String>,
    #[title("Name of field containing author name value node")]
    artist_field: Option<String>,
    #[title("Name of field containing thumbnail image file node")]
    thumbnail_field: Option<String>,
    #[title("Alignment in parent")]
    align: Align,
}

#[derive(Clone, rooting_forms::Form)]
struct WidgetList {
    #[title("Data source")]
    data: QueryOrField,
    #[title("Layout for data")]
    layout: Layout,
}

#[derive(Clone, rooting_forms::Form)]
enum Layout {
    #[title("Independently sized")]
    Individual(LayoutIndividual),
    #[title("Table")]
    Table(LayoutTable),
}

#[derive(Clone, rooting_forms::Form)]
enum Widget {
    #[title("Nested")]
    Nest(WidgetNest),
    #[title("Text (single line)")]
    TextLine(WidgetTextLine),
    #[title("Image")]
    Image(WidgetImage),
    #[title("Audio")]
    Audio(WidgetAudio),
    #[title("Video")]
    Video(WidgetVideo),
    #[title("Expand sublist")]
    Sublist(WidgetList),
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
    main: El,
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

fn style_tree(type_: &str, depth: usize, align: Align, widget: &El) {
    widget.ref_classes(&[
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
            style_tree(CSS_TREE_MEDIA_BUTTON, depth, d.align, &out);
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
            for (trans_i, trans_data) in data.iter().enumerate() {
                let rev_trans_i = data.len() - trans_i - 1;
                for (con_i, cell_def) in d.columns.iter().enumerate() {
                    let rev_con_i = d.columns.len() - con_i - 1;
                    let cell_out = el("div");
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

fn build_page_query(pc: &mut ProcessingContext, state: &State, original_def: Rc<WidgetList>) {
    let current_def = Rc::new(RefCell::new(original_def.as_ref().clone()));
    let edit = Prim::new(pc, false);
    let view_body = el_group();
    let title_middle = el_group();
    state.main.ref_replace(vec![el("div").classes(&["titlebar"]).extend(vec![
        //. .
        el("div").classes(&["title"]).text("Albums"),
        el_hbox().classes(&["transport"]).extend(vec![
            //. .
            el("div").classes(&["left"]),
            el("div").classes(&["middle"]).extend(vec![
                //. .
                el_button_icon("previous").on("click", {
                    let eg = pc.eg();
                    move |_| eg.event(|pc| { })
                }),
                el_stack().extend(vec![
                    //. .
                    el("div").classes(&["time_layer", "time_gutter"]),
                    el("div").classes(&["time_layer", "time_fill"]),
                    el("span").classes(&["time_layer", "time_label"])
                ]).on("click", {
                    let eg = pc.eg();
                    move |_| eg.event(|pc| { })
                }),
                el_button_icon("next").on("click", {
                    let eg = pc.eg();
                    move |_| eg.event(|pc| { })
                }),
                el_button_icon_switch("play", "pause", &state.playing).on("click", {
                    let eg = pc.eg();
                    move |_| eg.event(|pc| { })
                })
            ]),
            el("div").classes(&["right"])
        ]),
        el_hbox().classes(&["cornernav"]).extend(vec![
            //. .
            el_button_icon_toggle_switch("edit", "view", "Edit view", edit).on("click", {
                let eg = pc.eg();
                let view_body = view_body.weak();
                let title_middle = title_middle.clone();
                move |_| eg.event(|pc| {
                    let Some(view_body) = view_body.upgrade() else {
                        return;
                    };
                })
            }),
            el_button_icon("menu", "Menu").on("click", {
                let eg = pc.eg();
                move |_| eg.event(|pc| { })
            })
        ])
    ]).own(|_| {
        link!(
            (pc = pc),
            (edit = edit.clone()),
            (),
            (current_def = current_def.clone(), title_middle = title_middle.clone(), view_body = view_body.clone()) {
                state.playing.set(pc, false);
                state.playing_i.set(pc, None);
                if edit.borrow() {
                    let form_state = WidgetList::new_form("", Some(&def));
                    title_middle.ref_replace(
                        vec![el("div").classes(&["edit_buttons"]).extend(vec![el_button("Save: New").on("click", {
                            let eg = pc.eg();
                            move |_| eg.event(|pc| { })
                        }), el_button("Save: Replace").on("click", {
                            let eg = pc.eg();
                            move |_| eg.event(|pc| { })
                        }), el_button("Discard").on("click", {
                            let eg = pc.eg();
                            move |_| eg.event(|pc| { })
                        })])],
                    );
                    let form_state_elements = form_state.elements();
                    let mut form_elements = vec![];
                    if let Some(error) = form_state_elements.error {
                        form_elements.push(error);
                    }
                    form_elements.extend(form_state_elements.elements);
                    view_body.ref_replace(form_elements);
                } else {
                    view_body.ref_replace(
                        vec![build_widget_query(pc, &state, 0, &query, &Rc::new(HashMap::new()))],
                    );
                }
            }
        )
    })]);
}

fn main() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    let eg = EventGraph::new();
    eg.event(|pc| {
        let origin = window().location().origin().unwrap_throw();
        let media_session = window().navigator().media_session();
        let show_sidebar = Prim::new(pc, false);
        let main = el_group();
        let state = State::new(State_ {
            origin: origin,
            playlist: RefCell::new(vec![]),
            playing: Prim::new(pc, false),
            playing_i: HistPrim::new(pc, None),
            media_session: media_session,
            main: main,
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
        build_page_query(Rc::new(WidgetList {
            data: QueryOrField::Query(BigString(include_str!("query_albums.datalog").to_string())),
            layout: Layout::Individual(LayoutIndividual {
                orientation: Orientation::DownRight,
                align: Align::Start,
                item: WidgetNest {
                    orientation: Orientation::RightDown,
                    align: Align::Start,
                    children: vec![
                        //. .
                        Widget::Image(WidgetImage {
                            data: FieldOrLiteral::Field("cover".to_string()),
                            size_mode: BlockSizeMode::Cover,
                            width: Some("5cm".to_string()),
                            height: Some("5cm".to_string()),
                            align: Align::Start,
                        }),
                        Widget::Nest(WidgetNest {
                            orientation: Orientation::DownRight,
                            align: Align::Start,
                            children: vec![
                                //. .
                                Widget::TextLine(WidgetTextLine {
                                    data: FieldOrLiteral::Field("album".to_string()),
                                    prefix: "".to_string(),
                                    suffix: "".to_string(),
                                    size: "14pt".to_string(),
                                    size_mode: LineSizeMode::Ellipsize,
                                    orientation: Orientation::RightDown,
                                    align: Align::Start,
                                }),
                                Widget::Sublist(WidgetList {
                                    data: QueryOrField::Query(
                                        BigString(include_str!("query_tracks.datalog").to_string()),
                                    ),
                                    layout: Layout::Table(LayoutTable {
                                        orientation: Orientation::DownRight,
                                        align: Align::Start,
                                        columns: vec![
                                            //. .
                                            Widget::Audio(WidgetAudio {
                                                field: "file".to_string(),
                                                name_field: Some("name".to_string()),
                                                album_field: Some("album".to_string()),
                                                artist_field: Some("artist".to_string()),
                                                thumbnail_field: Some("cover".to_string()),
                                                align: Align::Start,
                                            }),
                                            Widget::TextLine(WidgetTextLine {
                                                data: FieldOrLiteral::Field("index".to_string()),
                                                prefix: "".to_string(),
                                                suffix: ".".to_string(),
                                                size: "12pt".to_string(),
                                                size_mode: LineSizeMode::Full,
                                                orientation: Orientation::DownRight,
                                                align: Align::End,
                                            }),
                                            Widget::TextLine(WidgetTextLine {
                                                data: FieldOrLiteral::Field("artist".to_string()),
                                                prefix: "".to_string(),
                                                suffix: "".to_string(),
                                                size: "12pt".to_string(),
                                                size_mode: LineSizeMode::Full,
                                                orientation: Orientation::DownRight,
                                                align: Align::Start,
                                            }),
                                            Widget::TextLine(WidgetTextLine {
                                                data: FieldOrLiteral::Literal(" - ".to_string()),
                                                prefix: "".to_string(),
                                                suffix: "".to_string(),
                                                size: "12pt".to_string(),
                                                size_mode: LineSizeMode::Full,
                                                orientation: Orientation::DownRight,
                                                align: Align::Start,
                                            }),
                                            Widget::TextLine(WidgetTextLine {
                                                data: FieldOrLiteral::Field("name".to_string()),
                                                prefix: "".to_string(),
                                                suffix: "".to_string(),
                                                size: "12pt".to_string(),
                                                size_mode: LineSizeMode::Full,
                                                orientation: Orientation::DownRight,
                                                align: Align::Start,
                                            })
                                        ],
                                    }),
                                })
                            ],
                        })
                    ],
                },
            }),
        }));
        set_root(vec![
            //. .
            el("div").extend(vec![
                //. .
                el("div").classes(&["sidebar"]).own(|e| link!()).extend(vec![
                    //. .
                    el_group().own(|e| link!()),
                    el_button_icon_text("settings", "Settings").on("click", {
                        let eg = eg.clone();
                        move |_| eg.event(|pc| { })
                    })
                ]),
                state.main.clone()
            ]).own(|_| {
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
            })
        ]);
    });
}
