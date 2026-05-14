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
            // 1. Populate subjobj from old triple's subjects and objects
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

            // 2. Populate predicate from old triple's predicates
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

            // 3. Copy triple data into triple2 with integer references
            // (good_query! doesn't support JOINs, so use raw SQL)
            db.0.execute(
                r#"INSERT INTO "triple2" ("subject", "predicate", "object", "commit_", "exists")
                   SELECT s."id", p."id", o."id", t."commit_", t."exists"
                   FROM "triple" t
                   JOIN "subjobj" s ON t."subject" = s."value"
                   JOIN "predicate" p ON t."predicate" = p."value"
                   JOIN "subjobj" o ON t."object" = o."value""#,
                [],
            )?;

            // 4. Populate triple_snapshot with integer references (latest existing state)
            db.0.execute(
                r#"INSERT INTO "triple_snapshot" ("subject", "predicate", "object", "commit_")
                   SELECT s."id", p."id", o."id", t1."commit_"
                   FROM "triple" t1
                   JOIN "subjobj" s ON t1."subject" = s."value"
                   JOIN "predicate" p ON t1."predicate" = p."value"
                   JOIN "subjobj" o ON t1."object" = o."value"
                   WHERE t1."commit_" = (
                       SELECT MAX(t2."commit_")
                       FROM "triple" t2
                       WHERE t1."subject" = t2."subject"
                         AND t1."predicate" = t2."predicate"
                         AND t1."object" = t2."object"
                   )
                   AND t1."exists" = true"#,
                [],
            )?;
        },
        _ => { },
    }
    Ok(())
}
