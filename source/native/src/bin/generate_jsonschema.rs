use {
    native::interface::{
        self,
    },
    schemars::schema_for,
    std::{
        env,
        fs::{
            create_dir_all,
            write,
        },
        path::PathBuf,
    },
};

fn main() {
    let root = PathBuf::from(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("../../generated/jsonschema");
    create_dir_all(&root).unwrap();
    write(
        root.join("config.schema.json"),
        serde_json::to_vec_pretty(&schema_for!(interface::config::Config)).unwrap(),
    ).unwrap();
    write(
        root.join("fdap.schema.json"),
        serde_json::to_vec_pretty(&schema_for!(interface::config::GlobalConfig)).unwrap(),
    ).unwrap();
    write(
        root.join("fdap_user.schema.json"),
        serde_json::to_vec_pretty(&schema_for!(interface::config::UserConfig)).unwrap(),
    ).unwrap();
}
