use good_ormning::good_module;

good_module!(pub db);
pub mod access;
pub mod dbutil;
pub mod dbwrite;
pub mod defaultviews;
pub mod filesutil;
pub mod fsutil;
pub mod migrate;
pub mod query;
pub mod query_test;
pub mod state;
pub mod subsystems;

use {
    crate::{
        cap_fn,
        interface::{
            config::{
                Config,
                MaybeFdap,
            },
            triple::{
                DbFileHash,
                DbNode,
            },
        },
        server::{
            access::{
                can_access_file,
                AccessRes,
                AccessSourceId,
                DbAccessSourceId,
                Identity,
            },
            filesutil::{
                file_path,
                hash_file_sha256,
            },
            state::{
                BackgroundJob,
                BgCheckResult,
                IamGrants,
            },
        },
    },
    aargvark::{
        traits_impls::AargvarkJson,
        Aargvark,
    },
    access::{
        check_is_admin,
        identify_requester,
    },
    chrono::Utc,
    dbutil::tx,
    deadpool_sqlite::{
        Hook,
        HookError,
    },
    flowcontrol::{
        shed,
        ta_return,
    },
    fsutil::create_dirs,
    good_ormning::runtime::GoodError,
    http::{
        status,
        HeaderMap,
        HeaderName,
        HeaderValue,
        Uri,
    },
    http_body_util::{
        combinators::BoxBody,
        BodyExt,
    },
    htwrap::htserve::{
        responses::{
            response_401,
            response_403,
            response_404,
        },
        viserr::{
            ResultVisErr,
            VisErr,
        },
    },
    hyper::{
        body::{
            Bytes,
            Incoming,
        },
        server::conn::http1,
        service::service_fn,
        Method,
        Request,
        Response,
    },
    hyper_util::rt::TokioIo,
    loga::{
        ea,
        DebugDisplay,
        ErrContext,
        Log,
        ResultContext,
    },
    moka::future::Cache,
    shared::interface::{
        config::{
            form::FormId,
            view::ViewId,
        },
        query::Query,
        triple::{
            FileHash,
            Node,
        },
        wire::{
            AutocompleteField,
            C2SReq,
            NodeMeta,
            Pagination,
            ReqCommit,
            ReqHistoryFilterPredicate,
            RespCheck,
            RespHistory,
            RespHistoryEvent,
            RespQuery,
            RespQueryRows,
            RespWhoAmI,
            TreeNode,
            Triple,
        },
    },
    state::{
        build_global_config,
        get_global_config,
        get_iam_grants,
        FdapGlobalState,
        FdapState,
        FdapUsersState,
        GlobalState,
        LocalUsersState,
        State,
        UsersState,
    },
    std::{
        collections::{
            HashMap,
            HashSet,
        },
        hash::{
            DefaultHasher,
            Hash,
            Hasher,
        },
        os::unix::ffi::OsStrExt,
        str::FromStr,
        sync::{
            Arc,
            Mutex,
        },
        time::Duration,
    },
    subsystems::{
        background::start_background_job,
        files::{
            handle_commit,
            handle_file_get,
            handle_file_head,
            handle_file_post,
            handle_finish_upload,
            handle_form_commit,
        },
        link::{
            handle_link_ws,
            handle_ws_link,
            handle_ws_main,
        },
        menu::handle_get_filtered_client_config,
        oidc,
        static_,
    },
    tokio::{
        net::TcpListener,
        select,
        spawn,
        sync::{
            mpsc,
            oneshot,
        },
    },
    tokio_stream::wrappers::TcpListenerStream,
};

fn gather_record_files(files: &mut Vec<FileHash>, r: &TreeNode) {
    match r {
        TreeNode::Scalar(s) => {
            if let Node::File(s) = s {
                files.push(s.clone());
            }
        },
        TreeNode::Array(a) => {
            for v in a {
                gather_record_files(files, v);
            }
        },
        TreeNode::Record(r) => {
            for v in r.values() {
                gather_record_files(files, v);
            }
        },
    }
}

/// Build an FTS5 query string from user input for autocomplete.
/// Splits into terms, quotes each, adds prefix `*` to each term for partial matching.
/// Returns `raw:` prefixed string for the Search AST node.
fn build_autocomplete_fts_query(text: &str) -> String {
    let terms: Vec<&str> = text.split_whitespace().collect();
    if terms.is_empty() {
        return "raw:\"\"*".to_string();
    }
    let fts_terms: Vec<String> =
        terms
            .iter()
            .map(|t| format!("\"{}\"*", t.replace("\"", "\"\"")))
            .collect();
    format!("raw:{}", fts_terms.join(" AND "))
}

async fn autocomplete_values_via_query(
    state: &Arc<State>,
    search_text: &str,
    predicate_context: Option<(&str, shared::interface::query::MoveDirection)>,
) -> Result<Vec<String>, VisErr<loga::Error>> {
    use shared::interface::query::*;
    let fts_query = build_autocomplete_fts_query(search_text);
    let mut steps = vec![];
    if let Some((pred, dir)) = predicate_context {
        steps.push(Step {
            specific: StepSpecific::Move(StepMove {
                dir,
                predicate: StrValue::Literal(pred.to_string()),
                filter: None,
            }),
            sort: None,
            first: false,
        });
    }
    let query = Query {
        chain_head: ChainHead {
            root: Some(ChainRoot::Search(StrValue::Literal(fts_query))),
            steps,
        },
        suffix: None,
    };
    let results =
        query::execute_query(
            &state.db,
            query,
            HashMap::new(),
            Some(Pagination { count: 20, seed: None, key: None }),
        ).await?;
    let mut out = vec![];
    match results {
        query::QueryResults::Scalar(nodes) => {
            for node in nodes {
                if let Node::Value(serde_json::Value::String(s)) = node {
                    out.push(s);
                }
            }
        },
        _ => { },
    }
    Ok(out)
}

