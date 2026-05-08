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
    let subject_str = serde_json_canonicalizer::to_string(subject).unwrap();
    let object_str = serde_json_canonicalizer::to_string(object).unwrap();
    conn.0.execute(r#"insert or ignore into "subjobj" ("value")
           values
             (?)"#, [&subject_str]).context("Error inserting subject into subjobj")?;
    conn.0.execute(r#"insert or ignore into "subjobj" ("value")
           values
             (?)"#, [&object_str]).context("Error inserting object into subjobj")?;
    conn.0.execute(r#"insert or ignore into "predicate" ("value")
           values
             (?)"#, [predicate]).context("Error inserting predicate")?;
    conn
        .0
        .execute(
            r#"insert into "triple2" ("subject", "predicate", "object", "commit_", "exists")
           values
             (?, ?, ?, ?, ?)"#,
            rusqlite::params![subject_str, predicate, object_str, commit_.timestamp_millis(), exist],
        )
        .context("Error inserting triple")?;
    if exist {
        conn
            .0
            .execute(
                r#"insert into "triple_snapshot" ("subject", "predicate", "object", "commit_")
               values
                 (?, ?, ?, ?)
               on conflict("subject", "predicate", "object") do update
               set
                 "commit_" = excluded."commit_""#,
                rusqlite::params![subject_str, predicate, object_str, commit_.timestamp_millis()],
            )
            .context("Error upserting triple snapshot")?;
    } else {
        conn
            .0
            .execute(r#"delete from "triple_snapshot"
               where
                 (
                   "subject" = ?
                   and "predicate" = ?
                   and "object" = ?
                 )"#, rusqlite::params![subject_str, predicate, object_str])
            .context("Error deleting triple snapshot")?;
    }
    return Ok(());
}
