use {
    crate::interface::{
        config::form::FormId,
        triple::Node,
    },
    schemars::JsonSchema,
    serde::{
        Deserialize,
        Serialize,
    },
    std::collections::BTreeMap,
    ts_rs::TS,
};

#[derive(Serialize, Deserialize, Clone, Debug, Hash, JsonSchema, TS, PartialEq, Eq, PartialOrd, Ord)]
#[serde(
    //. rename_all = "snake_case",
    deny_unknown_fields
)]
pub struct ViewId(pub String);

impl std::fmt::Display for ViewId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return self.0.fmt(f);
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, JsonSchema, TS, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, JsonSchema, TS, Hash)]
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

#[derive(Serialize, Deserialize, Clone, Copy, Debug, JsonSchema, TS, Hash, Default)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum TransAlign {
    #[default]
    Start,
    Middle,
    End,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, JsonSchema, TS, Hash, Default)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum TextSizeMode {
    #[default]
    Wrap,
    Ellipsize,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, TS, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum FieldOrLiteral {
    Field(String),
    Literal(Node),
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, TS, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum FieldOrLiteralString {
    Field(String),
    Literal(String),
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, TS, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum QueryOrField {
    Field(String),
    Query(String),
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, TS, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct LinkDestView {
    pub id: ViewId,
    /// Provide initial query parameters.
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub parameters: BTreeMap<String, FieldOrLiteral>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, TS, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct LinkDestForm {
    pub id: FormId,
    /// Provide other initial parameters for fields, by field id.
    pub parameters: BTreeMap<String, FieldOrLiteral>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, TS, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum LinkDest {
    Plain(FieldOrLiteral),
    View(LinkDestView),
    Form(LinkDestForm),
    Node(FieldOrLiteral),
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, TS, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Link {
    pub title: FieldOrLiteral,
    pub dest: LinkDest,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, TS, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WidgetText {
    pub data: FieldOrLiteralString,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub prefix: String,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub suffix: String,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub font_size: Option<String>,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub color: Option<String>,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub conv_size_mode: TextSizeMode,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub conv_size_max: Option<String>,
    pub orientation: Orientation,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub trans_align: TransAlign,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub link: Option<Link>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, TS, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WidgetDate {
    pub data: FieldOrLiteralString,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub prefix: String,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub suffix: String,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub font_size: Option<String>,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub color: Option<String>,
    pub orientation: Orientation,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub trans_align: TransAlign,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, TS, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WidgetTime {
    pub data: FieldOrLiteralString,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub prefix: String,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub suffix: String,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub font_size: Option<String>,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub color: Option<String>,
    pub orientation: Orientation,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub trans_align: TransAlign,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, TS, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WidgetDatetime {
    pub data: FieldOrLiteralString,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub prefix: String,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub suffix: String,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub font_size: Option<String>,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub color: Option<String>,
    pub orientation: Orientation,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub trans_align: TransAlign,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, TS, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WidgetColor {
    pub data: FieldOrLiteralString,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub width: String,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub height: String,
    pub orientation: Orientation,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub trans_align: TransAlign,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, TS, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WidgetMedia {
    pub data: FieldOrLiteral,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub alt: Option<FieldOrLiteral>,
    /// For image/video, the width.  For audio, the length of the controls regardless
    /// of direction.
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub width: Option<String>,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub height: Option<String>,
    // For audio, the controls orientation direction.
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub direction: Option<Direction>,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub trans_align: TransAlign,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub link: Option<Link>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, TS, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WidgetIcon {
    /// The unicode string for the google material icon font icon
    pub data: String,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub width: Option<String>,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub height: Option<String>,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub link: Option<Link>,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub color: Option<String>,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub trans_align: TransAlign,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub orientation: Option<Orientation>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, TS, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WidgetPlayButton {
    pub media_file_field: String,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub show_image: bool,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub width: Option<String>,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub height: Option<String>,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub name_field: Option<String>,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub album_field: Option<String>,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub artist_field: Option<String>,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub cover_field: Option<String>,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub orientation: Option<Orientation>,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub trans_align: TransAlign,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, TS, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DataRowsLayoutUnaligned {
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub gap: Option<String>,
    pub direction: Option<Direction>,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub trans_align: TransAlign,
    pub widget: Box<Widget>,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub conv_scroll: bool,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub conv_size_max: Option<String>,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub trans_size_max: Option<String>,
    // Wrap layout instead of shrinking elements individually first when out of space.
    // Can't be set at the same time as x_scroll or undefined things will happen.
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub wrap: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, TS, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DataRowsLayoutTable {
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub gap: Option<String>,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub conv_scroll: bool,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub conv_size_max: Option<String>,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub trans_size_max: Option<String>,
    pub orientation: Orientation,
    pub elements: Vec<Widget>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, TS, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum DataRowsLayout {
    Unaligned(DataRowsLayoutUnaligned),
    Table(DataRowsLayoutTable),
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, TS, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WidgetDataRows {
    /// Where to get the data for the sublist.
    pub data: QueryOrField,
    /// How the data rows are displayed.
    pub row_widget: DataRowsLayout,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub trans_align: TransAlign,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, TS, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Block {
    /// Sets the default width of the block. If not specified, space will be divided
    /// with other unsized blocks.
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub width: Option<String>,
    /// The contents of the block.
    pub widget: Widget,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, TS, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WidgetRootDataRows {
    /// Where to get the data for the sublist.
    pub data: QueryOrField,
    /// How the data rows are displayed.
    pub row_blocks: Vec<Block>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, TS, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WidgetLayout {
    pub direction: Direction,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub trans_align: TransAlign,
    pub elements: Vec<Widget>,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub gap: Option<String>,
    // Add a scrollbar to the layout that appears when it exceeds bounds (typically
    // horizontal direction only).
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub conv_scroll: bool,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub conv_size_max: Option<String>,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub trans_size_max: Option<String>,
    // Wrap layout instead of shrinking elements individually first when out of space.
    // Can't be set at the same time as x_scroll or undefined things will happen.
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub wrap: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, TS, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WidgetNode {
    pub name: FieldOrLiteralString,
    pub node: FieldOrLiteral,
    pub orientation: Orientation,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub trans_align: TransAlign,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, TS, Hash)]
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
    Icon(WidgetIcon),
    PlayButton(WidgetPlayButton),
    Space,
    Node(WidgetNode),
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, TS, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ClientViewParam {
    /// A simple text box.
    ///
    /// Note that if this is used as part of a `search` root in a query, it must follow
    /// sqlite's `fts5` syntax. Basically, you need at least one string with quotes
    /// around it.
    Text,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ClientView {
    pub root: WidgetRootDataRows,
    pub parameters: BTreeMap<String, ClientViewParam>,
    pub query_parameters: BTreeMap<String, Vec<String>>,
    pub shuffle: bool,
}
