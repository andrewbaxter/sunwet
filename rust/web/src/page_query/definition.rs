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
use serde::{
    de::DeserializeOwned,
    Deserialize,
    Serialize,
};
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

#[derive(Serialize, Deserialize, Clone, Copy, rooting_forms::Form)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
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

#[derive(Serialize, Deserialize, Clone, Copy, rooting_forms::Form)]
#[serde(rename_all = "snake_case")]
pub enum Orientation {
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
    pub fn css(self) -> [&'static str; 3] {
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

    pub fn con(self) -> Direction {
        match self {
            Orientation::UpLeft | Orientation::UpRight => return Direction::Up,
            Orientation::DownLeft | Orientation::DownRight => return Direction::Down,
            Orientation::LeftUp | Orientation::LeftDown => return Direction::Left,
            Orientation::RightUp | Orientation::RightDown => return Direction::Right,
        }
    }

    pub fn trans(self) -> Direction {
        match self {
            Orientation::UpLeft | Orientation::DownLeft => return Direction::Left,
            Orientation::UpRight | Orientation::DownRight => return Direction::Right,
            Orientation::LeftUp | Orientation::RightUp => return Direction::Up,
            Orientation::LeftDown | Orientation::RightDown => return Direction::Down,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, rooting_forms::Form)]
#[serde(rename_all = "snake_case")]
pub enum Align {
    #[title("Start")]
    Start,
    #[title("Middle")]
    Middle,
    #[title("End")]
    End,
}

#[derive(Serialize, Deserialize, Clone, rooting_forms::Form)]
#[serde(rename_all = "snake_case")]
pub struct WidgetNest {
    #[title("Orientation")]
    pub orientation: Orientation,
    #[title("Alignment")]
    pub align: Align,
    #[title("Elements")]
    pub children: Vec<Widget>,
}

#[derive(Serialize, Deserialize, Clone, rooting_forms::Form)]
#[serde(rename_all = "snake_case")]
pub struct LayoutIndividual {
    #[title("Orientation")]
    pub orientation: Orientation,
    #[title("Alignment")]
    pub align: Align,
    #[title("Item settings")]
    pub item: WidgetNest,
}

#[derive(Serialize, Deserialize, Clone, rooting_forms::Form)]
#[serde(rename_all = "snake_case")]
pub struct LayoutTable {
    #[title("Orientation")]
    pub orientation: Orientation,
    #[title("Alignment")]
    pub align: Align,
    #[title("Columns")]
    pub columns: Vec<Widget>,
}

#[derive(Serialize, Deserialize, Clone, Copy, rooting_forms::Form)]
#[serde(rename_all = "snake_case")]
pub enum LineSizeMode {
    #[title("Expand to show everything")]
    Full,
    #[title("Ellipsize")]
    Ellipsize,
    #[title("Wrap")]
    Wrap,
    #[title("Scroll")]
    Scroll,
}

#[derive(Serialize, Deserialize, Clone, rooting_forms::Form)]
#[serde(rename_all = "snake_case")]
pub enum FieldOrLiteral {
    #[title("Field")]
    Field(String),
    #[title("Literal")]
    Literal(String),
}

#[derive(Serialize, Deserialize, Clone, rooting_forms::Form)]
#[serde(rename_all = "snake_case")]
pub enum QueryOrField {
    #[title("Field/parameter")]
    Field(String),
    #[title("Query")]
    Query(BigString),
}

#[derive(Serialize, Deserialize, Clone, rooting_forms::Form)]
#[serde(rename_all = "snake_case")]
pub struct WidgetTextLine {
    #[title("Data source")]
    pub data: FieldOrLiteral,
    #[title("Prefix text")]
    pub prefix: String,
    #[title("Suffix text")]
    pub suffix: String,
    #[title("Font size")]
    pub size: String,
    #[title("Line sizing")]
    pub size_mode: LineSizeMode,
    #[title("Orientation")]
    pub orientation: Orientation,
    #[title("Alignment")]
    pub align: Align,
}

#[derive(Serialize, Deserialize, Clone, rooting_forms::Form)]
#[serde(rename_all = "snake_case")]
pub enum BlockSizeMode {
    #[title("Cover area")]
    Cover,
    #[title("Fit into area")]
    Contain,
}

#[derive(Serialize, Deserialize, Clone, rooting_forms::Form)]
#[serde(rename_all = "snake_case")]
pub struct WidgetImage {
    #[title("Data source")]
    pub data: FieldOrLiteral,
    #[title("How to size imge")]
    pub size_mode: BlockSizeMode,
    #[title("Set image width (any valid css measurement)")]
    pub width: Option<String>,
    #[title("Set image height (any valid css measurement)")]
    pub height: Option<String>,
    #[title("Alignment in parent")]
    pub align: Align,
}

#[derive(Serialize, Deserialize, Clone, rooting_forms::Form)]
#[serde(rename_all = "snake_case")]
pub struct WidgetAudio {
    #[title("Name of field containing audio file node")]
    pub field: String,
    #[title("Name of field containing video name value node")]
    pub name_field: Option<String>,
    #[title("Name of field containing album name value node")]
    pub album_field: Option<String>,
    #[title("Name of field containing artist name value node")]
    pub artist_field: Option<String>,
    #[title("Name of field containing thumbnail image file node")]
    pub thumbnail_field: Option<String>,
    #[title("Alignment in parent")]
    pub align: Align,
}

#[derive(Serialize, Deserialize, Clone, rooting_forms::Form)]
#[serde(rename_all = "snake_case")]
pub struct WidgetVideo {
    #[title("Name of field containing video file node")]
    pub field: String,
    #[title("Name of field containing video name value node")]
    pub name_field: Option<String>,
    #[title("Name of field containing album name value node")]
    pub album_field: Option<String>,
    #[title("Name of field containing author name value node")]
    pub artist_field: Option<String>,
    #[title("Name of field containing thumbnail image file node")]
    pub thumbnail_field: Option<String>,
    #[title("Alignment in parent")]
    pub align: Align,
}

#[derive(Serialize, Deserialize, Clone, rooting_forms::Form)]
#[serde(rename_all = "snake_case")]
pub struct WidgetList {
    #[title("Data source")]
    pub data: QueryOrField,
    #[title("Layout for data")]
    pub layout: Layout,
}

#[derive(Serialize, Deserialize, Clone, rooting_forms::Form)]
#[serde(rename_all = "snake_case")]
pub enum Layout {
    #[title("Independently sized")]
    Individual(LayoutIndividual),
    #[title("Table")]
    Table(LayoutTable),
}

#[derive(Serialize, Deserialize, Clone, rooting_forms::Form)]
#[serde(rename_all = "snake_case")]
pub enum Widget {
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
