//! Generate a `phf::Set` of allowed GraphQL operation names at build time.
//!
//! The source list lives in [`src/api/graphql/known_operations.txt`]; the
//! resulting `OUT_DIR/known_ops.rs` is `include!`-d at runtime to give
//! O(1) lookup without a HashMap allocation.

use std::env;
use std::fs;
use std::path::PathBuf;

const SOURCE: &str = "src/api/graphql/known_operations.txt";

fn main() {
    println!("cargo:rerun-if-changed={SOURCE}");

    let input = fs::read_to_string(SOURCE)
        .unwrap_or_else(|e| panic!("failed to read {SOURCE}: {e}"));

    let mut builder = phf_codegen::Set::<String>::new();
    for line in input.lines() {
        let name = line.split('#').next().unwrap().trim();
        if !name.is_empty() {
            builder.entry(name.to_owned());
        }
    }

    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR not set")).join("known_ops.rs");
    let body = format!("pub static KNOWN_OPS: phf::Set<&'static str> = {};\n", builder.build());
    fs::write(&out, body).unwrap_or_else(|e| panic!("failed to write {}: {e}", out.display()));
}
