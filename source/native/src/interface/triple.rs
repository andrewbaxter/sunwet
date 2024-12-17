use {
    good_ormning_runtime::sqlite::GoodOrmningCustomString,
    shared::interface::{
        iam::IamTargetId,
        triple::Node,
    },
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DbNode(pub Node);

impl GoodOrmningCustomString<DbNode> for DbNode {
    fn to_sql<'a>(value: &'a Self) -> String {
        return serde_json::to_string(&value.0).unwrap();
    }

    fn from_sql(value: String) -> Result<Self, String> {
        return Ok(DbNode(serde_json::from_str(&value).map_err(|e| e.to_string())?));
    }
}

#[derive(Clone, Debug)]
pub struct DbIamTargetId(pub IamTargetId);

impl GoodOrmningCustomString<DbIamTargetId> for DbIamTargetId {
    fn to_sql<'a>(value: &'a Self) -> String {
        return serde_json::to_string(&value.0).unwrap();
    }

    fn from_sql(value: String) -> Result<Self, String> {
        return Ok(DbIamTargetId(serde_json::from_str(&value).map_err(|e| e.to_string())?));
    }
}

#[derive(Clone, Debug)]
pub struct DbIamTargetIds(pub Vec<IamTargetId>);

impl GoodOrmningCustomString<DbIamTargetIds> for DbIamTargetIds {
    fn to_sql<'a>(value: &'a Self) -> String {
        return serde_json::to_string(&value.0).unwrap();
    }

    fn from_sql(value: String) -> Result<Self, String> {
        return Ok(DbIamTargetIds(serde_json::from_str(&value).map_err(|e| e.to_string())?));
    }
}
