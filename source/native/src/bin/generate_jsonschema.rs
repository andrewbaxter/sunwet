use {
    aargvark::{
        vark,
        Aargvark,
    },
    native::interface::config::Config,
    schemars::schema_for,
    std::{
        fs::{
            create_dir_all,
            write,
        },
        path::PathBuf,
    },
};

#[derive(Aargvark)]
struct Args {
    dir: PathBuf,
}

fn main() {
    let args = vark::<Args>();
    create_dir_all(&args.dir).unwrap();
    write(args.dir.join("config.schema.json"), serde_json::to_vec_pretty(&schema_for!(Config)).unwrap()).unwrap();
}
