use {
    crate::buildlib::BuildDbInput,
    good_ormning::sqlite::{
        Query,
        Version,
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
    let queries = vec![];

    let node_type = version.custom_type("node").rust_type(input.node_type_path).base_type(type_str().build());
    let filehash_type = version.custom_type("filehash").rust_type(input.filehash_type_path).base_type(type_str().build());
    let access_source_type = version.custom_type("access_source").rust_type(input.access_source_type_path).base_type(type_str().build());

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
        let commit = t.field("commit_", field_utctime_ms_chrono().build());
        t.primary_key("triple_snapshot_pk", &[&subject, &predicate, &object]);
        t.unique_index("triple_snapshot_obj_pred_subj", &[&object, &predicate, &subject]);
        t.index("triple_snapshot_pred_subj", &[&predicate, &subject]);
        t.index("triple_snapshot_pred_obj", &[&predicate, &object]);
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

    return (version.build(), queries);
}
