use {
    good_ormning::runtime::sqlite::GoodOrmningCustomString,
    shared::interface::triple::{
        FileHash,
        Node,
        StrNode,
    },
    serde::{
        Serialize,
        Deserialize,
    },
    std::str::FromStr,
};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DbNode(pub Node);

impl GoodOrmningCustomString<DbNode> for DbNode {
    fn to_sql<'a>(value: &'a Self) -> String {
        return StrNode(value.0.clone()).to_string();
    }

    fn from_sql(value: String) -> Result<Self, String> {
        if value.starts_with("f=") || value.starts_with("v=") {
            return Ok(Self(StrNode::from_str(&value)?.0));
        }
        return Ok(Self(serde_json::from_str(&value).map_err(|e| e.to_string())?));
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DbFileHash(pub FileHash);

impl GoodOrmningCustomString<DbFileHash> for DbFileHash {
    fn to_sql<'a>(value: &'a Self) -> String {
        return value.0.to_string();
    }

    fn from_sql(value: String) -> Result<Self, String> {
        if value.contains(':') {
            return Ok(Self(FileHash::from_str(&value)?));
        }
        return Ok(Self(serde_json::from_str(&value).map_err(|e| e.to_string())?));
    }
}
