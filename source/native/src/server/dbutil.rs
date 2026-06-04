use {
    chrono::{
        DateTime,
        Utc,
    },
    crate::{
        interface::triple::DbNode,
        server::db,
    },
    deadpool_sqlite::Pool,
    good_ormning::runtime::sqlite::{
        GoodOrmningCustomString,
        GoodOrmningSqliteTimestamp,
        SqliteConnection,
    },
    loga::ResultContext,
    rusqlite::Transaction,
};

pub async fn tx<
    O: 'static + Send + Sync,
    F: 'static + Send + FnOnce(&mut db::Db<&mut Transaction<'_>>) -> Result<O, loga::Error>,
>(pool: &Pool, cb: F) -> Result<O, loga::Error> {
    let conn = pool.get().await?;
    return Ok(conn.interact(|conn| {
        let mut tx = conn.transaction()?;
        let mut db = db::Db(&mut tx);
        match cb(&mut db) {
            Ok(res) => {
                tx.commit().context("Failed to commit transaction")?;
                Ok(res)
            },
            Err(e) => {
                let e = e.context("Error during transaction");
                match tx.rollback().context("Error rolling back transaction due to error") {
                    Err(re) => {
                        return Err(e.also(re));
                    },
                    Ok(_) => {
                        return Err(e);
                    },
                };
            },
        }
    }).await??);
}

pub enum Txr<T> {
    Ok(T),
    Abort,
}

pub async fn abortable_tx<
    O: 'static + Send + Sync,
    F: 'static + Send + FnOnce(&mut db::Db<&mut Transaction<'_>>) -> Result<Txr<O>, loga::Error>,
>(pool: &Pool, cb: F) -> Result<Option<O>, loga::Error> {
    let conn = pool.get().await?;
    return Ok(conn.interact(|conn| {
        let mut tx = conn.transaction()?;
        let mut db = db::Db(&mut tx);
        match cb(&mut db) {
            Ok(Txr::Ok(res)) => {
                tx.commit().context("Failed to commit transaction")?;
                Ok(Some(res))
            },
            Ok(Txr::Abort) => {
                match tx.rollback().context("Error rolling back transaction due to abort") {
                    Err(re) => {
                        return Err(re);
                    },
                    Ok(_) => {
                        return Ok(None);
                    },
                };
            },
            Err(e) => {
                let e = e.context("Error during transaction");
                match tx.rollback().context("Error rolling back transaction due to error") {
                    Err(re) => {
                        return Err(e.also(re));
                    },
                    Ok(_) => {
                        return Err(e);
                    },
                };
            },
        }
    }).await??);
}

pub fn triple_gc_deleted(
    db: &mut db::Db<impl good_ormning::runtime::sqlite::SqliteConnection>,
    epoch: chrono::DateTime<chrono::Utc>,
) -> Result<(), loga::Error> {
    good_ormning::sqlite::good_query!(
        db,
        //# genemichaels-external: sql-formatter-sqlite
        r#"delete from "triple"
           where
             (
               "triple"."commit_" < ${utctime_ms_chrono = epoch}
               and (
                 "triple"."exists" = false
                 or not exists (
                   select
                     1
                   from
                     "triple_snapshot"
                   where
                     (
                       "triple"."subject" = "triple_snapshot"."subject"
                       and "triple"."predicate" = "triple_snapshot"."predicate"
                       and "triple"."object" = "triple_snapshot"."object"
                       and "triple"."commit_" = "triple_snapshot"."commit_"
                     )
                 )
               )
             )
           "#;
        db
    ).context("Error executing triple_gc_deleted")?;
    Ok(())
}

pub fn subjobj_gc(db: &mut db::Db<impl good_ormning::runtime::sqlite::SqliteConnection>) -> Result<(), loga::Error> {
    good_ormning::sqlite::good_query!(
        db,
        //# genemichaels-external: sql-formatter-sqlite
        r#"delete from "subjobj"
           where
             (
               not exists (
                 select
                   1
                 from
                   "triple"
                 where
                   "subjobj"."id" = "triple"."subject"
               )
               and not exists (
                 select
                   1
                 from
                   "triple"
                 where
                   "subjobj"."id" = "triple"."object"
               )
             )
           "#;
        db
    ).context("Error executing subjobj_gc")?;
    Ok(())
}

