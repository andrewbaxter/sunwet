use good_ormning_runtime::GoodError;
use good_ormning_runtime::ToGoodError;

pub fn migrate(db: &mut rusqlite::Connection) -> Result<(), GoodError> {
    {
        let query =
            "create table if not exists __good_version (rid int primary key, version bigint not null, lock int not null);";
        db.execute(query, ()).to_good_error_query(query)?;
    }
    {
        let query = "insert into __good_version (rid, version, lock) values (0, -1, 0) on conflict do nothing;";
        db.execute(query, ()).to_good_error_query(query)?;
    }
    loop {
        let txn = db.transaction().to_good_error(|| "Starting transaction".to_string())?;
        match (|| {
            let query = "update __good_version set lock = 1 where rid = 0 and lock = 0 returning version";
            let mut stmt = txn.prepare(query).to_good_error_query(query)?;
            let mut rows = stmt.query(()).to_good_error_query(query)?;
            let version = match rows.next().to_good_error_query(query)? {
                Some(r) => {
                    let ver: i64 = r.get(0usize).to_good_error_query(query)?;
                    ver
                },
                None => return Ok(false),
            };
            drop(rows);
            stmt.finalize().to_good_error_query(query)?;
            if version > 0i64 {
                return Err(
                    GoodError(
                        format!(
                            "The latest known version is {}, but the schema is at unknown version {}",
                            0i64,
                            version
                        ),
                    ),
                );
            }
            if version < 0i64 {
                {
                    let query =
                        "create table \"triple\" ( \"predicate\" text not null , \"subject\" text not null , \"timestamp\" text not null , \"object\" text not null , \"iam_target\" integer not null , \"exists\" integer not null , constraint \"triple_pk\" primary key ( \"subject\" , \"predicate\" , \"object\" , \"timestamp\" ) )";
                    txn.execute(query, ()).to_good_error_query(query)?
                };
                {
                    let query =
                        "create unique index \"triple_index_pred_subj\" on \"triple\" ( \"predicate\" , \"subject\" , \"timestamp\" )";
                    txn.execute(query, ()).to_good_error_query(query)?
                };
                {
                    let query =
                        "create unique index \"triple_index_obj_pred_subj\" on \"triple\" ( \"object\" , \"predicate\" , \"subject\" , \"timestamp\" )";
                    txn.execute(query, ()).to_good_error_query(query)?
                };
                {
                    let query =
                        "create unique index \"triple_index_pred_obj\" on \"triple\" ( \"predicate\" , \"object\" , \"timestamp\" )";
                    txn.execute(query, ()).to_good_error_query(query)?
                };
                {
                    let query =
                        "create table \"meta\" ( \"mimetype\" text not null , \"fulltext\" text not null , \"node\" text not null , constraint \"meta_node\" primary key ( \"node\" ) )";
                    txn.execute(query, ()).to_good_error_query(query)?
                };
                {
                    let query =
                        "create table \"commit\" ( \"timestamp\" text not null , \"description\" text not null , constraint \"commit_timestamp\" primary key ( \"timestamp\" ) )";
                    txn.execute(query, ()).to_good_error_query(query)?
                };
            }
            let query = "update __good_version set version = $1, lock = 0";
            txn.execute(query, rusqlite::params![0i64]).to_good_error_query(query)?;
            let out: Result<bool, GoodError> = Ok(true);
            out
        })() {
            Err(e) => {
                match txn.rollback() {
                    Err(e1) => {
                        return Err(
                            GoodError(
                                format!("{}\n\nRolling back the transaction due to the above also failed: {}", e, e1),
                            ),
                        );
                    },
                    Ok(_) => {
                        return Err(e);
                    },
                };
            },
            Ok(migrated) => {
                match txn.commit() {
                    Err(e) => {
                        return Err(GoodError(format!("Error committing the migration transaction: {}", e)));
                    },
                    Ok(_) => {
                        if migrated {
                            return Ok(())
                        } else {
                            std::thread::sleep(std::time::Duration::from_millis(5 * 1000));
                        }
                    },
                };
            },
        }
    }
}

