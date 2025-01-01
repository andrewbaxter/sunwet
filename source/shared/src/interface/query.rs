use {
    crate::interface::triple::Node,
    schemars::JsonSchema,
    serde::{
        Deserialize,
        Serialize,
    },
};

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Value {
    Literal(Node),
    Parameter(String),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum MoveDirection {
    Down,
    Up,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum FilterExprExistsType {
    Exists,
    DoesntExist,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum FilterSuffixSimpleOperator {
    Eq,
    Neq,
    Lt,
    Gt,
    Lte,
    Gte,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FilterSuffixSimple {
    pub op: FilterSuffixSimpleOperator,
    pub value: Value,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FilterSuffixLike {
    pub value: String,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum FilterSuffix {
    Simple(FilterSuffixSimple),
    Like(FilterSuffixLike),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FilterExprExists {
    pub type_: FilterExprExistsType,
    pub subchain: ChainBody,
    pub suffix: Option<FilterSuffix>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FilterExprDisjunction {
    pub first: Box<FilterExpr>,
    pub second: Box<FilterExpr>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Copy, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum JunctionType {
    And,
    Or,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FilterExprJunction {
    pub type_: JunctionType,
    pub subexprs: Vec<FilterExpr>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum FilterExpr {
    Exists(FilterExprExists),
    Junction(FilterExprJunction),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct StepMove {
    pub dir: MoveDirection,
    pub predicate: String,
    pub filter: Option<FilterExpr>,
    pub first: bool,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct StepRecurse {
    pub subchain: ChainBody,
    pub first: bool,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct StepJunction {
    pub type_: JunctionType,
    pub subchains: Vec<ChainBody>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Step {
    Move(StepMove),
    Recurse(StepRecurse),
    Junction(StepJunction),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ChainRoot {
    Value(Value),
    Search(String),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChainBody {
    pub root: Option<ChainRoot>,
    pub steps: Vec<Step>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Chain {
    pub body: ChainBody,
    pub select: Option<String>,
    pub subchains: Vec<Chain>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum QuerySortDir {
    Asc,
    Desc,
}

/// Right now, all fields are turned into a single top level record - this is
/// useful for recursion which could otherwise lead to large nested objects when a
/// flat list is desired.  A new `nest` step may be introduced later to create
/// intermediate records (as `QueryResType::Record`).
#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Query {
    pub chain: Chain,
    pub sort: Vec<(QuerySortDir, String)>,
}
