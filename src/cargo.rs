use std::io::{Result as IoResult, Write};
use std::fs::File;
use std::path::Path;
use std::process::Command;

static TEMPLATE: &'static str = r##"
[package]
name = "ios-tests"
version = "0.0.0"

[lib]
name = "ios_tests"
path = "lib.rs"
crate-type = ["staticlib"]
"##;

static ARCHS: [&'static str; 5] = [
    "i386",
    "x86_64",
    "armv7",
    "armv7s",
    "aarch64",
];

pub struct Dependency<'a> {
    pub name: &'a str,
    pub path: &'a Path,
    pub features: &'a [&'a str],
}

impl<'a> Dependency<'a> {
    fn to_toml(&self) -> String {
        let mut toml = format!("\n[dependencies.{}]\npath = \"{}\"\n",
            self.name, self.path.to_str().unwrap());
        if self.features.len() > 0 {
            toml.push_str("features = [\"");
            toml.push_str(&self.features.join("\", \""));
            toml.push_str("\"]\n");
        }
        toml
    }
}

pub fn create_config(dir: &Path, dependency: Dependency) -> IoResult<()> {
    let mut config_file = try!(File::create(dir.join("Cargo.toml")));
    try!(config_file.write(TEMPLATE.as_bytes()));
    try!(config_file.write(dependency.to_toml().as_bytes()));
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
    let lib_name = "libios_tests.a";
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
