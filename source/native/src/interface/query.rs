use {
    super::triple::Node,
};

#[derive(PartialEq, Eq, PartialOrd, Hash, Clone)]
pub enum Value {
    Literal(Node),
    Parameter(String),
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub enum MoveDirection {
    Down,
    Up,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub enum FilterExprComparisonType {
    Exists,
    DoesntExist,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub enum FilterChainComparisonOperator {
    Eq,
    Lt,
    Gt,
    Lte,
    Gte,
}

#[derive(PartialEq, Eq, PartialOrd, Hash, Clone)]
pub struct FilterExprComparison {
    pub type_: FilterExprComparisonType,
    pub subchain: Subchain,
    pub operator: FilterChainComparisonOperator,
    pub value: Value,
}

#[derive(PartialEq, Eq, PartialOrd, Hash, Clone)]
pub struct FilterExprDisjunction {
    pub first: Box<FilterExpr>,
    pub second: Box<FilterExpr>,
}

#[derive(PartialEq, Eq, PartialOrd, Hash, Clone, Copy)]
pub enum JunctionType {
    And,
    Or,
}

#[derive(PartialEq, Eq, PartialOrd, Hash, Clone)]
pub struct FilterExprJunction {
    pub type_: JunctionType,
    pub subexprs: Vec<FilterExpr>,
}

#[derive(PartialEq, Eq, PartialOrd, Hash, Clone)]
pub enum FilterExpr {
    Comparison(FilterExprComparison),
    Junction(FilterExprJunction),
}

#[derive(PartialEq, Eq, PartialOrd, Hash, Clone)]
pub struct StepMove {
    pub dir: MoveDirection,
    pub predicate: String,
    pub filter: Option<FilterExpr>,
    pub first: bool,
}

#[derive(PartialEq, Eq, PartialOrd, Hash, Clone)]
pub struct StepRecurse {
    pub subchain: Subchain,
    pub first: bool,
}

#[derive(PartialEq, Eq, PartialOrd, Hash, Clone)]
pub struct StepJunction {
    pub type_: JunctionType,
    pub subchains: Vec<Subchain>,
}

#[derive(PartialEq, Eq, PartialOrd, Hash, Clone)]
pub enum Step {
    Move(StepMove),
    Recurse(StepRecurse),
    Junction(StepJunction),
}

#[derive(PartialEq, Eq, PartialOrd, Hash, Clone)]
pub struct Subchain {
    pub root: Option<Value>,
    pub steps: Vec<Step>,
}

pub struct Chain {
    pub subchain: Subchain,
    pub select: Option<String>,
    pub children: Vec<Chain>,
}

pub enum QuerySortDir {
    Asc,
    Desc,
}

pub struct Query {
    pub chain: Chain,
    pub sort: Vec<(QuerySortDir, String)>,
}
