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
    let root = PathBuf::from(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("../generated/jsonschema");
    create_dir_all(&root).unwrap();

    // Server config
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

    // Cli
    write(
        root.join("cli_commit.schema.json"),
        serde_json::to_vec_pretty(&schema_for!(shared::interface::cli::CliCommit)).unwrap(),
    ).unwrap();

    // Api
    write(
        root.join("api_request.schema.json"),
        serde_json::to_vec_pretty(&schema_for!(shared::interface::wire::C2SReq)).unwrap(),
    ).unwrap();
    write(
        root.join("api_response_commit.schema.json"),
        serde_json::to_vec_pretty(&schema_for!(shared::interface::wire::RespCommit)).unwrap(),
    ).unwrap();
    write(
        root.join("api_response_history.schema.json"),
        serde_json::to_vec_pretty(&schema_for!(shared::interface::wire::RespHistory)).unwrap(),
    ).unwrap();
    write(
        root.join("api_response_query.schema.json"),
        serde_json::to_vec_pretty(&schema_for!(shared::interface::wire::RespQuery)).unwrap(),
    ).unwrap();
    write(
        root.join("api_response_upload_finish.schema.json"),
        serde_json::to_vec_pretty(&schema_for!(shared::interface::wire::RespUploadFinish)).unwrap(),
    ).unwrap();
    write(
        root.join("api_response_get_client_config.schema.json"),
        serde_json::to_vec_pretty(&schema_for!(shared::interface::config::ClientConfig)).unwrap(),
    ).unwrap();
    write(
        root.join("api_response_who_ami_i.schema.json"),
        serde_json::to_vec_pretty(&schema_for!(shared::interface::wire::RespWhoAmI)).unwrap(),
    ).unwrap();
}
