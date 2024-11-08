use {
    good_ormning::sqlite::{
        new_delete,
        new_insert,
        new_select,
        new_select_body,
        new_update,
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
                field_i32,
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
    let node_type = type_str().custom("crate::interface::triple::Node").build();
    let iam_config_type = type_str().custom("crate::interface::iam::IamConfig").build();

    // Global
    {
        let t = latest_version.table("zS7B13HD5", "singleton");
        let unique = t.field(&mut latest_version, "z0YQ7NWRC", "unique", field_i32().build());
        let iam_config = t.field(&mut latest_version, "zM7F6WA5F", "iam_config", FieldType::with(&iam_config_type));
        t.constraint(
            &mut latest_version,
            "z118L67XL",
            "singleton_unique",
            PrimaryKey(PrimaryKeyDef { fields: vec![unique.clone()] }),
        );
        queries.push(
            new_insert(&t, vec![(unique.clone(), Expr::LitI32(0)), set_field("iam_config", &iam_config)])
                .on_conflict(InsertConflict::DoUpdate(vec![(unique.clone(), Expr::LitI32(0))]))
                .return_fields(&[&iam_config])
                .build_query("singleton_init", QueryResCount::One),
        );
        queries.push(new_update(&t, vec![set_field("iam_config", &iam_config)]).where_(Expr::BinOp {
            left: Box::new(Expr::Binding(Binding::field(&unique))),
            op: BinOp::Equals,
            right: Box::new(Expr::LitI32(0)),
        }).build_query("singleton_iam_config_set", QueryResCount::None));
    }

    // Commits
    let commit_table;
    let commit_stamp;
    {
        let t = latest_version.table("z1YCS4PD2", "commit");
        let event_stamp = t.field(&mut latest_version, "zNKHCTSZK", "timestamp", field_utctime_ms().build());
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
        commit_table = t;
        commit_stamp = event_stamp;
    }

    // Triple
    let triple_table;
    let triple_event_stamp;
    {
        let t = latest_version.table("zQLEK3CT0", "triple");
        let subject = t.field(&mut latest_version, "zLQI9HQUQ", "subject", FieldType::with(&node_type));
        let predicate = t.field(&mut latest_version, "zSZVNBP0E", "predicate", field_str().build());
        let object = t.field(&mut latest_version, "zII52SWQB", "object", FieldType::with(&node_type));
        let event_stamp = t.field(&mut latest_version, "zK21ECBE5", "timestamp", field_utctime_ms().build());
        let event_exist = t.field(&mut latest_version, "z0ZOJM2UT", "exists", field_bool().build());
        let iam_target = t.field(&mut latest_version, "zFN1MRJMO", "iam_target", field_i64().build());
        t.constraint(
            &mut latest_version,
            "z1T10QI43",
            "triple_pk",
            PrimaryKey(
                PrimaryKeyDef {
                    fields: vec![subject.clone(), predicate.clone(), object.clone(), event_stamp.clone()],
                },
            ),
        );
        t
            .index("zXIMPRLIR", "triple_index_obj_pred_subj", &[&object, &predicate, &subject, &event_stamp])
            .unique()
            .build(&mut latest_version);
        t
            .index("zBZVX51AR", "triple_index_pred_subj", &[&predicate, &subject, &event_stamp])
            .unique()
            .build(&mut latest_version);
        t
            .index("zTVLKA6GQ", "triple_index_pred_obj", &[&predicate, &object, &event_stamp])
            .unique()
            .build(&mut latest_version);
        queries.push(
            new_insert(
                &t,
                vec![
                    set_field("subject", &subject),
                    set_field("predicate", &predicate),
                    set_field("object", &object),
                    set_field("stamp", &event_stamp),
                    set_field("exist", &event_exist),
                    set_field("iam_target", &iam_target)
                ],
            )
                .on_conflict(InsertConflict::DoUpdate(vec![set_field("exist", &event_exist)]))
                .build_query("triple_insert", QueryResCount::None),
        );
        queries.push(
            new_select(&t)
                .return_field(&subject)
                .return_field(&predicate)
                .return_field(&object)
                .return_field(&event_stamp)
                .return_field(&event_exist)
                .return_field(&iam_target)
                .build_query("triple_get_all", QueryResCount::Many),
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
                        .return_named("timestamp", fn_max(Expr::field(&event_stamp)))
                        .build(),
                );
            let current_subject = current.field("subject", subject.type_.type_.clone());
            let current_predicate = current.field("predicate", predicate.type_.type_.clone());
            let current_object = current.field("object", object.type_.type_.clone());
            let current_stamp = current.field("event_stamp", event_stamp.type_.type_.clone());
            let (current_table, current_cte) = current.build();
            new_delete(&t).with(With {
                recursive: false,
                ctes: vec![current_cte],
            }).where_(expr_and(vec![
                //. .
                expr_field_lt("epoch", &event_stamp),
                Expr::BinOp {
                    left: Box::new(Expr::BinOp {
                        left: Box::new(Expr::Binding(Binding::field(&event_exist))),
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
                                    left: Box::new(Expr::Binding(Binding::field(&event_stamp))),
                                    op: BinOp::Equals,
                                    right: Box::new(Expr::Binding(Binding::field(&current_stamp))),
                                }
                            ])).build(),
                        ),
                        body_junctions: vec![],
                    }),
                }
            ])).build_query("triple_gc_deleted", QueryResCount::None)
        });
        triple_table = t;
        triple_event_stamp = event_stamp;
    }
    queries.push({
        let mut active_commits =
            CteBuilder::new(
                "active_commits",
                new_select_body(&triple_table).distinct().return_field(&triple_event_stamp).build(),
            );
        let active_commits_stamp = active_commits.field("stamp", triple_event_stamp.type_.type_.clone());
        let (table_active_commits, cte_active) = active_commits.build();
        new_delete(&commit_table).with(With {
            recursive: false,
            ctes: vec![cte_active],
        }).where_(Expr::Exists {
            not: true,
            body: Box::new(
                new_select_body(&table_active_commits).return_named("x", Expr::LitI32(1)).where_(Expr::BinOp {
                    left: Box::new(Expr::field(&commit_stamp)),
                    op: BinOp::Equals,
                    right: Box::new(Expr::field(&active_commits_stamp)),
                }).build(),
            ),
            body_junctions: vec![],
        }).build_query("commit_gc", QueryResCount::None)
    });

    // Metadata
    {
        let t = latest_version.table("z7B1CHM4F", "meta");
        let node = t.field(&mut latest_version, "zLQI9HQUQ", "node", FieldType::with(&node_type));
        let mimetype = t.field(&mut latest_version, "zSZVNBP0E", "mimetype", field_str().build());
        let fulltext = t.field(&mut latest_version, "zPI3TKEA8", "fulltext", field_str().build());
        t.constraint(
            &mut latest_version,
            "zCW5WMK7U",
            "meta_node",
            PrimaryKey(PrimaryKeyDef { fields: vec![node.clone()] }),
        );
        queries.push(
            new_insert(
                &t,
                vec![set_field("node", &node), set_field("mimetype", &mimetype), set_field("fulltext", &fulltext)],
            )
                .on_conflict(InsertConflict::DoNothing)
                .build_query("meta_insert", QueryResCount::None),
        );
        queries.push(
            new_delete(&t).where_(expr_field_eq("node", &node)).build_query("meta_delete", QueryResCount::None),
        );
        queries.push(
            new_select(&t)
                .where_(expr_field_eq("node", &node))
                .return_fields(&[&mimetype, &fulltext])
                .build_query("meta_get", QueryResCount::MaybeOne),
        );
    }
    match good_ormning::sqlite::generate(&root.join("src/db.rs"), vec![
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
