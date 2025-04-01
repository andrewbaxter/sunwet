use {
    aargvark::{
        vark,
        Aargvark,
    },
    std::path::PathBuf,
    wasm_bindgen_cli_support::Bindgen,
};

#[derive(Aargvark)]
struct Args {
    #[vark(flag = "--in-wasm")]
    in_wasm: PathBuf,
    #[vark(flag = "--out-name")]
    out_name: String,
    #[vark(flag = "--out-dir")]
    out_dir: PathBuf,
}

fn main() {
    let args = vark::<Args>();
    let mut b = Bindgen::new();
    b.input_path(args.in_wasm);
    b.web(true).unwrap();
    b.split_linked_modules(true);
    b.keep_debug(true);
    b.out_name(&args.out_name);
    b.generate(&args.out_dir).unwrap();
    let wasm_path = args.out_dir.join(format!("{}_bg.wasm", args.out_name));
    let map_path = wasm_path.with_extension(format!("{}.map", wasm_path.extension().unwrap().to_str().unwrap()));
    let mut mapper = wasm2map::WASM::load(&wasm_path).unwrap();
    std::fs::write(&map_path, &mapper.map_v3()).unwrap();
    mapper.patch(map_path.file_name().unwrap().to_str().unwrap()).unwrap();
}
