use {
    crate::{
        client::req::{
            req_simple,
            server_headers,
            server_url,
        },
        server::fsutil::create_dirs,
    },
    aargvark::{
        Aargvark,
        help::{
            HelpPattern,
            HelpPatternElement,
        },
        traits_impls::{
            AargvarkFile,
            AargvarkFromStr,
            AargvarkJson,
        },
    },
    chrono::{
        DateTime,
        Local,
        NaiveDateTime,
        Utc,
    },
    flowcontrol::ta_return,
    http::Uri,
    htwrap::{
        htreq::{
            self,
            Conn,
        },
        url::UriJoin,
    },
    loga::{
        DebugDisplay,
        Log,
        ResultContext,
        ea,
    },
    serde::Serialize,
    shared::{
        interface::{
            cli::{
                CliCommit,
                CliNode,
                CliTriple,
            },
            query::Query,
            triple::{
                Node,
                StrNode,
            },
            wire::{
                ReqCheckGet,
                ReqCheckStart,
                ReqCommit,
                ReqCommitFree,
                ReqGetTriplesAround,
                ReqHistory,
                ReqHistoryFilter,
                ReqHistoryFilterPredicate,
                ReqQuery,
                RespQueryRows,
                Triple,
            },
        },
        query_parser::compile_query,
    },
    std::{
        collections::{
            HashMap,
            HashSet,
        },
        path::PathBuf,
        str::FromStr,
        time::Duration,
        usize,
    },
    tokio::{
        fs::{
            File,
            read,
            write,
        },
        time::sleep,
    },
    uuid::Uuid,
};

pub mod req;
pub mod commit;
pub mod media_import;

pub struct AargvarkStrNode(pub Node);

impl AargvarkFromStr for AargvarkStrNode {
    fn from_str(s: &str) -> Result<Self, String> {
        return Ok(AargvarkStrNode(StrNode::from_str(s)?.0));
    }

    fn build_help_pattern(_state: &mut aargvark::help::HelpState) -> aargvark::help::HelpPattern {
        return HelpPattern(
            vec![
                HelpPatternElement::Variant(
                    vec![
                        HelpPattern(vec![HelpPatternElement::Type("f=FILEHASH".to_string())]),
                        HelpPattern(vec![HelpPatternElement::Type("v=JSON".to_string())])
                    ],
                )
            ],
        );
    }
}

#[derive(Aargvark)]
pub enum QueryCommandSource {
    /// Read query json from a file. You can compile a query to json with the
    /// `compile-query` subcommand.
    #[vark(name = "-f")]
    File(AargvarkJson<Query>),
    /// Inline (pass query as a command line argument)
    #[vark(name = "-i")]
    Inline(String),
}

#[derive(Aargvark)]
pub struct QueryCommand {
    debug: Option<()>,
    source: QueryCommandSource,
    parameters: HashMap<String, AargvarkStrNode>,
}

pub async fn handle_query(c: QueryCommand) -> Result<(), loga::Error> {
    let log = Log::new_root(if c.debug.is_some() {
        loga::DEBUG
    } else {
        loga::INFO
    });
    let query = match c.source {
        QueryCommandSource::File(v) => v.value,
        QueryCommandSource::Inline(v) => {
            compile_query(&v).map_err(loga::err)?
        },
    };
    let out = req::req_simple(&log, ReqQuery {
        query: query,
        parameters: c.parameters.iter().map(|(k, v)| (k.clone(), v.0.clone())).collect(),
        pagination: None,
    }).await?.rows;
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
    return Ok(());
}

#[derive(Aargvark)]
pub enum ExportCommandSource {
    /// Read query json from a file. You can compile a query to json with the
    /// `compile-query` subcommand.
    #[vark(name = "-qf")]
    QueryFile(AargvarkJson<Query>),
    /// Inline (pass query as a command line argument)
    #[vark(name = "-q")]
    Inline(String),
    #[vark(name = "-rf")]
    ResultFile(AargvarkJson<RespQueryRows>),
}

#[derive(Aargvark)]
pub struct ExportCommand {
    debug: Option<()>,
    /// The query or file to get the list of nodes to export from. If using a query,
    /// the query should be struct-less (no `{}`, i.e. it should just output a list of
    /// nodes).
    source: ExportCommandSource,
    /// Parameters for the query, if using a query source.
    #[vark(flag = "--parameters", flag = "--params", flag = "-p")]
    parameters: HashMap<String, AargvarkStrNode>,
    /// Write the exported `commit.json` and files to this directory.
    dest: PathBuf,
    /// Only include relations with the listed predicates. However if this is not
    /// specified all relations are included.
    include: Option<HashSet<String>>,
    /// Exclude relations with the listed predicates. Takes precedence over `--include`
    exclude: Option<HashSet<String>>,
}

