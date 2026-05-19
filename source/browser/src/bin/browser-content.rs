use sunwet_browser::create_capture_button;

fn main() {
    // Force linker to include the library's wasm-bindgen exports. The actual entry
    // points are the #[wasm_bindgen] exports in lib.rs.
    let _ = create_capture_button;
}
