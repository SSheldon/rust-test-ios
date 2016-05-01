use std::error::Error;
use std::io::{Result as IoResult, Write};
use std::fs::File;
use std::path::Path;
use std::process::Command;

use serde_json::{Value, self};

static TEMPLATE: &'static str = r##"
[package]
name = "tests-ios"
version = "0.0.0"

[lib]
name = "tests_ios"
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

struct Dependency<'a> {
    name: &'a str,
    path: &'a Path,
    features: &'a [&'a str],
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

fn read_name(crate_dir: &Path) -> Result<String, Box<Error>> {
    let out = Command::new("cargo")
        .arg("read-manifest")
        .arg("--manifest-path").arg(&crate_dir.join("Cargo.toml"))
        .output();
    let out = try!(out);
    if !out.status.success() {
        err!("cargo read-manifest failed with status {}", out.status);
    }

    let value: Value = try!(serde_json::from_slice(&out.stdout));
    let mut obj = match value {
        Value::Object(o) => o,
        _ => err!("crate manifest was not a JSON object"),
    };
    match obj.remove("name") {
        Some(Value::String(s)) => Ok(s),
        _ => err!("crate manifest did not include key \"name\""),
    }
}

pub fn create_config(dir: &Path, crate_dir: &Path) -> Result<(), Box<Error>> {
    let crate_name = try!(read_name(crate_dir));
    let dependency = Dependency {
        name: &crate_name,
        path: crate_dir,
        features: &[],
    };

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
            .arg("--manifest-path").arg(&dir.join("Cargo.toml"))
            .status();
        if !try!(result).success() {
            return Ok(false);
        }
    }

    let cargo_mode = "debug";
    let lib_name = "libtests_ios.a";
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