async fn handle_query_req(
    state: Arc<State>,
    query: Query,
    parameters: HashMap<String, Node>,
    pagination: Option<Pagination>,
    view_access: Option<(ViewId, u64)>,
) -> Result<RespQuery, VisErr<loga::Error>> {
    let expect_count = pagination.as_ref().map(|x| x.count);
    let results = query::execute_query(&state.db, query, parameters, pagination).await?;
    let page_end = expect_count.and_then(|x| match &results {
        query::QueryResults::Scalar(rows) => {
            if rows.len() < x {
                return None;
            } else {
                return rows.last().cloned();
            }
        },
        query::QueryResults::Record(rows) => {
            if rows.len() < x {
                return None;
            } else {
                return rows.last().map(|x| x.head_data.clone());
            }
        },
    });
    let mut files = vec![];
    let out_rows;
    match results {
        query::QueryResults::Scalar(rows) => {
            for scalar in &rows {
                if let Node::File(s) = scalar {
                    files.push(s.clone());
                }
            }
            out_rows = RespQueryRows::Scalar(rows);
        },
        query::QueryResults::Record(rows) => {
            let mut out_rows1 = vec![];
            for record in rows {
                for v in record.tail_data.values() {
                    gather_record_files(&mut files, v);
                }
                out_rows1.push(record.tail_data);
            }
            out_rows = RespQueryRows::Record(out_rows1);
        },
    }
    let meta = tx(&state.db, {
        move |db| -> Result<_, loga::Error> {
            if let Some((view_id, view_hash)) = view_access {
                let access_source_id = DbAccessSourceId(AccessSourceId::ViewId(view_id.clone()));
                let view_hash_i64 = view_hash as i64;
                dbutil::file_access_gc(db, &access_source_id, &view_hash_i64).context("Error clearing file access")?;
                for file in &files {
                    let filehash = DbFileHash(file.clone());
                    dbutil::file_access_insert(
                        db,
                        &filehash,
                        &access_source_id,
                        &view_hash_i64,
                    ).context("Error inserting file access")?;
                }
            }
            let mut meta = HashMap::new();
            for file in files {
                let node = DbNode(Node::File(file.clone()));
                if let Some(mimetype) = dbutil::meta_get_mimetype(db, &node)?.flatten() {
                    meta.insert(Node::File(file), NodeMeta { mime: Some(mimetype) });
                }
            }
            return Ok(meta);
        }
    }).await.err_internal()?;
    return Ok(RespQuery {
        rows: out_rows,
        meta: meta.into_iter().collect(),
        next_page_key: page_end,
    });
}

