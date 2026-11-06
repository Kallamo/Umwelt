// SPDX-License-Identifier: AGPL-3.0-only

use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let config = cbindgen::Config::from_file(crate_dir.join("cbindgen.toml"))?;
    let bindings = cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_config(config)
        .generate()?;
    let output = crate_dir.join("include/umwelt.h");
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    bindings.write_to_file(output);
    Ok(())
}