pub fn predicate_gc(
    db: &mut db::Db<impl good_ormning::runtime::sqlite::SqliteConnection>,
) -> Result<(), loga::Error> {
    good_ormning::sqlite::good_query!(
        db,
        //# genemichaels-external: sql-formatter-sqlite
        r#"delete from "predicate"
           where
             not exists (
               select
                 1
               from
                 "triple"
               where
                 "predicate"."id" = "triple"."predicate"
             )
           "#;
        db
    ).context("Error executing predicate_gc")?;
    Ok(())
}

pub fn meta_gc(db: &mut db::Db<impl good_ormning::runtime::sqlite::SqliteConnection>) -> Result<(), loga::Error> {
    good_ormning::sqlite::good_query!(
        db,
        //# genemichaels-external: sql-formatter-sqlite
        r#"delete from "meta"
           where
             not exists (
               select
                 1
               from
                 "subjobj"
               where
                 (
                   "subjobj"."value" = "meta"."node"
                   and (
                     exists (
                       select
                         1
                       from
                         "triple"
                       where
                         "triple"."subject" = "subjobj"."id"
                     )
                     or exists (
                       select
                         1
                       from
                         "triple"
                       where
                         "triple"."object" = "subjobj"."id"
                     )
                   )
                 )
             )
           "#;
        db
    ).context("Error executing meta_gc")?;
    Ok(())
}

pub fn commit_gc(db: &mut db::Db<impl good_ormning::runtime::sqlite::SqliteConnection>) -> Result<(), loga::Error> {
    good_ormning::sqlite::good_query!(
        db,
        //# genemichaels-external: sql-formatter-sqlite
        r#"with
             active_commits (stamp) as (
               select distinct
                 "triple"."commit_"
               from
                 "triple"
             )
           delete from "commit"
           where
             not exists (
               select
                 1
               from
                 "active_commits"
               where
                 "commit"."idtimestamp" = "active_commits"."stamp"
             )
           "#;
        db
    ).context("Error executing commit_gc")?;
    Ok(())
}

pub fn gen_gc(db: &mut db::Db<impl good_ormning::runtime::sqlite::SqliteConnection>) -> Result<(), loga::Error> {
    good_ormning::sqlite::good_query!(
        db,
        //# genemichaels-external: sql-formatter-sqlite
        r#"delete from "generated"
           where
             not exists (
               select
                 1
               from
                 "subjobj"
               where
                 (
                   "subjobj"."value" = "generated"."node"
                   and (
                     exists (
                       select
                         1
                       from
                         "triple_snapshot"
                       where
                         "triple_snapshot"."subject" = "subjobj"."id"
                     )
                     or exists (
                       select
                         1
                       from
                         "triple_snapshot"
                       where
                         "triple_snapshot"."object" = "subjobj"."id"
                     )
                   )
                 )
             )
           "#;
        db
    ).context("Error executing gen_gc")?;
    Ok(())
}

pub fn file_access_gc(
    db: &mut db::Db<impl good_ormning::runtime::sqlite::SqliteConnection>,
    source: &crate::server::access::DbAccessSourceId,
    hash: &i64,
) -> Result<(), loga::Error> {
    good_ormning::sqlite::good_query!(
        db,
        //# genemichaels-external: sql-formatter-sqlite
        r#"delete from "file_access"
           where
             "access_source" = ${access_source = source}
             and "spec_hash" != ${i64 = *hash}
           "#;
        db
    ).context("Error executing file_access_gc")?;
    Ok(())
}

pub fn file_access_insert(
    db: &mut db::Db<impl good_ormning::runtime::sqlite::SqliteConnection>,
    file: &crate::interface::triple::DbFileHash,
    source: &crate::server::access::DbAccessSourceId,
    hash: &i64,
) -> Result<(), loga::Error> {
    good_ormning::sqlite::good_query!(
        db,
        //# genemichaels-external: sql-formatter-sqlite
        r#"insert or ignore into
             "file_access" ("file", "access_source", "spec_hash")
           values
             (
               ${filehash = file},
               ${access_source = source},
               ${i64 = *hash}
             )
           "#;
        db
    ).context("Error executing file_access_insert")?;
    Ok(())
}