pub fn triple_insert(
    db: &rusqlite::Connection,
    subject: &native::interface::triple::DbNode,
    predicate: &str,
    object: &native::interface::triple::DbNode,
    stamp: chrono::DateTime<chrono::Utc>,
    exist: bool,
    iam_target: i64,
) -> Result<(), GoodError> {
    let query =
        "insert into \"triple\" ( \"subject\" , \"predicate\" , \"object\" , \"timestamp\" , \"exists\" , \"iam_target\" ) values ( $1 , $2 , $3 , $4 , $5 , $6 ) on conflict do update set \"exists\" = $5";
    db
        .execute(
            query,
            rusqlite::params![
                <native::interface::triple::DbNode as good_ormning_runtime
                ::sqlite
                ::GoodOrmningCustomString<native::interface::triple::DbNode>>::to_sql(
                    &subject,
                ),
                predicate,
                <native::interface::triple::DbNode as good_ormning_runtime
                ::sqlite
                ::GoodOrmningCustomString<native::interface::triple::DbNode>>::to_sql(
                    &object,
                ),
                stamp.to_rfc3339(),
                exist,
                iam_target
            ],
        )
        .to_good_error_query(query)?;
    Ok(())
}

pub struct DbRes1 {
    pub subject: native::interface::triple::DbNode,
    pub predicate: String,
    pub object: native::interface::triple::DbNode,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub exists: bool,
    pub iam_target: i64,
}

pub fn triple_get(
    db: &rusqlite::Connection,
    subject: &native::interface::triple::DbNode,
    predicate: &str,
    object: &native::interface::triple::DbNode,
) -> Result<Option<DbRes1>, GoodError> {
    let query =
        "select \"triple\" . \"subject\" , \"triple\" . \"predicate\" , \"triple\" . \"object\" , \"triple\" . \"timestamp\" , \"triple\" . \"exists\" , \"triple\" . \"iam_target\" from \"triple\" where ( ( \"triple\" . \"subject\" = $1 ) and ( \"triple\" . \"predicate\" = $2 ) and ( \"triple\" . \"object\" = $3 ) ) order by \"triple\" . \"timestamp\" desc limit 1 ";
    let mut stmt = db.prepare(query).to_good_error_query(query)?;
    let mut rows =
        stmt
            .query(
                rusqlite::params![
                    <native::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<native::interface::triple::DbNode>>::to_sql(
                        &subject,
                    ),
                    predicate,
                    <native::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<native::interface::triple::DbNode>>::to_sql(
                        &object,
                    )
                ],
            )
            .to_good_error_query(query)?;
    let r = rows.next().to_good_error(|| format!("Getting row in query [{}]", query))?;
    if let Some(r) = r {
        return Ok(Some(DbRes1 {
            subject: {
                let x: String = r.get(0usize).to_good_error(|| format!("Getting result {}", 0usize))?;
                let x =
                    <native::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<native::interface::triple::DbNode>>::from_sql(
                        x,
                    ).to_good_error(|| format!("Parsing result {}", 0usize))?;
                x
            },
            predicate: {
                let x: String = r.get(1usize).to_good_error(|| format!("Getting result {}", 1usize))?;
                x
            },
            object: {
                let x: String = r.get(2usize).to_good_error(|| format!("Getting result {}", 2usize))?;
                let x =
                    <native::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<native::interface::triple::DbNode>>::from_sql(
                        x,
                    ).to_good_error(|| format!("Parsing result {}", 2usize))?;
                x
            },
            timestamp: {
                let x: String = r.get(3usize).to_good_error(|| format!("Getting result {}", 3usize))?;
                let x =
                    chrono::DateTime::<chrono::Utc>::from(
                        chrono::DateTime::<chrono::FixedOffset>::parse_from_rfc3339(
                            &x,
                        ).to_good_error(|| format!("Getting result {}", 3usize))?,
                    );
                x
            },
            exists: {
                let x: bool = r.get(4usize).to_good_error(|| format!("Getting result {}", 4usize))?;
                x
            },
            iam_target: {
                let x: i64 = r.get(5usize).to_good_error(|| format!("Getting result {}", 5usize))?;
                x
            },
        }));
    }
    Ok(None)
}