pub async fn handle_export(c: ExportCommand) -> Result<(), loga::Error> {
    let log = Log::new_root(if c.debug.is_some() {
        loga::DEBUG
    } else {
        loga::INFO
    });
    create_dirs(&c.dest).await?;
    let triples_path = c.dest.join("triples.json");
    let triples = if triples_path.exists() {
        serde_json::from_slice::<Vec<Triple>>(
            &read(&triples_path)
                .await
                .context_with("Error reading existing triples", ea!(path = triples_path.dbg_str()))?,
        ).context_with("Error reading existing triples", ea!(path = triples_path.dbg_str()))?
    } else {
        fn check_query(q: &Query) -> Result<(), loga::Error> {
            if q.suffix.is_some() {
                return Err(loga::err("The export query has a struct, it must be struct-less (no `{}`)"));
            }
            return Ok(());
        }

        let RespQueryRows::Scalar(nodes) = (match c.source {
            ExportCommandSource::QueryFile(s) => {
                check_query(&s.value)?;
                req::req_simple(&log, ReqQuery {
                    query: s.value,
                    parameters: c.parameters.iter().map(|(k, v)| (k.clone(), v.0.clone())).collect(),
                    pagination: None,
                }).await?.rows
            },
            ExportCommandSource::Inline(s) => {
                let query = compile_query(&s).map_err(loga::err)?;
                check_query(&query)?;
                req::req_simple(&log, ReqQuery {
                    query: query,
                    parameters: c.parameters.iter().map(|(k, v)| (k.clone(), v.0.clone())).collect(),
                    pagination: None,
                }).await?.rows
            },
            ExportCommandSource::ResultFile(s) => {
                s.value
            },
        }) else {
            return Err(loga::err("The list of nodes has structured elements, the input list must be plain nodes."));
        };
        let triples = req::req_simple(&log, ReqGetTriplesAround { nodes: nodes }).await?;
        write(&triples_path, serde_json::to_string_pretty(&triples).unwrap())
            .await
            .context_with("Error writing received triples", ea!(path = triples_path.dbg_str()))?;
        triples
    };
    let mut commit_add = vec![];
    let server_url = server_url()?;
    let mut conn = None;
    for triple in &triples {
        if let Some(include) = c.include.as_ref() {
            if !include.contains(&triple.predicate) {
                continue;
            }
        }
        if let Some(exclude) = c.exclude.as_ref() {
            if exclude.contains(&triple.predicate) {
                continue;
            }
        }

        async fn download(
            log: &Log,
            server_url: &Uri,
            conn: &mut Option<Conn>,
            dest: &PathBuf,
            n: &Node,
        ) -> Result<CliNode, loga::Error> {
            let limits = htreq::Limits {
                read_body_size: usize::MAX,
                read_body_time: Duration::MAX,
                ..Default::default()
            };
            match n {
                Node::File(n) => {
                    let out = n.to_string().replace(":", "_");
                    let out_path = dest.join(&out);
                    if !out_path.exists() {
                        const RETRIES: usize = 5;
                        for i in 0 .. RETRIES {
                            match async {
                                ta_return!((), loga::Error);
                                let mut conn1 = match conn.take() {
                                    Some(c) => c,
                                    None => {
                                        htreq::connect(limits, &server_url).await?
                                    },
                                };
                                let mut req = http::Request::builder().method(http::Method::GET);
                                for (k, v) in server_headers()? {
                                    req = req.header(k, v);
                                }
                                let (code, _, body) =
                                    htreq::send(
                                        &log,
                                        limits,
                                        &mut conn1,
                                        req
                                            .uri(&server_url.join(format!("file/{}", n.to_string())))
                                            .body(
                                                http_body_util::Full::<hyper::body::Bytes>::new(
                                                    hyper::body::Bytes::new(),
                                                ),
                                            )
                                            .unwrap(),
                                    ).await?;
                                if code.as_u16() != 200 {
                                    return Err(loga::err(format!("Got non-200 status response for file {}", code)));
                                }
                                let mut out_file = File::create(&out_path).await?;
                                htreq::receive_stream(body, &mut out_file).await?;
                                *conn = Some(conn1);
                                return Ok(());
                            }.await {
                                Ok(_) => {
                                    break;
                                },
                                Err(e) => {
                                    log.log_err(
                                        loga::WARN,
                                        e.context(&format!("Download failed (attempt {}/{})", i, RETRIES)),
                                    );
                                },
                            }
                            sleep(Duration::from_secs(2)).await;
                        }
                    }
                    return Ok(CliNode::Upload(out_path));
                },
                Node::Value(n) => {
                    return Ok(CliNode::Value(n.clone()));
                },
            }
        }

        commit_add.push(CliTriple {
            subject: download(&log, &server_url, &mut conn, &c.dest, &triple.subject).await?,
            predicate: triple.predicate.clone(),
            object: download(&log, &server_url, &mut conn, &c.dest, &triple.object).await?,
        });
    }
    write(c.dest.join("commit.json"), serde_json::to_string_pretty(&CliCommit {
        remove: vec![],
        add: commit_add,
    }).unwrap()).await?;
    return Ok(());
}

