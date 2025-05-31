use {
    crate::interface::triple::Node,
    schemars::JsonSchema,
    serde::{
        Deserialize,
        Serialize,
    },
    std::collections::{
        BTreeMap,
    },
};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, JsonSchema, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, JsonSchema, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
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
    pub fn conv(self) -> Direction {
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

#[derive(Serialize, Deserialize, Clone, Copy, Debug, JsonSchema, Hash, Default)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum TransAlign {
    #[default]
    Start,
    Middle,
    End,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, JsonSchema, Hash, Default)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum TextSizeMode {
    #[default]
    Wrap,
    Ellipsize,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum FieldOrLiteral {
    Field(String),
    Literal(Node),
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum FieldOrLiteralString {
    Field(String),
    Literal(String),
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum QueryOrField {
    Field(String),
    Query(String),
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Link {
    pub value: FieldOrLiteral,
    pub title: FieldOrLiteral,
    pub to_node: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WidgetText {
    pub data: FieldOrLiteralString,
    #[serde(default)]
    pub prefix: String,
    #[serde(default)]
    pub suffix: String,
    #[serde(default)]
    pub font_size: Option<String>,
    #[serde(default)]
    pub cons_size_mode: TextSizeMode,
    #[serde(default)]
    pub cons_size_max: Option<String>,
    pub orientation: Orientation,
    #[serde(default)]
    pub trans_align: TransAlign,
    #[serde(default)]
    pub link: Option<Link>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WidgetDate {
    pub data: FieldOrLiteralString,
    #[serde(default)]
    pub prefix: String,
    #[serde(default)]
    pub suffix: String,
    #[serde(default)]
    pub font_size: Option<String>,
    pub orientation: Orientation,
    #[serde(default)]
    pub trans_align: TransAlign,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WidgetTime {
    pub data: FieldOrLiteralString,
    #[serde(default)]
    pub prefix: String,
    #[serde(default)]
    pub suffix: String,
    #[serde(default)]
    pub font_size: Option<String>,
    pub orientation: Orientation,
    #[serde(default)]
    pub trans_align: TransAlign,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WidgetDatetime {
    pub data: FieldOrLiteralString,
    #[serde(default)]
    pub prefix: String,
    #[serde(default)]
    pub suffix: String,
    #[serde(default)]
    pub font_size: Option<String>,
    pub orientation: Orientation,
    #[serde(default)]
    pub trans_align: TransAlign,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WidgetColor {
    pub data: FieldOrLiteralString,
    #[serde(default)]
    pub width: String,
    #[serde(default)]
    pub height: String,
    pub orientation: Orientation,
    #[serde(default)]
    pub trans_align: TransAlign,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WidgetMedia {
    pub data: FieldOrLiteral,
    #[serde(default)]
    pub alt: Option<FieldOrLiteral>,
    /// For image/video, the width.  For audio, the length of the controls regardless
    /// of direction.
    #[serde(default)]
    pub width: Option<String>,
    #[serde(default)]
    pub height: Option<String>,
    // For audio, the controls orientation direction.
    #[serde(default)]
    pub direction: Option<Direction>,
    #[serde(default)]
    pub trans_align: TransAlign,
    #[serde(default)]
    pub link: Option<Link>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WidgetPlayButton {
    pub media_file_field: String,
    #[serde(default)]
    pub name_field: Option<String>,
    #[serde(default)]
    pub album_field: Option<String>,
    #[serde(default)]
    pub artist_field: Option<String>,
    #[serde(default)]
    pub cover_field: Option<String>,
    #[serde(default)]
    pub orientation: Option<Orientation>,
    #[serde(default)]
    pub trans_align: TransAlign,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DataRowsLayoutUnaligned {
    #[serde(default)]
    pub gap: Option<String>,
    #[serde(default)]
    pub x_scroll: bool,
    pub direction: Option<Direction>,
    #[serde(default)]
    pub trans_align: TransAlign,
    pub widget: Box<Widget>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DataRowsLayoutTable {
    #[serde(default)]
    pub gap: Option<String>,
    #[serde(default)]
    pub x_scroll: bool,
    pub orientation: Orientation,
    pub elements: Vec<Widget>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum DataRowsLayout {
    Unaligned(DataRowsLayoutUnaligned),
    Table(DataRowsLayoutTable),
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WidgetDataRows {
    /// Where to get the data for the sublist.
    pub data: QueryOrField,
    /// How the data rows are displayed.
    pub row_widget: DataRowsLayout,
    #[serde(default)]
    pub trans_align: TransAlign,
    #[serde(default)]
    pub x_scroll: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Block {
    /// Sets the default width of the block. If not specified, space will be divided
    /// with other unsized blocks.
    pub width: Option<String>,
    /// The contents of the block.
    pub widget: Widget,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WidgetRootDataRows {
    /// Where to get the data for the sublist.
    pub data: QueryOrField,
    /// How the data rows are displayed.
    pub row_blocks: Vec<Block>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WidgetLayout {
    pub direction: Direction,
    #[serde(default)]
    pub trans_align: TransAlign,
    #[serde(default)]
    pub x_scroll: bool,
    pub elements: Vec<Widget>,
    #[serde(default)]
    pub gap: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Widget {
    Layout(WidgetLayout),
    DataRows(WidgetDataRows),
    Text(WidgetText),
    Date(WidgetDate),
    Time(WidgetTime),
    Datetime(WidgetDatetime),
    Color(WidgetColor),
    Media(WidgetMedia),
    PlayButton(WidgetPlayButton),
    Space,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ClientViewParam {
    /// A simple text box.
    ///
    /// Note that if this is used as part of a `search` root in a query, it must follow
    /// sqlite's `fts5` syntax. Basically, you need at least one string with quotes
    /// around it.
    Text,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ClientView {
    pub id: String,
    pub name: String,
    pub root: WidgetRootDataRows,
    pub parameters: BTreeMap<String, ClientViewParam>,
    pub query_parameters: BTreeMap<String, Vec<String>>,
}
