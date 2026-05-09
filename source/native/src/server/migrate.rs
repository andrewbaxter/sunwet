use {
    crate::{
        interface::triple::DbNode,
        server::db::DbVersions,
    },
    good_ormning::runtime::sqlite::{
        GoodOrmningCustomString,
        SqliteConnection,
    },
};

pub fn migrate<
    C: SqliteConnection,
>(versions: &mut DbVersions<C>) -> Result<(), good_ormning::runtime::GoodError> {
    match versions {
        DbVersions::V0(db) => {
            let data = db.0.query(
                "select subject, predicate, object, commit_, \"exists\" from triple",
                [],
                |row: &rusqlite::Row| {
                    Ok((
                        row.get::<_, String>(0).unwrap(),
                        row.get::<_, String>(1).unwrap(),
                        row.get::<_, String>(2).unwrap(),
                        row.get::<_, i64>(3).unwrap(),
                        row.get::<_, bool>(4).unwrap(),
                    ))
                }
            ).map_err(|e: rusqlite::Error| good_ormning::runtime::GoodError(e.to_string()))?;
            db.0.execute("delete from triple", []).map_err(|e: rusqlite::Error| good_ormning::runtime::GoodError(e.to_string()))?;
            for row in data {
                db.0.execute(
                    "insert into triple (subject, predicate, object, commit_, \"exists\") values (?, ?, ?, ?, ?)",
                    rusqlite::params![
                        <DbNode as GoodOrmningCustomString<DbNode>>::to_sql(&DbNode::from_sql(row.0).map_err(|e| good_ormning::runtime::GoodError(e))?),
                        row.1,
                        <DbNode as GoodOrmningCustomString<DbNode>>::to_sql(&DbNode::from_sql(row.2).map_err(|e| good_ormning::runtime::GoodError(e))?),
                        row.3,
                        row.4
                    ],
                ).map_err(|e: rusqlite::Error| good_ormning::runtime::GoodError(e.to_string()))?;
            }
        },
        DbVersions::V2(db) => {
            db.0.execute(
                r#"insert or ignore into "subjobj" ("value")
                   select
                     "subject"
                   from
                     "triple"
                   union
                   select
                     "object"
                   from
                     "triple"
                   "#,
                [],
            ).map_err(|e: rusqlite::Error| good_ormning::runtime::GoodError(e.to_string()))?;
            db.0.execute(
                r#"insert or ignore into "predicate" ("value")
                   select
                     "predicate"
                   from
                     "triple"
                   "#,
                [],
            ).map_err(|e: rusqlite::Error| good_ormning::runtime::GoodError(e.to_string()))?;
            db.0.execute(
                r#"insert into "triple2" ("subject", "predicate", "object", "commit_", "exists")
                   select
                     "subject",
                     "predicate",
                     "object",
                     "commit_",
                     "exists"
                   from
                     "triple"
                   "#,
                [],
            ).map_err(|e: rusqlite::Error| good_ormning::runtime::GoodError(e.to_string()))?;
            db.0.execute(
                r#"insert into "triple_snapshot" ("subject", "predicate", "object", "commit_")
                   select
                     "subject",
                     "predicate",
                     "object",
                     "commit_"
                   from
                     "triple" t1
                   where
                     (
                       "commit_" = (
                         select
                           max("commit_")
                         from
                           "triple" t2
                         where
                           (
                             "t1"."subject" = "t2"."subject"
                             and "t1"."predicate" = "t2"."predicate"
                             and "t1"."object" = "t2"."object"
                           )
                       )
                       and "exists" = true
                     )
                   "#,
                [],
            ).map_err(|e: rusqlite::Error| good_ormning::runtime::GoodError(e.to_string()))?;
        },
        _ => { },
    }
    Ok(())
}