pub fn file_access_exists(
    db: &mut db::Db<impl good_ormning::runtime::sqlite::SqliteConnection>,
    file: &crate::interface::triple::DbFileHash,
    source: &crate::server::access::DbAccessSourceId,
    hash: &i64,
) -> Result<bool, loga::Error> {
    Ok(good_ormning::sqlite::good_query_opt!(
        db,
        //# genemichaels-external: sql-formatter-sqlite
        r#"select
             1 as x
           from
             file_access
           where
             file = ${filehash = file}
             and access_source = ${access_source = source}
             and spec_hash = ${i64 = *hash}
           "#;
        db
    ).context("Error executing file_access_exists")?.is_some())
}

pub fn file_access_get_sources(
    db: &mut db::Db<impl good_ormning::runtime::sqlite::SqliteConnection>,
    file: &crate::interface::triple::DbFileHash,
) -> Result<Vec<crate::server::access::DbAccessSourceId>, loga::Error> {
    Ok(good_ormning::sqlite::good_query_many!(
        db,
        //# genemichaels-external: sql-formatter-sqlite
        r#"select
             access_source
           from
             file_access
           where
             file = ${filehash = file}
           "#;
        db
    ).context("Error executing file_access_get_sources")?)
}

pub fn meta_get_mimetype(
    db: &mut db::Db<impl good_ormning::runtime::sqlite::SqliteConnection>,
    node: &crate::interface::triple::DbNode,
) -> Result<Option<Option<String>>, loga::Error> {
    Ok(good_ormning::sqlite::good_query_opt!(
        db,
        //# genemichaels-external: sql-formatter-sqlite
        r#"select
             mimetype
           from
             meta
           where
             node = ${node = node}
           "#;
        db
    ).context("Error executing meta_get_mimetype")?)
}

pub fn meta_upsert_mimetype(
    db: &mut db::Db<impl good_ormning::runtime::sqlite::SqliteConnection>,
    node: &crate::interface::triple::DbNode,
    mimetype: &Option<String>,
) -> Result<(), loga::Error> {
    good_ormning::sqlite::good_query!(
        db,
        //# genemichaels-external: sql-formatter-sqlite
        r#"insert into
             meta (node, mimetype, fulltext)
           values
             (
               ${node = node},
               ${opt string = mimetype.as_deref()},
               ''
             )
           on conflict (node) do update
           set
             mimetype = excluded.mimetype
           "#;
        db
    ).context("Error executing meta_upsert_mimetype")?;
    Ok(())
}

pub fn meta_upsert_fulltext(
    db: &mut db::Db<impl good_ormning::runtime::sqlite::SqliteConnection>,
    node: &crate::interface::triple::DbNode,
    fulltext: &str,
) -> Result<(), loga::Error> {
    good_ormning::sqlite::good_query!(
        db,
        //# genemichaels-external: sql-formatter-sqlite
        r#"insert into
             meta (node, mimetype, fulltext)
           values
             (${node = node}, null, ${string = fulltext})
           on conflict (node) do update
           set
             fulltext = excluded.fulltext
           "#;
        db
    ).context("Error executing meta_upsert_fulltext")?;
    Ok(())
}

pub fn triple_snapshot_exists(
    db: &mut db::Db<impl good_ormning::runtime::sqlite::SqliteConnection>,
    subject: &crate::interface::triple::DbNode,
    predicate: &str,
    object: &crate::interface::triple::DbNode,
) -> Result<bool, loga::Error> {
    Ok(good_ormning::sqlite::good_query_opt!(
        db,
        //# genemichaels-external: sql-formatter-sqlite
        r#"select
             1 as x
           from
             "triple_snapshot"
           where
             "subject" = (
               select
                 "id"
               from
                 "subjobj"
               where
                 "value" = ${node = subject}
             )
             and "predicate" = (
               select
                 "id"
               from
                 "predicate"
               where
                 "value" = ${string = predicate}
             )
             and "object" = (
               select
                 "id"
               from
                 "subjobj"
               where
                 "value" = ${node = object}
             )
           "#;
        db
    ).context("Error executing triple_snapshot_exists")?.is_some())
}

