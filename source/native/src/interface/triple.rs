use {
    good_ormning_runtime::sqlite::GoodOrmningCustomString,
    serde::{
        de,
        Deserialize,
        Serialize,
    },
};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum FileHash {
    Sha256(String),
}

#[derive(Clone)]
pub enum Node {
    Id(String),
    File(FileHash),
    Value(serde_json::Value),
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum SerdeNodeType {
    I,
    F,
    V,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
struct SerdeNode {
    t: SerdeNodeType,
    v: serde_json::Value,
}

impl Serialize for Node {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        return Ok(match self {
            Node::Id(n) => SerdeNode {
                t: SerdeNodeType::I,
                v: serde_json::to_value(n).unwrap(),
            },
            Node::File(n) => SerdeNode {
                t: SerdeNodeType::F,
                v: serde_json::to_value(n).unwrap(),
            },
            Node::Value(n) => SerdeNode {
                t: SerdeNodeType::V,
                v: n.clone(),
            },
        }.serialize(serializer)?);
    }
}

impl<'de> Deserialize<'de> for Node {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de> {
        let n = SerdeNode::deserialize(deserializer)?;
        match n.t {
            SerdeNodeType::I => {
                let serde_json::Value::String(v) = n.v else {
                    return Err(de::Error::custom(format!("ID node value is not a string")));
                };
                return Ok(Node::Id(v));
            },
            SerdeNodeType::F => {
                let v = serde_json::from_value::<FileHash>(n.v).map_err(|e| de::Error::custom(e.to_string()))?;
                return Ok(Node::File(v));
            },
            SerdeNodeType::V => {
                return Ok(Node::Value(n.v));
            },
        }
    }
}

impl GoodOrmningCustomString<Node> for Node {
    fn to_sql<'a>(value: &'a Node) -> std::borrow::Cow<'a, str> {
        return serde_json::to_string(value).unwrap().into();
    }

    fn from_sql(value: String) -> Result<Node, String> {
        return serde_json::from_str(&value).map_err(|e| e.to_string())?;
    }
}
