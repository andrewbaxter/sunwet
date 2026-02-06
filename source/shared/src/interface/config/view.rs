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

#[derive(Serialize, Deserialize, Clone, Copy, Debug, JsonSchema, TS, Hash, Default)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Orientation {
    UpLeft,
    UpRight,
    DownLeft,
    DownRight,
    LeftUp,
    LeftDown,
    RightUp,
    #[default]
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
    /// Set the media aspect ratio. Exactly one of `width` or `height` must also be
    /// specified. Can be any valid css aspect ratio.
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub aspect: Option<String>,
    // For audio, the controls orientation direction.
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub audio_direction: Option<Direction>,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub orientation: Option<Orientation>,
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
    /// The converse direction is the direction of elements. The transverse direction
    /// is only used for `trans_align` in children. If unspecified, keep the parent
    /// widget's orientation.
    pub orientation: Option<Orientation>,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub conv_align: TransAlign,
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
    pub trans_scroll: bool,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub conv_size_max: Option<String>,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub trans_size_max: Option<String>,
    /// The converse direction is the direction of cells in a row. The transitive
    /// direction is the direction of rows.
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
pub struct WidgetRootDataRows {
    /// Where to get the data for the sublist.
    pub data: QueryOrField,
    /// How each element is displayed
    pub element_body: Widget,
    /// The width of the body of each element. If blank, defaults to 100%. Takes any
    /// css size value, including `calc`.
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub element_width: Option<String>,
    /// The height of the body of each element. If blank, uses the row's tallest
    /// element height. Takes any css size value, including `calc`. This must be
    /// specified if you provide an expansion (in order to prevent the expansion from
    /// covering up any of the row elements).
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub element_height: Option<String>,
    /// When the element body is clicked, this expansion is toggled on the next row.
    /// It's always 100% width.
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub element_expansion: Option<Widget>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, TS, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WidgetLayout {
    /// The converse direction is the direction of elements. The transverse direction
    /// is only important for `trans_align` in child widgets.
    pub orientation: Orientation,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub trans_align: TransAlign,
    pub elements: Vec<Widget>,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub gap: Option<String>,
    /// Add a scrollbar to the layout that appears when it exceeds bounds (typically
    /// horizontal direction only).
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub conv_scroll: bool,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub conv_size_max: Option<String>,
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub trans_size_max: Option<String>,
    /// Wrap layout instead of shrinking elements individually first when out of space.
    /// Can't be set at the same time as x_scroll or undefined things will happen.
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
    pub parameter_specs: BTreeMap<String, ClientViewParam>,
    pub query_parameter_keys: BTreeMap<String, Vec<String>>,
    pub shuffle: bool,
}
