use {
    serde::{
        Deserialize,
        Serialize,
    },
    std::hash::Hash,
};

const HASH_PREFIX_SHA256: &'static str = "sha256";

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum FileHash {
    Sha256(String),
}

impl ToString for FileHash {
    fn to_string(&self) -> String {
        let prefix;
        let hash;
        match self {
            FileHash::Sha256(v) => {
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
                return Ok(FileHash::Sha256(suffix.to_string()));
            },
            _ => {
                return Err(format!("Invalid file hash; unknown hash prefix [{}]", prefix));
            },
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct Node(pub serde_json::Value);

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
        hash_value(&self.0, state);
    }
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
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

        return json_cmp(&self.0, &other.0);
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        return Some(self.cmp(other));
    }
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
