use std::io::{Result as IoResult, Write};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;

use regex::Regex;
use serde_json::{Value, self};
use toml::{Table, Value as TomlValue};

use BuildResult;

static ARCHS: [&'static str; 5] = [
    "i386",
    "x86_64",
    "armv7",
    "armv7s",
    "aarch64",
];

enum DependencySource {
    Local(PathBuf),
    Remote(String),
}

impl DependencySource {
    fn from_metadata(package: Value) -> BuildResult<DependencySource> {
        let mut obj = match package {
            Value::Object(o) => o,
            _ => err!("metadata package was not a JSON object"),
        };
        let id = match obj.remove("id") {
            Some(Value::String(s)) => s,
            _ => err!("metadata package did not include key \"id\""),
        };

        lazy_static! {
            static ref LOCAL_ID_REGEX: Regex =
                Regex::new("\\(path\\+file://(.*)\\)$").unwrap();
            static ref REMOTE_ID_REGEX: Regex =
                Regex::new("\\(registry\\+https://github.com/rust-lang/crates.io-index\\)$").unwrap();
        }

        if LOCAL_ID_REGEX.is_match(&id) {
            let mut path = match obj.remove("manifest_path") {
                Some(Value::String(s)) => PathBuf::from(s),
                _ => err!("metadata package did not include key \"manifest_path\""),
            };
            path.pop();
            Ok(DependencySource::Local(path))
        } else if REMOTE_ID_REGEX.is_match(&id) {
            let version = match obj.remove("version") {
                Some(Value::String(s)) => s,
                _ => err!("metadata package did not include key \"version\""),
            };
            Ok(DependencySource::Remote(version))
        } else {
            err!("Unsupported source type for {}", id);
        }
    }
}

struct Dependency {
    name: String,
    source: DependencySource,
    features: Vec<String>,
}

impl Dependency {
    fn into_toml(self) -> (String, Table) {
        let Dependency { name, source, features } = self;
        let mut dep = Table::new();
        match source {
            DependencySource::Local(path) => {
                let path = path.into_os_string().into_string().unwrap();
                dep.insert("path".to_owned(), TomlValue::String(path));
            }
            DependencySource::Remote(version) => {
                dep.insert("version".to_owned(), TomlValue::String(version));
            }
        }
        if features.len() > 0 {
            let features = features.into_iter()
                .map(TomlValue::String)
                .collect();
            dep.insert("features".to_owned(), TomlValue::Array(features));
        }
        (name, dep)
    }
}

struct Config {
    crate_dep: Dependency,
    dev_deps: Vec<Dependency>,
}

impl Config {
    fn into_toml(self) -> Table {
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
        let (name, crate_dep) = self.crate_dep.into_toml();
        dependencies.insert(name, TomlValue::Table(crate_dep));
        for dep in self.dev_deps {
            let (name, dep) = dep.into_toml();
            dependencies.insert(name, TomlValue::Table(dep));
        }
        config.insert("dependencies".to_owned(), TomlValue::Table(dependencies));

        config
    }

    fn into_string(self) -> String {
        TomlValue::Table(self.into_toml()).to_string()
    }
}

fn filter_result<T, E>(res: Result<Option<T>, E>) -> Option<Result<T, E>> {
    match res {
        Ok(Some(t)) => Some(Ok(t)),
        Ok(None) => None,
        Err(e) => Some(Err(e)),
    }
}

fn read_package(package: Value, dev_deps: &mut Vec<String>)
        -> BuildResult<Option<Dependency>> {
    let position = {
        let name = match package.find("name") {
            Some(&Value::String(ref s)) => s,
            _ => err!("metadata package did not include key \"name\""),
        };
        dev_deps.iter().position(|n| n == name)
    };
    let name = match position {
        Some(i) => dev_deps.swap_remove(i),
        None => return Ok(None),
    };
    let source = try!(DependencySource::from_metadata(package));
    Ok(Some(Dependency {
        name: name,
        source: source,
        features: Vec::new(),
    }))
}

fn read_deps_metadata(crate_dir: &Path, dev_deps: Vec<String>)
        -> BuildResult<Vec<Dependency>> {
    if dev_deps.len() == 0 {
        return Ok(Vec::new());
    }

    let out = Command::new("cargo")
        .arg("metadata")
        .arg("--manifest-path").arg(&crate_dir.join("Cargo.toml"))
        .output();
    let out = try!(out);
    if !out.status.success() {
        err!("cargo metadata failed with status {}", out.status);
    }

    let value: Value = try!(serde_json::from_slice(&out.stdout));
    let mut obj = match value {
        Value::Object(o) => o,
        _ => err!("crate metadata was not a JSON object"),
    };
    let packages = match obj.remove("packages") {
        Some(Value::Array(a)) => a,
        _ => err!("crate metadata did not include \"packages\""),
    };

    let mut dev_deps = dev_deps;
    packages.into_iter()
        .filter_map(|p| filter_result(read_package(p, &mut dev_deps)))
        .collect()
}

fn read_dev_dep(dep: Value) -> BuildResult<Option<String>> {
    let mut dep_obj = match dep {
        Value::Object(o) => o,
        _ => err!("Dependency from cargo read-manifest was not a JSON object"),
    };
    match dep_obj.get("kind") {
        Some(&Value::String(ref s)) if s == "dev" => (),
        None | Some(&Value::Null) | Some(&Value::String(_)) => return Ok(None),
        Some(v) => err!("Dependency \"kind\" not a string: {:?}", v),
    }
    match dep_obj.remove("name") {
        Some(Value::String(s)) => Ok(Some(s)),
        Some(v) => err!("Dependency \"name\" not a string: {:?}", v),
        None => err!("Missing \"name\" in dependency: {:?}", dep_obj),
    }
}

fn read_config(crate_dir: &Path) -> BuildResult<Config> {
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
    let name = match obj.remove("name") {
        Some(Value::String(s)) => s,
        _ => err!("crate manifest did not include key \"name\""),
    };
    let crate_dep = Dependency {
        name: name,
        source: DependencySource::Local(crate_dir.to_owned()),
        features: Vec::new(),
    };

    let deps = match obj.remove("dependencies") {
        Some(Value::Array(a)) => a,
        _ => Vec::new(),
    };
    let dev_deps: Vec<_> = try!(deps.into_iter()
        .filter_map(|d| filter_result(read_dev_dep(d)))
        .collect());
    let dev_deps = try!(read_deps_metadata(crate_dir, dev_deps));

    Ok(Config { crate_dep: crate_dep, dev_deps: dev_deps })
}

pub fn create_config(dir: &Path, crate_dir: &Path) -> BuildResult {
    let config = try!(read_config(crate_dir));
    let config = config.into_string();
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
