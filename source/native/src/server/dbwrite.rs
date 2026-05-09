use {
    crate::{
        server::db,
        interface::triple::DbNode,
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
    good_ormning::sqlite::good_query!(
        db,
        "",
        3,
        r#"insert or ignore into "subjobj" ("value")
           values
             ($node)"#;
        conn,
        node: node = subject
    ).context("Error inserting subject into subjobj")?;
    good_ormning::sqlite::good_query!(
        db,
        "",
        3,
        r#"insert or ignore into "subjobj" ("value")
           values
             ($node)"#;
        conn,
        node: node = object
    ).context("Error inserting object into subjobj")?;
    good_ormning::sqlite::good_query!(
        db,
        "",
        3,
        r#"insert or ignore into "predicate" ("value")
           values
             ($value)"#;
        conn,
        value: string = predicate
    ).context("Error inserting predicate")?;
    good_ormning::sqlite::good_query!(
        db,
        "",
        3,
        r#"insert into "triple2" ("subject", "predicate", "object", "commit_", "exists")
           values
             (
               $subject,
               $predicate,
               $object,
               $commit_,
               $exist
             )"#;
        conn,
        subject: node = subject,
        predicate: string = predicate,
        object: node = object,
        commit_: utctime_ms_chrono = commit_,
        exist: bool = exist
    ).context("Error inserting triple")?;
    if exist {
        good_ormning::sqlite::good_query!(
            db,
            "",
            3,
            r#"insert into "triple_snapshot" ("subject", "predicate", "object", "commit_")
               values
                 (
                   $subject,
                   $predicate,
                   $object,
                   $commit_
                 )
               on conflict ( "subject" , "predicate" , "object" )
               do update
               set
                 "commit_" = excluded."commit_""#;
            conn,
            subject: node = subject,
            predicate: string = predicate,
            object: node = object,
            commit_: utctime_ms_chrono = commit_
        ).context("Error upserting triple snapshot")?;
    } else {
        good_ormning::sqlite::good_query!(
            db,
            "",
            3,
            r#"delete from "triple_snapshot"
               where
                 (
                   "subject" = $subject
                   and "predicate" = $predicate
                   and "object" = $object
                 )"#;
            conn,
            subject: node = subject,
            predicate: string = predicate,
            object: node = object
        ).context("Error deleting triple snapshot")?;
    }
    return Ok(());
}
