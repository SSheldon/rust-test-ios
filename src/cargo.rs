use std::io::{Result as IoResult, Write};
use std::fs::File;
use std::path::Path;
use std::process::Command;

static TEMPLATE: &'static str = r##"
[package]
name = "objc-tests"
version = "0.0.0"
authors = ["Steven Sheldon"]

[lib]
name = "objc_tests"
path = "lib.rs"
crate-type = ["staticlib"]

[dependencies.objc]
path = ".."
features = ["exception"]
"##;

static ARCHS: [&'static str; 5] = [
    "i386",
    "x86_64",
    "armv7",
    "armv7s",
    "aarch64",
];

pub fn create_config(dir: &Path) -> IoResult<()> {
    let mut config_file = try!(File::create(dir.join("Cargo.toml")));
    try!(config_file.write(TEMPLATE.as_bytes()));
    Ok(())
}

pub fn build(dir: &Path) -> IoResult<bool> {
    let targets: Vec<_> = ARCHS.iter()
        .map(|a| format!("{}-apple-ios", a))
        .collect();

    for target in &targets {
        let result = Command::new("cargo")
            .arg("build")
            .arg("--target").arg(target)
            .status();
        if !try!(result).success() {
            return Ok(false);
        }
    }

    let cargo_mode = "debug";
    let lib_name = "libobjc_tests.a";
    let target_libs: Vec<_> = targets.iter()
        .map(|t| dir.join("target").join(t).join(cargo_mode).join(lib_name))
        .collect();

    let result = Command::new("lipo")
        .arg("-create")
        .arg("-output").arg(&dir.join("libRustTests.a"))
        .args(&target_libs)
        .status();
    result.map(|s| s.success())
}
