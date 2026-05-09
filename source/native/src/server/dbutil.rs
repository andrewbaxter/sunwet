use {
    crate::server::db,
    crate::dbm,
    deadpool_sqlite::Pool,
    loga::ResultContext,
    rusqlite::Transaction,
};

pub struct ConnWrap<'a>(pub &'a mut rusqlite::Connection);

impl<'a> good_ormning::runtime::sqlite::SqliteConnection for ConnWrap<'a> {
    fn execute(&mut self, query: &str, params: impl rusqlite::Params) -> rusqlite::Result<usize> {
        self.0.execute(query, params)
    }

    fn query<
        T,
        F: FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
    >(&mut self, query: &str, params: impl rusqlite::Params, mut f: F) -> rusqlite::Result<Vec<T>> {
        let mut stmt = self.0.prepare(query)?;
        let rows = stmt.query_map(params, |row| f(row))?;
        let mut res = vec![];
        for row in rows {
            res.push(row?);
        }
        Ok(res)
    }

    fn load_array_module(&mut self) -> rusqlite::Result<()> {
        rusqlite::vtab::array::load_module(self.0)
    }
}

/// Wraps a `&mut Transaction` so it can be used with generated
/// `db::Db3<impl SqliteConnection>` APIs.
pub struct TxnWrap<'a, 'b>(pub &'a mut Transaction<'b>);

impl<'a, 'b> good_ormning::runtime::sqlite::SqliteConnection for TxnWrap<'a, 'b> {
    fn execute(&mut self, query: &str, params: impl rusqlite::Params) -> rusqlite::Result<usize> {
        self.0.execute(query, params)
    }

    fn query<
        T,
        F: FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
    >(&mut self, query: &str, params: impl rusqlite::Params, mut f: F) -> rusqlite::Result<Vec<T>> {
        let mut stmt = self.0.prepare(query)?;
        let rows = stmt.query_map(params, |row| f(row))?;
        let mut res = vec![];
        for row in rows {
            res.push(row?);
        }
        Ok(res)
    }

    fn load_array_module(&mut self) -> rusqlite::Result<()> {
        // Assume loaded on connection
        Ok(())
    }
}

/// Convenience helper to wrap a `&mut Transaction` into a `db::Db3` for use with
/// generated queries.
pub fn db3<'a, 'b>(txn: &'a mut Transaction<'b>) -> crate::server::db::Db3<TxnWrap<'a, 'b>> {
    crate::server::db::Db3(TxnWrap(txn))
}

pub async fn tx<
    O: 'static + Send + Sync,
    F: 'static + Send + FnOnce(&mut Transaction) -> Result<O, loga::Error>,