pub fn triple_list_all(db: &rusqlite::Connection) -> Result<Vec<DbRes1>, GoodError> {
    let mut out = vec![];
    let query =
        "select \"triple\" . \"subject\" , \"triple\" . \"predicate\" , \"triple\" . \"object\" , \"triple\" . \"timestamp\" , \"triple\" . \"exists\" , \"triple\" . \"iam_target\" from \"triple\" ";
    let mut stmt = db.prepare(query).to_good_error_query(query)?;
    let mut rows = stmt.query(rusqlite::params![]).to_good_error_query(query)?;
    while let Some(r) = rows.next().to_good_error(|| format!("Getting row in query [{}]", query))? {
        out.push(DbRes1 {
            subject: {
                let x: String = r.get(0usize).to_good_error(|| format!("Getting result {}", 0usize))?;
                let x =
                    <native::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<native::interface::triple::DbNode>>::from_sql(
                        x,
                    ).to_good_error(|| format!("Parsing result {}", 0usize))?;
                x
            },
            predicate: {
                let x: String = r.get(1usize).to_good_error(|| format!("Getting result {}", 1usize))?;
                x
            },
            object: {
                let x: String = r.get(2usize).to_good_error(|| format!("Getting result {}", 2usize))?;
                let x =
                    <native::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<native::interface::triple::DbNode>>::from_sql(
                        x,
                    ).to_good_error(|| format!("Parsing result {}", 2usize))?;
                x
            },
            timestamp: {
                let x: String = r.get(3usize).to_good_error(|| format!("Getting result {}", 3usize))?;
                let x =
                    chrono::DateTime::<chrono::Utc>::from(
                        chrono::DateTime::<chrono::FixedOffset>::parse_from_rfc3339(
                            &x,
                        ).to_good_error(|| format!("Getting result {}", 3usize))?,
                    );
                x
            },
            exists: {
                let x: bool = r.get(4usize).to_good_error(|| format!("Getting result {}", 4usize))?;
                x
            },
            iam_target: {
                let x: i64 = r.get(5usize).to_good_error(|| format!("Getting result {}", 5usize))?;
                x
            },
        });
    }
    Ok(out)
}

pub fn triple_list_between(
    db: &rusqlite::Connection,
    start_incl: chrono::DateTime<chrono::Utc>,
    end_excl: chrono::DateTime<chrono::Utc>,
) -> Result<Vec<DbRes1>, GoodError> {
    let mut out = vec![];
    let query =
        "select \"triple\" . \"subject\" , \"triple\" . \"predicate\" , \"triple\" . \"object\" , \"triple\" . \"timestamp\" , \"triple\" . \"exists\" , \"triple\" . \"iam_target\" from \"triple\" where ( ( \"triple\" . \"timestamp\" >= $1 ) and ( \"triple\" . \"timestamp\" < $2 ) ) ";
    let mut stmt = db.prepare(query).to_good_error_query(query)?;
    let mut rows =
        stmt.query(rusqlite::params![start_incl.to_rfc3339(), end_excl.to_rfc3339()]).to_good_error_query(query)?;
    while let Some(r) = rows.next().to_good_error(|| format!("Getting row in query [{}]", query))? {
        out.push(DbRes1 {
            subject: {
                let x: String = r.get(0usize).to_good_error(|| format!("Getting result {}", 0usize))?;
                let x =
                    <native::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<native::interface::triple::DbNode>>::from_sql(
                        x,
                    ).to_good_error(|| format!("Parsing result {}", 0usize))?;
                x
            },
            predicate: {
                let x: String = r.get(1usize).to_good_error(|| format!("Getting result {}", 1usize))?;
                x
            },
            object: {
                let x: String = r.get(2usize).to_good_error(|| format!("Getting result {}", 2usize))?;
                let x =
                    <native::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<native::interface::triple::DbNode>>::from_sql(
                        x,
                    ).to_good_error(|| format!("Parsing result {}", 2usize))?;
                x
            },
            timestamp: {
                let x: String = r.get(3usize).to_good_error(|| format!("Getting result {}", 3usize))?;
                let x =
                    chrono::DateTime::<chrono::Utc>::from(
                        chrono::DateTime::<chrono::FixedOffset>::parse_from_rfc3339(
                            &x,
                        ).to_good_error(|| format!("Getting result {}", 3usize))?,
                    );
                x
            },
            exists: {
                let x: bool = r.get(4usize).to_good_error(|| format!("Getting result {}", 4usize))?;
                x
            },
            iam_target: {
                let x: i64 = r.get(5usize).to_good_error(|| format!("Getting result {}", 5usize))?;
                x
            },
        });
    }
    Ok(out)
}

