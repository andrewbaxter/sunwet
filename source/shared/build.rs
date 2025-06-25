use {
    std::{
        env,
        path::PathBuf,
    },
};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let root = PathBuf::from(&env::var("CARGO_MANIFEST_DIR").unwrap());

    // Query parser
    let path = root.join("src/query_parser.rustemo");
    println!("cargo:rerun-if-changed={}", path.to_str().unwrap());
    rustemo_compiler::Settings::new()
        .builder_type(rustemo_compiler::BuilderType::Default)
        .actions(true)
        .fancy_regex(true)
        .process_grammar(&path)
        .unwrap();
}