#[derive(Aargvark)]
pub struct CompileQueryCommand {
    inline: Option<String>,
    file: Option<AargvarkFile>,
}

pub fn handle_compile_query(c: CompileQueryCommand) -> Result<(), loga::Error> {
    let query;
    if let Some(q) = c.inline {
        query = q;
        if c.file.is_some() {
            return Err(
                loga::err("A query was both specified on the command line and via file, you can only do one"),
            );
        }
    } else if let Some(q_file) = c.file {
        query = String::from_utf8(q_file.value).context("Query was not valid utf-8")?;
    } else {
        return Err(loga::err("Must specify a query, either on the command line or as a file"));
    }
    let out = compile_query(&query).map_err(|e| loga::err(e))?;
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
    return Ok(());
}

pub struct StrDatetime(pub DateTime<Utc>);

impl AargvarkFromStr for StrDatetime {
    fn from_str(s: &str) -> Result<Self, String> {
        if let Ok(t) = DateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%z") {
            return Ok(Self(t.into()));
        }
        if let Ok(t) = DateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%z") {
            return Ok(Self(t.into()));
        }
        if let Ok(t) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
            return Ok(Self(t.and_local_timezone(Local).unwrap().into()));
        }
        if let Ok(t) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
            return Ok(Self(t.and_local_timezone(Local).unwrap().into()));
        }
        if let Ok(t) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d") {
            return Ok(Self(t.and_local_timezone(Local).unwrap().into()));
        }
        return Err(format!("Unrecognized time format"));
    }

    fn build_help_pattern(_state: &mut aargvark::help::HelpState) -> HelpPattern {
        return HelpPattern(vec![HelpPatternElement::Type("DATETIME/DATE".to_string())]);
    }
}

#[derive(Aargvark)]
pub struct DeleteNodesCommand {
    debug: Option<()>,
    nodes: Vec<AargvarkStrNode>,
}

pub async fn handle_delete_nodes(args: DeleteNodesCommand) -> Result<(), loga::Error> {
    let log = Log::new_root(if args.debug.is_some() {
        loga::DEBUG
    } else {
        loga::INFO
    });
    req_simple(&log, ReqCommit::Free(ReqCommitFree {
        comment: format!("CLI delete note"),
        add: vec![],
        remove: req_simple(
            &log,
            ReqGetTriplesAround { nodes: args.nodes.into_iter().map(|x| x.0).collect() },
        ).await?,
        files: vec![],
    })).await?;
    return Ok(());
}

#[derive(Aargvark)]
pub struct MergeNodesCommand {
    debug: Option<()>,
    dest_node: AargvarkStrNode,
    merge_nodes: Vec<AargvarkStrNode>,
}

pub async fn handle_merge_nodes_command(args: MergeNodesCommand) -> Result<(), loga::Error> {
    let log = Log::new_root(if args.debug.is_some() {
        loga::DEBUG
    } else {
        loga::INFO
    });
    let merge_nodes_lookup = args.merge_nodes.iter().map(|x| &x.0).collect::<HashSet<_>>();
    let mut remove = vec![];
    let mut add = vec![];
    for t in req_simple(
        &log,
        ReqGetTriplesAround { nodes: args.merge_nodes.iter().map(|x| x.0.clone()).collect() },
    ).await? {
        if merge_nodes_lookup.contains(&t.subject) {
            add.push(Triple {
                subject: args.dest_node.0.clone(),
                predicate: t.predicate.clone(),
                object: t.object.clone(),
            });
        } else {
            add.push(Triple {
                subject: t.subject.clone(),
                predicate: t.predicate.clone(),
                object: args.dest_node.0.clone(),
            });
        }
        remove.push(t);
    }
    req_simple(&log, ReqCommit::Free(ReqCommitFree {
        comment: format!("CLI merge nodes"),
        add: add,
        remove: remove,
        files: vec![],
    })).await?;
    return Ok(());
}

#[derive(Aargvark)]
pub struct DuplicateNodesCommand {
    debug: Option<()>,
    /// Node to duplicate
    node: AargvarkStrNode,
}

