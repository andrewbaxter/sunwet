use serde::{
    Deserialize,
    Serialize,
};

#[derive(Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Up,
    Down,
    Left,
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

#[derive(Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum Orientation {
    UpLeft,
    UpRight,
    DownLeft,
    DownRight,
    LeftUp,
    LeftDown,
    RightUp,
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

#[derive(Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum Align {
    Start,
    Middle,
    End,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct WidgetNest {
    pub orientation: Orientation,
    pub align: Align,
    pub children: Vec<Widget>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct LayoutIndividual {
    pub orientation: Orientation,
    pub align: Align,
    pub x_scroll: bool,
    pub item: WidgetNest,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct LayoutTable {
    pub orientation: Orientation,
    pub align: Align,
    pub x_scroll: bool,
    pub columns: Vec<Widget>,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum LineSizeMode {
    Ellipsize,
    Wrap,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum FieldOrLiteral {
    Field(String),
    Literal(String),
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum QueryParameter {
    Text,
    Number,
    Bool,
    Datetime,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum QueryOrField {
    Field(String),
    Query(String),
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct WidgetTextLine {
    pub data: FieldOrLiteral,
    pub prefix: String,
    pub suffix: String,
    pub size: String,
    pub size_mode: LineSizeMode,
    pub orientation: Orientation,
    pub align: Align,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum BlockSizeMode {
    Cover,
    Contain,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct WidgetImage {
    pub data: FieldOrLiteral,
    pub size_mode: BlockSizeMode,
    pub width: String,
    pub height: String,
    pub align: Align,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct WidgetAudio {
    pub field: String,
    pub name_field: String,
    pub album_field: String,
    pub artist_field: String,
    pub thumbnail_field: String,
    pub align: Align,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct WidgetVideo {
    pub field: String,
    pub name_field: String,
    pub album_field: String,
    pub artist_field: String,
    pub thumbnail_field: String,
    pub align: Align,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct ViewPartList {
    /// Where to get the data for the sublist.
    pub data: QueryOrField,
    /// A field of the returned data that can be used as a unique key for
    /// saving/restoring position in playback.
    pub key_field: String,
    /// How to display the received data.
    pub layout: Layout,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Layout {
    /// Each row is layed out with independent sizing.
    Individual(LayoutIndividual),
    /// Rows are laid out as a grid/table.
    Table(LayoutTable),
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Widget {
    Nest(WidgetNest),
    TextLine(WidgetTextLine),
    Image(WidgetImage),
    Audio(WidgetAudio),
    Video(WidgetVideo),
    Sublist(ViewPartList),
}