pub fn triple_gc_deleted(db: &rusqlite::Connection, epoch: chrono::DateTime<chrono::Utc>) -> Result<(), GoodError> {
    let query =
        "with current ( subject , predicate , object , event_stamp ) as ( select \"triple\" . \"subject\" , \"triple\" . \"predicate\" , \"triple\" . \"object\" , max ( \"triple\" . \"timestamp\" ) as \"timestamp\" from \"triple\" group by \"triple\" . \"subject\" , \"triple\" . \"predicate\" , \"triple\" . \"object\" ) delete from \"triple\" where ( ( \"triple\" . \"timestamp\" < $1 ) and ( ( \"triple\" . \"exists\" = false ) or not exists ( select 1 as \"x\" from \"current\" where ( ( \"triple\" . \"subject\" = \"current\" . \"subject\" ) and ( \"triple\" . \"predicate\" = \"current\" . \"predicate\" ) and ( \"triple\" . \"object\" = \"current\" . \"object\" ) and ( \"triple\" . \"timestamp\" = \"current\" . \"event_stamp\" ) )  ) ) )";
    db.execute(query, rusqlite::params![epoch.to_rfc3339()]).to_good_error_query(query)?;
    Ok(())
}

pub fn commit_insert(
    db: &rusqlite::Connection,
    stamp: chrono::DateTime<chrono::Utc>,
    desc: &str,
) -> Result<(), GoodError> {
    let query = "insert into \"commit\" ( \"timestamp\" , \"description\" ) values ( $1 , $2 )";
    db.execute(query, rusqlite::params![stamp.to_rfc3339(), desc]).to_good_error_query(query)?;
    Ok(())
}

pub struct DbRes2 {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub description: String,
}

pub fn commit_list_between(
    db: &rusqlite::Connection,
    start_incl: chrono::DateTime<chrono::Utc>,
    end_excl: chrono::DateTime<chrono::Utc>,
) -> Result<Vec<DbRes2>, GoodError> {
    let mut out = vec![];
    let query =
        "select \"commit\" . \"timestamp\" , \"commit\" . \"description\" from \"commit\" where ( ( \"commit\" . \"timestamp\" >= $1 ) and ( \"commit\" . \"timestamp\" < $2 ) ) ";
    let mut stmt = db.prepare(query).to_good_error_query(query)?;
    let mut rows =
        stmt.query(rusqlite::params![start_incl.to_rfc3339(), end_excl.to_rfc3339()]).to_good_error_query(query)?;
    while let Some(r) = rows.next().to_good_error(|| format!("Getting row in query [{}]", query))? {
        out.push(DbRes2 {
            timestamp: {
                let x: String = r.get(0usize).to_good_error(|| format!("Getting result {}", 0usize))?;
                let x =
                    chrono::DateTime::<chrono::Utc>::from(
                        chrono::DateTime::<chrono::FixedOffset>::parse_from_rfc3339(
                            &x,
                        ).to_good_error(|| format!("Getting result {}", 0usize))?,
                    );
                x
            },
            description: {
                let x: String = r.get(1usize).to_good_error(|| format!("Getting result {}", 1usize))?;
                x
            },
        });
    }
    Ok(out)
}

