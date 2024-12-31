use {
    good_ormning_runtime::sqlite::GoodOrmningCustomString,
    serde::{
        Deserialize,
        Serialize,
    },
    shared::interface::triple::{
        FileHash,
        Node,
    },
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DbNode(pub Node);

impl GoodOrmningCustomString<DbNode> for DbNode {
    fn to_sql<'a>(value: &'a Self) -> String {
        return serde_json::to_string(&value.0).unwrap();
    }

    fn from_sql(value: String) -> Result<Self, String> {
        return Ok(Self(serde_json::from_str(&value).map_err(|e| e.to_string())?));
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DbFileHash(pub FileHash);

impl GoodOrmningCustomString<DbFileHash> for DbFileHash {
    fn to_sql<'a>(value: &'a Self) -> String {
        return serde_json::to_string(&value.0).unwrap();
    }

    fn from_sql(value: String) -> Result<Self, String> {
        return Ok(Self(serde_json::from_str(&value).map_err(|e| e.to_string())?));
    }
}
