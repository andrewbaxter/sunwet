use {
    good_ormning::sqlite::{
        new_insert,
        query::helpers::set_field,
        schema::{
            constraint::{
                ConstraintType::PrimaryKey,
                PrimaryKeyDef,
            },
            field::{
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
    let node_type = type_str().custom("crate::Node").build();
    {
        let t = latest_version.table("zQLEK3CT0", "triple");
        let subject = t.field(&mut latest_version, "zLQI9HQUQ", "subject", FieldType::with(&node_type));
        let predicate = t.field(&mut latest_version, "zSZVNBP0E", "predicate", field_str().build());
        let object = t.field(&mut latest_version, "zII52SWQB", "object", FieldType::with(&node_type));
        t.constraint(
            &mut latest_version,
            "z1T10QI43",
            "triple_pk",
            PrimaryKey(PrimaryKeyDef { fields: vec![subject.clone(), predicate.clone(), object.clone()] }),
        );
        t
            .index("zXIMPRLIR", "triple_index_obj_pred_subj", &[&object, &predicate, &subject])
            .unique()
            .build(&mut latest_version);
        t.index("zBZVX51AR", "triple_index_pred_subj", &[&predicate, &subject]).unique().build(&mut latest_version);
        t.index("zTVLKA6GQ", "triple_index_pred_obj", &[&predicate, &object]).unique().build(&mut latest_version);
        queries.push(
            new_insert(
                &t,
                vec![
                    set_field("subject", &subject),
                    set_field("predicate", &predicate),
                    set_field("object", &object)
                ],
            )
                .on_conflict(good_ormning::sqlite::query::insert::InsertConflict::DoNothing)
                .build_query("triple_insert", QueryResCount::None),
        );
    }
    good_ormning::sqlite::generate(&root.join("src/db.rs"), vec![
        // Versions
        (0usize, latest_version)
    ], queries).unwrap();
}
