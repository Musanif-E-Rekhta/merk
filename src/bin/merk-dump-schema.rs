//! Dumps the GraphQL SDL to a file.
//!
//! Default destination is `../musanif-contracts/schema.graphql` so
//! `cargo run --bin merk-dump-schema` Just Works from the merk/
//! directory. Pass an explicit path to override.

use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out: PathBuf = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join("musanif-contracts")
                .join("schema.graphql")
        });

    let sdl = merk::api::graphql::schema_sdl();
    std::fs::write(&out, sdl)?;
    println!("schema written to {}", out.display());
    Ok(())
}