pub fn commit_insert(
    db: &mut db::Db<impl good_ormning::runtime::sqlite::SqliteConnection>,
    id: &chrono::DateTime<chrono::Utc>,
    description: &str,
) -> Result<(), loga::Error> {
    good_ormning::sqlite::good_query!(
        db,
        //# genemichaels-external: sql-formatter-sqlite
        r#"insert into
             "commit" (idtimestamp, description)
           values
             (
               ${utctime_ms_chrono = *id},
               ${string = description}
             )
           "#;
        db
    ).context("Error executing commit_insert")?;
    Ok(())
}

pub fn commit_get_description(
    db: &mut db::Db<impl good_ormning::runtime::sqlite::SqliteConnection>,
    id: &chrono::DateTime<chrono::Utc>,
) -> Result<Option<String>, loga::Error> {
    Ok(good_ormning::sqlite::good_query_opt!(
        db,
        //# genemichaels-external: sql-formatter-sqlite
        r#"select
             description
           from
             "commit"
           where
             idtimestamp = ${utctime_ms_chrono = *id}
           "#;
        db
    ).context("Error executing commit_get_description")?)
}

pub fn generated_get_mimetype(
    db: &mut db::Db<impl good_ormning::runtime::sqlite::SqliteConnection>,
    node: &crate::interface::triple::DbNode,
    gentype: &str,
) -> Result<Option<String>, loga::Error> {
    Ok(good_ormning::sqlite::good_query_opt!(
        db,
        //# genemichaels-external: sql-formatter-sqlite
        r#"select
             mimetype
           from
             generated
           where
             node = ${node = node}
             and gentype = ${string = gentype}
           "#;
        db
    ).context("Error executing generated_get_mimetype")?)
}

pub fn generated_upsert(
    db: &mut db::Db<impl good_ormning::runtime::sqlite::SqliteConnection>,
    node: &crate::interface::triple::DbNode,
    gentype: &str,
    mimetype: &str,
) -> Result<(), loga::Error> {
    good_ormning::sqlite::good_query!(
        db,
        //# genemichaels-external: sql-formatter-sqlite
        r#"insert into
             generated (node, gentype, mimetype)
           values
             (
               ${node = node},
               ${string = gentype},
               ${string = mimetype}
             )
           on conflict (node, gentype) do update
           set
             mimetype = excluded.mimetype
           "#;
        db
    ).context("Error executing generated_upsert")?;
    Ok(())
}

pub fn meta_filter_existing_nodes(
    db: &mut db::Db<impl good_ormning::runtime::sqlite::SqliteConnection>,
    nodes: Vec<&crate::interface::triple::DbNode>,
) -> Result<Vec<crate::interface::triple::DbNode>, loga::Error> {
    Ok(good_ormning::sqlite::good_query_many!(
        db,
        //# genemichaels-external: sql-formatter-sqlite
        r#"select
             node
           from
             meta
           where
             node in (
               select
                 value
               from
                 rarray (${arr node = nodes})
             )
           "#;
        db
    ).context("Error executing meta_filter_existing_nodes")?)
}

pub fn generated_filter_existing_nodes(
    db: &mut db::Db<impl good_ormning::runtime::sqlite::SqliteConnection>,
    nodes: Vec<&crate::interface::triple::DbNode>,
) -> Result<Vec<crate::interface::triple::DbNode>, loga::Error> {
    Ok(good_ormning::sqlite::good_query_many!(
        db,
        //# genemichaels-external: sql-formatter-sqlite
        r#"select
             node
           from
             generated
           where
             node in (
               select
                 value
               from
                 rarray (${arr node = nodes})
             )
           "#;
        db
    ).context("Error executing generated_filter_existing_nodes")?)
}

