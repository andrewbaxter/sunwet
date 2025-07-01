use good_ormning_runtime::GoodError;
use good_ormning_runtime::ToGoodError;

pub fn migrate(db: &mut rusqlite::Connection) -> Result<(), GoodError> {
    rusqlite::vtab::array::load_module(
        &db,
    ).to_good_error(|| "Error loading array extension for array values".to_string())?;
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
                        "create table \"triple\" ( \"predicate\" text not null , \"subject\" text not null , \"commit_\" text not null , \"object\" text not null , \"exists\" integer not null , constraint \"triple_pk\" primary key ( \"subject\" , \"predicate\" , \"object\" , \"commit_\" ) )";
                    txn.execute(query, ()).to_good_error_query(query)?
                };
                {
                    let query =
                        "create index \"triple_index_pred_subj\" on \"triple\" ( \"predicate\" , \"subject\" , \"commit_\" )";
                    txn.execute(query, ()).to_good_error_query(query)?
                };
                {
                    let query =
                        "create unique index \"triple_index_obj_pred_subj\" on \"triple\" ( \"object\" , \"predicate\" , \"subject\" , \"commit_\" )";
                    txn.execute(query, ()).to_good_error_query(query)?
                };
                {
                    let query =
                        "create index \"triple_index_pred_obj\" on \"triple\" ( \"predicate\" , \"object\" , \"commit_\" )";
                    txn.execute(query, ()).to_good_error_query(query)?
                };
                {
                    let query = "create index \"triple_commit_exists\" on \"triple\" ( \"commit_\" , \"exists\" )";
                    txn.execute(query, ()).to_good_error_query(query)?
                };
                {
                    let query =
                        "create table \"file_access\" ( \"spec_hash\" integer not null , \"menu_item_id\" text not null , \"file\" text not null , constraint \"file_access_pk\" primary key ( \"file\" , \"menu_item_id\" , \"spec_hash\" ) )";
                    txn.execute(query, ()).to_good_error_query(query)?
                };
                {
                    let query =
                        "create table \"meta\" ( \"mimetype\" text , \"fulltext\" text not null , \"node\" text not null , constraint \"meta_node\" primary key ( \"node\" ) )";
                    txn.execute(query, ()).to_good_error_query(query)?
                };
                {
                    let query =
                        "create table \"commit\" ( \"idtimestamp\" text not null , \"description\" text not null , constraint \"commit_timestamp\" primary key ( \"idtimestamp\" ) )";
                    txn.execute(query, ()).to_good_error_query(query)?
                };
                {
                    let query =
                        "create table \"generated\" ( \"node\" text not null , \"mimetype\" text not null , \"gentype\" text not null , constraint \"generated_pk\" primary key ( \"node\" , \"gentype\" ) )";
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
    subject: &crate::interface::triple::DbNode,
    predicate: &str,
    object: &crate::interface::triple::DbNode,
    commit_: chrono::DateTime<chrono::Utc>,
    exist: bool,
) -> Result<(), GoodError> {
    let query =
        "insert into \"triple\" ( \"subject\" , \"predicate\" , \"object\" , \"commit_\" , \"exists\" ) values ( $1 , $2 , $3 , $4 , $5 ) on conflict do update set \"exists\" = $5";
    db
        .execute(
            query,
            rusqlite::params![
                <crate::interface::triple::DbNode as good_ormning_runtime
                ::sqlite
                ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::to_sql(
                    &subject,
                ),
                predicate,
                <crate::interface::triple::DbNode as good_ormning_runtime
                ::sqlite
                ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::to_sql(
                    &object,
                ),
                commit_.to_rfc3339(),
                exist
            ],
        )
        .to_good_error_query(query)?;
    Ok(())
}

pub struct DbRes1 {
    pub subject: crate::interface::triple::DbNode,
    pub predicate: String,
    pub object: crate::interface::triple::DbNode,
    pub commit_: chrono::DateTime<chrono::Utc>,
}

