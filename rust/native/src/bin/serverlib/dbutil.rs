use std::collections::HashMap;
use cozo::{
    DataValue,
    Num,
};
use loga::ResultContext;
use serde_json::Number;
use shared::model::Node;

pub fn node_to_meta_row(rows: &mut Vec<HashMap<String, DataValue>>, n: &Node) -> Result<(), loga::Error> {
    let Node:: Value(serde_json::Value::String(v)) = n else {
        return Ok(());
    };
    let mut out = HashMap::new();
    out.insert("node".to_string(), node_to_row(n)?);
    out.insert("mimetype".to_string(), DataValue::Str("text/plain".into()));
    out.insert("text".to_string(), DataValue::Str(v.into()));
    rows.push(out);
    return Ok(());
}

pub fn node_to_row(n: &Node) -> Result<DataValue, loga::Error> {
    return Ok(match n {
        Node::Id(id) => DataValue::List(vec![DataValue::Str("id".into()), DataValue::Str(id.into())]),
        Node::File(hash) => DataValue::List(
            vec![DataValue::Str("file".into()), DataValue::Str(hash.to_string().into())],
        ),
        Node::Value(v) => DataValue::List(vec![DataValue::Str("value".into()), match v {
            serde_json::Value::Null => return Err(loga::err("Got null value; value nodes must be non-null")),
            serde_json::Value::Bool(v) => DataValue::Bool(*v),
            serde_json::Value::Number(v) => DataValue::Num(if v.is_f64() {
                Num::Float(v.as_f64().context("Json float out of range")?)
            } else {
                Num::Int(v.as_i64().context("Json float out of range")?)
            }),
            serde_json::Value::String(v) => DataValue::Str(v.into()),
            serde_json::Value::Array(_) => return Err(loga::err("Got array value; value nodes must be primitive")),
            serde_json::Value::Object(_) => return Err(loga::err("Got obj value; value nodes must be primitive")),
        }]),
    });
}

pub fn json_to_cozo(d: serde_json::Value) -> Result<DataValue, loga::Error> {
    match d {
        serde_json::Value::Null => return Ok(DataValue::Null),
        serde_json::Value::Bool(v) => return Ok(DataValue::Bool(v)),
        serde_json::Value::Number(v) => return Ok(DataValue::Num(if v.is_f64() {
            Num::Float(v.as_f64().context("Json float out of range")?)
        } else {
            Num::Int(v.as_i64().context("Json float out of range")?)
        })),
        serde_json::Value::String(v) => return Ok(DataValue::Str(v.into())),
        serde_json::Value::Array(v) => {
            let mut out = vec![];
            for v in v {
                out.push(json_to_cozo(v)?);
            }
            return Ok(DataValue::List(out));
        },
        serde_json::Value::Object(_) => return Err(loga::err("Objects aren't valid parameters")),
    }
}

pub fn cozo_to_json(d: DataValue) -> Result<serde_json::Value, loga::Error> {
    return Ok(match d {
        DataValue::Null => serde_json::Value::Null,
        DataValue::Bool(v) => serde_json::Value::Bool(v),
        DataValue::Num(v) => match v {
            Num::Int(v) => serde_json::Value::Number(Number::from(v)),
            Num::Float(v) => serde_json::Value::Number(Number::from_f64(v).unwrap()),
        },
        DataValue::Str(v) => serde_json::Value::String(v.to_string()),
        DataValue::List(v) => {
            let mut out = vec![];
            for v in v {
                out.push(cozo_to_json(v)?);
            }
            serde_json::Value::Array(out)
        },
        DataValue::Json(v) => v.0,
        DataValue::Validity(v) => {
            let mut o = serde_json::Map::new();
            o.insert("is_assert".to_string(), serde_json::Value::Bool(v.is_assert.0));
            o.insert("timestamp".to_string(), serde_json::Value::Number(Number::from(v.timestamp.0.0)));
            serde_json::Value::Object(o)
        },
        DataValue::Bot => panic!(),
        DataValue::Bytes(v) => serde_json::Value::String(hex::encode(&v)),
        DataValue::Uuid(v) => serde_json::Value::String(v.0.to_string()),
        DataValue::Regex(_) => panic!(),
        DataValue::Set(_) => panic!(),
        DataValue::Vec(v) => {
            let mut out = vec![];
            match v {
                cozo::Vector::F32(v) => {
                    for x in v {
                        out.push(
                            serde_json::Value::Number(
                                Number::from_f64(
                                    x as f64,
                                ).context("Received non-finite number which isn't supported in json")?,
                            ),
                        );
                    }
                },
                cozo::Vector::F64(v) => {
                    for x in v {
                        out.push(
                            serde_json::Value::Number(
                                Number::from_f64(
                                    x,
                                ).context("Received non-finite number which isn't supported in json")?,
                            ),
                        );
                    }
                },
            }
            serde_json::Value::Array(out)
        },
    });
}