>(pool: &Pool, cb: F) -> Result<O, loga::Error> {
    let conn = pool.get().await?;
    return Ok(conn.interact(|conn| {
        let mut tx = conn.transaction()?;
        match cb(&mut tx) {
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
    F: 'static + Send + FnOnce(&mut Transaction) -> Result<Txr<O>, loga::Error>,
>(pool: &Pool, cb: F) -> Result<Option<O>, loga::Error> {
    let conn = pool.get().await?;
    return Ok(conn.interact(|conn| {
        let mut tx = conn.transaction()?;
        match cb(&mut tx) {
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
    db: &mut db::Db3<impl good_ormning::runtime::sqlite::SqliteConnection>,
    epoch: chrono::DateTime<chrono::Utc>,
) -> Result<(), loga::Error> {
    good_ormning::sqlite::good_query!(
        //# genemichaels-external: sql-formatter-sqlite
        r#"delete from "triple2"
           where
             (
               "triple2"."commit_" < ${utctime_ms_chrono = epoch}
               and (
                 "triple2"."exists" = false
                 or not exists (
                   select
                     1
                   from
                     "triple_snapshot"
                   where
                     (
                       "triple2"."subject" = "triple_snapshot"."subject"
                       and "triple2"."predicate" = "triple_snapshot"."predicate"
                       and "triple2"."object" = "triple_snapshot"."object"
                       and "triple2"."commit_" = "triple_snapshot"."commit_"
                     )
                 )
               )
             )
           "#;
        db
    ).context("Error executing triple_gc_deleted")?;
    Ok(())
}

pub fn subjobj_gc(
    db: &mut db::Db3<impl good_ormning::runtime::sqlite::SqliteConnection>,
) -> Result<(), loga::Error> {
    good_ormning::sqlite::good_query!(
        //# genemichaels-external: sql-formatter-sqlite
        r#"delete from "subjobj"
           where
             (
               not exists (
                 select
                   1
                 from
                   "triple2"
                 where
                   "subjobj"."value" = "triple2"."subject"
               )
               and not exists (
                 select
                   1
                 from
                   "triple2"
                 where
                   "subjobj"."value" = "triple2"."object"
               )
             )
           "#;
        db
    ).context("Error executing subjobj_gc")?;
    Ok(())
}

pub fn predicate_gc(
    db: &mut db::Db3<impl good_ormning::runtime::sqlite::SqliteConnection>,
) -> Result<(), loga::Error> {
    good_ormning::sqlite::good_query!(
        //# genemichaels-external: sql-formatter-sqlite
        r#"delete from "predicate"
           where
             not exists (
               select
                 1
               from
                 "triple2"
               where
                 "predicate"."value" = "triple2"."predicate"
             )
           "#;
        db
    ).context("Error executing predicate_gc")?;
    Ok(())
}

pub fn meta_gc(db: &mut db::Db3<impl good_ormning::runtime::sqlite::SqliteConnection>) -> Result<(), loga::Error> {
    good_ormning::sqlite::good_query!(
        //# genemichaels-external: sql-formatter-sqlite
        r#"delete from "meta"
           where
             not exists (
               select
                 1
               from
                 "triple2"
               where
                 (
                   "meta"."node" = "triple2"."subject"
                   or "meta"."node" = "triple2"."object"
                 )
             )
           "#;
        db
    ).context("Error executing meta_gc")?;
    Ok(())
}

pub fn commit_gc(db: &mut db::Db3<impl good_ormning::runtime::sqlite::SqliteConnection>) -> Result<(), loga::Error> {
    good_ormning::sqlite::good_query!(
        //# genemichaels-external: sql-formatter-sqlite
        r#"with
             active_commits (stamp) as (
               select distinct
                 "triple2"."commit_"
               from
                 "triple2"
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

pub fn gen_gc(db: &mut db::Db3<impl good_ormning::runtime::sqlite::SqliteConnection>) -> Result<(), loga::Error> {
    good_ormning::sqlite::good_query!(
        //# genemichaels-external: sql-formatter-sqlite
        r#"delete from "generated"
           where
             not exists (
               select
                 1
               from
                 "triple_snapshot"
               where
                 (
                   "generated"."node" = "triple_snapshot"."object"
                   or "generated"."node" = "triple_snapshot"."subject"
                 )
             )
           "#;
        db
    ).context("Error executing gen_gc")?;
    Ok(())
}

pub fn file_access_gc(
    db: &mut db::Db3<impl good_ormning::runtime::sqlite::SqliteConnection>,
    source: &crate::server::access::DbAccessSourceId,
    hash: &i64,
) -> Result<(), loga::Error> {
    let hash = *hash;
    good_ormning::sqlite::good_query!(
        r#"delete from "file_access" where "access_source" = $source and "spec_hash" != $hash"#;
        db,
        source: access_source = source,
        hash: i64 = hash
    ).context("Error executing file_access_gc")?;
    Ok(())
}

pub fn file_access_insert(
    db: &mut db::Db3<impl good_ormning::runtime::sqlite::SqliteConnection>,
    file: &crate::interface::triple::DbFileHash,
    source: &crate::server::access::DbAccessSourceId,
    hash: &i64,
) -> Result<(), loga::Error> {
    let hash = *hash;
    good_ormning::sqlite::good_query!(
        r#"insert or ignore into "file_access" ("file", "access_source", "spec_hash") values ($file, $source, $hash)"#;
        db,
        file: filehash = file,
        source: access_source = source,
        hash: i64 = hash
    ).context("Error executing file_access_insert")?;
    Ok(())
}

pub fn file_access_exists(
    db: &mut db::Db3<impl good_ormning::runtime::sqlite::SqliteConnection>,
    file: &crate::interface::triple::DbFileHash,
    source: &crate::server::access::DbAccessSourceId,
    hash: &i64,
) -> Result<bool, loga::Error> {
    let hash = *hash;
    Ok(
        good_ormning::sqlite::good_query_opt!(
            r#"select 1 as x from file_access where file = $file and access_source = $source and spec_hash = $hash"#;
            db,
            file: filehash = file,
            source: access_source = source,
            hash: i64 = hash
        ).context("Error executing file_access_exists")?.is_some()
    )
}

pub fn file_access_get_sources(
    db: &mut db::Db3<impl good_ormning::runtime::sqlite::SqliteConnection>,
    file: &crate::interface::triple::DbFileHash,
) -> Result<Vec<crate::server::access::DbAccessSourceId>, loga::Error> {
    Ok(
        good_ormning::sqlite::good_query_many!(
            r#"select access_source from file_access where file = $file"#;
            db,
            file: filehash = file
        ).context("Error executing file_access_get_sources")?
    )
}

pub fn meta_get_mimetype(
    db: &mut db::Db3<impl good_ormning::runtime::sqlite::SqliteConnection>,
    node: &crate::interface::triple::DbNode,
) -> Result<Option<Option<String>>, loga::Error> {
    Ok(
        good_ormning::sqlite::good_query_opt!(
            r#"select mimetype from meta where node = $node"#;
            db,
            node: node = node
        ).context("Error executing meta_get_mimetype")?
    )
}


pub fn meta_upsert_mimetype(
    db: &mut db::Db3<impl good_ormning::runtime::sqlite::SqliteConnection>,
    node: &crate::interface::triple::DbNode,
    mimetype: &Option<String>,
) -> Result<(), loga::Error> {
    good_ormning::sqlite::good_query!(
        r#"insert into meta (node, mimetype, fulltext) values ($node, $mimetype, '') on conflict (node) do update set mimetype = excluded.mimetype"#;
        db,
        node: node = node,
        mimetype: opt string = mimetype.as_deref()
    ).context("Error executing meta_upsert_mimetype")?;
    Ok(())
}

pub fn meta_upsert_fulltext(
    db: &mut db::Db3<impl good_ormning::runtime::sqlite::SqliteConnection>,
    node: &crate::interface::triple::DbNode,
    fulltext: &str,
) -> Result<(), loga::Error> {
    good_ormning::sqlite::good_query!(
        r#"insert into meta (node, mimetype, fulltext) values ($node, null, $fulltext) on conflict (node) do update set fulltext = excluded.fulltext"#;
        db,
        node: node = node,
        fulltext: string = fulltext
    ).context("Error executing meta_upsert_fulltext")?;
    Ok(())
}

pub fn triple_snapshot_exists(
    db: &mut db::Db3<impl good_ormning::runtime::sqlite::SqliteConnection>,
    subject: &crate::interface::triple::DbNode,
    predicate: &str,
    object: &crate::interface::triple::DbNode,
) -> Result<bool, loga::Error> {
    Ok(
        good_ormning::sqlite::good_query_opt!(
            r#"select 1 as x from "triple_snapshot" where "subject" = $subject and "predicate" = $predicate and "object" = $object"#;
            db,
            subject: node = subject,
            predicate: string = predicate,
            object: node = object
        ).context("Error executing triple_snapshot_exists")?.is_some()
    )
}


pub fn commit_insert(
    db: &mut db::Db3<impl good_ormning::runtime::sqlite::SqliteConnection>,
    id: &chrono::DateTime<chrono::Utc>,
    description: &str,
) -> Result<(), loga::Error> {
    let id = *id;
    good_ormning::sqlite::good_query!(
        r#"insert into "commit" (idtimestamp, description) values ($id, $description)"#;
        db,
        id: utctime_ms_chrono = id,
        description: string = description
    ).context("Error executing commit_insert")?;
    Ok(())
}

pub fn commit_get_description(
    db: &mut db::Db3<impl good_ormning::runtime::sqlite::SqliteConnection>,
    id: &chrono::DateTime<chrono::Utc>,
) -> Result<Option<String>, loga::Error> {
    let id = *id;
    Ok(
        good_ormning::sqlite::good_query_opt!(
            r#"select description from "commit" where idtimestamp = $id"#;
            db,
            id: utctime_ms_chrono = id
        ).context("Error executing commit_get_description")?
    )
}

pub fn generated_get_mimetype(
    db: &mut db::Db3<impl good_ormning::runtime::sqlite::SqliteConnection>,
    node: &crate::interface::triple::DbNode,
    gentype: &str,
) -> Result<Option<String>, loga::Error> {
    Ok(
        good_ormning::sqlite::good_query_opt!(
            r#"select mimetype from generated where node = $node and gentype = $gentype"#;
            db,
            node: node = node,
            gentype: string = gentype
        ).context("Error executing generated_get_mimetype")?
    )
}

pub fn generated_upsert(
    db: &mut db::Db3<impl good_ormning::runtime::sqlite::SqliteConnection>,
    node: &crate::interface::triple::DbNode,
    gentype: &str,
    mimetype: &str,
) -> Result<(), loga::Error> {
    good_ormning::sqlite::good_query!(
        r#"insert into generated (node, gentype, mimetype) values ($node, $gentype, $mimetype) on conflict (node, gentype) do update set mimetype = excluded.mimetype"#;
        db,
        node: node = node,
        gentype: string = gentype,
        mimetype: string = mimetype
    ).context("Error executing generated_upsert")?;
    Ok(())
}

pub fn meta_filter_existing_nodes(
    db: &mut db::Db3<impl good_ormning::runtime::sqlite::SqliteConnection>,
    nodes: Vec<&crate::interface::triple::DbNode>,
) -> Result<Vec<crate::interface::triple::DbNode>, loga::Error> {
    Ok(
        good_ormning::sqlite::good_query_many!(
            r#"select node from meta where node in (select value from rarray($nodes))"#;
            db,
            nodes: arr node = nodes
        ).context("Error executing meta_filter_existing_nodes")?
    )
}

pub fn generated_filter_existing_nodes(
    db: &mut db::Db3<impl good_ormning::runtime::sqlite::SqliteConnection>,
    nodes: Vec<&crate::interface::triple::DbNode>,
) -> Result<Vec<crate::interface::triple::DbNode>, loga::Error> {
    Ok(
        good_ormning::sqlite::good_query_many!(
            r#"select node from generated where node in (select value from rarray($nodes))"#;
            db,
            nodes: arr node = nodes
        ).context("Error executing generated_filter_existing_nodes")?
    )
}