pub async fn handle_duplicate_nodes_command(args: DuplicateNodesCommand) -> Result<(), loga::Error> {
    let log = Log::new_root(if args.debug.is_some() {
        loga::DEBUG
    } else {
        loga::INFO
    });
    let dest_str = Uuid::new_v4().hyphenated().to_string();
    let dest = Node::Value(serde_json::Value::String(dest_str.clone()));
    let mut add = vec![];
    for t in req_simple(&log, ReqGetTriplesAround { nodes: vec![args.node.0.clone()] }).await? {
        if t.subject == dest {
            add.push(Triple {
                subject: dest.clone(),
                predicate: t.predicate.clone(),
                object: t.object.clone(),
            });
        } else {
            add.push(Triple {
                subject: t.subject.clone(),
                predicate: t.predicate.clone(),
                object: dest.clone(),
            });
        }
    }
    req_simple(&log, ReqCommit::Free(ReqCommitFree {
        comment: format!("CLI duplicate [{}]", serde_json::to_string(&args.node.0).unwrap()),
        add: add,
        remove: vec![],
        files: vec![],
    })).await?;
    print!("{}", dest_str);
    return Ok(());
}

#[derive(Aargvark)]
pub struct HistoryCommand {
    debug: Option<()>,
    /// Get commits starting before this time
    before: Option<StrDatetime>,
    /// Get commits no earlier than this time
    until: Option<StrDatetime>,
    /// Restrict to history affecting this subject
    subject: Option<AargvarkStrNode>,
    /// Restrict to history involving relations with this predicate
    predicate: Option<String>,
    /// Restrict to history affecting this object
    object: Option<AargvarkStrNode>,
}

pub async fn handle_history(args: HistoryCommand) -> Result<(), loga::Error> {
    let log = Log::new_root(if args.debug.is_some() {
        loga::DEBUG
    } else {
        loga::INFO
    });

    #[derive(Serialize)]
    struct HistoryEvent {
        delete: bool,
        triple: Triple,
    }

    #[derive(Serialize)]
    struct HistoryCommit {
        id_timestmap: DateTime<Utc>,
        description: String,
        events: Vec<HistoryEvent>,
    }

    let mut commits = HashMap::new();
    let mut page_key = None;
    'paginate : loop {
        let res = req::req_simple(&log, ReqHistory {
            page_key: page_key,
            filter: match (
                args.subject.as_ref().map(|x| x.0.clone()),
                args.object.as_ref().map(|x| x.0.clone()),
            ) {
                (None, None) => None,
                (None, Some(n)) => Some(ReqHistoryFilter {
                    node: n,
                    predicate: args.predicate.clone().map(|p| ReqHistoryFilterPredicate::Incoming(p)),
                }),
                (Some(n), None) => Some(ReqHistoryFilter {
                    predicate: args.predicate.clone().map(|p| ReqHistoryFilterPredicate::Outgoing(p)),
                    node: n,
                }),
                (Some(_), Some(_)) => return Err(
                    loga::err("History only usefully filters with an open subject or object, not both"),
                ),
            },
        }).await?;
        if res.events.is_empty() {
            break 'paginate;
        }
        page_key = res.events.last().map(|x| (x.commit, x.triple.clone()));
        for c in res.commit_descriptions {
            commits.entry(c.0).or_insert_with(|| HistoryCommit {
                id_timestmap: c.0,
                description: c.1,
                events: Default::default(),
            });
        }
        for event in res.events {
            if let Some(s) = &args.before {
                if event.commit > s.0 {
                    continue;
                }
            }
            if let Some(s) = &args.until {
                if event.commit < s.0 {
                    break 'paginate;
                }
            }
            commits.get_mut(&event.commit).unwrap().events.push(HistoryEvent {
                delete: event.delete,
                triple: event.triple,
            });
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&commits.values().filter(|x| !x.events.is_empty()).collect::<Vec<_>>(),).unwrap()
    );
    return Ok(());
}

#[derive(Aargvark)]
pub struct GetNodeCommand {
    debug: Option<()>,
    node: AargvarkStrNode,
}

pub async fn handle_get_node(c: GetNodeCommand) -> Result<(), loga::Error> {
    let log = Log::new_root(if c.debug.is_some() {
        loga::DEBUG
    } else {
        loga::INFO
    });
    let res = req::req_simple(&log, ReqGetTriplesAround { nodes: vec![c.node.0] }).await?;
    println!("{}", serde_json::to_string_pretty(&res).unwrap());
    return Ok(());
}

#[derive(Aargvark)]
pub struct CheckCommand {
    debug: Option<()>,
    /// If a check is already running, stop it first
    restart: Option<()>,
}

pub async fn handle_check(c: CheckCommand) -> Result<(), loga::Error> {
    let log = Log::new_root(if c.debug.is_some() {
        loga::DEBUG
    } else {
        loga::INFO
    });
    req::req_simple(&log, ReqCheckStart { restart: c.restart.is_some() }).await?;
    let results = loop {
        if let Some(results) = req::req_simple(&log, ReqCheckGet).await? {
            break results;
        }
        sleep(Duration::from_secs(10)).await;
    };
    println!("{}", serde_json::to_string_pretty(&results).unwrap());
    return Ok(());
}
