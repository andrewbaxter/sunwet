use {
    crate::interface::{
        query::Query,
        triple::Node,
    },
    schemars::JsonSchema,
    serde::{
        Deserialize,
        Serialize,
    },
};

#[derive(Serialize, Deserialize, Clone, Debug, Hash, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FormFieldId {
    pub id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FormFieldComment {
    pub text: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FormFieldConstant {
    pub id: String,
    pub value: Node,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FormFieldText {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub placeholder: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FormFieldNumber {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub placeholder: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FormFieldBool {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub initial_on: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FormFieldDate {
    pub id: String,
    pub label: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FormFieldTime {
    pub id: String,
    pub label: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FormFieldDatetime {
    pub id: String,
    pub label: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FormFieldRgbU8 {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub initial: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FormFieldConstEnum {
    pub id: String,
    pub label: String,
    pub choices: Vec<(String, Node)>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FormFieldQueryEnum {
    pub id: String,
    pub label: String,
    pub query: Query,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum FormField {
    /// Generate a unique id (uuid) - no visible entry.
    Id(FormFieldId),
    /// Add text to the form, no interactive entry.
    Comment(FormFieldComment),
    Text(FormFieldText),
    Number(FormFieldNumber),
    Bool(FormFieldBool),
    Date(FormFieldDate),
    Time(FormFieldTime),
    Datetime(FormFieldDatetime),
    RgbU8(FormFieldRgbU8),
    /// Present a selection of fixed choices.
    ConstEnum(FormFieldConstEnum),
    /// Present a selection of choices by performing a query. The query must return two
    /// fields: `name` (the text presented to the user) and `id` (the value to store in
    /// the relation).
    QueryEnum(FormFieldQueryEnum),
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum InputOrInlineText {
    Input(String),
    Inline(String),
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum InputOrInline {
    Input(String),
    Inline(Node),
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FormOutput {
    pub subject: InputOrInline,
    pub predicate: InputOrInlineText,
    pub object: InputOrInline,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ClientForm {
    /// Form fields and generated data (ids)
    pub fields: Vec<FormField>,
    /// Triples to generate from the inputs
    pub outputs: Vec<FormOutput>,
}
