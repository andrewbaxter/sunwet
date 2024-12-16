use {
    serde::{
        Deserialize,
        Serialize,
    },
    crate::interface::triple::Node,
};

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Value {
    Literal(Node),
    Parameter(String),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum MoveDirection {
    Down,
    Up,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum FilterExprExistsType {
    Exists,
    DoesntExist,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum FilterChainComparisonOperator {
    Eq,
    Lt,
    Gt,
    Lte,
    Gte,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FilterExprExists {
    pub type_: FilterExprExistsType,
    pub subchain: Subchain,
    pub filter: Option<(FilterChainComparisonOperator, Value)>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FilterExprDisjunction {
    pub first: Box<FilterExpr>,
    pub second: Box<FilterExpr>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Copy, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum JunctionType {
    And,
    Or,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FilterExprJunction {
    pub type_: JunctionType,
    pub subexprs: Vec<FilterExpr>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum FilterExpr {
    Exists(FilterExprExists),
    Junction(FilterExprJunction),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct StepMove {
    pub dir: MoveDirection,
    pub predicate: String,
    pub filter: Option<FilterExpr>,
    pub first: bool,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct StepRecurse {
    pub subchain: Subchain,
    pub first: bool,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct StepJunction {
    pub type_: JunctionType,
    pub subchains: Vec<Subchain>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Step {
    Move(StepMove),
    Recurse(StepRecurse),
    Junction(StepJunction),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Subchain {
    pub root: Option<Value>,
    pub steps: Vec<Step>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Chain {
    pub select: Option<String>,
    pub subchain: Subchain,
    pub children: Vec<Chain>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum QuerySortDir {
    Asc,
    Desc,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Query {
    pub chain: Chain,
    pub sort: Vec<(QuerySortDir, String)>,
}
