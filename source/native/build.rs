use {
    good_ormning::sqlite::{
        new_delete,
        new_insert,
        new_select,
        new_update,
        query::{
            expr::{
                BinOp,
                Expr,
            },
            helpers::{
                eq_field,
                set_field,
            },
            insert::InsertConflict,
        },
        schema::{
            constraint::{
                ConstraintType::PrimaryKey,
                PrimaryKeyDef,
            },
            field::{
                field_bool,
                field_fixed_offset_time_ms,
                field_i32,
                field_i64,
                field_str,
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
            left: Box::new(Expr::Field(unique.clone())),
            op: BinOp::Equals,
            right: Box::new(Expr::LitI32(0)),
        }).build_query("singleton_iam_config_set", QueryResCount::None));
    }

    // Triple
    {
        let t = latest_version.table("zQLEK3CT0", "triple");
        let subject = t.field(&mut latest_version, "zLQI9HQUQ", "subject", FieldType::with(&node_type));
        let predicate = t.field(&mut latest_version, "zSZVNBP0E", "predicate", field_str().build());
        let object = t.field(&mut latest_version, "zII52SWQB", "object", FieldType::with(&node_type));
        let event_stamp =
            t.field(&mut latest_version, "zK21ECBE5", "event_stamp", field_fixed_offset_time_ms().build());
        let event_exist = t.field(&mut latest_version, "z0ZOJM2UT", "event_exist", field_bool().build());
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
    }

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
            new_delete(&t).where_(eq_field("node", &node)).build_query("meta_delete", QueryResCount::None),
        );
        queries.push(
            new_select(&t)
                .where_(eq_field("node", &node))
                .return_fields(&[&mimetype, &fulltext])
                .build_query("meta_get", QueryResCount::MaybeOne),
        );
    }
    good_ormning::sqlite::generate(&root.join("src/db.rs"), vec![
        // Versions
        (0usize, latest_version)
    ], queries).unwrap();
}
