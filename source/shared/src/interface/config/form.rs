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
    ts_rs::TS,
};

#[derive(Serialize, Deserialize, Clone, Debug, Hash, JsonSchema, TS, PartialEq, Eq, PartialOrd, Ord)]
#[serde(
    //. TS bug https://github.com/Aleph-Alpha/ts-rs/issues/421: rename_all = "snake_case",
    deny_unknown_fields
)]
pub struct FormId(pub String);

impl std::fmt::Display for FormId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return self.0.fmt(f);
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FormFieldComment {
    pub text: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FormFieldText {
    #[serde(default)]
    pub placeholder: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FormFieldNumber {
    #[serde(default)]
    pub placeholder: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FormFieldBool {
    #[serde(default)]
    pub initial_on: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FormFieldRgbU8 {
    #[serde(default)]
    pub initial: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FormFieldConstEnum {
    pub choices: Vec<(String, Node)>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FormFieldQueryEnum {
    pub query: Query,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum FormFieldType {
    /// Generate a unique id (uuid) - no visible entry.
    Id,
    /// Add text to the form, no interactive entry.
    Comment(FormFieldComment),
    Text(FormFieldText),
    Number(FormFieldNumber),
    Bool(FormFieldBool),
    Date,
    Time,
    Datetime,
    RgbU8(FormFieldRgbU8),
    /// Present a selection of fixed choices.
    ConstEnum(FormFieldConstEnum),
    /// Present a selection of choices by performing a query. The query must return two
    /// fields: `name` (the text presented to the user) and `id` (the value to store in
    /// the relation).
    QueryEnum(FormFieldQueryEnum),
    File,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FormField {
    pub id: String,
    pub label: String,
    pub r#type: FormFieldType,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum InputOrInlineText {
    Input(String),
    Inline(String),
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum InputOrInline {
    Input(String),
    Inline(Node),
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FormOutput {
    pub subject: InputOrInline,
    pub predicate: InputOrInlineText,
    pub object: InputOrInline,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ClientForm {
    /// Form fields and generated data (ids)
    pub fields: Vec<FormField>,
    /// Triples to generate from the inputs
    pub outputs: Vec<FormOutput>,
}