pub fn triple_get(
    db: &rusqlite::Connection,
    subject: &crate::interface::triple::DbNode,
    predicate: &str,
    object: &crate::interface::triple::DbNode,
) -> Result<Option<DbRes1>, GoodError> {
    let query =
        "with current0 ( subject , predicate , object , commit_ , exist ) as ( select \"triple\" . \"subject\" , \"triple\" . \"predicate\" , \"triple\" . \"object\" , max ( \"triple\" . \"commit_\" ) as \"commit_\" , \"triple\" . \"exists\" from \"triple\" group by \"triple\" . \"subject\" , \"triple\" . \"predicate\" , \"triple\" . \"object\" ) , current ( subject , predicate , object , commit_ ) as ( select \"current0\" . \"subject\" , \"current0\" . \"predicate\" , \"current0\" . \"object\" , \"current0\" . \"commit_\" from \"current0\" where ( \"current0\" . \"exist\" = true ) ) select \"current\" . \"subject\" , \"current\" . \"predicate\" , \"current\" . \"object\" , \"current\" . \"commit_\" from \"current\" where ( ( \"current\" . \"subject\" = $1 ) and ( \"current\" . \"predicate\" = $2 ) and ( \"current\" . \"object\" = $3 ) ) limit 1 ";
    let mut stmt = db.prepare(query).to_good_error_query(query)?;
    let mut rows =
        stmt
            .query(
                rusqlite::params![
                    <crate::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::to_sql(
                        &subject,
                    ),
                    predicate,
                    <crate::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::to_sql(
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
                    <crate::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::from_sql(
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
                    <crate::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::from_sql(
                        x,
                    ).to_good_error(|| format!("Parsing result {}", 2usize))?;
                x
            },
            commit_: {
                let x: String = r.get(3usize).to_good_error(|| format!("Getting result {}", 3usize))?;
                let x =
                    chrono::DateTime::<chrono::Utc>::from(
                        chrono::DateTime::<chrono::FixedOffset>::parse_from_rfc3339(
                            &x,
                        ).to_good_error(|| format!("Getting result {}", 3usize))?,
                    );
                x
            },
        }));
    }
    Ok(None)
}

pub struct DbResTriple {
    pub subject: crate::interface::triple::DbNode,
    pub predicate: String,
    pub object: crate::interface::triple::DbNode,
    pub exists: bool,
    pub commit_: chrono::DateTime<chrono::Utc>,
}

