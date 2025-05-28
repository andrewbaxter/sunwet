use {
    crate::derive_canonical_serde,
    schemars::JsonSchema,
    serde::{
        de,
        Deserialize,
        Serialize,
    },
    sha2::{
        digest::{
            generic_array::GenericArray,
            OutputSizeUser,
        },
        Sha256,
    },
    std::hash::Hash,
};

const HASH_PREFIX_SHA256: &'static str = "sha256";

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum FileHash_ {
    Sha256(String),
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, JsonSchema)]
pub struct FileHash(pub FileHash_);

impl FileHash {
    pub fn from_sha256(hash: GenericArray<u8, <Sha256 as OutputSizeUser>::OutputSize>) -> Self {
        return Self(FileHash_::Sha256(hex::encode(&hash)));
    }
}

impl ToString for FileHash {
    fn to_string(&self) -> String {
        let prefix;
        let hash;
        match &self.0 {
            FileHash_::Sha256(v) => {
                prefix = HASH_PREFIX_SHA256;
                hash = v;
            },
        }
        return format!("{}:{}", prefix, hash);
    }
}

impl std::str::FromStr for FileHash {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Some((prefix, suffix)) = s.split_once(':') else {
            return Err("Invalid file hash; missing colon separating prefix and suffix".to_string());
        };
        match prefix {
            HASH_PREFIX_SHA256 => {
                const WANT_LEN: usize = 64;
                if suffix.len() != WANT_LEN {
                    return Err(format!("Invalid file hash; expected length {} but got {}", WANT_LEN, suffix.len()));
                }
                return Ok(FileHash(FileHash_::Sha256(suffix.to_string())));
            },
            _ => {
                return Err(format!("Invalid file hash; unknown hash prefix [{}]", prefix));
            },
        }
    }
}

derive_canonical_serde!(FileHash);

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Node {
    File(FileHash),
    Value(serde_json::Value),
}

impl JsonSchema for Node {
    fn schema_name() -> String {
        return "Node".to_string();
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        return SerdeNode::json_schema(gen);
    }
}

fn hash_value<H: std::hash::Hasher>(s: &serde_json::Value, state: &mut H) {
    core::mem::discriminant(s).hash(state);
    match s {
        serde_json::Value::Null => { },
        serde_json::Value::Bool(s) => {
            s.hash(state);
        },
        serde_json::Value::Number(n) => {
            n.to_string().hash(state);
        },
        serde_json::Value::String(s) => {
            s.hash(state);
        },
        serde_json::Value::Array(s) => {
            for v in s {
                hash_value(v, state);
            }
        },
        serde_json::Value::Object(s) => {
            for (k, v) in s {
                k.hash(state);
                hash_value(v, state);
            }
        },
    }
}

