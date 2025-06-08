use {
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
    rand::{
        thread_rng,
        Rng,
    },
    shared::interface::{
        query::Query,
        triple::{
            FileHash,
            Node,
        },
        wire::{
            Pagination,
            ReqGetTriplesAround,
            ReqHistory,
            ReqQuery,
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
    let mut out = vec![];
    const PAGE_SIZE: usize = 1000;
    let seed = thread_rng().gen();
    let mut pagination = Some(Pagination {
        count: PAGE_SIZE,
        seed: Some(seed),
        after: None,
    });
    loop {
        let res = req::req_simple(&log, ReqQuery {
            query: c.query.value.clone(),
            parameters: c.parameters.iter().map(|(k, v)| (k.clone(), v.0.clone())).collect(),
            pagination: pagination,
        }).await?;
        out.extend(res.records);
        let Some(next_after) = res.page_end else {
            break;
        };
        pagination = Some(Pagination {
            count: PAGE_SIZE,
            seed: Some(seed),
            after: Some(next_after),
        });
    }
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
pub struct HistoryCommand {
    debug: Option<()>,
    /// Get commits starting at this time or after
    start: Option<StrDatetime>,
    /// Get commits starting before this time
    end: Option<StrDatetime>,
}

pub async fn handle_history(c: HistoryCommand) -> Result<(), loga::Error> {
    let log = Log::new_root(if c.debug.is_some() {
        loga::DEBUG
    } else {
        loga::INFO
    });
    let res = req::req_simple(&log, ReqHistory {
        start_incl: c.start.unwrap_or(StrDatetime(DateTime::<Utc>::MIN_UTC)).0,
        end_excl: c.end.unwrap_or(StrDatetime(DateTime::<Utc>::MAX_UTC)).0,
    }).await?;
    println!("{}", serde_json::to_string_pretty(&res).unwrap());
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
