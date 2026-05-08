use {
    crate::server::dbutil::ConnWrap,
    loga::ResultContext,
};

pub fn migrate<
    C: good_ormning::runtime::sqlite::SqliteConnection,
>(_versions: &mut crate::server::db::DbVersions<C>) -> Result<(), good_ormning::runtime::GoodError> {
    let version = _versions.version;
    let db = &mut _versions.db;
    if version == 2 {
        db.execute(r#"insert or ignore into "subjobj" ("value")
               select
                 "subject"
               from
                 "triple"
               union
               select
                 "object"
               from
                 "triple"
               "#, []).map_err(|e| good_ormning::runtime::GoodError(e.to_string()))?;
        db.execute(r#"insert or ignore into "predicate" ("value")
               select
                 "predicate"
               from
                 "triple"
               "#, []).map_err(|e| good_ormning::runtime::GoodError(e.to_string()))?;
        db.execute(r#"insert into "triple2" ("subject", "predicate", "object", "commit_", "exists")
               select
                 "subject",
                 "predicate",
                 "object",
                 "commit_",
                 "exists"
               from
                 "triple"
               "#, []).map_err(|e| good_ormning::runtime::GoodError(e.to_string()))?;
        db.execute(r#"insert into "triple_snapshot" ("subject", "predicate", "object", "commit_")
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
               "#, []).map_err(|e| good_ormning::runtime::GoodError(e.to_string()))?;
    }
    Ok(())
}
