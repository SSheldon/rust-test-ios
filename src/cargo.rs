use std::error::Error;
use std::io::{Result as IoResult, Write};
use std::fs::File;
use std::path::Path;
use std::process::Command;

use serde_json::{Value, self};
use toml::{Table, Value as TomlValue};

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

fn toml_config(dep: Dependency) -> Table {
    let mut config = Table::new();

    let mut package = Table::new();
    package.insert("name".to_owned(), TomlValue::String("tests-ios".to_owned()));
    package.insert("version".to_owned(), TomlValue::String("0.0.0".to_owned()));
    config.insert("package".to_owned(), TomlValue::Table(package));

    let mut lib = Table::new();
    lib.insert("name".to_owned(), TomlValue::String("tests_ios".to_owned()));
    lib.insert("path".to_owned(), TomlValue::String("lib.rs".to_owned()));
    let crate_type = vec![TomlValue::String("staticlib".to_owned())];
    lib.insert("crate-type".to_owned(), TomlValue::Array(crate_type));
    config.insert("lib".to_owned(), TomlValue::Table(lib));

    let mut dependencies = Table::new();
    let mut crate_dep = Table::new();
    let path = dep.path.to_str().unwrap().to_owned();
    crate_dep.insert("path".to_owned(), TomlValue::String(path));
    if dep.features.len() > 0 {
        let features = dep.features.iter()
            .map(|&f| TomlValue::String(f.to_owned()))
            .collect();
        crate_dep.insert("features".to_owned(), TomlValue::Array(features));
    }
    dependencies.insert(dep.name.to_owned(), TomlValue::Table(crate_dep));
    config.insert("dependencies".to_owned(), TomlValue::Table(dependencies));

    config
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

    let config = TomlValue::Table(toml_config(dependency)).to_string();
    let mut config_file = try!(File::create(dir.join("Cargo.toml")));
    try!(config_file.write(config.as_bytes()));
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
