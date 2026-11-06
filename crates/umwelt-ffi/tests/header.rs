// SPDX-License-Identifier: AGPL-3.0-only

#[test]
fn generated_header_matches_the_versioned_header() {
    let crate_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let config = cbindgen::Config::from_file(crate_dir.join("cbindgen.toml"))
        .expect("cbindgen config should load");
    let bindings = cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_config(config)
        .generate()
        .expect("header should generate");
    let mut generated = Vec::new();
    bindings.write(&mut generated);
    let current =
        std::fs::read(crate_dir.join("include/umwelt.h")).expect("versioned header should exist");
    assert_eq!(
        generated, current,
        "include/umwelt.h has drifted; run cargo run -p umwelt-ffi --example generate-header"
    );
}
