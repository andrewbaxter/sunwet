use {
    crate::client::req::req_simple,
    aargvark::{
        help::{
            HelpPattern,
            HelpPatternElement,
        },
        traits_impls::{
            AargvarkFile,
            AargvarkFromStr,
            AargvarkJson,
            Source,
        },
        Aargvark,
    },
    chrono::{
        DateTime,
        Local,
        NaiveDateTime,
        Utc,
    },
    loga::{
        Log,
        ResultContext,
    },
    query::compile_query,
    serde::Serialize,
    shared::interface::{
        query::Query,
        triple::{
            FileHash,
            Node,
        },
        wire::{
            ReqCommit,
            ReqGetTriplesAround,
            ReqHistory,
            ReqHistoryFilter,
            ReqHistoryFilterPredicate,
            ReqQuery,
            Triple,
        },
    },
    std::{
        collections::HashMap,
        env::current_dir,
        str::FromStr,
    },
};

pub mod req;
pub mod commit;
pub mod query;
pub mod query_test;
pub mod import_;

pub struct StrNode(pub Node);

impl AargvarkFromStr for StrNode {
    fn from_str(s: &str) -> Result<Self, String> {
        let Some((k, v)) = s.split_once("=") else {
            return Err(format!("Invalid node format: [{}]", s));
        };
        match k {
            "f" => {
                return Ok(StrNode(Node::File(
                    //. .
                    FileHash::from_str(v).map_err(|e| format!("File node [{}] isn't in a valid format: {}", v, e))?,
                )));
            },
            "v" => {
                return Ok(StrNode(Node::Value(
                    //. .
                    serde_json::from_str(v).map_err(|e| format!("Value node has invalid json [{}]: {}", v, e))?,
                )));
            },
            _ => {
                return Err(format!("Unknown node prefix [{}]", k));
            },
        }
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
pub struct QueryCommand {
    debug: Option<()>,
    query: AargvarkJson<Query>,
    parameters: HashMap<String, StrNode>,
}

pub async fn handle_query(c: QueryCommand) -> Result<(), loga::Error> {
    let log = Log::new_root(if c.debug.is_some() {
        loga::DEBUG
    } else {
        loga::INFO
    });
    let out = req::req_simple(&log, ReqQuery {
        query: c.query.value.clone(),
        parameters: c.parameters.iter().map(|(k, v)| (k.clone(), v.0.clone())).collect(),
        pagination: None,
    }).await?.records;
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
    return Ok(());
}

#[derive(Aargvark)]
pub struct CompileQueryCommand {
    query: Option<String>,
    file: Option<AargvarkFile>,
}

pub fn handle_compile_query(c: CompileQueryCommand) -> Result<(), loga::Error> {
    let query;
    let query_dir;
    if let Some(q) = c.query {
        query = q;
        if c.file.is_some() {
            return Err(
                loga::err("A query was both specified on the command line and via file, you can only do one"),
            );
        }
        query_dir = current_dir()?;
    } else if let Some(q_file) = c.file {
        query = String::from_utf8(q_file.value).context("Query was not valid utf-8")?;
        match q_file.source {
            Source::Stdin => {
                query_dir = current_dir()?;
            },
            Source::File(source) => {
                query_dir =
                    source
                        .canonicalize()
                        .context("Unable to resolve query file path")?
                        .parent()
                        .context("Query file path has no parent, required for includes")?
                        .to_path_buf();
            },
        }
    } else {
        return Err(loga::err("Must specify a query, either on the command line or as a file"));
    }
    let out = compile_query(Some(&query_dir), &query)?;
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
    nodes: Vec<StrNode>,
}

pub async fn handle_delete_nodes(args: DeleteNodesCommand) -> Result<(), loga::Error> {
    let log = Log::new_root(if args.debug.is_some() {
        loga::DEBUG
    } else {
        loga::INFO
    });
    let mut triples = vec![];
    for node in args.nodes {
        let node_triples = req_simple(&log, ReqGetTriplesAround { node: node.0 }).await?;
        triples.extend(node_triples.incoming);
        triples.extend(node_triples.outgoing);
    }
    req_simple(&log, ReqCommit {
        comment: format!("CLI delete note"),
        add: vec![],
        remove: triples,
        files: vec![],
    }).await?;
    return Ok(());
}

#[derive(Aargvark)]
pub struct MergeNodesCommand {
    debug: Option<()>,
    dest_node: StrNode,
    merge_nodes: Vec<StrNode>,
}

pub async fn handle_merge_nodes_command(args: MergeNodesCommand) -> Result<(), loga::Error> {
    let log = Log::new_root(if args.debug.is_some() {
        loga::DEBUG
    } else {
        loga::INFO
    });
    let mut remove = vec![];
    let mut add = vec![];
    for node in args.merge_nodes {
        let node_triples = req_simple(&log, ReqGetTriplesAround { node: node.0 }).await?;
        for t in node_triples.incoming {
            add.push(Triple {
                subject: t.subject.clone(),
                predicate: t.predicate.clone(),
                object: args.dest_node.0.clone(),
            });
            remove.push(t);
        }
        for t in node_triples.outgoing {
            add.push(Triple {
                subject: args.dest_node.0.clone(),
                predicate: t.predicate.clone(),
                object: t.object.clone(),
            });
            remove.push(t);
        }
    }
    req_simple(&log, ReqCommit {
        comment: format!("CLI merge nodes"),
        add: add,
        remove: remove,
        files: vec![],
    }).await?;
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
    subject: Option<StrNode>,
    /// Restrict to history involving relations with this predicate
    predicate: Option<String>,
    /// Restrict to history affecting this object
    object: Option<StrNode>,
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
    let mut before_commit = args.before.map(|x| x.0);
    let mut after_triple = None;
    'paginate : loop {
        let res = req::req_simple(&log, ReqHistory {
            before_commit: before_commit,
            after_triple: after_triple,
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
        before_commit = res.events.last().map(|x| x.commit);
        after_triple = res.events.last().map(|x| x.triple.clone());
        for c in res.commit_descriptions {
            commits.entry(c.0).or_insert_with(|| HistoryCommit {
                id_timestmap: c.0,
                description: c.1,
                events: Default::default(),
            });
        }
        for e in res.events {
            if let Some(until) = &args.until {
                if e.commit < until.0 {
                    break 'paginate;
                }
            }
            commits.get_mut(&e.commit).unwrap().events.push(HistoryEvent {
                delete: e.delete,
                triple: e.triple,
            });
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &commits.values().filter(|x| !x.events.is_empty()).collect::<Vec<_>>(),
        ).unwrap()
    );
    return Ok(());
}

#[derive(Aargvark)]
pub struct GetNodeCommand {
    debug: Option<()>,
    node: StrNode,
}

pub async fn handle_get_node(c: GetNodeCommand) -> Result<(), loga::Error> {
    let log = Log::new_root(if c.debug.is_some() {
        loga::DEBUG
    } else {
        loga::INFO
    });
    let res = req::req_simple(&log, ReqGetTriplesAround { node: c.node.0 }).await?;
    println!("{}", serde_json::to_string_pretty(&res).unwrap());
    return Ok(());
}
