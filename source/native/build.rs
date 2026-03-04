use {
    crate::buildlib::BuildDbInput,
    good_ormning::sqlite::{
        types::type_str,
    },
    std::{
        env,
        path::PathBuf,
    },
};

pub mod buildlib;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let root = PathBuf::from(&env::var("CARGO_MANIFEST_DIR").unwrap());
    let db_build_input = BuildDbInput {
        node_type: type_str().custom("crate::interface::triple::DbNode").build(),
        node_array_type: type_str().custom("crate::interface::triple::DbNode").array().build(),
        filehash_type: type_str().custom("crate::interface::triple::DbFileHash").build(),
        access_source_type: type_str().custom("crate::server::access::DbAccessSourceId").build(),
    };
    let latest = buildlib::dbv1::build(db_build_input.clone());
    match good_ormning::sqlite::generate(&root.join("src/server/db.rs"), vec![
        // Versions
        (0usize, buildlib::dbv0::build(db_build_input.clone()).0),
        (1usize, latest.0)
    ], latest.1) {
        Ok(_) => { },
        Err(e) => {
            for e in e {
                eprintln!(" - {}", e);
            }
            panic!("Generate failed.");
        },
    };
}
