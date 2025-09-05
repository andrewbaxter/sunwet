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
    serde::Serialize,
    shared::{
        interface::{
            query::Query,
            triple::{
                Node,
                StrNode,
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
        query_parser::{
            compile_fragment_query_head,
            compile_fragment_query_tail,
            compile_query,
        },
    },
    std::{
        collections::HashMap,
        str::FromStr,
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
pub struct QueryCommand {
    debug: Option<()>,
    query: AargvarkJson<Query>,
    parameters: HashMap<String, AargvarkStrNode>,
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

#[derive(Aargvark)]
pub struct CompileQueryHeadCommand {
    inline: Option<String>,
    file: Option<AargvarkFile>,
}

pub fn handle_compile_query_head(c: CompileQueryHeadCommand) -> Result<(), loga::Error> {
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
    let out = compile_fragment_query_head(&query).map_err(|e| loga::err(e))?;
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
    return Ok(());
}

#[derive(Aargvark)]
pub struct CompileQueryTailCommand {
    inline: Option<String>,
    file: Option<AargvarkFile>,
}

pub fn handle_compile_query_tail(c: CompileQueryTailCommand) -> Result<(), loga::Error> {
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
    let out = compile_fragment_query_tail(&query).map_err(|e| loga::err(e))?;
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
    dest_node: AargvarkStrNode,
    merge_nodes: Vec<AargvarkStrNode>,
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
    let node_triples = req_simple(&log, ReqGetTriplesAround { node: args.node.0.clone() }).await?;
    for t in node_triples.incoming {
        add.push(Triple {
            subject: t.subject.clone(),
            predicate: t.predicate.clone(),
            object: dest.clone(),
        });
    }
    for t in node_triples.outgoing {
        add.push(Triple {
            subject: dest.clone(),
            predicate: t.predicate.clone(),
            object: t.object.clone(),
        });
    }
    req_simple(&log, ReqCommit {
        comment: format!("CLI duplicate [{}]", serde_json::to_string(&args.node.0).unwrap()),
        add: add,
        remove: vec![],
        files: vec![],
    }).await?;
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
        serde_json::to_string_pretty(
            &commits.values().filter(|x| !x.events.is_empty()).collect::<Vec<_>>(),
        ).unwrap()
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
    let res = req::req_simple(&log, ReqGetTriplesAround { node: c.node.0 }).await?;
    println!("{}", serde_json::to_string_pretty(&res).unwrap());
    return Ok(());
}
