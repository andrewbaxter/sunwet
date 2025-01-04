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
    shared::interface::{
        query::{
            Query,
        },
        triple::{
            FileHash,
            Node,
        },
        wire::{
            ReqGetTriplesAround,
            ReqHistory,
            ReqQuery,
        },
    },
    std::{
        collections::HashMap,
        str::FromStr,
    },
};

pub mod req;
pub mod change;
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
    pub query: AargvarkJson<Query>,
    pub parameters: HashMap<String, StrNode>,
}

pub async fn handle_query(c: QueryCommand) -> Result<(), loga::Error> {
    let log = Log::new_root(loga::INFO);
    let res = req::req_simple(&log, ReqQuery {
        query: c.query.value,
        parameters: c.parameters.into_iter().map(|(k, v)| (k, v.0)).collect(),
    }).await?;
    println!("{}", serde_json::to_string_pretty(&res).unwrap());
    return Ok(());
}

#[derive(Aargvark)]
pub struct CompileQueryCommand {
    query: Option<String>,
    file: Option<AargvarkFile>,
}

pub fn handle_compile_query(c: CompileQueryCommand) -> Result<(), loga::Error> {
    let query;
    if let Some(q) = c.query {
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
    let out = compile_query(&query)?;
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
    /// Get commits starting at this time or after
    pub start: Option<StrDatetime>,
    /// Get commits starting before this time
    pub end: Option<StrDatetime>,
}

pub async fn handle_history(c: HistoryCommand) -> Result<(), loga::Error> {
    let log = Log::new_root(loga::INFO);
    let res = req::req_simple(&log, ReqHistory {
        start_incl: c.start.unwrap_or(StrDatetime(DateTime::<Utc>::MIN_UTC)).0,
        end_excl: c.end.unwrap_or(StrDatetime(DateTime::<Utc>::MAX_UTC)).0,
    }).await?;
    println!("{}", serde_json::to_string_pretty(&res).unwrap());
    return Ok(());
}

#[derive(Aargvark)]
pub struct GetNodeCommand {
    pub node: StrNode,
}

pub async fn handle_get_node(c: GetNodeCommand) -> Result<(), loga::Error> {
    let log = Log::new_root(loga::INFO);
    let res = req::req_simple(&log, ReqGetTriplesAround { node: c.node.0 }).await?;
    println!("{}", serde_json::to_string_pretty(&res).unwrap());
    return Ok(());
}
