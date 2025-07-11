use {
    crate::interface::triple::Node,
    schemars::JsonSchema,
    serde::{
        Deserialize,
        Serialize,
    },
    ts_rs::TS,
};

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum StrValue {
    Literal(String),
    Parameter(String),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Value {
    Literal(Node),
    Parameter(String),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum MoveDirection {
    Forward,
    Backward,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum FilterExprExistsType {
    Exists,
    DoesntExist,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum FilterSuffixSimpleOperator {
    Eq,
    Neq,
    Lt,
    Gt,
    Lte,
    Gte,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FilterSuffixSimple {
    pub op: FilterSuffixSimpleOperator,
    pub value: Value,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FilterSuffixLike {
    pub value: StrValue,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum FilterSuffix {
    Simple(FilterSuffixSimple),
    Like(FilterSuffixLike),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FilterExprExistance {
    pub type_: FilterExprExistsType,
    pub subchain: ChainBody,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub suffix: Option<FilterSuffix>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FilterExprDisjunction {
    pub first: Box<FilterExpr>,
    pub second: Box<FilterExpr>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Copy, Debug, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum JunctionType {
    And,
    Or,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FilterExprJunction {
    pub type_: JunctionType,
    pub subexprs: Vec<FilterExpr>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum FilterExpr {
    Exists(FilterExprExistance),
    Junction(FilterExprJunction),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct StepMove {
    pub dir: MoveDirection,
    pub predicate: StrValue,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub filter: Option<FilterExpr>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    #[serde(default)]
    pub first: bool,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct StepRecurse {
    pub subchain: ChainBody,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    #[serde(default)]
    pub first: bool,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct StepJunction {
    pub type_: JunctionType,
    pub subchains: Vec<ChainBody>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Step {
    Move(StepMove),
    Recurse(StepRecurse),
    Junction(StepJunction),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ChainRoot {
    Value(Value),
    Search(StrValue),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChainBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub root: Option<ChainRoot>,
    pub steps: Vec<Step>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Chain {
    pub body: ChainBody,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub bind: Option<String>,
    #[serde(skip_serializing_if = "std::vec::Vec::is_empty")]
    #[serde(default)]
    pub subchains: Vec<Chain>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum QuerySortDir {
    Asc,
    Desc,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum QuerySort {
    Random,
    Fields(Vec<(QuerySortDir, String)>),
}

/// Right now, all fields are turned into a single top level record - this is
/// useful for recursion which could otherwise lead to large nested objects when a
/// flat list is desired.  A new `nest` step may be introduced later to create
/// intermediate records (as `QueryResType::Record`).
#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Query {
    pub chain: Chain,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub sort: Option<QuerySort>,
}