pub fn snapshot_filter_nodes_by_end(
    db: &mut db::Db<impl good_ormning::runtime::sqlite::SqliteConnection>,
    col: &str,
    nodes: Vec<&crate::interface::triple::DbNode>,
) -> Result<Vec<crate::interface::triple::DbNode>, loga::Error> {
    let node_strings: Vec<String> = nodes.iter().map(|n| DbNode::to_sql(n)).collect();
    let values =
        std::rc::Rc::new(
            node_strings.iter().map(|s| rusqlite::types::Value::Text(s.clone())).collect::<Vec<_>>(),
        );
    let sql = format!(r#"SELECT DISTINCT so."value" AS "node"
           FROM "triple_snapshot" ts
           JOIN "subjobj" so ON ts."{col}" = so."id"
           WHERE so."value" IN (SELECT value FROM rarray(?1))"#,);
    Ok(db.0.query(&sql, rusqlite::params![values], |row| {
        let node_str: String = row.get(0)?;
        let node =
            DbNode::from_sql(
                node_str,
            ).map_err(
                |e| rusqlite::Error::ToSqlConversionFailure(
                    Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
                ),
            )?;
        Ok(node)
    }).map_err(|e| loga::err(e.to_string()))?)
}

// History query types and functions (manual SQL with JOINs for normalized schema)
pub struct HistoryRow {
    pub subject: DbNode,
    pub predicate: String,
    pub object: DbNode,
    pub commit_: DateTime<Utc>,
    pub exists: bool,
}

fn parse_history_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<HistoryRow> {
    let subject_str: String = row.get(0)?;
    let predicate: String = row.get(1)?;
    let object_str: String = row.get(2)?;
    let commit_ts: GoodOrmningSqliteTimestamp = row.get(3)?;
    let exists: bool = row.get(4)?;
    let subject =
        DbNode::from_sql(
            subject_str,
        ).map_err(
            |e| rusqlite::Error::ToSqlConversionFailure(
                Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
            ),
        )?;
    let object =
        DbNode::from_sql(
            object_str,
        ).map_err(
            |e| rusqlite::Error::ToSqlConversionFailure(
                Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
            ),
        )?;
    let commit_ = match commit_ts {
        GoodOrmningSqliteTimestamp::String(s) => {
            chrono::DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
        },
        GoodOrmningSqliteTimestamp::I64(ms) => {
            chrono::DateTime::from_timestamp_millis(
                ms,
            ).ok_or_else(
                || rusqlite::Error::ToSqlConversionFailure(
                    Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid timestamp millis")),
                ),
            )?
        },
    };
    Ok(HistoryRow {
        subject,
        predicate,
        object,
        commit_,
        exists,
    })
}

const HIST_BASE_SQL: &str = r#"
    SELECT s."value", p."value", o."value", t."commit_", t."exists"
    FROM "triple" t
    JOIN "subjobj" s ON t."subject" = s."id"
    JOIN "predicate" p ON t."predicate" = p."id"
    JOIN "subjobj" o ON t."object" = o."id"
"#;
const HIST_ORDER: &str = r#"
    ORDER BY t."commit_" DESC, s."value" DESC, p."value" DESC, o."value" DESC
    LIMIT 100
"#;
const HIST_AFTER: &str = r#"
    (t."commit_", s."value", p."value", o."value") < (?1, ?2, ?3, ?4)
"#;

fn commit_to_ts(c: &DateTime<Utc>) -> GoodOrmningSqliteTimestamp {
    GoodOrmningSqliteTimestamp::String(c.to_rfc3339())
}

pub fn hist_list_all(db: &mut db::Db<impl SqliteConnection>) -> Result<Vec<HistoryRow>, loga::Error> {
    let sql = format!("{}{}", HIST_BASE_SQL, HIST_ORDER);
    Ok(db.0.query(&sql, [], parse_history_row).map_err(|e| loga::err(e.to_string()))?)
}

pub fn hist_list_all_after(
    db: &mut db::Db<impl SqliteConnection>,
    after_commit: DateTime<Utc>,
    after_subject: &DbNode,
    after_predicate: &str,
    after_object: &DbNode,
) -> Result<Vec<HistoryRow>, loga::Error> {
    let sql = format!("{} WHERE {} {}", HIST_BASE_SQL, HIST_AFTER, HIST_ORDER);
    let ac = commit_to_ts(&after_commit);
    let as_ = DbNode::to_sql(after_subject);
    let ao = DbNode::to_sql(after_object);
    Ok(
        db
            .0
            .query(&sql, rusqlite::params![ac, as_, after_predicate, ao], parse_history_row)
            .map_err(|e| loga::err(e.to_string()))?,
    )
}

pub fn hist_list_by_node(
    db: &mut db::Db<impl SqliteConnection>,
    node: &DbNode,
) -> Result<Vec<HistoryRow>, loga::Error> {
    let sql = format!("{} WHERE (s.\"value\" = ?1 OR o.\"value\" = ?1) {}", HIST_BASE_SQL, HIST_ORDER);
    let n = DbNode::to_sql(node);
    Ok(db.0.query(&sql, rusqlite::params![n], parse_history_row).map_err(|e| loga::err(e.to_string()))?)
}

pub fn hist_list_by_node_after(
    db: &mut db::Db<impl SqliteConnection>,
    after_commit: DateTime<Utc>,
    after_subject: &DbNode,
    after_predicate: &str,
    after_object: &DbNode,
    node: &DbNode,
) -> Result<Vec<HistoryRow>, loga::Error> {
    let sql =
        format!("{} WHERE {} AND (s.\"value\" = ?5 OR o.\"value\" = ?5) {}", HIST_BASE_SQL, HIST_AFTER, HIST_ORDER);
    let ac = commit_to_ts(&after_commit);
    let as_ = DbNode::to_sql(after_subject);
    let ao = DbNode::to_sql(after_object);
    let n = DbNode::to_sql(node);
    Ok(
        db
            .0
            .query(&sql, rusqlite::params![ac, as_, after_predicate, ao, n], parse_history_row)
            .map_err(|e| loga::err(e.to_string()))?,
    )
}

pub fn hist_list_by_subject_predicate(
    db: &mut db::Db<impl SqliteConnection>,
    subject: &DbNode,
    predicate: &str,
) -> Result<Vec<HistoryRow>, loga::Error> {
    let sql = format!("{} WHERE s.\"value\" = ?1 AND p.\"value\" = ?2 {}", HIST_BASE_SQL, HIST_ORDER);
    let s = DbNode::to_sql(subject);
    Ok(
        db.0.query(&sql, rusqlite::params![s, predicate], parse_history_row).map_err(|e| loga::err(e.to_string()))?,
    )
}

pub fn hist_list_by_subject_predicate_after(
    db: &mut db::Db<impl SqliteConnection>,
    after_commit: DateTime<Utc>,
    after_subject: &DbNode,
    after_predicate: &str,
    after_object: &DbNode,
    subject: &DbNode,
    predicate: &str,
) -> Result<Vec<HistoryRow>, loga::Error> {
    let sql =
        format!("{} WHERE {} AND s.\"value\" = ?5 AND p.\"value\" = ?6 {}", HIST_BASE_SQL, HIST_AFTER, HIST_ORDER);
    let ac = commit_to_ts(&after_commit);
    let as_ = DbNode::to_sql(after_subject);
    let ao = DbNode::to_sql(after_object);
    let s = DbNode::to_sql(subject);
    Ok(
        db
            .0
            .query(&sql, rusqlite::params![ac, as_, after_predicate, ao, s, predicate], parse_history_row)
            .map_err(|e| loga::err(e.to_string()))?,
    )
}

pub fn hist_list_by_predicate_object(
    db: &mut db::Db<impl SqliteConnection>,
    predicate: &str,
    object: &DbNode,
) -> Result<Vec<HistoryRow>, loga::Error> {
    let sql = format!("{} WHERE p.\"value\" = ?1 AND o.\"value\" = ?2 {}", HIST_BASE_SQL, HIST_ORDER);
    let o = DbNode::to_sql(object);
    Ok(
        db.0.query(&sql, rusqlite::params![predicate, o], parse_history_row).map_err(|e| loga::err(e.to_string()))?,
    )
}

pub fn hist_list_by_predicate_object_after(
    db: &mut db::Db<impl SqliteConnection>,
    after_commit: DateTime<Utc>,
    after_subject: &DbNode,
    after_predicate: &str,
    after_object: &DbNode,
    predicate: &str,
    object: &DbNode,
) -> Result<Vec<HistoryRow>, loga::Error> {
    let sql =
        format!("{} WHERE {} AND p.\"value\" = ?5 AND o.\"value\" = ?6 {}", HIST_BASE_SQL, HIST_AFTER, HIST_ORDER);
    let ac = commit_to_ts(&after_commit);
    let as_ = DbNode::to_sql(after_subject);
    let ao = DbNode::to_sql(after_object);
    let o = DbNode::to_sql(object);
    Ok(
        db
            .0
            .query(&sql, rusqlite::params![ac, as_, after_predicate, ao, predicate, o], parse_history_row)
            .map_err(|e| loga::err(e.to_string()))?,
    )
}

pub struct SnapshotTriple {
    pub subject: DbNode,
    pub predicate: String,
    pub object: DbNode,
}

pub fn snapshot_file_nodes(
    db: &mut db::Db<impl SqliteConnection>,
    triple_end: &str,
    pivot: Option<&DbNode>,
) -> Result<Vec<DbNode>, loga::Error> {
    let col = match triple_end {
        "subject" => "subject",
        "object" => "object",
        _ => unreachable!(),
    };
    let (sql, params): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = match pivot {
        None => {
            (format!(r#"SELECT DISTINCT so."value" AS "node"
                       FROM "triple_snapshot" ts
                       JOIN "subjobj" so ON ts."{col}" = so."id"
                       WHERE so."value" LIKE 'f=%'
                       ORDER BY so."value"
                       LIMIT 100"#,), vec![])
        },
        Some(pivot) => {
            (format!(r#"SELECT DISTINCT so."value" AS "node"
                       FROM "triple_snapshot" ts
                       JOIN "subjobj" so ON ts."{col}" = so."id"
                       WHERE so."value" LIKE 'f=%'
                         AND so."value" > ?1
                       ORDER BY so."value"
                       LIMIT 100"#,), vec![Box::new(DbNode::to_sql(pivot))])
        },
    };
    Ok(db.0.query(&sql, rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())), |row| {
        let node_str: String = row.get(0)?;
        let node =
            DbNode::from_sql(
                node_str,
            ).map_err(
                |e| rusqlite::Error::ToSqlConversionFailure(
                    Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
                ),
            )?;
        Ok(node)
    }).map_err(|e| loga::err(e.to_string()))?)
}

pub fn snapshot_triples_around(
    db: &mut db::Db<impl SqliteConnection>,
    nodes: Vec<&DbNode>,
) -> Result<Vec<SnapshotTriple>, loga::Error> {
    let node_strings: Vec<String> = nodes.iter().map(|n| DbNode::to_sql(n)).collect();
    let values =
        std::rc::Rc::new(
            node_strings.iter().map(|s| rusqlite::types::Value::Text(s.clone())).collect::<Vec<_>>(),
        );
    let sql = r#"
        SELECT s."value", p."value", o."value"
        FROM "triple_snapshot" ts
        JOIN "subjobj" s ON ts."subject" = s."id"
        JOIN "predicate" p ON ts."predicate" = p."id"
        JOIN "subjobj" o ON ts."object" = o."id"
        WHERE s."value" IN (SELECT value FROM rarray(?1))
           OR o."value" IN (SELECT value FROM rarray(?1))
    "#;
    Ok(db.0.query(sql, rusqlite::params![values], |row| {
        let subject_str: String = row.get(0)?;
        let predicate: String = row.get(1)?;
        let object_str: String = row.get(2)?;
        let subject =
            DbNode::from_sql(
                subject_str,
            ).map_err(
                |e| rusqlite::Error::ToSqlConversionFailure(
                    Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
                ),
            )?;
        let object =
            DbNode::from_sql(
                object_str,
            ).map_err(
                |e| rusqlite::Error::ToSqlConversionFailure(
                    Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
                ),
            )?;
        Ok(SnapshotTriple {
            subject,
            predicate,
            object,
        })
    }).map_err(|e| loga::err(e.to_string()))?)
}

/// Escape a string for use in a SQL LIKE pattern.
pub fn like_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '%' | '_' | '\\' => {
                out.push('\\');
                out.push(c);
            },
            _ => out.push(c),
        }
    }
    out
}

/// Autocomplete predicates matching a pattern where cursor position is a wildcard.
/// prefix and suffix are the text before and after the cursor.
pub fn autocomplete_predicates(
    db: &mut db::Db<impl SqliteConnection>,
    prefix: &str,
    suffix: &str,
) -> Result<Vec<String>, loga::Error> {
    let pattern = format!("{}%{}", like_escape(prefix), like_escape(suffix));
    Ok(good_ormning::sqlite::good_query_many!(
        db,
        //# genemichaels-external: sql-formatter-sqlite
        r#"select distinct
             p."value"
           from
             "predicate" p
             join "triple_snapshot" ts on ts."predicate" = p."id"
           where
             p."value" like ${string = pattern.as_str()} escape '\'
           order by
             p."value" asc
           limit
             20
           "#;
        db
    ).context("Error executing autocomplete_predicates")?)
}
