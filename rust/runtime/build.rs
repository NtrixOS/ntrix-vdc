use cbindgen::Language;
use std::env;

fn main() {
    println!("cargo:rerun-if-changed=src/lib.rs");
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    cbindgen::Builder::new()
        .with_crate(manifest_dir)
        .with_language(Language::C)
        .with_autogen_warning("/* Generated File — DO NOT EDIT, This file was generated using cbindgen.*/")
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file("../target/include/ntrix_vdc_runtime.h");
}