impl Hash for Node {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            Node::File(s) => s.hash(state),
            Node::Value(s) => hash_value(s, state),
        }
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        return Some(self.cmp(other));
    }
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        {
            fn prio(n: &Node) -> u8 {
                return match n {
                    Node::File(_) => 1,
                    Node::Value(_) => 2,
                };
            }

            let self_prio = prio(self);
            let other_prio = prio(other);
            if self_prio != other_prio {
                return self_prio.cmp(&other_prio);
            }
        }
        match (self, other) {
            (Node::File(self_v), Node::File(other_v)) => {
                return self_v.cmp(other_v);
            },
            (Node::Value(self_v), Node::Value(other_v)) => {
                fn json_cmp_seq(
                    self_iter: &mut dyn Iterator<Item = Option<&serde_json::Value>>,
                    other_iter: &mut dyn Iterator<Item = Option<&serde_json::Value>>,
                ) -> std::cmp::Ordering {
                    for (s, o) in Iterator::zip(&mut *self_iter, &mut *other_iter) {
                        if s.is_none() && o.is_none() {
                            continue;
                        }
                        if s.is_some() {
                            return std::cmp::Ordering::Greater;
                        } else if o.is_some() {
                            return std::cmp::Ordering::Less;
                        }
                        let s = s.unwrap();
                        let o = o.unwrap();
                        let c = json_cmp(&s, &o);
                        if c == std::cmp::Ordering::Equal {
                            continue;
                        }
                        return c;
                    }
                    if self_iter.next().is_some() {
                        return std::cmp::Ordering::Greater;
                    } else if other_iter.next().is_some() {
                        return std::cmp::Ordering::Less;
                    } else {
                        return std::cmp::Ordering::Equal;
                    }
                }

                fn json_cmp(self_v: &serde_json::Value, other_v: &serde_json::Value) -> std::cmp::Ordering {
                    {
                        fn prio(v: &serde_json::Value) -> u8 {
                            return match v {
                                serde_json::Value::Null => 0,
                                serde_json::Value::Bool(_) => 1,
                                serde_json::Value::Number(_) => 2,
                                serde_json::Value::String(_) => 3,
                                serde_json::Value::Array(_) => 4,
                                serde_json::Value::Object(_) => 5,
                            };
                        }

                        let self_prio = prio(self_v);
                        let other_prio = prio(other_v);
                        if self_prio != other_prio {
                            return self_prio.cmp(&other_prio);
                        }
                    }
                    match (self_v, other_v) {
                        (serde_json::Value::Null, serde_json::Value::Null) => {
                            return std::cmp::Ordering::Equal;
                        },
                        (serde_json::Value::Bool(self_v), serde_json::Value::Bool(other_v)) => {
                            return self_v.cmp(other_v);
                        },
                        (serde_json::Value::Number(self_v), serde_json::Value::Number(other_v)) => {
                            #[derive(Clone, Copy)]
                            enum NumEnum {
                                U64(u64),
                                I64(i64),
                                F64(f64),
                            }

                            fn to_enum(v: &serde_json::Number) -> NumEnum {
                                if v.is_u64() {
                                    return NumEnum::U64(v.as_u64().unwrap());
                                } else if v.is_i64() {
                                    return NumEnum::I64(v.as_i64().unwrap());
                                } else if v.is_f64() {
                                    return NumEnum::F64(v.as_f64().unwrap());
                                } else {
                                    unreachable!();
                                }
                            }

                            let self_v = to_enum(self_v);
                            let other_v = to_enum(other_v);
                            {
                                fn prio(v: NumEnum) -> u8 {
                                    return match v {
                                        NumEnum::U64(_) => 0,
                                        NumEnum::I64(_) => 1,
                                        NumEnum::F64(_) => 2,
                                    };
                                }

                                let self_prio = prio(self_v);
                                let other_prio = prio(other_v);
                                if self_prio != other_prio {
                                    return self_prio.cmp(&other_prio);
                                }
                            }
                            match (self_v, other_v) {
                                (NumEnum::U64(self_v), NumEnum::U64(other_v)) => {
                                    return self_v.cmp(&other_v);
                                },
                                (NumEnum::I64(self_v), NumEnum::I64(other_v)) => {
                                    return self_v.cmp(&other_v);
                                },
                                (NumEnum::F64(self_v), NumEnum::F64(other_v)) => {
                                    return self_v.total_cmp(&other_v);
                                },
                                _ => unreachable!(),
                            }
                        },
                        (serde_json::Value::String(self_v), serde_json::Value::String(other_v)) => {
                            return self_v.cmp(other_v);
                        },
                        (serde_json::Value::Array(self_v), serde_json::Value::Array(other_v)) => {
                            return json_cmp_seq(
                                &mut self_v.iter().map(|x| Some(x)),
                                &mut other_v.iter().map(|x| Some(x)),
                            );
                        },
                        (serde_json::Value::Object(self_v), serde_json::Value::Object(other_v)) => {
                            let mut ord_keys = vec![];
                            ord_keys.reserve(self_v.len() + other_v.len());
                            ord_keys.extend(self_v.keys());
                            ord_keys.extend(other_v.keys());
                            ord_keys.sort();
                            return json_cmp_seq(
                                &mut ord_keys.iter().map(|k| self_v.get(*k)),
                                &mut ord_keys.iter().map(|k| other_v.get(*k)),
                            );
                        },
                        _ => unreachable!(),
                    }
                }

                return json_cmp(self_v, other_v);
            },
            _ => unreachable!(),
        }
    }
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
enum SerdeNodeType {
    F,
    V,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
struct SerdeNode_ {
    t: SerdeNodeType,
    v: serde_json::Value,
}

#[derive(JsonSchema)]
struct SerdeNode(SerdeNode_);

derive_canonical_serde!(SerdeNode);

impl Serialize for Node {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        return Ok(match self {
            Node::File(n) => SerdeNode(SerdeNode_ {
                t: SerdeNodeType::F,
                v: serde_json::to_value(n).unwrap(),
            }),
            Node::Value(n) => SerdeNode(SerdeNode_ {
                t: SerdeNodeType::V,
                v: n.clone(),
            }),
        }.serialize(serializer)?);
    }
}

impl<'de> Deserialize<'de> for Node {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de> {
        let n = SerdeNode::deserialize(deserializer)?;
        match n.0.t {
            SerdeNodeType::F => {
                let v = serde_json::from_value::<FileHash>(n.0.v).map_err(|e| de::Error::custom(e.to_string()))?;
                return Ok(Node::File(v));
            },
            SerdeNodeType::V => {
                return Ok(Node::Value(n.0.v));
            },
        }
    }
}