async fn handle_req(state: Arc<State>, mut req: Request<Incoming>) -> Response<BoxBody<Bytes, std::io::Error>> {
    let url = req.uri().clone();
    match {
        let state = state.clone();
        async move {
            if (|| false)() {
                return Err(loga::err("")).err_internal() as Result<_, VisErr<loga::Error>>;
            }
            if hyper_tungstenite::is_upgrade_request(&req) {
                // Websocket req
                let upgrade = hyper_tungstenite::upgrade(&mut req, None);
                let (head, _) = req.into_parts();
                let mut path_iter = head.uri.path().trim_matches('/').split('/');
                let link_type = path_iter.next().unwrap();
                let link_session = path_iter.next().unwrap();
                match link_type {
                    "link" => {
                        return Ok(
                            handle_link_ws(state, link_session.to_string(), head, upgrade, handle_ws_link).await,
                        );
                    },
                    "main" => {
                        return Ok(
                            handle_link_ws(state, link_session.to_string(), head, upgrade, handle_ws_main).await,
                        );
                    },
                    _ => {
                        state
                            .log
                            .log_with(loga::DEBUG, "Websocket connection on unknown path", ea!(path = link_type));
                        return Ok(response_404());
                    },
                }
            } else {
                // Normal HTTP req
                let (head, body) = req.into_parts();
                let mut path_iter = head.uri.path().trim_matches('/').split('/');
                let path_first = path_iter.next().unwrap();
                match path_first {
                    "oidc" => {
                        if let Some(oidc_state) = state.oidc_state.as_ref() {
                            return Ok(oidc::handle_oidc(oidc_state, head).await?);
                        } else {
                            return Ok(response_404());
                        }
                    },
                    "logout" => {
                        if let Some(oidc_state) = state.oidc_state.as_ref() {
                            return Ok(oidc::handle_logout(oidc_state, &state.log, head).await?);
                        } else {
                            return Ok(response_404());
                        }
                    },
                    "api" => {
                        let identity = identify_requester(&state, &head.headers).await?;
                        let req =
                            serde_json::from_slice::<C2SReq>(
                                &body
                                    .collect()
                                    .await
                                    .context("Error reading request bytes")
                                    .err_external()?
                                    .to_bytes(),
                            )
                                .context("Failed to parse json request body")
                                .err_external()?;

                        pub mod resp {
                            use {
                                http::Response,
                                http_body_util::combinators::BoxBody,
                                htwrap::htserve::responses::response_200_json,
                                hyper::body::Bytes,
                                serde::Serialize,
                                shared::interface::{
                                    wire::C2SReqTrait,
                                },
                            };

                            // Private constructor
                            pub struct RespToken(());

                            pub type Resp = Response<BoxBody<Bytes, std::io::Error>>;

                            pub trait ReqResp: C2SReqTrait {
                                fn respond(&self) -> fn(Self::Resp) -> (RespToken, Resp) {
                                    fn acceptor<T: Serialize>(r: T) -> (RespToken, Resp) {
                                        return (RespToken(()), response_200_json(r));
                                    }

                                    return acceptor::<Self::Resp>;
                                }
                            }

                            impl ReqResp for shared::interface::wire::ReqCommit { }

                            impl ReqResp for shared::interface::wire::ReqGetClientConfig { }

                            impl ReqResp for shared::interface::wire::ReqQuery { }

                            impl ReqResp for shared::interface::wire::ReqViewQuery { }

                            impl ReqResp for shared::interface::wire::ReqGetNodeMeta { }

                            impl ReqResp for shared::interface::wire::ReqHistory { }

                            impl ReqResp for shared::interface::wire::ReqGetTriplesAround { }

                            impl ReqResp for shared::interface::wire::ReqUploadFinish { }

                            impl ReqResp for shared::interface::wire::ReqWhoAmI { }

                            impl ReqResp for shared::interface::wire::ReqCheckStart { }

                            impl ReqResp for shared::interface::wire::ReqCheckGet { }

                            impl ReqResp for shared::interface::wire::ReqAutocompleteFree { }

                            impl ReqResp for shared::interface::wire::ReqAutocompleteFormField { }

                            impl ReqResp for shared::interface::wire::ReqAutocompleteViewParam { }
                        }

                        use resp::ReqResp;

                        let resp: (resp::RespToken, resp::Resp);
                        match req {
                            C2SReq::Commit(req) => {
                                let responder = req.respond();
                                match req {
                                    ReqCommit::Free(req) => {
                                        // Check access
                                        match check_is_admin(&state, &identity, "Commit").await.err_internal()? {
                                            AccessRes::Yes => { },
                                            AccessRes::NoAccess => {
                                                return Ok(response_403());
                                            },
                                            AccessRes::NoIdent => {
                                                return Ok(response_401());
                                            },
                                        }
                                        resp = responder(handle_commit(state, req).await.err_internal()?);
                                    },
                                    ReqCommit::Form(req) => {
                                        {
                                            // Check access
                                            let grants = get_iam_grants(&state, &identity).await.err_internal()?;
                                            let res = shed!{
                                                'ok _;
                                                match &grants {
                                                    IamGrants::Admin => {
                                                        break 'ok AccessRes::Yes;
                                                    },
                                                    IamGrants::Limited(grants) => {
                                                        if grants.forms.contains(&req.form_id) {
                                                            break 'ok AccessRes::Yes;
                                                        }
                                                    },
                                                }
                                                if matches!(identity, Identity::Public) {
                                                    break 'ok AccessRes::NoIdent;
                                                }
                                                else {
                                                    break 'ok AccessRes::NoAccess;
                                                    return Ok(response_403());
                                                }
                                            };
                                            state
                                                .log
                                                .log_with(
                                                    loga::DEBUG,
                                                    "Form commit access result",
                                                    ea!(
                                                        identity = identity.dbg_str(),
                                                        grants = grants.dbg_str(),
                                                        form_id = req.form_id,
                                                        result = res.dbg_str()
                                                    ),
                                                );
                                            match res {
                                                AccessRes::Yes => { },
                                                AccessRes::NoIdent => {
                                                    return Ok(response_401());
                                                },
                                                AccessRes::NoAccess => {
                                                    return Ok(response_403());
                                                },
                                            }
                                        }
                                        resp = responder(handle_form_commit(state, req).await?);
                                    },
                                }
                            },
                            C2SReq::UploadFinish(req) => {
                                let responder = req.respond();
                                match can_access_file(&state, &identity, &req.0).await.err_internal()? {
                                    AccessRes::Yes => (),
                                    AccessRes::NoIdent => return Ok(response_401()),
                                    AccessRes::NoAccess => return Ok(response_403()),
                                }
                                let Some(res) = handle_finish_upload(state, req.0).await.err_internal()? else {
                                    return Ok(response_401());
                                };
                                resp = responder(res);
                            },
                            C2SReq::Query(req) => {
                                match check_is_admin(&state, &identity, "Query").await.err_internal()? {
                                    AccessRes::Yes => { },
                                    AccessRes::NoAccess => {
                                        return Ok(response_403());
                                    },
                                    AccessRes::NoIdent => {
                                        return Ok(response_401());
                                    },
                                }
                                let responder = req.respond();
                                resp =
                                    responder(
                                        handle_query_req(
                                            state,
                                            req.query,
                                            req.parameters,
                                            req.pagination,
                                            None,
                                        ).await?,
                                    );
                            },
                            C2SReq::ViewQuery(req) => {
                                let responder = req.respond();
                                let global_config = get_global_config(&state).await.err_internal()?;
                                let Some(view) = global_config.views.get(&req.view_id) else {
                                    return Err(
                                        loga::err_with("No known view menu_item with id", ea!(view = req.view_id)),
                                    ).err_external();
                                };
                                {
                                    // Access check
                                    let grants = get_iam_grants(&state, &identity).await.err_internal()?;
                                    let res = shed!{
                                        'ok _;
                                        match &grants {
                                            IamGrants::Admin => {
                                                break 'ok AccessRes::Yes;
                                            },
                                            IamGrants::Limited(grants) => {
                                                if grants.views.contains(&req.view_id) {
                                                    break 'ok AccessRes::Yes;
                                                }
                                            },
                                        }
                                        if matches!(identity, Identity::Public) {
                                            break 'ok AccessRes::NoIdent;
                                        }
                                        else {
                                            break 'ok AccessRes::NoAccess;
                                            return Ok(response_403());
                                        }
                                    };
                                    state
                                        .log
                                        .log_with(
                                            loga::DEBUG,
                                            "View query access result",
                                            ea!(
                                                identity = identity.dbg_str(),
                                                grants = grants.dbg_str(),
                                                view_id = req.view_id,
                                                result = res.dbg_str()
                                            ),
                                        );
                                    match res {
                                        AccessRes::Yes => { },
                                        AccessRes::NoIdent => {
                                            return Ok(response_401());
                                        },
                                        AccessRes::NoAccess => {
                                            return Ok(response_403());
                                        },
                                    }
                                }
                                let Some(query) = view.item.queries.get(&req.query) else {
                                    return Err(
                                        loga::err_with(
                                            "No known query with id in view",
                                            ea!(view = req.view_id, query = req.query),
                                        ),
                                    ).err_external();
                                };
                                let mut view_hash = DefaultHasher::new();
                                view.item.hash(&mut view_hash);
                                let view_hash = view_hash.finish();
                                resp =
                                    responder(
                                        handle_query_req(
                                            state,
                                            query.clone(),
                                            req.parameters,
                                            req.pagination,
                                            Some((req.view_id.clone(), view_hash)),
                                        ).await?,
                                    );
                            },
                            C2SReq::GetNodeMeta(req) => {
                                let responder = req.respond();
                                let meta = tx(&state.db, move |db| -> Result<_, loga::Error> {
                                    let node = DbNode(req.node);
                                    return Ok(dbutil::meta_get_mimetype(db, &node)?.flatten());
                                }).await.err_internal()?;
                                resp = responder(match meta {
                                    Some(mimetype) => Some(NodeMeta { mime: Some(mimetype) }),
                                    None => None,
                                });
                            },
                            C2SReq::History(req) => {
                                let responder = req.respond();
                                {
                                    // Check access
                                    match check_is_admin(&state, &identity, "History").await.err_internal()? {
                                        AccessRes::Yes => { },
                                        AccessRes::NoAccess => {
                                            return Ok(response_403());
                                        },
                                        AccessRes::NoIdent => {
                                            return Ok(response_401());
                                        },
                                    }
                                }
                                let (events, commit_descriptions): (Vec<_>, _) =
                                    tx(&state.db, move |db| -> Result<_, loga::Error> {
                                        let events: Vec<dbutil::HistoryRow>;
                                        if let Some(f) = req.filter {
                                            if let Some(p) = f.predicate {
                                                match p {
                                                    ReqHistoryFilterPredicate::Incoming(p) => {
                                                        events = if let Some(after) = req.page_key {
                                                            dbutil::hist_list_by_predicate_object_after(
                                                                db,
                                                                after.0,
                                                                &DbNode(after.1.subject),
                                                                &after.1.predicate,
                                                                &DbNode(after.1.object),
                                                                &p,
                                                                &DbNode(f.node.clone()),
                                                            )?
                                                        } else {
                                                            dbutil::hist_list_by_predicate_object(
                                                                db,
                                                                &p,
                                                                &DbNode(f.node.clone()),
                                                            )?
                                                        };
                                                    },
                                                    ReqHistoryFilterPredicate::Outgoing(p) => {
                                                        events = if let Some(after) = req.page_key {
                                                            dbutil::hist_list_by_subject_predicate_after(
                                                                db,
                                                                after.0,
                                                                &DbNode(after.1.subject),
                                                                &after.1.predicate,
                                                                &DbNode(after.1.object),
                                                                &DbNode(f.node.clone()),
                                                                &p,
                                                            )?
                                                        } else {
                                                            dbutil::hist_list_by_subject_predicate(
                                                                db,
                                                                &DbNode(f.node.clone()),
                                                                &p,
                                                            )?
                                                        };
                                                    },
                                                }
                                            } else {
                                                events = if let Some(after) = req.page_key {
                                                    dbutil::hist_list_by_node_after(
                                                        db,
                                                        after.0,
                                                        &DbNode(after.1.subject),
                                                        &after.1.predicate,
                                                        &DbNode(after.1.object),
                                                        &DbNode(f.node.clone()),
                                                    )?
                                                } else {
                                                    dbutil::hist_list_by_node(db, &DbNode(f.node.clone()))?
                                                };
                                            }
                                        } else {
                                            events = if let Some(after) = req.page_key {
                                                dbutil::hist_list_all_after(
                                                    db,
                                                    after.0,
                                                    &DbNode(after.1.subject),
                                                    &after.1.predicate,
                                                    &DbNode(after.1.object),
                                                )?
                                            } else {
                                                dbutil::hist_list_all(db)?
                                            };
                                        }
                                        let mut commit_descriptions = HashMap::new();
                                        for ev in &events {
                                            match commit_descriptions.entry(ev.commit_) {
                                                std::collections::hash_map::Entry::Occupied(_) => (),
                                                std::collections::hash_map::Entry::Vacant(entry) => {
                                                    let commit_id = ev.commit_;
                                                    entry.insert(
                                                        dbutil::commit_get_description(
                                                            db,
                                                            &commit_id,
                                                        )?.ok_or_else(
                                                            || GoodError(
                                                                format!(
                                                                    "Triple references nonexistent commit [{}]",
                                                                    ev.commit_.to_rfc3339()
                                                                ),
                                                            ),
                                                        )?,
                                                    );
                                                },
                                            }
                                        }
                                        return Ok((events, commit_descriptions));
                                    }).await.err_internal()?;
                                resp = responder(RespHistory {
                                    events: events.into_iter().map(|x| RespHistoryEvent {
                                        delete: !x.exists,
                                        commit: x.commit_,
                                        triple: Triple {
                                            subject: x.subject.0,
                                            predicate: x.predicate,
                                            object: x.object.0,
                                        },
                                    }).collect(),
                                    commit_descriptions: commit_descriptions,
                                });
                            },
                            C2SReq::GetTriplesAround(req) => {
                                {
                                    // Check access
                                    match check_is_admin(&state, &identity, "Get triples around").await.err_internal()? {
                                        AccessRes::Yes => { },
                                        AccessRes::NoAccess => {
                                            return Ok(response_403());
                                        },
                                        AccessRes::NoIdent => {
                                            return Ok(response_401());
                                        },
                                    }
                                }
                                let responder = req.respond();
                                let triples = tx(&state.db, {
                                    let nodes = req.nodes.clone();
                                    move |db| -> Result<_, loga::Error> {
                                        let nodes = nodes.into_iter().map(DbNode).collect::<Vec<_>>();
                                        return Ok(dbutil::snapshot_triples_around(db, nodes.iter().collect())?);
                                    }
                                }).await.err_internal()?;
                                resp = responder(triples.into_iter().map(|t| Triple {
                                    subject: t.subject.0,
                                    predicate: t.predicate,
                                    object: t.object.0,
                                }).collect());
                            },
                            C2SReq::GetClientConfig(req) => {
                                resp =
                                    req.respond()(
                                        handle_get_filtered_client_config(state, &identity).await.err_internal()?,
                                    );
                            },
                            C2SReq::WhoAmI(req) => {
                                resp = req.respond()(match identity {
                                    access::Identity::Token(_) => RespWhoAmI::Token,
                                    access::Identity::User(ident) => RespWhoAmI::User(ident.0),
                                    access::Identity::Link(_) => RespWhoAmI::Public,
                                    access::Identity::Public => RespWhoAmI::Public,
                                });
                            },
                            C2SReq::CheckStart(req) => shed!{
                                'done _;
                                {
                                    // Check access
                                    match check_is_admin(&state, &identity, "Start check").await.err_internal()? {
                                        AccessRes::Yes => { },
                                        AccessRes::NoAccess => {
                                            return Ok(response_403());
                                        },
                                        AccessRes::NoIdent => {
                                            return Ok(response_401());
                                        },
                                    }
                                }
                                let mut bg = state.bg_check.lock().unwrap();
                                match &*bg {
                                    Some(bg) => match bg {
                                        BgCheckResult::Fut(x) => {
                                            if !x.is_terminated() && !req.restart {
                                                resp = req.respond()(());
                                                break 'done;
                                            }
                                        },
                                        BgCheckResult::Value(_) => { },
                                    },
                                    None => (),
                                }
                                let (mut res_tx, res_rx) = oneshot::channel();
                                *bg = Some(BgCheckResult::Fut(res_rx));
                                drop(bg);
                                spawn(async move {
                                    let work = async {
                                        ta_return!(RespCheck, loga::Error);
                                        let started = Utc::now();
                                        let mut seen = HashSet::new();
                                        let mut node_issues = HashMap::new();
                                        for triple_end in ["subject", "object"] {
                                            let mut pivot: Option<DbNode> = None;
                                            loop {
                                                let batch = tx(&state.db, {
                                                    let pivot = pivot.clone();
                                                    let triple_end = triple_end.to_string();
                                                    move |db| -> Result<Vec<DbNode>, loga::Error> {
                                                        dbutil::snapshot_file_nodes(db, &triple_end, pivot.as_ref())
                                                    }
                                                }).await?;
                                                let Some(pivot1) = batch.last().cloned() else {
                                                    break;
                                                };
                                                pivot = Some(pivot1);
                                                for node in batch {
                                                    let Node::File(hash) = &node.0 else {
                                                        unreachable!();
                                                    };
                                                    if !seen.insert(hash.clone()) {
                                                        continue;
                                                    }
                                                    match async {
                                                        ta_return!((), loga::Error);
                                                        let path = file_path(&state, &hash)?;
                                                        if !path.exists() {
                                                            return Err(loga::err("File doesn't exist for file node"));
                                                        }
                                                        let real_hash = hash_file_sha256(&state.log, &path).await?;
                                                        if real_hash != *hash {
                                                            return Err(
                                                                loga::err_with(
                                                                    "Disk hash doesn't match expected (node) hash",
                                                                    ea!(disk = real_hash, node = hash),
                                                                ),
                                                            );
                                                        }
                                                        return Ok(());
                                                    }.await {
                                                        Ok(_) => { },
                                                        Err(e) => {
                                                            node_issues.insert(
                                                                DbNode(Node::File(hash.clone())),
                                                                e.to_string()
                                                            );
                                                        },
                                                    }
                                                }
                                            }
                                        }
                                        return Ok(RespCheck {
                                            started: started,
                                            completed: Utc::now(),
                                            files_count: seen.len(),
                                            nodes_issues: node_issues
                                                .into_iter()
                                                .map(|(k, v)| (serde_json::to_string(&k.0).unwrap(), v))
                                                .collect(),
                                        });
                                    };
                                    select!{
                                        work = work => {
                                            _ = res_tx.send(work);
                                        },
                                        _ = res_tx.closed() => {
                                        }
                                    }
                                });
                                resp = req.respond()(());
                            },
                            C2SReq::AutocompleteFree(req) => {
                                match check_is_admin(&state, &identity, "Autocomplete").await.err_internal()? {
                                    AccessRes::Yes => { },
                                    AccessRes::NoAccess => {
                                        return Ok(response_403());
                                    },
                                    AccessRes::NoIdent => {
                                        return Ok(response_401());
                                    },
                                }
                                let responder = req.respond();
                                let results = match req.field {
                                    AutocompleteField::Predicate => {
                                        tx(&state.db, {
                                            move |db| -> Result<_, loga::Error> {
                                                dbutil::autocomplete_predicates(db, &req.prefix, &req.suffix)
                                            }
                                        }).await.err_internal()?
                                    },
                                    AutocompleteField::Value => {
                                        autocomplete_values_via_query(&state, &req.prefix, None).await?
                                    },
                                };
                                resp = responder(results);
                            },
                            C2SReq::AutocompleteFormField(req) => {
                                let global_config = get_global_config(&state).await.err_internal()?;
                                let Some(form) = global_config.forms.get(&req.form_id) else {
                                    return Err(
                                        loga::err_with(
                                            "No known form with id",
                                            ea!(form = req.form_id),
                                        ),
                                    ).err_external();
                                };
                                // Check access
                                {
                                    let grants = get_iam_grants(&state, &identity).await.err_internal()?;
                                    let res = shed!{
                                        'ok _;
                                        match &grants {
                                            IamGrants::Admin => {
                                                break 'ok AccessRes::Yes;
                                            },
                                            IamGrants::Limited(grants) => {
                                                if grants.forms.contains(&req.form_id) {
                                                    break 'ok AccessRes::Yes;
                                                }
                                            },
                                        }
                                        if matches!(identity, Identity::Public) {
                                            break 'ok AccessRes::NoIdent;
                                        } else {
                                            break 'ok AccessRes::NoAccess;
                                        }
                                    };
                                    match res {
                                        AccessRes::Yes => { },
                                        AccessRes::NoIdent => {
                                            return Ok(response_401());
                                        },
                                        AccessRes::NoAccess => {
                                            return Ok(response_403());
                                        },
                                    }
                                }
                                let responder = req.respond();
                                // Analyze form outputs to find predicate context
                                use shared::interface::config::form::{
                                    InputOrInline,
                                    InputOrInlineText,
                                };
                                use shared::interface::query::MoveDirection;
                                let mut predicate_context: Option<(String, MoveDirection)> = None;
                                let mut is_predicate = false;
                                for output in &form.item.outputs {
                                    // Check if field is used as predicate
                                    if let InputOrInlineText::Input(id) = &output.predicate {
                                        if *id == req.field_id {
                                            is_predicate = true;
                                            break;
                                        }
                                    }
                                    // Check if field is used as subject
                                    if matches!(&output.subject, InputOrInline::Input(id) if *id == req.field_id) {
                                        if let InputOrInlineText::Inline(pred) = &output.predicate {
                                            // Subject follows predicate forward to reach object
                                            predicate_context = Some((pred.clone(), MoveDirection::Forward));
                                        }
                                        break;
                                    }
                                    // Check if field is used as object
                                    if matches!(&output.object, InputOrInline::Input(id) if *id == req.field_id) {
                                        if let InputOrInlineText::Inline(pred) = &output.predicate {
                                            // Object follows predicate backward to reach subject
                                            predicate_context = Some((pred.clone(), MoveDirection::Backward));
                                        }
                                        break;
                                    }
                                }
                                let results = if is_predicate {
                                    tx(&state.db, {
                                        move |db| -> Result<_, loga::Error> {
                                            dbutil::autocomplete_predicates(db, &req.prefix, &req.suffix)
                                        }
                                    }).await.err_internal()?
                                } else {
                                    autocomplete_values_via_query(
                                        &state,
                                        &req.prefix,
                                        predicate_context.as_ref().map(|(p, d)| (p.as_str(), *d)),
                                    ).await?
                                };
                                resp = responder(results);
                            },
                            C2SReq::AutocompleteViewParam(req) => {
                                let global_config = get_global_config(&state).await.err_internal()?;
                                let Some(view) = global_config.views.get(&req.view_id) else {
                                    return Err(
                                        loga::err_with(
                                            "No known view with id",
                                            ea!(view = req.view_id),
                                        ),
                                    ).err_external();
                                };
                                // Check access
                                {
                                    let grants = get_iam_grants(&state, &identity).await.err_internal()?;
                                    let res = shed!{
                                        'ok _;
                                        match &grants {
                                            IamGrants::Admin => {
                                                break 'ok AccessRes::Yes;
                                            },
                                            IamGrants::Limited(grants) => {
                                                if grants.views.contains(&req.view_id) {
                                                    break 'ok AccessRes::Yes;
                                                }
                                            },
                                        }
                                        if matches!(identity, Identity::Public) {
                                            break 'ok AccessRes::NoIdent;
                                        } else {
                                            break 'ok AccessRes::NoAccess;
                                        }
                                    };
                                    match res {
                                        AccessRes::Yes => { },
                                        AccessRes::NoIdent => {
                                            return Ok(response_401());
                                        },
                                        AccessRes::NoAccess => {
                                            return Ok(response_403());
                                        },
                                    }
                                }
                                let responder = req.respond();
                                // Analyze view queries to find context for this parameter
                                use shared::interface::query::{
                                    StrValue,
                                    Value,
                                    MoveDirection,
                                    StepSpecific,
                                    FilterExpr,
                                    FilterSuffix,
                                    ChainRoot,
                                };
                                let mut is_predicate = false;
                                let mut predicate_context: Option<(String, MoveDirection)> = None;
                                'outer: for query in view.item.queries.values() {
                                    fn find_param_context(
                                        chain: &shared::interface::query::ChainHead,
                                        param_key: &str,
                                        is_predicate: &mut bool,
                                        predicate_context: &mut Option<(String, MoveDirection)>,
                                    ) -> bool {
                                        if let Some(root) = &chain.root {
                                            match root {
                                                ChainRoot::Value(Value::Parameter(k)) if k == param_key => {
                                                    return true;
                                                },
                                                ChainRoot::Search(StrValue::Parameter(k)) if k == param_key => {
                                                    return true;
                                                },
                                                _ => {},
                                            }
                                        }
                                        for step in &chain.steps {
                                            match &step.specific {
                                                StepSpecific::Move(m) => {
                                                    if let StrValue::Parameter(k) = &m.predicate {
                                                        if k == param_key {
                                                            *is_predicate = true;
                                                            return true;
                                                        }
                                                    }
                                                    if let Some(f) = &m.filter {
                                                        fn check_filter(
                                                            f: &FilterExpr,
                                                            param_key: &str,
                                                            step_pred: &StrValue,
                                                            step_dir: MoveDirection,
                                                            predicate_context: &mut Option<(String, MoveDirection)>,
                                                        ) -> bool {
                                                            match f {
                                                                FilterExpr::Exists(e) => {
                                                                    if let Some(suffix) = &e.suffix {
                                                                        let found = match suffix {
                                                                            FilterSuffix::Simple(s) => {
                                                                                matches!(&s.value, Value::Parameter(k) if k == param_key)
                                                                            },
                                                                            FilterSuffix::Like(s) => {
                                                                                matches!(&s.value, StrValue::Parameter(k) if k == param_key)
                                                                            },
                                                                        };
                                                                        if found {
                                                                            if let StrValue::Literal(pred) = step_pred {
                                                                                *predicate_context = Some((pred.clone(), step_dir));
                                                                            }
                                                                            return true;
                                                                        }
                                                                    }
                                                                    false
                                                                },
                                                                FilterExpr::Junction(j) => {
                                                                    j.subexprs.iter().any(
                                                                        |e| check_filter(e, param_key, step_pred, step_dir, predicate_context),
                                                                    )
                                                                },
                                                            }
                                                        }
                                                        if check_filter(f, param_key, &m.predicate, m.dir, predicate_context) {
                                                            return true;
                                                        }
                                                    }
                                                },
                                                StepSpecific::Recurse(r) => {
                                                    if find_param_context(&r.subchain, param_key, is_predicate, predicate_context) {
                                                        return true;
                                                    }
                                                },
                                                StepSpecific::Junction(j) => {
                                                    for c in &j.subchains {
                                                        if find_param_context(c, param_key, is_predicate, predicate_context) {
                                                            return true;
                                                        }
                                                    }
                                                },
                                            }
                                        }
                                        false
                                    }
                                    if find_param_context(&query.chain_head, &req.param_key, &mut is_predicate, &mut predicate_context) {
                                        break 'outer;
                                    }
                                    if let Some(suffix) = &query.suffix {
                                        for subchain in &suffix.chain_tail.subchains {
                                            if find_param_context(&subchain.head, &req.param_key, &mut is_predicate, &mut predicate_context) {
                                                break 'outer;
                                            }
                                        }
                                    }
                                }
                                let results = if is_predicate {
                                    tx(&state.db, {
                                        move |db| -> Result<_, loga::Error> {
                                            dbutil::autocomplete_predicates(db, &req.prefix, &req.suffix)
                                        }
                                    }).await.err_internal()?
                                } else {
                                    autocomplete_values_via_query(
                                        &state,
                                        &req.prefix,
                                        predicate_context.as_ref().map(|(p, d)| (p.as_str(), *d)),
                                    ).await?
                                };
                                resp = responder(results);
                            },
                            C2SReq::CheckGet(req) => shed!{
                                'done _;
                                {
                                    // Check access
                                    match check_is_admin(&state, &identity, "Start check").await.err_internal()? {
                                        AccessRes::Yes => { },
                                        AccessRes::NoAccess => {
                                            return Ok(response_403());
                                        },
                                        AccessRes::NoIdent => {
                                            return Ok(response_401());
                                        },
                                    }
                                }
                                let mut bg = state.bg_check.lock().unwrap();
                                let res = match bg.take() {
                                    Some(bg1) => match bg1 {
                                        BgCheckResult::Fut(mut f) => {
                                            match f.try_recv() {
                                                Ok(v) => {
                                                    *bg = Some(BgCheckResult::Value(v.clone()));
                                                    v
                                                },
                                                Err(_) => {
                                                    *bg = Some(BgCheckResult::Fut(f));
                                                    resp = req.respond()(None);
                                                    break 'done;
                                                },
                                            }
                                        },
                                        BgCheckResult::Value(v) => {
                                            *bg = Some(BgCheckResult::Value(v.clone()));
                                            v
                                        },
                                    },
                                    None => {
                                        resp = req.respond()(None);
                                        break 'done;
                                    },
                                };
                                resp = req.respond()(Some(res.err_external()?));
                            },
                        }
                        return Ok(resp.1);
                    },
                    "file" => {
                        let identity = identify_requester(&state, &head.headers).await?;
                        let hash_gentype = path_iter.next().context("Missing file hash in path").err_external()?;
                        let (hash, gentype) = hash_gentype.split_once(".").unwrap_or((hash_gentype, ""));
                        let file =
                            FileHash::from_str(hash)
                                .map_err(|e| loga::err(e).context_with("Couldn't parse hash", ea!(hash = hash)))
                                .err_external()?;
                        match can_access_file(&state, &identity, &file).await.err_internal()? {
                            AccessRes::Yes => (),
                            AccessRes::NoIdent => return Ok(response_401()),
                            AccessRes::NoAccess => return Ok(response_403()),
                        }
                        let gentype = gentype.to_string();
                        let subpath =
                            path_iter
                                .map(|x| urlencoding::decode(x).unwrap_or(x.into()))
                                .collect::<Vec<_>>()
                                .join("/");
                        match head.method {
                            Method::HEAD => {
                                // Inaccurate for non-file derivations, but HEAD is mostly intended for maybe
                                // media range request
                                return handle_file_head(state, file).await;
                            },
                            Method::GET => {
                                return handle_file_get(state, head, file, gentype, subpath).await;
                            },
                            Method::POST => {
                                return handle_file_post(state, head, file, body).await.err_internal();
                            },
                            _ => return Ok(response_404()),
                        }
                    },
                    _ => {
                        return static_::handle_static(
                            state,
                            &head.headers,
                            head.uri.path().trim_matches('/'),
                        ).await;
                    },
                }
            }
        }
    }.await {
        Ok(r) => {
            return r;
        },
        Err(e) => {
            match e {
                VisErr::External(e) => {
                    return Response::builder()
                        .status(status::StatusCode::BAD_REQUEST)
                        .body(
                            http_body_util::Full::new(Bytes::from(e.into_bytes()))
                                .map_err(|_| std::io::Error::other(""))
                                .boxed(),
                        )
                        .unwrap();
                },
                VisErr::Internal(e) => {
                    state.log.log_err(loga::WARN, e.context_with("Error serving response", ea!(url = url)));
                    return Response::builder()
                        .status(503)
                        .body(
                            http_body_util::Full::new(Bytes::new()).map_err(|_| std::io::Error::other("")).boxed(),
                        )
                        .unwrap();
                },
            }
        },
    }
}

