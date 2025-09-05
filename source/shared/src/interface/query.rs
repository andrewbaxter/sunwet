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
    pub subchain: ChainHead,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
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
    #[ts(optional, as = "Option<_>")]
    pub filter: Option<FilterExpr>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct StepRecurse {
    pub subchain: ChainHead,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct StepJunction {
    pub type_: JunctionType,
    pub subchains: Vec<ChainHead>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum SortDir {
    Asc,
    Desc,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum SortQuery {
    Shuffle,
    Fields(Vec<(SortDir, String)>),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Step {
    pub specific: StepSpecific,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub sort: Option<SortDir>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub first: bool,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum StepSpecific {
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
pub struct ChainHead {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub root: Option<ChainRoot>,
    pub steps: Vec<Step>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChainTail {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub bind: Option<String>,
    #[serde(skip_serializing_if = "std::vec::Vec::is_empty")]
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub subchains: Vec<Chain>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Chain {
    pub head: ChainHead,
    pub tail: ChainTail,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Query {
    pub chain: Chain,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[ts(optional, as = "Option<_>")]
    pub sort: Option<SortQuery>,
}
