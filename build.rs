use std::{
    env, fs,
    path::Path,
};

const COMMANDS: &[&str] = &[
    "load_translations",
    "translate",
    "set_locale",
    "get_locale",
    "get_available_locales",
];

fn main() {
    // Generate empty bundled_locales.rs by default
    // Projects using this plugin should generate their own bundled_locales.rs
    // and copy it to the OUT_DIR before compilation
    generate_empty_bundled_locales();
    tauri_plugin::Builder::new(COMMANDS).build();
}

fn generate_empty_bundled_locales() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("bundled_locales.rs");

    println!("cargo:info=Generating default empty bundled_locales.rs");
    println!("cargo:info=Projects should override this with their own locales via a build.rs script");

    let code = "pub fn get_bundled_data() -> Vec<(&'static str, &'static str, &'static str)> {\n    vec![]\n}\n";

    fs::write(dest_path, code).expect("Failed to write bundled_locales.rs");
}
