use {
    crate::server::db::{
        self,
        DbVersions,
    },
    good_ormning::runtime::sqlite::SqliteConnection,
};

pub fn migrate<C: SqliteConnection>(versions: &mut DbVersions<C>) -> Result<(), good_ormning::runtime::GoodError> {
    match versions {
        DbVersions::V0(db) => {
            let data = good_ormning::sqlite::good_query_many!(
                db,
                0,
                //# genemichaels-external: sql-formatter-sqlite
                r#"select
                     subject as s,
                     predicate as p,
                     object as o,
                     commit_ as c,
                     "exists" as e
                   from
                     triple
                   "#;
                db
            )?;
            good_ormning::sqlite::good_query!(
                db,
                0,
                //# genemichaels-external: sql-formatter-sqlite
                r#"delete from triple
                   "#;
                db
            )?;
            for row in data {
                good_ormning::sqlite::good_query!(
                    db,
                    0,
                    //# genemichaels-external: sql-formatter-sqlite
                    r#"insert into
                         triple (subject, predicate, object, commit_, "exists")
                       values
                         (
                           ${node = &row.s},
                           ${string = &row.p},
                           ${node = &row.o},
                           ${utctime_ms_chrono = row.c},
                           ${bool = row.e}
                         )
                       "#;
                    db
                )?;
            }
        },
        DbVersions::V2(db) => {
            good_ormning::sqlite::good_query!(
                db,
                2,
                //# genemichaels-external: sql-formatter-sqlite
                r#"insert or ignore into
                     "subjobj" ("value")
                   select
                     "subject"
                   from
                     "triple"
                   union
                   select
                     "object"
                   from
                     "triple"
                   "#;
                db
            )?;
            good_ormning::sqlite::good_query!(
                db,
                2,
                //# genemichaels-external: sql-formatter-sqlite
                r#"insert or ignore into
                     "predicate" ("value")
                   select
                     "predicate"
                   from
                     "triple"
                   "#;
                db
            )?;
            good_ormning::sqlite::good_query!(
                db,
                2,
                //# genemichaels-external: sql-formatter-sqlite
                r#"insert into
                     "triple2" (
                       "subject",
                       "predicate",
                       "object",
                       "commit_",
                       "exists"
                     )
                   select
                     "subject",
                     "predicate",
                     "object",
                     "commit_",
                     "exists"
                   from
                     "triple"
                   "#;
                db
            )?;
            good_ormning::sqlite::good_query!(
                db,
                2,
                //# genemichaels-external: sql-formatter-sqlite
                r#"insert into
                     "triple_snapshot" ("subject", "predicate", "object", "commit_")
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
                   "#;
                db
            )?;
        },
        _ => { },
    }
    Ok(())
}
