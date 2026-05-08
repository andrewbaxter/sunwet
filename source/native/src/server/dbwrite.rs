use {
    crate::{
        interface::triple::DbNode,
        server::db,
    },
    chrono::{
        DateTime,
        Utc,
    },
    loga::ResultContext,
};

pub fn write_triple<
    C: good_ormning::runtime::sqlite::SqliteConnection,
>(
    conn: &mut db::Db3<C>,
    subject: &DbNode,
    predicate: &str,
    object: &DbNode,
    commit_: DateTime<Utc>,
    exist: bool,
) -> Result<(), loga::Error> {
    db::subjobj_insert(conn, subject, "").context("Error inserting subject into subjobj")?;
    db::subjobj_insert(conn, object, "").context("Error inserting object into subjobj")?;
    db::predicate_insert(conn, predicate).context("Error inserting predicate")?;
    db::triple_insert(conn, subject, predicate, object, commit_, exist).context("Error inserting triple")?;
    if exist {
        db::triple_snapshot_upsert(
            conn,
            subject,
            predicate,
            object,
            commit_,
        ).context("Error upserting triple snapshot")?;
    } else {
        db::triple_snapshot_delete(conn, subject, predicate, object).context("Error deleting triple snapshot")?;
    }
    return Ok(());
}
