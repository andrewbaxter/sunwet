use {
    crate::server::db,
    crate::server::db as dbm,
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
