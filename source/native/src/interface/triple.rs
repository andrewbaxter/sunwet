use {
    good_ormning_runtime::sqlite::GoodOrmningCustomString,
    shared::interface::triple::Node,
};

#[derive(Clone, Debug)]
pub struct DbNode(pub Node);

impl GoodOrmningCustomString<DbNode> for DbNode {
    fn to_sql<'a>(value: &'a Self) -> std::borrow::Cow<'a, str> {
        return serde_json::to_string(&value.0).unwrap().into();
    }

    fn from_sql(value: String) -> Result<Self, String> {
        return Ok(DbNode(serde_json::from_str::<Node>(&value).map_err(|e| e.to_string())?));
    }
}
