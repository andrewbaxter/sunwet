use std::{
    collections::{
        BTreeMap,
        HashMap,
    },
    sync::Arc,
};
use cozo::{
    DataValue,
    NamedRows,
};
use http::Response;
use http_body_util::combinators::BoxBody;
use hyper::body::Bytes;
use native::cap_fn;
use shared::model::{
    View,
    ViewEnsure,
};
use tokio::task::spawn_blocking;
use super::{
    httpresp::{
        response_200,
        response_200_json,
    },
    state::State,
};

pub async fn handle_view_ensure(
    state: Arc<State>,
    args: ViewEnsure,
) -> Result<Response<BoxBody<Bytes, std::io::Error>>, loga::Error> {
    let mut params = BTreeMap::new();
    params.insert("view".to_string(), NamedRows {
        headers: vec!["id".to_string(), "def".to_string()],
        rows: vec![
            vec![
                DataValue::Str(args.id.as_str().into()),
                DataValue::Str(serde_json::to_string(&args.def).unwrap().as_str().into())
            ]
        ],
        next: None,
    });
    spawn_blocking(cap_fn!(()(state) {
        state.db.import_relations(params)
    })).await?.await.map_err(|e| loga::err(e.to_string()).context("Error running query"))?;
    return Ok(response_200());
}

pub async fn handle_view_delete(
    state: Arc<State>,
    id: String,
) -> Result<Response<BoxBody<Bytes, std::io::Error>>, loga::Error> {
    spawn_blocking(cap_fn!(()(state) {
        state.db.run_script_read_only(&"{?[id] <- [[$id]] :rm view {id}}", {
            let mut m = BTreeMap::new();
            m.insert("id".to_string(), DataValue::Str(id.as_str().into()));
            m
        })
    })).await?.await.map_err(|e| loga::err(e.to_string()))?;
    return Ok(response_200());
}

pub async fn handle_view_list(state: Arc<State>) -> Result<Response<BoxBody<Bytes, std::io::Error>>, loga::Error> {
    let res = spawn_blocking(cap_fn!(()(state) {
        state.db.run_script_read_only(&"{?[id, def] := *view{id: id, def: def}}", BTreeMap::new())
    })).await?.await.map_err(|e| loga::err(e.to_string()))?;
    let mut out = HashMap::new();
    for row in res.rows {
        out.insert(
            row.get(0).unwrap().get_str().unwrap().to_string(),
            serde_json::from_str::<View>(row.get(1).unwrap().get_str().unwrap()).unwrap(),
        );
    }
    return Ok(response_200_json(out));
}
