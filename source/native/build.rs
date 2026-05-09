use {
    crate::buildlib::BuildDbInput,
    good_ormning::sqlite::{
        generate,
        GenerateArgs,
    },
    std::{
        env,
        path::PathBuf,
    },
};

pub mod buildlib;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let _root = PathBuf::from(&env::var("CARGO_MANIFEST_DIR").unwrap());
    let db_build_input = BuildDbInput {
        node_type_path: "crate::interface::triple::DbNode",
        filehash_type_path: "crate::interface::triple::DbFileHash",
        access_source_type_path: "crate::server::access::DbAccessSourceId",
    };
    let latest = buildlib::dbv3::build(db_build_input.clone());
    match generate(GenerateArgs {
        db_name: None,
        versions: vec![
            (0usize, buildlib::dbv0::build(db_build_input.clone()).0),
            (1usize, buildlib::dbv1::build(db_build_input.clone()).0),
            (2usize, buildlib::dbv2::build(db_build_input.clone()).0),
            (3usize, latest.0)
        ],
        queries: latest.1,
    }) {
        Ok(_) => { },
        Err(e) => {
            for e in e {
                eprintln!(" - {}", e);
            }
            panic!("Generate failed.");
        },
    };

}
