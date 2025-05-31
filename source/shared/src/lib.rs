pub mod interface;

// (Hopefully, mostly) canonical serialization by going to value before
// serialization; value uses BTreeMap which sorts keys.
#[macro_export]
macro_rules! derive_canonical_serde{
    ($t: ty) => {
        impl serde:: Serialize for $t {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer {
                return serde_json::to_value(&self.0).unwrap().serialize(serializer);
            }
        }
        impl <'a > serde:: Deserialize <'a > for $t {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'a> {
                return Ok(
                    Self(
                        serde_json::from_value(
                            serde_json::Value::deserialize(deserializer)?,
                        ).map_err(serde::de::Error::custom)?,
                    ),
                );
            }
        }
    };
}
