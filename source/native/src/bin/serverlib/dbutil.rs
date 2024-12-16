use {
    deadpool_sqlite::Pool,
    loga::ResultContext,
    rusqlite::Transaction,
};

pub async fn tx<
    O: 'static + Send + Sync,
    F: 'static + Send + FnOnce(&Transaction) -> Result<O, loga::Error>,
>(pool: &Pool, cb: F) -> Result<O, loga::Error> {
    let conn = pool.get().await?;
    return Ok(conn.interact(|conn| {
        let tx = conn.transaction()?;
        match cb(&tx) {
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
    F: 'static + Send + FnOnce(&Transaction) -> Result<Txr<O>, loga::Error>,
>(pool: &Pool, cb: F) -> Result<Option<O>, loga::Error> {
    let conn = pool.get().await?;
    return Ok(conn.interact(|conn| {
        let tx = conn.transaction()?;
        match cb(&tx) {
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
