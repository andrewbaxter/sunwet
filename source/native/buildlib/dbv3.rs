use {
    crate::buildlib::BuildDbInput,
    good_ormning::{
        sqlite::{
            new_select,
            query::{
                expr::{BinOp, Expr},
                helpers::{expr_and, expr_or},
                select::Order,
            },
            schema::field::{field_bool, field_i64, field_str, field_utctime_ms_chrono},
            types::type_str,
            Query, Version,
        },
        QueryResCount,
    },
};

pub fn build(input: BuildDbInput) -> (Version, Vec<Query>) {
    let version = Version::new();
    let mut queries = vec![];

    let node_type = version
        .custom_type("node")
        .rust_type(input.node_type_path)
        .base_type(type_str().build());
    let filehash_type = version
        .custom_type("filehash")
        .rust_type(input.filehash_type_path)
        .base_type(type_str().build());
    let access_source_type = version
        .custom_type("access_source")
        .rust_type(input.access_source_type_path)
        .base_type(type_str().build());

    // Subjobj (deduplicated node values)
    {
        let t = version.table("subjobj");
        let value = t.field("value", node_type.field_type());
        t.primary_key("subjobj_pk", &[&value]);
        t.index("subjobj_value", &[&value]);
    }

    // Predicate (deduplicated predicates)
    {
        let t = version.table("predicate");
        let value = t.field("value", field_str().build());
        t.primary_key("predicate_pk", &[&value]);
        t.index("predicate_value", &[&value]);
    }

    // Triple snapshot (current state)
    {
        let t = version.table("triple_snapshot");
        let subject = t.field("subject", node_type.field_type());
        let predicate = t.field("predicate", field_str().build());
        let object = t.field("object", node_type.field_type());
        let _commit = t.field("commit_", field_utctime_ms_chrono().build());
        t.primary_key("triple_snapshot_pk", &[&subject, &predicate, &object]);
        t.unique_index(
            "triple_snapshot_obj_pred_subj",
            &[&object, &predicate, &subject],
        );
        t.index("triple_snapshot_pred_subj", &[&predicate, &subject]);
        t.index("triple_snapshot_pred_obj", &[&predicate, &object]);
    }

    // Triple2 (normalized history table)
    let triple2_table;
    let triple2_subject;
    let triple2_predicate;
    let triple2_object;
    let triple2_commit;
    let triple2_exists;
    {
        let t = version.table("triple2");
        triple2_subject = t.field("subject", node_type.field_type());
        triple2_predicate = t.field("predicate", field_str().build());
        triple2_object = t.field("object", node_type.field_type());
        triple2_commit = t.field("commit_", field_utctime_ms_chrono().build());
        triple2_exists = t.field("exists", field_bool().build());
        t.primary_key(
            "triple2_pk",
            &[
                &triple2_subject,
                &triple2_predicate,
                &triple2_object,
                &triple2_commit,
            ],
        );
        t.unique_index(
            "triple2_index_obj_pred_subj",
            &[
                &triple2_object,
                &triple2_predicate,
                &triple2_subject,
                &triple2_commit,
            ],
        );
        t.index(
            "triple2_index_pred_subj",
            &[&triple2_predicate, &triple2_subject, &triple2_commit],
        );
        t.index(
            "triple2_index_pred_obj",
            &[&triple2_predicate, &triple2_object, &triple2_commit],
        );
        t.index("triple2_commit_exists", &[&triple2_commit, &triple2_exists]);
        triple2_table = t;
    }

    // Commits
    {
        let t = version.table("commit");
        let event_stamp = t.field("idtimestamp", field_utctime_ms_chrono().build());
        let _desc = t.field("description", field_str().build());
        t.primary_key("commit_timestamp", &[&event_stamp]);
    }

    // Metadata (file mime types; fulltext for FTS)
    {
        let t = version.table("meta");
        let node = t.field("node", node_type.field_type());
        let _mimetype = t.field("mimetype", field_str().opt().build());
        let _fulltext = t.field("fulltext", field_str().build());
        t.primary_key("meta_node", &[&node]);
    }

    // Generated
    {
        let t = version.table("generated");
        let node = t.field("node", node_type.field_type());
        let gentype = t.field("gentype", field_str().build());
        let _mimetype = t.field("mimetype", field_str().build());
        t.primary_key("generated_pk", &[&node, &gentype]);
    }

    // File access
    {
        let t = version.table("file_access");
        let file = t.field("file", filehash_type.field_type());
        let access_source = t.field("access_source", access_source_type.field_type());
        let spec_hash = t.field("spec_hash", field_i64().build());
        t.primary_key("file_access_pk", &[&file, &access_source, &spec_hash]);
    }

    for (name0, filter) in [
        ("all", None),
        (
            "by_node",
            Some(expr_or(vec![
                Expr::BinOp {
                    left: Box::new(Expr::Field(triple2_subject.to_ref())),
                    op: BinOp::Equals,
                    right: Box::new(Expr::Param {
                        name: "node".to_string(),
                        type_: node_type.field_type().type_.clone(),
                    }),
                },
                Expr::BinOp {
                    left: Box::new(Expr::Field(triple2_object.to_ref())),
                    op: BinOp::Equals,
                    right: Box::new(Expr::Param {
                        name: "node".to_string(),
                        type_: node_type.field_type().type_.clone(),
                    }),
                },
            ])),
        ),
        (
            "by_subject_predicate",
            Some(expr_and(vec![
                Expr::BinOp {
                    left: Box::new(Expr::Field(triple2_subject.to_ref())),
                    op: BinOp::Equals,
                    right: Box::new(Expr::Param {
                        name: "subject".to_string(),
                        type_: node_type.field_type().type_.clone(),
                    }),
                },
                Expr::BinOp {
                    left: Box::new(Expr::Field(triple2_predicate.to_ref())),
                    op: BinOp::Equals,
                    right: Box::new(Expr::Param {
                        name: "predicate".to_string(),
                        type_: field_str().build().type_.clone(),
                    }),
                },
            ])),
        ),
        (
            "by_predicate_object",
            Some(expr_and(vec![
                Expr::BinOp {
                    left: Box::new(Expr::Field(triple2_predicate.to_ref())),
                    op: BinOp::Equals,
                    right: Box::new(Expr::Param {
                        name: "predicate".to_string(),
                        type_: field_str().build().type_.clone(),
                    }),
                },
                Expr::BinOp {
                    left: Box::new(Expr::Field(triple2_object.to_ref())),
                    op: BinOp::Equals,
                    right: Box::new(Expr::Param {
                        name: "object".to_string(),
                        type_: node_type.field_type().type_.clone(),
                    }),
                },
            ])),
        ),
    ] {
        for after in [false, true] {
            let suffix = if after { "_after" } else { "" };
            let mut where_exprs = vec![];
            if after {
                where_exprs.push(Expr::BinOp {
                    left: Box::new(Expr::LitArray(vec![
                        Expr::Field(triple2_commit.to_ref()),
                        Expr::Field(triple2_subject.to_ref()),
                        Expr::Field(triple2_predicate.to_ref()),
                        Expr::Field(triple2_object.to_ref()),
                    ])),
                    op: BinOp::LessThan,
                    right: Box::new(Expr::LitArray(vec![
                        Expr::Param {
                            name: "after_commit".to_string(),
                            type_: field_utctime_ms_chrono().build().type_.clone(),
                        },
                        Expr::Param {
                            name: "after_subject".to_string(),
                            type_: node_type.field_type().type_.clone(),
                        },
                        Expr::Param {
                            name: "after_predicate".to_string(),
                            type_: field_str().build().type_.clone(),
                        },
                        Expr::Param {
                            name: "after_object".to_string(),
                            type_: node_type.field_type().type_.clone(),
                        },
                    ])),
                });
            }
            if let Some(where_) = filter.clone() {
                where_exprs.push(where_);
            }
            let mut sel = new_select(&triple2_table)
                .return_field(&triple2_subject)
                .return_field(&triple2_predicate)
                .return_field(&triple2_object)
                .return_field(&triple2_commit)
                .return_field(&triple2_exists);
            if !where_exprs.is_empty() {
                sel = sel.where_(expr_and(where_exprs));
            }
            sel = sel
                .order(Expr::Field(triple2_commit.to_ref()), Order::Desc)
                .order(Expr::Field(triple2_subject.to_ref()), Order::Desc)
                .order(Expr::Field(triple2_predicate.to_ref()), Order::Desc)
                .order(Expr::Field(triple2_object.to_ref()), Order::Desc)
                .limit(Expr::LitI64(100));
            queries.push(sel.build_query_named_res(
                &format!("hist_list_{}{}", name0, suffix),
                QueryResCount::Many,
                "DbResHistory",
            ));
        }
    }

    return (version.build(), queries);
}
