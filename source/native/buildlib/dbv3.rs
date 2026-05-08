use {
    crate::buildlib::BuildDbInput,
    good_ormning::QueryResCount,
    good_ormning::sqlite::{
        Query,
        Version,
        new_delete,
        new_insert,
        new_select,
        query::{
            expr::{
                BinOp,
                Expr,
                SerialExpr,
            },
            helpers::{
                expr_and,
                expr_field_eq,
                expr_field_gt,
                expr_field_lt,
                field_param,
            },
            select::Order,
        },
        schema::field::{
            field_bool,
            field_i64,
            field_str,
            field_utctime_ms_chrono,
        },
        types::type_str,
    },
};

pub fn build(input: BuildDbInput) -> (Version, Vec<Query>) {
    let version = Version::new();
    let mut queries = vec![];

    let node_type = version.custom_type("node").rust_type(input.node_type_path).base_type(type_str().build());
    let filehash_type = version.custom_type("filehash").rust_type(input.filehash_type_path).base_type(type_str().build());
    let access_source_type = version.custom_type("access_source").rust_type(input.access_source_type_path).base_type(type_str().build());

    // Subjobj (deduplicated node values)
    {
        let t = version.table("subjobj");
        let value = t.field("value", node_type.field_type());
        let fulltext = t.field("fulltext", field_str().migrate_fill(SerialExpr::LitString("".to_string())).build());
        t.primary_key("subjobj_pk", &[&value]);
        t.index("subjobj_value", &[&value]);
        queries.push(
            new_insert(&t, vec![
                (value.clone(), field_param("value", &value)),
                (fulltext.clone(), field_param("fulltext", &fulltext)),
            ])
                .on_conflict_do_nothing()
                .build_query("subjobj_insert", QueryResCount::None),
        );
        queries.push(
            new_insert(&t, vec![
                (value.clone(), field_param("value", &value)),
                (fulltext.clone(), field_param("fulltext", &fulltext)),
            ])
                .on_conflict_do_update(&[&value], vec![
                    (fulltext.clone(), field_param("fulltext", &fulltext)),
                ])
                .build_query("subjobj_update_fulltext", QueryResCount::None),
        );
    }

    // Predicate (deduplicated predicates)
    {
        let t = version.table("predicate");
        let value = t.field("value", field_str().build());
        t.primary_key("predicate_pk", &[&value]);
        t.index("predicate_value", &[&value]);
        queries.push(
            new_insert(&t, vec![
                (value.clone(), field_param("value", &value)),
            ])
                .on_conflict_do_nothing()
                .build_query("predicate_insert", QueryResCount::None),
        );
    }

    // Triple snapshot (current state)
    {
        let t = version.table("triple_snapshot");
        let subject = t.field("subject", node_type.field_type());
        let predicate = t.field("predicate", field_str().build());
        let object = t.field("object", node_type.field_type());
        let commit_ = t.field("commit_", field_utctime_ms_chrono().build());
        t.primary_key("triple_snapshot_pk", &[&subject, &predicate, &object]);
        t.unique_index("triple_snapshot_obj_pred_subj", &[&object, &predicate, &subject]);
        t.index("triple_snapshot_pred_subj", &[&predicate, &subject]);
        t.index("triple_snapshot_pred_obj", &[&predicate, &object]);
        queries.push(
            new_insert(&t, vec![
                (subject.clone(), field_param("subject", &subject)),
                (predicate.clone(), field_param("predicate", &predicate)),
                (object.clone(), field_param("object", &object)),
                (commit_.clone(), field_param("commit_", &commit_)),
            ])
                .on_conflict_do_update(&[&subject, &predicate, &object], vec![
                    (commit_.clone(), field_param("commit_", &commit_)),
                ])
                .build_query("triple_snapshot_upsert", QueryResCount::None),
        );
        queries.push(
            new_delete(&t)
                .where_(expr_and(vec![
                    expr_field_eq("eq_subject", &subject),
                    expr_field_eq("eq_predicate", &predicate),
                    expr_field_eq("eq_object", &object),
                ]))
                .build_query("triple_snapshot_delete", QueryResCount::None),
        );
        queries.push(
            new_select(&t)
                .return_field(&subject)
                .return_field(&predicate)
                .return_field(&object)
                .return_field(&commit_)
                .where_(expr_and(vec![
                    expr_field_eq("eq_subject", &subject),
                    expr_field_eq("eq_predicate", &predicate),
                    expr_field_eq("eq_object", &object),
                ]))
                .limit(Expr::LitI64(1))
                .build_query_named_res("triple_get", QueryResCount::MaybeOne, "DbResTripleSnapshot"),
        );
        // triple_list_around: select triples where subject or object is in a list of nodes
        {
            let mut nodes_type = subject.r#type().type_.clone();
            nodes_type.arr = true;
            let nodes_param = Expr::Param { name: "nodes".into(), type_: nodes_type };
            queries.push(
                new_select(&t)
                    .return_field(&subject)
                    .return_field(&predicate)
                    .return_field(&object)
                    .where_(Expr::BinOpChain {
                        op: BinOp::Or,
                        exprs: vec![
                            Expr::BinOp {
                                left: Box::new(Expr::Field(subject.to_ref())),
                                op: BinOp::In,
                                right: Box::new(nodes_param.clone()),
                            },
                            Expr::BinOp {
                                left: Box::new(Expr::Field(object.to_ref())),
                                op: BinOp::In,
                                right: Box::new(nodes_param),
                            },
                        ],
                    })
                    .build_query_named_res("triple_list_around", QueryResCount::Many, "DbResTripleAround"),
            );
        }
        // node_include_current_existing_subj / obj: select nodes present in snapshot
        for (name, field) in [("subj", &subject), ("obj", &object)] {
            let mut nodes_type = field.r#type().type_.clone();
            nodes_type.arr = true;
            queries.push(
                new_select(&t)
                    .return_field(&field)
                    .where_(Expr::BinOp {
                        left: Box::new(Expr::Field(field.to_ref())),
                        op: BinOp::In,
                        right: Box::new(Expr::Param { name: "nodes".into(), type_: nodes_type }),
                    })
                    .build_query(&format!("node_include_current_existing_{}", name), QueryResCount::Many),
            );
        }
    }

    // Triple2 (normalized history table)
    {
        let t = version.table("triple2");
        let subject = t.field("subject", node_type.field_type());
        let predicate = t.field("predicate", field_str().build());
        let object = t.field("object", node_type.field_type());
        let commit = t.field("commit_", field_utctime_ms_chrono().build());
        let exist = t.field("exists", field_bool().build());
        t.primary_key("triple2_pk", &[&subject, &predicate, &object, &commit]);
        t.unique_index("triple2_index_obj_pred_subj", &[&object, &predicate, &subject, &commit]);
        t.index("triple2_index_pred_subj", &[&predicate, &subject, &commit]);
        t.index("triple2_index_pred_obj", &[&predicate, &object, &commit]);
        t.index("triple2_commit_exists", &[&commit, &exist]);
        queries.push(
            new_insert(&t, vec![
                (subject.clone(), field_param("subject", &subject)),
                (predicate.clone(), field_param("predicate", &predicate)),
                (object.clone(), field_param("object", &object)),
                (commit.clone(), field_param("commit_", &commit)),
                (exist.clone(), field_param("exists", &exist)),
            ])
                .build_query("triple_insert", QueryResCount::None),
        );

        for (name0, where0) in [
            ("all", None),
            ("by_node", Some(Expr::BinOpChain {
                op: BinOp::Or,
                exprs: vec![expr_field_eq("eq_node", &subject), expr_field_eq("eq_node", &object)],
            })),
            (
                "by_subject_predicate",
                Some(expr_and(vec![expr_field_eq("eq_subject", &subject), expr_field_eq("eq_predicate", &predicate)])),
            ),
            (
                "by_predicate_object",
                Some(expr_and(vec![expr_field_eq("eq_predicate", &predicate), expr_field_eq("eq_object", &object)])),
            ),
        ] {
            for after in [false, true] {
                let suffix = if after {
                    "_after"
                } else {
                    ""
                };
                let mut where_exprs = vec![];
                if after {
                    where_exprs.push(expr_field_lt("time", &commit));
                    where_exprs.push(Expr::BinOp {
                        left: Box::new(Expr::LitArray(vec![
                            Expr::Field(subject.to_ref()),
                            Expr::Field(predicate.to_ref()),
                            Expr::Field(object.to_ref()),
                        ])),
                        op: BinOp::GreaterThan,
                        right: Box::new(Expr::LitArray(vec![
                            field_param("page_subject", &subject),
                            field_param("page_predicate", &predicate),
                            field_param("page_object", &object),
                        ])),
                    });
                }
                if let Some(where_) = where0.clone() {
                    where_exprs.push(where_);
                }
                let mut sel =
                    new_select(&t)
                        .return_field(&subject)
                        .return_field(&predicate)
                        .return_field(&object)
                        .return_field(&exist)
                        .return_field(&commit);
                if !where_exprs.is_empty() {
                    sel = sel.where_(expr_and(where_exprs));
                }
                sel =
                    sel
                        .order(Expr::Field(commit.to_ref()), Order::Desc)
                        .order(Expr::Field(exist.to_ref()), Order::Asc)
                        .order(Expr::Field(subject.to_ref()), Order::Asc)
                        .order(Expr::Field(predicate.to_ref()), Order::Asc)
                        .order(Expr::Field(object.to_ref()), Order::Asc)
                        .limit(Expr::LitI64(500));
                queries.push(
                    sel.build_query_named_res(
                        &format!("hist_list_{}{}", name0, suffix),
                        QueryResCount::Many,
                        "DbResTriple",
                    ),
                );
            }
        }
        for (name, field) in [("subject", &subject), ("object", &object)] {
            let expr_like = Expr::BinOp {
                left: Box::new(Expr::Field(field.to_ref())),
                op: BinOp::Like,
                right: Box::new(Expr::LitString(r#"{"t":"f",%"#.to_string())),
            };
            {
                let mut sel = new_select(&t)
                    .where_(expr_like.clone())
                    .order(Expr::Field(field.to_ref()), Order::Asc)
                    .limit(Expr::LitI64(500))
                    .return_field(&field);
                sel.q.distinct = true;
                queries.push(sel.build_query(&format!("triples_get_{}_files_start", name), QueryResCount::Many));
            }
            {
                let mut sel = new_select(&t)
                    .where_(expr_and(vec![expr_like, expr_field_gt(name, &field)]))
                    .order(Expr::Field(field.to_ref()), Order::Asc)
                    .limit(Expr::LitI64(500))
                    .return_field(&field);
                sel.q.distinct = true;
                queries.push(sel.build_query(&format!("triples_get_{}_files_after", name), QueryResCount::Many));
            }
        }
    }

    // Commits
    {
        let t = version.table("commit");
        let event_stamp = t.field("idtimestamp", field_utctime_ms_chrono().build());
        let desc = t.field("description", field_str().build());
        t.primary_key("commit_timestamp", &[&event_stamp]);
        queries.push(
            new_insert(&t, vec![
                (event_stamp.clone(), field_param("idtimestamp", &event_stamp)),
                (desc.clone(), field_param("description", &desc)),
            ])
                .build_query("commit_insert", QueryResCount::None),
        );
        queries.push(
            new_select(&t)
                .return_field(&event_stamp)
                .return_field(&desc)
                .where_(expr_field_eq("eq_idtimestamp", &event_stamp))
                .build_query_named_res("commit_get", QueryResCount::MaybeOne, "DbResCommit"),
        );
    }

    // Metadata (file mime types; fulltext for FTS)
    {
        let t = version.table("meta");
        let node = t.field("node", node_type.field_type());
        let mimetype = t.field("mimetype", field_str().opt().build());
        let fulltext = t.field("fulltext", field_str().build());
        t.primary_key("meta_node", &[&node]);
        queries.push(
            new_select(&t)
                .return_field(&mimetype)
                .where_(expr_field_eq("eq_node", &node))
                .build_query_named_res("meta_get", QueryResCount::MaybeOne, "Metadata"),
        );
        queries.push(
            new_insert(&t, vec![
                (node.clone(), field_param("node", &node)),
                (mimetype.clone(), field_param("mimetype", &mimetype)),
                (fulltext.clone(), Expr::LitString("".into())),
            ])
                .on_conflict_do_update(&[&node], vec![
                    (mimetype.clone(), field_param("mimetype", &mimetype)),
                ])
                .build_query("meta_upsert_file", QueryResCount::None),
        );
        {
            let mut nodes_type = node.r#type().type_.clone();
            nodes_type.arr = true;
            queries.push(
                new_select(&t)
                    .return_field(&node)
                    .where_(Expr::BinOp {
                        left: Box::new(Expr::Field(node.to_ref())),
                        op: BinOp::In,
                        right: Box::new(Expr::Param { name: "nodes".into(), type_: nodes_type }),
                    })
                    .build_query("meta_include_existing", QueryResCount::Many),
            );
        }
    }

    // Generated
    {
        let t = version.table("generated");
        let node = t.field("node", node_type.field_type());
        let gentype = t.field("gentype", field_str().build());
        let mimetype = t.field("mimetype", field_str().build());
        t.primary_key("generated_pk", &[&node, &gentype]);
        queries.push(
            new_select(&t)
                .return_field(&mimetype)
                .where_(expr_and(vec![
                    expr_field_eq("eq_node", &node),
                    expr_field_eq("eq_gentype", &gentype),
                ]))
                .build_query_named_res("gen_get", QueryResCount::MaybeOne, "DbResGen"),
        );
        queries.push(
            new_insert(&t, vec![
                (node.clone(), field_param("node", &node)),
                (gentype.clone(), field_param("gentype", &gentype)),
                (mimetype.clone(), field_param("mimetype", &mimetype)),
            ])
                .on_conflict_do_update(&[&node, &gentype], vec![
                    (mimetype.clone(), field_param("mimetype", &mimetype)),
                ])
                .build_query("gen_ensure", QueryResCount::None),
        );
        {
            let mut nodes_type = node.r#type().type_.clone();
            nodes_type.arr = true;
            queries.push(
                new_select(&t)
                    .return_field(&node)
                    .where_(Expr::BinOp {
                        left: Box::new(Expr::Field(node.to_ref())),
                        op: BinOp::In,
                        right: Box::new(Expr::Param { name: "nodes".into(), type_: nodes_type }),
                    })
                    .build_query("gen_include_existing", QueryResCount::Many),
            );
        }
    }

    // File access
    {
        let t = version.table("file_access");
        let file = t.field("file", filehash_type.field_type());
        let access_source = t.field("access_source", access_source_type.field_type());
        let spec_hash = t.field("spec_hash", field_i64().build());
        t.primary_key("file_access_pk", &[&file, &access_source, &spec_hash]);
        queries.push(
            new_insert(&t, vec![
                (file.clone(), field_param("file", &file)),
                (access_source.clone(), field_param("access_source", &access_source)),
                (spec_hash.clone(), field_param("spec_hash", &spec_hash)),
            ])
                .on_conflict_do_nothing()
                .build_query("file_access_insert", QueryResCount::None),
        );
        queries.push(
            new_select(&t)
                .return_field(&file)
                .return_field(&access_source)
                .return_field(&spec_hash)
                .where_(expr_and(vec![
                    expr_field_eq("eq_file", &file),
                    expr_field_eq("eq_access_source", &access_source),
                    expr_field_eq("eq_spec_hash", &spec_hash),
                ]))
                .build_query("file_access_get", QueryResCount::MaybeOne),
        );
        queries.push(
            new_select(&t)
                .return_field(&access_source)
                .where_(expr_field_eq("eq_file", &file))
                .build_query("file_access_get_by_file", QueryResCount::Many),
        );
        queries.push(
            new_delete(&t)
                .where_(expr_and(vec![
                    expr_field_eq("eq_access_source", &access_source),
                    Expr::BinOp {
                        left: Box::new(Expr::Field(spec_hash.to_ref())),
                        op: BinOp::NotEquals,
                        right: Box::new(field_param("ne_spec_hash", &spec_hash)),
                    },
                ]))
                .build_query("file_access_clear_nonversion", QueryResCount::None),
        );
    }

    return (version.build(), queries);
}
