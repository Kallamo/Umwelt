// SPDX-License-Identifier: AGPL-3.0-only
use std::process::Command;

#[test]
fn version_reports_the_product_and_format_versions() {
    let output = Command::new(env!("CARGO_BIN_EXE_umwelt"))
        .arg("--version")
        .output()
        .expect("run umwelt --version");
    assert!(output.status.success());
    let line = String::from_utf8(output.stdout).expect("utf-8 output");
    assert_eq!(
        line.trim(),
        format!(
            "umwelt {} (contract {}, protocol 1, storage 5)",
            env!("CARGO_PKG_VERSION"),
            env!("CARGO_PKG_VERSION")
        )
    );
}