pub fn hist_list_all(db: &rusqlite::Connection) -> Result<Vec<DbResTriple>, GoodError> {
    let mut out = vec![];
    let query =
        "select \"triple\" . \"subject\" , \"triple\" . \"predicate\" , \"triple\" . \"object\" , \"triple\" . \"exists\" , \"triple\" . \"commit_\" from \"triple\" order by \"triple\" . \"commit_\" desc , \"triple\" . \"exists\" asc , \"triple\" . \"subject\" asc , \"triple\" . \"predicate\" asc , \"triple\" . \"object\" asc limit 500 ";
    let mut stmt = db.prepare(query).to_good_error_query(query)?;
    let mut rows = stmt.query(rusqlite::params![]).to_good_error_query(query)?;
    while let Some(r) = rows.next().to_good_error(|| format!("Getting row in query [{}]", query))? {
        out.push(DbResTriple {
            subject: {
                let x: String = r.get(0usize).to_good_error(|| format!("Getting result {}", 0usize))?;
                let x =
                    <crate::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::from_sql(
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
                    <crate::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::from_sql(
                        x,
                    ).to_good_error(|| format!("Parsing result {}", 2usize))?;
                x
            },
            exists: {
                let x: bool = r.get(3usize).to_good_error(|| format!("Getting result {}", 3usize))?;
                x
            },
            commit_: {
                let x: String = r.get(4usize).to_good_error(|| format!("Getting result {}", 4usize))?;
                let x =
                    chrono::DateTime::<chrono::Utc>::from(
                        chrono::DateTime::<chrono::FixedOffset>::parse_from_rfc3339(
                            &x,
                        ).to_good_error(|| format!("Getting result {}", 4usize))?,
                    );
                x
            },
        });
    }
    Ok(out)
}

pub fn hist_list_all_after(
    db: &rusqlite::Connection,
    time: chrono::DateTime<chrono::Utc>,
    page_subject: &crate::interface::triple::DbNode,
    page_predicate: &str,
    page_object: &crate::interface::triple::DbNode,
) -> Result<Vec<DbResTriple>, GoodError> {
    let mut out = vec![];
    let query =
        "select \"triple\" . \"subject\" , \"triple\" . \"predicate\" , \"triple\" . \"object\" , \"triple\" . \"exists\" , \"triple\" . \"commit_\" from \"triple\" where ( ( \"triple\" . \"commit_\" < $1 ) and ( ( \"triple\" . \"subject\" ,  \"triple\" . \"predicate\" ,  \"triple\" . \"object\" ) > ( $2 ,  $3 ,  $4 ) ) ) order by \"triple\" . \"commit_\" desc , \"triple\" . \"exists\" asc , \"triple\" . \"subject\" asc , \"triple\" . \"predicate\" asc , \"triple\" . \"object\" asc limit 500 ";
    let mut stmt = db.prepare(query).to_good_error_query(query)?;
    let mut rows =
        stmt
            .query(
                rusqlite::params![
                    time.to_rfc3339(),
                    <crate::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::to_sql(
                        &page_subject,
                    ),
                    page_predicate,
                    <crate::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::to_sql(
                        &page_object,
                    )
                ],
            )
            .to_good_error_query(query)?;
    while let Some(r) = rows.next().to_good_error(|| format!("Getting row in query [{}]", query))? {
        out.push(DbResTriple {
            subject: {
                let x: String = r.get(0usize).to_good_error(|| format!("Getting result {}", 0usize))?;
                let x =
                    <crate::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::from_sql(
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
                    <crate::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::from_sql(
                        x,
                    ).to_good_error(|| format!("Parsing result {}", 2usize))?;
                x
            },
            exists: {
                let x: bool = r.get(3usize).to_good_error(|| format!("Getting result {}", 3usize))?;
                x
            },
            commit_: {
                let x: String = r.get(4usize).to_good_error(|| format!("Getting result {}", 4usize))?;
                let x =
                    chrono::DateTime::<chrono::Utc>::from(
                        chrono::DateTime::<chrono::FixedOffset>::parse_from_rfc3339(
                            &x,
                        ).to_good_error(|| format!("Getting result {}", 4usize))?,
                    );
                x
            },
        });
    }
    Ok(out)
}

pub fn hist_list_by_node(
    db: &rusqlite::Connection,
    eq_node: &crate::interface::triple::DbNode,
) -> Result<Vec<DbResTriple>, GoodError> {
    let mut out = vec![];
    let query =
        "select \"triple\" . \"subject\" , \"triple\" . \"predicate\" , \"triple\" . \"object\" , \"triple\" . \"exists\" , \"triple\" . \"commit_\" from \"triple\" where ( ( ( \"triple\" . \"subject\" = $1 ) or ( \"triple\" . \"object\" = $1 ) ) ) order by \"triple\" . \"commit_\" desc , \"triple\" . \"exists\" asc , \"triple\" . \"subject\" asc , \"triple\" . \"predicate\" asc , \"triple\" . \"object\" asc limit 500 ";
    let mut stmt = db.prepare(query).to_good_error_query(query)?;
    let mut rows =
        stmt
            .query(
                rusqlite::params![
                    <crate::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::to_sql(
                        &eq_node,
                    )
                ],
            )
            .to_good_error_query(query)?;
    while let Some(r) = rows.next().to_good_error(|| format!("Getting row in query [{}]", query))? {
        out.push(DbResTriple {
            subject: {
                let x: String = r.get(0usize).to_good_error(|| format!("Getting result {}", 0usize))?;
                let x =
                    <crate::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::from_sql(
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
                    <crate::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::from_sql(
                        x,
                    ).to_good_error(|| format!("Parsing result {}", 2usize))?;
                x
            },
            exists: {
                let x: bool = r.get(3usize).to_good_error(|| format!("Getting result {}", 3usize))?;
                x
            },
            commit_: {
                let x: String = r.get(4usize).to_good_error(|| format!("Getting result {}", 4usize))?;
                let x =
                    chrono::DateTime::<chrono::Utc>::from(
                        chrono::DateTime::<chrono::FixedOffset>::parse_from_rfc3339(
                            &x,
                        ).to_good_error(|| format!("Getting result {}", 4usize))?,
                    );
                x
            },
        });
    }
    Ok(out)
}

pub fn hist_list_by_node_after(
    db: &rusqlite::Connection,
    time: chrono::DateTime<chrono::Utc>,
    page_subject: &crate::interface::triple::DbNode,
    page_predicate: &str,
    page_object: &crate::interface::triple::DbNode,
    eq_node: &crate::interface::triple::DbNode,
) -> Result<Vec<DbResTriple>, GoodError> {
    let mut out = vec![];
    let query =
        "select \"triple\" . \"subject\" , \"triple\" . \"predicate\" , \"triple\" . \"object\" , \"triple\" . \"exists\" , \"triple\" . \"commit_\" from \"triple\" where ( ( \"triple\" . \"commit_\" < $1 ) and ( ( \"triple\" . \"subject\" ,  \"triple\" . \"predicate\" ,  \"triple\" . \"object\" ) > ( $2 ,  $3 ,  $4 ) ) and ( ( \"triple\" . \"subject\" = $5 ) or ( \"triple\" . \"object\" = $5 ) ) ) order by \"triple\" . \"commit_\" desc , \"triple\" . \"exists\" asc , \"triple\" . \"subject\" asc , \"triple\" . \"predicate\" asc , \"triple\" . \"object\" asc limit 500 ";
    let mut stmt = db.prepare(query).to_good_error_query(query)?;
    let mut rows =
        stmt
            .query(
                rusqlite::params![
                    time.to_rfc3339(),
                    <crate::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::to_sql(
                        &page_subject,
                    ),
                    page_predicate,
                    <crate::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::to_sql(
                        &page_object,
                    ),
                    <crate::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::to_sql(
                        &eq_node,
                    )
                ],
            )
            .to_good_error_query(query)?;
    while let Some(r) = rows.next().to_good_error(|| format!("Getting row in query [{}]", query))? {
        out.push(DbResTriple {
            subject: {
                let x: String = r.get(0usize).to_good_error(|| format!("Getting result {}", 0usize))?;
                let x =
                    <crate::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::from_sql(
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
                    <crate::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::from_sql(
                        x,
                    ).to_good_error(|| format!("Parsing result {}", 2usize))?;
                x
            },
            exists: {
                let x: bool = r.get(3usize).to_good_error(|| format!("Getting result {}", 3usize))?;
                x
            },
            commit_: {
                let x: String = r.get(4usize).to_good_error(|| format!("Getting result {}", 4usize))?;
                let x =
                    chrono::DateTime::<chrono::Utc>::from(
                        chrono::DateTime::<chrono::FixedOffset>::parse_from_rfc3339(
                            &x,
                        ).to_good_error(|| format!("Getting result {}", 4usize))?,
                    );
                x
            },
        });
    }
    Ok(out)
}

pub fn hist_list_by_subject_predicate(
    db: &rusqlite::Connection,
    eq_subject: &crate::interface::triple::DbNode,
    eq_predicate: &str,
) -> Result<Vec<DbResTriple>, GoodError> {
    let mut out = vec![];
    let query =
        "select \"triple\" . \"subject\" , \"triple\" . \"predicate\" , \"triple\" . \"object\" , \"triple\" . \"exists\" , \"triple\" . \"commit_\" from \"triple\" where ( ( ( \"triple\" . \"subject\" = $1 ) and ( \"triple\" . \"predicate\" = $2 ) ) ) order by \"triple\" . \"commit_\" desc , \"triple\" . \"exists\" asc , \"triple\" . \"subject\" asc , \"triple\" . \"predicate\" asc , \"triple\" . \"object\" asc limit 500 ";
    let mut stmt = db.prepare(query).to_good_error_query(query)?;
    let mut rows =
        stmt
            .query(
                rusqlite::params![
                    <crate::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::to_sql(
                        &eq_subject,
                    ),
                    eq_predicate
                ],
            )
            .to_good_error_query(query)?;
    while let Some(r) = rows.next().to_good_error(|| format!("Getting row in query [{}]", query))? {
        out.push(DbResTriple {
            subject: {
                let x: String = r.get(0usize).to_good_error(|| format!("Getting result {}", 0usize))?;
                let x =
                    <crate::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::from_sql(
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
                    <crate::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::from_sql(
                        x,
                    ).to_good_error(|| format!("Parsing result {}", 2usize))?;
                x
            },
            exists: {
                let x: bool = r.get(3usize).to_good_error(|| format!("Getting result {}", 3usize))?;
                x
            },
            commit_: {
                let x: String = r.get(4usize).to_good_error(|| format!("Getting result {}", 4usize))?;
                let x =
                    chrono::DateTime::<chrono::Utc>::from(
                        chrono::DateTime::<chrono::FixedOffset>::parse_from_rfc3339(
                            &x,
                        ).to_good_error(|| format!("Getting result {}", 4usize))?,
                    );
                x
            },
        });
    }
    Ok(out)
}

pub fn hist_list_by_subject_predicate_after(
    db: &rusqlite::Connection,
    time: chrono::DateTime<chrono::Utc>,
    page_subject: &crate::interface::triple::DbNode,
    page_predicate: &str,
    page_object: &crate::interface::triple::DbNode,
    eq_subject: &crate::interface::triple::DbNode,
    eq_predicate: &str,
) -> Result<Vec<DbResTriple>, GoodError> {
    let mut out = vec![];
    let query =
        "select \"triple\" . \"subject\" , \"triple\" . \"predicate\" , \"triple\" . \"object\" , \"triple\" . \"exists\" , \"triple\" . \"commit_\" from \"triple\" where ( ( \"triple\" . \"commit_\" < $1 ) and ( ( \"triple\" . \"subject\" ,  \"triple\" . \"predicate\" ,  \"triple\" . \"object\" ) > ( $2 ,  $3 ,  $4 ) ) and ( ( \"triple\" . \"subject\" = $5 ) and ( \"triple\" . \"predicate\" = $6 ) ) ) order by \"triple\" . \"commit_\" desc , \"triple\" . \"exists\" asc , \"triple\" . \"subject\" asc , \"triple\" . \"predicate\" asc , \"triple\" . \"object\" asc limit 500 ";
    let mut stmt = db.prepare(query).to_good_error_query(query)?;
    let mut rows =
        stmt
            .query(
                rusqlite::params![
                    time.to_rfc3339(),
                    <crate::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::to_sql(
                        &page_subject,
                    ),
                    page_predicate,
                    <crate::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::to_sql(
                        &page_object,
                    ),
                    <crate::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::to_sql(
                        &eq_subject,
                    ),
                    eq_predicate
                ],
            )
            .to_good_error_query(query)?;
    while let Some(r) = rows.next().to_good_error(|| format!("Getting row in query [{}]", query))? {
        out.push(DbResTriple {
            subject: {
                let x: String = r.get(0usize).to_good_error(|| format!("Getting result {}", 0usize))?;
                let x =
                    <crate::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::from_sql(
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
                    <crate::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::from_sql(
                        x,
                    ).to_good_error(|| format!("Parsing result {}", 2usize))?;
                x
            },
            exists: {
                let x: bool = r.get(3usize).to_good_error(|| format!("Getting result {}", 3usize))?;
                x
            },
            commit_: {
                let x: String = r.get(4usize).to_good_error(|| format!("Getting result {}", 4usize))?;
                let x =
                    chrono::DateTime::<chrono::Utc>::from(
                        chrono::DateTime::<chrono::FixedOffset>::parse_from_rfc3339(
                            &x,
                        ).to_good_error(|| format!("Getting result {}", 4usize))?,
                    );
                x
            },
        });
    }
    Ok(out)
}

pub fn hist_list_by_predicate_object(
    db: &rusqlite::Connection,
    eq_predicate: &str,
    eq_object: &crate::interface::triple::DbNode,
) -> Result<Vec<DbResTriple>, GoodError> {
    let mut out = vec![];
    let query =
        "select \"triple\" . \"subject\" , \"triple\" . \"predicate\" , \"triple\" . \"object\" , \"triple\" . \"exists\" , \"triple\" . \"commit_\" from \"triple\" where ( ( ( \"triple\" . \"predicate\" = $1 ) and ( \"triple\" . \"object\" = $2 ) ) ) order by \"triple\" . \"commit_\" desc , \"triple\" . \"exists\" asc , \"triple\" . \"subject\" asc , \"triple\" . \"predicate\" asc , \"triple\" . \"object\" asc limit 500 ";
    let mut stmt = db.prepare(query).to_good_error_query(query)?;
    let mut rows =
        stmt
            .query(
                rusqlite::params![
                    eq_predicate,
                    <crate::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::to_sql(
                        &eq_object,
                    )
                ],
            )
            .to_good_error_query(query)?;
    while let Some(r) = rows.next().to_good_error(|| format!("Getting row in query [{}]", query))? {
        out.push(DbResTriple {
            subject: {
                let x: String = r.get(0usize).to_good_error(|| format!("Getting result {}", 0usize))?;
                let x =
                    <crate::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::from_sql(
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
                    <crate::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::from_sql(
                        x,
                    ).to_good_error(|| format!("Parsing result {}", 2usize))?;
                x
            },
            exists: {
                let x: bool = r.get(3usize).to_good_error(|| format!("Getting result {}", 3usize))?;
                x
            },
            commit_: {
                let x: String = r.get(4usize).to_good_error(|| format!("Getting result {}", 4usize))?;
                let x =
                    chrono::DateTime::<chrono::Utc>::from(
                        chrono::DateTime::<chrono::FixedOffset>::parse_from_rfc3339(
                            &x,
                        ).to_good_error(|| format!("Getting result {}", 4usize))?,
                    );
                x
            },
        });
    }
    Ok(out)
}

pub fn hist_list_by_predicate_object_after(
    db: &rusqlite::Connection,
    time: chrono::DateTime<chrono::Utc>,
    page_subject: &crate::interface::triple::DbNode,
    page_predicate: &str,
    page_object: &crate::interface::triple::DbNode,
    eq_predicate: &str,
    eq_object: &crate::interface::triple::DbNode,
) -> Result<Vec<DbResTriple>, GoodError> {
    let mut out = vec![];
    let query =
        "select \"triple\" . \"subject\" , \"triple\" . \"predicate\" , \"triple\" . \"object\" , \"triple\" . \"exists\" , \"triple\" . \"commit_\" from \"triple\" where ( ( \"triple\" . \"commit_\" < $1 ) and ( ( \"triple\" . \"subject\" ,  \"triple\" . \"predicate\" ,  \"triple\" . \"object\" ) > ( $2 ,  $3 ,  $4 ) ) and ( ( \"triple\" . \"predicate\" = $5 ) and ( \"triple\" . \"object\" = $6 ) ) ) order by \"triple\" . \"commit_\" desc , \"triple\" . \"exists\" asc , \"triple\" . \"subject\" asc , \"triple\" . \"predicate\" asc , \"triple\" . \"object\" asc limit 500 ";
    let mut stmt = db.prepare(query).to_good_error_query(query)?;
    let mut rows =
        stmt
            .query(
                rusqlite::params![
                    time.to_rfc3339(),
                    <crate::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::to_sql(
                        &page_subject,
                    ),
                    page_predicate,
                    <crate::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::to_sql(
                        &page_object,
                    ),
                    eq_predicate,
                    <crate::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::to_sql(
                        &eq_object,
                    )
                ],
            )
            .to_good_error_query(query)?;
    while let Some(r) = rows.next().to_good_error(|| format!("Getting row in query [{}]", query))? {
        out.push(DbResTriple {
            subject: {
                let x: String = r.get(0usize).to_good_error(|| format!("Getting result {}", 0usize))?;
                let x =
                    <crate::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::from_sql(
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
                    <crate::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::from_sql(
                        x,
                    ).to_good_error(|| format!("Parsing result {}", 2usize))?;
                x
            },
            exists: {
                let x: bool = r.get(3usize).to_good_error(|| format!("Getting result {}", 3usize))?;
                x
            },
            commit_: {
                let x: String = r.get(4usize).to_good_error(|| format!("Getting result {}", 4usize))?;
                let x =
                    chrono::DateTime::<chrono::Utc>::from(
                        chrono::DateTime::<chrono::FixedOffset>::parse_from_rfc3339(
                            &x,
                        ).to_good_error(|| format!("Getting result {}", 4usize))?,
                    );
                x
            },
        });
    }
    Ok(out)
}

pub struct DbRes3 {
    pub subject: crate::interface::triple::DbNode,
    pub predicate: String,
    pub object: crate::interface::triple::DbNode,
}

pub fn triple_list_around(
    db: &rusqlite::Connection,
    node: &crate::interface::triple::DbNode,
) -> Result<Vec<DbRes3>, GoodError> {
    let mut out = vec![];
    let query =
        "with current0 ( subject , predicate , object , commit_ , exist ) as ( select \"triple\" . \"subject\" , \"triple\" . \"predicate\" , \"triple\" . \"object\" , max ( \"triple\" . \"commit_\" ) as \"commit_\" , \"triple\" . \"exists\" from \"triple\" group by \"triple\" . \"subject\" , \"triple\" . \"predicate\" , \"triple\" . \"object\" ) , current ( subject , predicate , object , commit_ ) as ( select \"current0\" . \"subject\" , \"current0\" . \"predicate\" , \"current0\" . \"object\" , \"current0\" . \"commit_\" from \"current0\" where ( \"current0\" . \"exist\" = true ) ) select \"current\" . \"subject\" , \"current\" . \"predicate\" , \"current\" . \"object\" from \"current\" where ( ( \"current\" . \"subject\" = $1 ) or ( \"current\" . \"object\" = $1 ) ) ";
    let mut stmt = db.prepare(query).to_good_error_query(query)?;
    let mut rows =
        stmt
            .query(
                rusqlite::params![
                    <crate::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::to_sql(
                        &node,
                    )
                ],
            )
            .to_good_error_query(query)?;
    while let Some(r) = rows.next().to_good_error(|| format!("Getting row in query [{}]", query))? {
        out.push(DbRes3 {
            subject: {
                let x: String = r.get(0usize).to_good_error(|| format!("Getting result {}", 0usize))?;
                let x =
                    <crate::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::from_sql(
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
                    <crate::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::from_sql(
                        x,
                    ).to_good_error(|| format!("Parsing result {}", 2usize))?;
                x
            },
        });
    }
    Ok(out)
}

pub fn triple_gc_deleted(db: &rusqlite::Connection, epoch: chrono::DateTime<chrono::Utc>) -> Result<(), GoodError> {
    let query =
        "with current0 ( subject , predicate , object , commit_ , exist ) as ( select \"triple\" . \"subject\" , \"triple\" . \"predicate\" , \"triple\" . \"object\" , max ( \"triple\" . \"commit_\" ) as \"commit_\" , \"triple\" . \"exists\" from \"triple\" group by \"triple\" . \"subject\" , \"triple\" . \"predicate\" , \"triple\" . \"object\" ) , current ( subject , predicate , object , commit_ ) as ( select \"current0\" . \"subject\" , \"current0\" . \"predicate\" , \"current0\" . \"object\" , \"current0\" . \"commit_\" from \"current0\" where ( \"current0\" . \"exist\" = true ) ) delete from \"triple\" where ( ( \"triple\" . \"commit_\" < $1 ) and ( ( \"triple\" . \"exists\" = false ) or not exists ( select 1 as \"x\" from \"current\" where ( ( \"triple\" . \"subject\" = \"current\" . \"subject\" ) and ( \"triple\" . \"predicate\" = \"current\" . \"predicate\" ) and ( \"triple\" . \"object\" = \"current\" . \"object\" ) and ( \"triple\" . \"commit_\" = \"current\" . \"commit_\" ) )  ) ) )";
    db.execute(query, rusqlite::params![epoch.to_rfc3339()]).to_good_error_query(query)?;
    Ok(())
}

pub fn commit_insert(
    db: &rusqlite::Connection,
    stamp: chrono::DateTime<chrono::Utc>,
    desc: &str,
) -> Result<(), GoodError> {
    let query = "insert into \"commit\" ( \"idtimestamp\" , \"description\" ) values ( $1 , $2 )";
    db.execute(query, rusqlite::params![stamp.to_rfc3339(), desc]).to_good_error_query(query)?;
    Ok(())
}

pub struct DbRes4 {
    pub idtimestamp: chrono::DateTime<chrono::Utc>,
    pub description: String,
}

pub fn commit_get(
    db: &rusqlite::Connection,
    stamp: chrono::DateTime<chrono::Utc>,
) -> Result<Option<DbRes4>, GoodError> {
    let query =
        "select \"commit\" . \"idtimestamp\" , \"commit\" . \"description\" from \"commit\" where ( \"commit\" . \"idtimestamp\" = $1 ) ";
    let mut stmt = db.prepare(query).to_good_error_query(query)?;
    let mut rows = stmt.query(rusqlite::params![stamp.to_rfc3339()]).to_good_error_query(query)?;
    let r = rows.next().to_good_error(|| format!("Getting row in query [{}]", query))?;
    if let Some(r) = r {
        return Ok(Some(DbRes4 {
            idtimestamp: {
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
        }));
    }
    Ok(None)
}

pub fn commit_gc(db: &rusqlite::Connection) -> Result<(), GoodError> {
    let query =
        "with active_commits ( stamp ) as ( select distinct \"triple\" . \"commit_\" from \"triple\" ) delete from \"commit\" where not exists ( select 1 as \"x\" from \"active_commits\" where ( \"commit\" . \"idtimestamp\" = \"active_commits\" . \"stamp\" )  )";
    db.execute(query, rusqlite::params![]).to_good_error_query(query)?;
    Ok(())
}

pub fn meta_upsert_file(
    db: &rusqlite::Connection,
    node: &crate::interface::triple::DbNode,
    mimetype: Option<&str>,
) -> Result<(), GoodError> {
    let query =
        "insert into \"meta\" ( \"node\" , \"mimetype\" , \"fulltext\" ) values ( $1 , $2 , '' ) on conflict do update set \"mimetype\" = $2";
    db
        .execute(
            query,
            rusqlite::params![
                <crate::interface::triple::DbNode as good_ormning_runtime
                ::sqlite
                ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::to_sql(
                    &node,
                ),
                mimetype.map(|mimetype| mimetype)
            ],
        )
        .to_good_error_query(query)?;
    Ok(())
}

pub fn meta_upsert_fulltext(
    db: &rusqlite::Connection,
    node: &crate::interface::triple::DbNode,
    fulltext: &str,
) -> Result<(), GoodError> {
    let query =
        "insert into \"meta\" ( \"node\" , \"fulltext\" ) values ( $1 , $2 ) on conflict do update set \"fulltext\" = $2";
    db
        .execute(
            query,
            rusqlite::params![
                <crate::interface::triple::DbNode as good_ormning_runtime
                ::sqlite
                ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::to_sql(
                    &node,
                ),
                fulltext
            ],
        )
        .to_good_error_query(query)?;
    Ok(())
}

pub fn meta_delete(db: &rusqlite::Connection, node: &crate::interface::triple::DbNode) -> Result<(), GoodError> {
    let query = "delete from \"meta\" where ( \"meta\" . \"node\" = $1 )";
    db
        .execute(
            query,
            rusqlite::params![
                <crate::interface::triple::DbNode as good_ormning_runtime
                ::sqlite
                ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::to_sql(
                    &node,
                )
            ],
        )
        .to_good_error_query(query)?;
    Ok(())
}

pub struct Metadata {
    pub mimetype: Option<String>,
    pub fulltext: String,
}

pub fn meta_get(
    db: &rusqlite::Connection,
    node: &crate::interface::triple::DbNode,
) -> Result<Option<Metadata>, GoodError> {
    let query =
        "select \"meta\" . \"mimetype\" , \"meta\" . \"fulltext\" from \"meta\" where ( \"meta\" . \"node\" = $1 ) ";
    let mut stmt = db.prepare(query).to_good_error_query(query)?;
    let mut rows =
        stmt
            .query(
                rusqlite::params![
                    <crate::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::to_sql(
                        &node,
                    )
                ],
            )
            .to_good_error_query(query)?;
    let r = rows.next().to_good_error(|| format!("Getting row in query [{}]", query))?;
    if let Some(r) = r {
        return Ok(Some(Metadata {
            mimetype: {
                let x: Option<String> = r.get(0usize).to_good_error(|| format!("Getting result {}", 0usize))?;
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

pub fn meta_filter_existing(
    db: &rusqlite::Connection,
    nodes: Vec<&crate::interface::triple::DbNode>,
) -> Result<Vec<crate::interface::triple::DbNode>, GoodError> {
    let mut out = vec![];
    let query = "select \"meta\" . \"node\" from \"meta\" where ( \"meta\" . \"node\" in rarray($1) ) ";
    let mut stmt = db.prepare(query).to_good_error_query(query)?;
    let mut rows =
        stmt
            .query(
                rusqlite::params![
                    std::rc::Rc::new(
                        nodes
                            .into_iter()
                            .map(
                                |nodes| rusqlite::types::Value::from(
                                    <crate::interface::triple::DbNode as good_ormning_runtime
                                    ::sqlite
                                    ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::to_sql(
                                        &nodes,
                                    ),
                                ),
                            )
                            .collect::<Vec<_>>(),
                    )
                ],
            )
            .to_good_error_query(query)?;
    while let Some(r) = rows.next().to_good_error(|| format!("Getting row in query [{}]", query))? {
        out.push({
            let x: String = r.get(0usize).to_good_error(|| format!("Getting result {}", 0usize))?;
            let x =
                <crate::interface::triple::DbNode as good_ormning_runtime
                ::sqlite
                ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::from_sql(
                    x,
                ).to_good_error(|| format!("Parsing result {}", 0usize))?;
            x
        });
    }
    Ok(out)
}

pub fn meta_gc(db: &rusqlite::Connection) -> Result<(), GoodError> {
    let query =
        "delete from \"meta\" where not exists ( select 1 as \"x\" from \"triple\" where ( ( \"meta\" . \"node\" = \"triple\" . \"subject\" ) or ( \"meta\" . \"node\" = \"triple\" . \"object\" ) )  )";
    db.execute(query, rusqlite::params![]).to_good_error_query(query)?;
    Ok(())
}

pub fn gen_insert(
    db: &rusqlite::Connection,
    node: &crate::interface::triple::DbNode,
    gentype: &str,
    mimetype: &str,
) -> Result<(), GoodError> {
    let query =
        "insert into \"generated\" ( \"node\" , \"gentype\" , \"mimetype\" ) values ( $1 , $2 , $3 ) on conflict do nothing";
    db
        .execute(
            query,
            rusqlite::params![
                <crate::interface::triple::DbNode as good_ormning_runtime
                ::sqlite
                ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::to_sql(
                    &node,
                ),
                gentype,
                mimetype
            ],
        )
        .to_good_error_query(query)?;
    Ok(())
}

pub fn gen_get(
    db: &rusqlite::Connection,
    node: &crate::interface::triple::DbNode,
    gentype: &str,
) -> Result<Option<String>, GoodError> {
    let query =
        "select \"generated\" . \"mimetype\" from \"generated\" where ( ( \"generated\" . \"node\" = $1 ) and ( \"generated\" . \"gentype\" = $2 ) ) ";
    let mut stmt = db.prepare(query).to_good_error_query(query)?;
    let mut rows =
        stmt
            .query(
                rusqlite::params![
                    <crate::interface::triple::DbNode as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<crate::interface::triple::DbNode>>::to_sql(
                        &node,
                    ),
                    gentype
                ],
            )
            .to_good_error_query(query)?;
    let r = rows.next().to_good_error(|| format!("Getting row in query [{}]", query))?;
    if let Some(r) = r {
        return Ok(Some({
            let x: String = r.get(0usize).to_good_error(|| format!("Getting result {}", 0usize))?;
            x
        }));
    }
    Ok(None)
}

pub fn gen_gc(db: &rusqlite::Connection) -> Result<(), GoodError> {
    let query =
        "delete from \"generated\" where not exists ( select 1 as \"x\" from \"meta\" where ( \"generated\" . \"node\" = \"meta\" . \"node\" )  )";
    db.execute(query, rusqlite::params![]).to_good_error_query(query)?;
    Ok(())
}

pub fn file_access_insert(
    db: &rusqlite::Connection,
    file: &crate::interface::triple::DbFileHash,
    menu_item_id: &str,
    spec_hash: i64,
) -> Result<(), GoodError> {
    let query =
        "insert into \"file_access\" ( \"file\" , \"menu_item_id\" , \"spec_hash\" ) values ( $1 , $2 , $3 ) on conflict do nothing";
    db
        .execute(
            query,
            rusqlite::params![
                <crate::interface::triple::DbFileHash as good_ormning_runtime
                ::sqlite
                ::GoodOrmningCustomString<crate::interface::triple::DbFileHash>>::to_sql(
                    &file,
                ),
                menu_item_id,
                spec_hash
            ],
        )
        .to_good_error_query(query)?;
    Ok(())
}

pub fn file_access_clear_nonversion(
    db: &rusqlite::Connection,
    menu_item_id: &str,
    version_hash: i64,
) -> Result<(), GoodError> {
    let query =
        "delete from \"file_access\" where ( ( \"file_access\" . \"menu_item_id\" = $1 ) and ( \"file_access\" . \"spec_hash\" != $2 ) )";
    db.execute(query, rusqlite::params![menu_item_id, version_hash]).to_good_error_query(query)?;
    Ok(())
}

pub fn file_access_get(
    db: &rusqlite::Connection,
    file: &crate::interface::triple::DbFileHash,
) -> Result<Vec<String>, GoodError> {
    let mut out = vec![];
    let query =
        "select \"file_access\" . \"menu_item_id\" from \"file_access\" where ( \"file_access\" . \"file\" = $1 ) ";
    let mut stmt = db.prepare(query).to_good_error_query(query)?;
    let mut rows =
        stmt
            .query(
                rusqlite::params![
                    <crate::interface::triple::DbFileHash as good_ormning_runtime
                    ::sqlite
                    ::GoodOrmningCustomString<crate::interface::triple::DbFileHash>>::to_sql(
                        &file,
                    )
                ],
            )
            .to_good_error_query(query)?;
    while let Some(r) = rows.next().to_good_error(|| format!("Getting row in query [{}]", query))? {
        out.push({
            let x: String = r.get(0usize).to_good_error(|| format!("Getting result {}", 0usize))?;
            x
        });
    }
    Ok(out)
}