#[derive(Aargvark)]
pub struct Args {
    config: AargvarkJson<Config>,
    validate: Option<()>,
}

pub async fn main(args: Args) -> Result<(), loga::Error> {
    let config = args.config.value;
    if args.validate.is_some() {
        return Ok(());
    }
    let log =
        Log::new_root(
            match config.debug || std::env::var_os("SUNWET_DEBUG").filter(|x| x.as_bytes() != b"n").is_some() {
                true => loga::DEBUG,
                false => loga::INFO,
            },
        );
    let tm = taskmanager::TaskManager::new();
    {
        let genfiles_dir = config.cache_dir.join("genfiles");
        create_dirs(&genfiles_dir).await?;
        let genfiles_stage_dir = config.cache_dir.join("genfiles_temp");
        create_dirs(&genfiles_stage_dir).await?;
        let stage_dir = config.persistent_dir.join("stage_files");
        create_dirs(&stage_dir).await?;
        let files_dir = config.persistent_dir.join("live/files");
        create_dirs(&files_dir).await?;
        let db_path = config.persistent_dir.join("live/db.sqlite3");
        let db =
            deadpool_sqlite::Config::new(&db_path)
                .builder(deadpool_sqlite::Runtime::Tokio1)
                .context("Error creating sqlite pool builder")?
                .post_create(Hook::async_fn(|db, _| {
                    Box::pin(async {
                        db
                            .interact(|db| {
                                db.busy_timeout(Duration::from_secs(60 * 10))?;
                                rusqlite::vtab::array::load_module(db)?;
                                return Ok(());
                            })
                            .await
                            .map_err(|e| HookError::Message(e.to_string().into()))?
                            .map_err(|e| HookError::Backend(e))?;
                        return Ok(());
                    })
                }))
                .build()
                .context("Error creating sqlite pool")?;
        db.get().await?.interact({
            let log = log.clone();
            move |db| {
                db::migrate(&mut *db, Some(&|v| migrate::migrate(v)))?;
                if db
                    .prepare("select 1 from sqlite_master where type='table' and name='meta_fts'")?
                    .query([])
                    .context("Error checking for meta_fts")?
                    .next()
                    .context("Error checking for meta_fts")?
                    .is_none() {
                    log.log(loga::DEBUG, "Initializing fts table");
                    db.execute_batch(include_str!("setup_fts.sql")).context("Error setting up meta_fts")?;
                    log.log(loga::DEBUG, "Done initializing fts table");
                }
                return Ok(()) as Result<_, loga::Error>;
            }
        }).await?.context_with("Migration failed", ea!(action = "db_init", path = db_path.to_string_lossy()))?;

        // Setup state
        let oidc_state = match &config.oidc {
            Some(oidc_config) => Some(
                oidc::new_state(&log, oidc_config.clone()).await.context("Error creating oidc state")?,
            ),
            None => None,
        };
        let fdap_state = match &config.fdap {
            Some(fdap_config) => Some(
                FdapState {
                    fdap_client: fdap::Client::builder()
                        .with_base_url(Uri::from_str(&fdap_config.url).context("Invalid fdap url")?)
                        .with_token(fdap_config.token.clone())
                        .build()
                        .context("Error setting up fdap client")?,
                },
            ),
            None => None,
        };
        let global_state = match &config.global {
            MaybeFdap::Fdap(subpath) => {
                let Some(fdap) = &fdap_state else {
                    return Err(loga::err("Global config set to use FDAP but no FDAP configured at config root"));
                };
                GlobalState::Fdap(FdapGlobalState {
                    fdap: fdap.clone(),
                    subpath: subpath.clone(),
                    cache: Mutex::new(None),
                })
            },
            MaybeFdap::Local(global_config) => GlobalState::Local(
                build_global_config(&log, global_config).context("Error assembling local config")?,
            ),
        };
        let users_state = match &config.users {
            Some(MaybeFdap::Fdap(subpath)) => {
                let Some(fdap) = &fdap_state else {
                    return Err(loga::err("User config set to use FDAP but no FDAP configured at config root"));
                };
                UsersState::Fdap(FdapUsersState {
                    fdap: fdap.clone(),
                    user_subpath: subpath.clone(),
                    cache: Cache::builder().time_to_live(Duration::from_secs(10)).build(),
                })
            },
            Some(MaybeFdap::Local(users_config)) => UsersState::Local(
                LocalUsersState {
                    users: users_config.users.iter().map(|(k, v)| (k.clone(), Arc::new(v.clone()))).collect(),
                },
            ),
            None => UsersState::Local(LocalUsersState { users: Default::default() }),
        };
        let (background_tx, background_rx) = mpsc::unbounded_channel();
        let state = Arc::new(State {
            oidc_state: oidc_state,
            fdap_state: fdap_state,
            global_state: global_state,
            users_state: users_state,
            tm: tm.clone(),
            db: db.clone(),
            log: log.clone(),
            files_dir: files_dir.clone(),
            files_stage_dir: stage_dir,
            genfiles_dir: genfiles_dir.clone(),
            genfiles_stage_dir: genfiles_stage_dir.clone(),
            finishing_uploads: Mutex::new(HashSet::new()),
            background: background_tx,
            bg_check: Default::default(),
            http_resp_headers: HeaderMap::from_iter([
                //. .
                ("cross-origin-embedder-policy", "require-corp"),
                ("cross-origin-opener-policy", "same-origin"),
            ].into_iter().map(|(k, v)| (HeaderName::from_static(k), HeaderValue::from_static(v)))),
            link_bg: Mutex::new(None),
            link_sessions: Cache::builder().time_to_idle(Duration::from_secs(24 * 60 * 60)).build(),
        });

        // Background tasks
        state.background.send(BackgroundJob::All).log(&log, loga::WARN, "Error triggering initial generate files scan");
        start_background_job(&state, &tm, background_rx);

        // Client<->server
        tm.critical_stream(
            "Server",
            TcpListenerStream::new(
                TcpListener::bind(config.bind_addr).await.stack_context(&log, "Error binding to address")?,
            ),
            {
                cap_fn!((stream)(log, state) {
                    let stream = match stream {
                        Ok(s) => s,
                        Err(e) => {
                            log.log_err(loga::DEBUG, e.context("Error opening peer stream"));
                            return Ok(());
                        },
                    };
                    let io = TokioIo::new(stream);
                    tokio::task::spawn(async move {
                        match async {
                            ta_return!((), loga::Error);
                            http1::Builder::new().serve_connection(io, service_fn(cap_fn!((req)(state) {
                                return Ok(handle_req(state, req).await) as Result<_, std::io::Error>;
                            }))).with_upgrades().await?;
                            return Ok(());
                        }.await {
                            Ok(_) => (),
                            Err(e) => {
                                log.log_err(loga::DEBUG, e.context("Error serving connection"));
                            },
                        }
                    });
                    return Ok(());
                })
            },
        );
    }

    // Wait for shutdown, cleanup
    tm.join(&log).await?;
    return Ok(());
}