pub fn commit_gc(db: &rusqlite::Connection) -> Result<(), GoodError> {
    let query =
        "with active_commits ( stamp ) as ( select distinct \"triple\" . \"timestamp\" from \"triple\" ) delete from \"commit\" where not exists ( select 1 as \"x\" from \"active_commits\" where ( \"commit\" . \"timestamp\" = \"active_commits\" . \"stamp\" )  )";
    db.execute(query, rusqlite::params![]).to_good_error_query(query)?;
    Ok(())
}

pub fn meta_insert(
    db: &rusqlite::Connection,
    node: &native::interface::triple::DbNode,
    mimetype: &str,
    fulltext: &str,
) -> Result<(), GoodError> {
    let query =
        "insert into \"meta\" ( \"node\" , \"mimetype\" , \"fulltext\" ) values ( $1 , $2 , $3 ) on conflict do nothing";
    db
        .execute(
            query,
            rusqlite::params![
                <native::interface::triple::DbNode as good_ormning_runtime
                ::sqlite
                ::GoodOrmningCustomString<native::interface::triple::DbNode>>::to_sql(
                    &node,
                ),
                mimetype,
                fulltext
            ],
        )
        .to_good_error_query(query)?;
    Ok(())
}

pub fn meta_delete(db: &rusqlite::Connection, node: &native::interface::triple::DbNode) -> Result<(), GoodError> {
    let query = "delete from \"meta\" where ( \"meta\" . \"node\" = $1 )";
    db
        .execute(
            query,
            rusqlite::params![
                <native::interface::triple::DbNode as good_ormning_runtime
                ::sqlite
                ::GoodOrmningCustomString<native::interface::triple::DbNode>>::to_sql(
                    &node,
                )
            ],
        )
        .to_good_error_query(query)?;
    Ok(())
}

pub struct DbRes3 {
    pub mimetype: String,
    pub fulltext: String,
}

pub fn meta_get(
    db: &rusqlite::Connection,
    node: &native::interface::triple::DbNode,
) -> Result<Option<DbRes3>, GoodError> {
    let query =
        "select \"meta\" . \"mimetype\" , \"meta\" . \"fulltext\" from \"meta\" where ( \"meta\" . \"node\" = $1 ) ";
    let mut stmt = db.prepare(query).to_good_error_query(query)?;
    let mut rows =
        stmt
            .query(
                rusqlite::params![
                    <native::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<native::interface::triple::DbNode>>::to_sql(
                        &node,
                    )
                ],
            )
            .to_good_error_query(query)?;
    let r = rows.next().to_good_error(|| format!("Getting row in query [{}]", query))?;
    if let Some(r) = r {
        return Ok(Some(DbRes3 {
            mimetype: {
                let x: String = r.get(0usize).to_good_error(|| format!("Getting result {}", 0usize))?;
                x
            },
            fulltext: {
                let x: String = r.get(1usize).to_good_error(|| format!("Getting result {}", 1usize))?;
                x
            },
        }));
    }
    Ok(None)
}

pub fn meta_gc(db: &rusqlite::Connection) -> Result<(), GoodError> {
    let query =
        "delete from \"meta\" where not exists ( select 1 as \"x\" from \"triple\" where ( ( \"meta\" . \"node\" = \"triple\" . \"subject\" ) or ( \"meta\" . \"node\" = \"triple\" . \"object\" ) )  )";
    db.execute(query, rusqlite::params![]).to_good_error_query(query)?;
    Ok(())
}
