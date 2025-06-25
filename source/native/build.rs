use {
    good_ormning::sqlite::{
        new_delete,
        new_insert,
        new_select,
        new_select_body,
        query::{
            expr::{
                BinOp,
                Binding,
                Expr,
            },
            helpers::{
                expr_and,
                expr_field_eq,
                expr_field_lt,
                fn_max,
                set_field,
            },
            insert::InsertConflict,
            select_body::{
                Order,
            },
            utils::{
                CteBuilder,
                With,
            },
        },
        schema::{
            constraint::{
                ConstraintType::PrimaryKey,
                PrimaryKeyDef,
            },
            field::{
                field_bool,
                field_i64,
                field_str,
                field_utctime_ms,
                FieldType,
            },
        },
        types::type_str,
        QueryResCount,
        Version,
    },
    std::{
        env,
        path::PathBuf,
    },
};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let root = PathBuf::from(&env::var("CARGO_MANIFEST_DIR").unwrap());
    let mut latest_version = Version::default();
    let mut queries = vec![];
    let node_type = type_str().custom("crate::interface::triple::DbNode").build();
    let node_array_type = type_str().custom("crate::interface::triple::DbNode").array().build();
    let filehash_type = type_str().custom("crate::interface::triple::DbFileHash").build();
    let file_access_type = type_str().build();

    // Triple
    let triple_table;
    let triple_commit;
    let triple_subject;
    let triple_object;
    {
        let t = latest_version.table("zQLEK3CT0", "triple");
        let subject = t.field(&mut latest_version, "zLQI9HQUQ", "subject", FieldType::with(&node_type));
        let predicate = t.field(&mut latest_version, "zSZVNBP0E", "predicate", field_str().build());
        let object = t.field(&mut latest_version, "zII52SWQB", "object", FieldType::with(&node_type));
        let commit = t.field(&mut latest_version, "zK21ECBE5", "commit_", field_utctime_ms().build());
        let exist = t.field(&mut latest_version, "z0ZOJM2UT", "exists", field_bool().build());
        t.constraint(
            &mut latest_version,
            "z1T10QI43",
            "triple_pk",
            PrimaryKey(
                PrimaryKeyDef { fields: vec![subject.clone(), predicate.clone(), object.clone(), commit.clone()] },
            ),
        );
        t
            .index("zXIMPRLIR", "triple_index_obj_pred_subj", &[&object, &predicate, &subject, &commit])
            .unique()
            .build(&mut latest_version);
        t.index("zBZVX51AR", "triple_index_pred_subj", &[&predicate, &subject, &commit]).build(&mut latest_version);
        t.index("zTVLKA6GQ", "triple_index_pred_obj", &[&predicate, &object, &commit]).build(&mut latest_version);
        t.index("woeehiw2a9lszj", "triple_commit_exists", &[&commit, &exist]).build(&mut latest_version);
        queries.push(
            new_insert(
                &t,
                vec![
                    set_field("subject", &subject),
                    set_field("predicate", &predicate),
                    set_field("object", &object),
                    set_field("commit_", &commit),
                    set_field("exist", &exist)
                ],
            )
                .on_conflict(InsertConflict::DoUpdate(vec![set_field("exist", &exist)]))
                .build_query("triple_insert", QueryResCount::None),
        );
        queries.push(
            new_select(&t)
                .return_field(&subject)
                .return_field(&predicate)
                .return_field(&object)
                .return_field(&commit)
                .return_field(&exist)
                .where_(
                    expr_and(
                        vec![
                            expr_field_eq("subject", &subject),
                            expr_field_eq("predicate", &predicate),
                            expr_field_eq("object", &object)
                        ],
                    ),
                )
                .limit(Expr::LitI32(1))
                .order(Expr::field(&commit), Order::Desc)
                .build_query("triple_get", QueryResCount::MaybeOne),
        );
        for (name0, where0) in [
            ("all", None),
            ("by_node", Some(Expr::BinOpChain {
                op: BinOp::Or,
                exprs: vec![expr_field_eq("eq_node", &subject), expr_field_eq("eq_node", &object)],
            })),
            (
                "by_subject_predicate",
                Some(
                    expr_and(vec![expr_field_eq("eq_subject", &subject), expr_field_eq("eq_predicate", &predicate)]),
                ),
            ),
            (
                "by_predicate_object",
                Some(
                    expr_and(vec![expr_field_eq("eq_predicate", &predicate), expr_field_eq("eq_object", &object)]),
                ),
            ),
        ] {
            let mut where_exprs = vec![expr_field_lt("time", &commit), Expr::BinOp {
                left: Box::new(Expr::LitArray(vec![
                    //. .
                    Expr::field(&subject),
                    Expr::field(&predicate),
                    Expr::field(&object)
                ])),
                op: BinOp::GreaterThan,
                right: Box::new(Expr::LitArray(vec![
                    //. .
                    Expr::Param {
                        name: format!("page_subject"),
                        type_: subject.type_.type_.clone(),
                    },
                    Expr::Param {
                        name: format!("page_predicate"),
                        type_: predicate.type_.type_.clone(),
                    },
                    Expr::Param {
                        name: format!("page_object"),
                        type_: object.type_.type_.clone(),
                    }
                ])),
            }];
            if let Some(where_) = where0 {
                where_exprs.push(where_);
            }
            queries.push(
                new_select(&t)
                    .return_field(&subject)
                    .return_field(&predicate)
                    .return_field(&object)
                    .return_field(&commit)
                    .return_field(&exist)
                    .where_(expr_and(where_exprs))
                    .order(Expr::field(&commit), Order::Desc)
                    .order(Expr::field(&exist), Order::Asc)
                    .order(Expr::field(&subject), Order::Asc)
                    .order(Expr::field(&predicate), Order::Asc)
                    .order(Expr::field(&object), Order::Asc)
                    .limit(Expr::LitI32(500))
                    .build_query_named_res(&format!("triple_list_{}", name0), QueryResCount::Many, "DbResTriple"),
            );
        }
        queries.push(
            new_select(&t)
                .return_field(&subject)
                .return_field(&predicate)
                .return_field(&object)
                .where_(expr_and(vec![Expr::BinOpChain {
                    op: BinOp::Or,
                    exprs: vec![expr_field_eq("node", &subject), expr_field_eq("node", &object)],
                }, Expr::BinOp {
                    left: Box::new(Expr::field(&exist)),
                    op: BinOp::Equals,
                    right: Box::new(Expr::LitBool(true)),
                }]))
                .build_query("triple_list_exist_around", QueryResCount::Many),
        );
        queries.push({
            let mut current =
                CteBuilder::new(
                    "current",
                    new_select_body(&t)
                        .group(vec![Expr::field(&subject), Expr::field(&predicate), Expr::field(&object)])
                        .return_field(&subject)
                        .return_field(&predicate)
                        .return_field(&object)
                        .return_named("commit_", fn_max(Expr::field(&commit)))
                        .build(),
                );
            let current_subject = current.field("subject", subject.type_.type_.clone());
            let current_predicate = current.field("predicate", predicate.type_.type_.clone());
            let current_object = current.field("object", object.type_.type_.clone());
            let current_commit = current.field("commit_", commit.type_.type_.clone());
            let (current_table, current_cte) = current.build();
            new_delete(&t).with(With {
                recursive: false,
                ctes: vec![current_cte],
            }).where_(expr_and(vec![
                //. .
                expr_field_lt("epoch", &commit),
                Expr::BinOp {
                    left: Box::new(Expr::BinOp {
                        left: Box::new(Expr::Binding(Binding::field(&exist))),
                        op: BinOp::Equals,
                        right: Box::new(Expr::LitBool(false)),
                    }),
                    op: BinOp::Or,
                    right: Box::new(Expr::Exists {
                        not: true,
                        body: Box::new(
                            new_select_body(&current_table).return_named("x", Expr::LitI32(1)).where_(expr_and(vec![
                                //. .
                                Expr::BinOp {
                                    left: Box::new(Expr::Binding(Binding::field(&subject))),
                                    op: BinOp::Equals,
                                    right: Box::new(Expr::Binding(Binding::field(&current_subject))),
                                },
                                Expr::BinOp {
                                    left: Box::new(Expr::Binding(Binding::field(&predicate))),
                                    op: BinOp::Equals,
                                    right: Box::new(Expr::Binding(Binding::field(&current_predicate))),
                                },
                                Expr::BinOp {
                                    left: Box::new(Expr::Binding(Binding::field(&object))),
                                    op: BinOp::Equals,
                                    right: Box::new(Expr::Binding(Binding::field(&current_object))),
                                },
                                Expr::BinOp {
                                    left: Box::new(Expr::Binding(Binding::field(&commit))),
                                    op: BinOp::Equals,
                                    right: Box::new(Expr::Binding(Binding::field(&current_commit))),
                                }
                            ])).build(),
                        ),
                        body_junctions: vec![],
                    }),
                }
            ])).build_query("triple_gc_deleted", QueryResCount::None)
        });
        triple_table = t;
        triple_commit = commit;
        triple_subject = subject;
        triple_object = object;
    }

    // Commits
    {
        let t = latest_version.table("z1YCS4PD2", "commit");
        let event_stamp = t.field(&mut latest_version, "zNKHCTSZK", "idtimestamp", field_utctime_ms().build());
        t.constraint(
            &mut latest_version,
            "zN5R3XY01",
            "commit_timestamp",
            PrimaryKey(PrimaryKeyDef { fields: vec![event_stamp.clone()] }),
        );
        let desc = t.field(&mut latest_version, "z7K4EDCAB", "description", field_str().build());
        queries.push(
            new_insert(
                &t,
                vec![set_field("stamp", &event_stamp), set_field("desc", &desc)],
            ).build_query("commit_insert", QueryResCount::None),
        );
        queries.push(
            new_select(&t)
                .return_field(&event_stamp)
                .return_field(&desc)
                .where_(expr_field_eq("stamp", &event_stamp))
                .build_query("commit_get", QueryResCount::MaybeOne),
        );
        queries.push({
            let mut active_commits =
                CteBuilder::new(
                    "active_commits",
                    new_select_body(&triple_table).distinct().return_field(&triple_commit).build(),
                );
            let active_commits_stamp = active_commits.field("stamp", triple_commit.type_.type_.clone());
            let (table_active_commits, cte_active) = active_commits.build();
            new_delete(&t).with(With {
                recursive: false,
                ctes: vec![cte_active],
            }).where_(Expr::Exists {
                not: true,
                body: Box::new(
                    new_select_body(&table_active_commits).return_named("x", Expr::LitI32(1)).where_(Expr::BinOp {
                        left: Box::new(Expr::field(&event_stamp)),
                        op: BinOp::Equals,
                        right: Box::new(Expr::field(&active_commits_stamp)),
                    }).build(),
                ),
                body_junctions: vec![],
            }).build_query("commit_gc", QueryResCount::None)
        });
    }

    // Metadata
    let meta_table;
    let meta_node;
    let meta_mimetype;
    {
        meta_table = latest_version.table("z7B1CHM4F", "meta");
        meta_node = meta_table.field(&mut latest_version, "zLQI9HQUQ", "node", FieldType::with(&node_type));
        meta_mimetype = meta_table.field(&mut latest_version, "zSZVNBP0E", "mimetype", field_str().opt().build());
        let fulltext = meta_table.field(&mut latest_version, "zPI3TKEA8", "fulltext", field_str().build());
        meta_table.constraint(
            &mut latest_version,
            "zCW5WMK7U",
            "meta_node",
            PrimaryKey(PrimaryKeyDef { fields: vec![meta_node.clone()] }),
        );
        queries.push(
            new_insert(
                &meta_table,
                vec![
                    set_field("node", &meta_node),
                    set_field("mimetype", &meta_mimetype),
                    (fulltext.clone(), Expr::LitString(format!("")))
                ],
            )
                .on_conflict(InsertConflict::DoUpdate(vec![set_field("mimetype", &meta_mimetype)]))
                .build_query("meta_upsert_file", QueryResCount::None),
        );
        queries.push(
            new_insert(&meta_table, vec![set_field("node", &meta_node), set_field("fulltext", &fulltext)])
                .on_conflict(InsertConflict::DoUpdate(vec![set_field("fulltext", &fulltext)]))
                .build_query("meta_upsert_fulltext", QueryResCount::None),
        );
        queries.push(
            new_delete(&meta_table)
                .where_(expr_field_eq("node", &meta_node))
                .build_query("meta_delete", QueryResCount::None),
        );
        queries.push(
            new_select(&meta_table)
                .where_(expr_field_eq("node", &meta_node))
                .return_fields(&[&meta_mimetype, &fulltext])
                .build_query_named_res("meta_get", QueryResCount::MaybeOne, "Metadata"),
        );
        queries.push(new_select(&meta_table).where_(Expr::BinOp {
            left: Box::new(Expr::field(&meta_node)),
            op: BinOp::In,
            right: Box::new(Expr::Param {
                name: "nodes".to_string(),
                type_: node_array_type.clone(),
            }),
        }).return_field(&meta_node).build_query("meta_filter_existing", QueryResCount::Many));
        queries.push(new_delete(&meta_table).where_(Expr::Exists {
            not: true,
            body: Box::new(new_select_body(&triple_table).return_named("x", Expr::LitI32(1)).where_(Expr::BinOp {
                left: Box::new(Expr::BinOp {
                    left: Box::new(Expr::field(&meta_node)),
                    op: BinOp::Equals,
                    right: Box::new(Expr::field(&triple_subject)),
                }),
                op: BinOp::Or,
                right: Box::new(Expr::BinOp {
                    left: Box::new(Expr::field(&meta_node)),
                    op: BinOp::Equals,
                    right: Box::new(Expr::field(&triple_object)),
                }),
            }).build()),
            body_junctions: vec![],
        }).build_query("meta_gc", QueryResCount::None));
    }

    // Generated
    {
        let t = latest_version.table("ywyc97a308uwk6", "generated");
        let node = t.field(&mut latest_version, "ll73nt097vqp9h", "node", FieldType::with(&node_type));
        let gentype = t.field(&mut latest_version, "9ws4mwxpqo8f2t", "gentype", field_str().build());
        let mimetype = t.field(&mut latest_version, "cxp4q2vrhu3164", "mimetype", field_str().build());
        t.constraint(
            &mut latest_version,
            "66tg3ve8apuxrz",
            "generated_pk",
            PrimaryKey(PrimaryKeyDef { fields: vec![node.clone(), gentype.clone()] }),
        );
        queries.push(
            new_insert(
                &t,
                vec![set_field("node", &node), set_field("gentype", &gentype), set_field("mimetype", &mimetype)],
            )
                .on_conflict(InsertConflict::DoNothing)
                .build_query("gen_insert", QueryResCount::None),
        );
        queries.push(
            new_select(&t)
                .where_(expr_and(vec![expr_field_eq("node", &node), expr_field_eq("gentype", &gentype)]))
                .return_fields(&[&mimetype])
                .build_query_named_res("gen_get", QueryResCount::MaybeOne, "GenMetadata"),
        );
        queries.push(new_delete(&t).where_(Expr::Exists {
            not: true,
            body: Box::new(new_select_body(&meta_table).return_named("x", Expr::LitI32(1)).where_(Expr::BinOp {
                left: Box::new(Expr::field(&node)),
                op: BinOp::Equals,
                right: Box::new(Expr::field(&meta_node)),
            }).build()),
            body_junctions: vec![],
        }).build_query("gen_gc", QueryResCount::None));
    }

    // File access
    {
        let t = latest_version.table("zFFF18JKY", "file_access");
        let file = t.field(&mut latest_version, "zLQI9HQUQ", "file", FieldType::with(&filehash_type));
        let menu_item_id =
            t.field(&mut latest_version, "zSZVNBP0E", "menu_item_id", FieldType::with(&file_access_type));
        let spec_hash = t.field(&mut latest_version, "zWZT5PZHR", "spec_hash", field_i64().build());
        t.constraint(
            &mut latest_version,
            "zCW5WMK7U",
            "file_access_pk",
            PrimaryKey(PrimaryKeyDef { fields: vec![file.clone(), menu_item_id.clone(), spec_hash.clone()] }),
        );
        queries.push(
            new_insert(
                &t,
                vec![
                    set_field("file", &file),
                    set_field("menu_item_id", &menu_item_id),
                    set_field("spec_hash", &spec_hash)
                ],
            )
                .on_conflict(InsertConflict::DoNothing)
                .build_query("file_access_insert", QueryResCount::None),
        );
        queries.push(new_delete(&t).where_(expr_and(vec![expr_field_eq("menu_item_id", &menu_item_id), Expr::BinOp {
            left: Box::new(Expr::Binding(Binding::field(&spec_hash))),
            op: BinOp::NotEquals,
            right: Box::new(Expr::Param {
                name: "version_hash".into(),
                type_: spec_hash.type_.type_.clone(),
            }),
        }])).build_query("file_access_clear_nonversion", QueryResCount::None));
        queries.push(
            new_select(&t)
                .where_(expr_field_eq("file", &file))
                .return_field(&menu_item_id)
                .build_query("file_access_get", QueryResCount::Many),
        );
    }

    // Generate
    match good_ormning::sqlite::generate(&root.join("src/server/db.rs"), vec![
        // Versions
        (0usize, latest_version)
    ], queries) {
        Ok(_) => { },
        Err(e) => {
            for e in e {
                eprintln!(" - {}", e);
            }
            panic!("Generate failed.");
        },
    };
}
