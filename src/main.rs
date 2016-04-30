extern crate regex;
extern crate serde_json;
extern crate walkdir;

mod cargo;
mod tests;
mod xcode;

use std::env;
use std::fs;

static TESTS_PRELUDE: &'static str = r##"
#[macro_use]
extern crate objc;

pub use objc::*;
use objc::runtime::*;

#[path = "../src/test_utils.rs"]
mod test_utils;
"##;

fn main() {
    let crate_dir = env::current_dir().unwrap();

    let build_dir = crate_dir.join("tests-ios");
    fs::create_dir_all(&build_dir).unwrap();

    let src_dir = crate_dir.join("src");
    let prelude = TESTS_PRELUDE.to_owned();
    tests::create_test_module(&build_dir, &src_dir, prelude).unwrap();

    let dep = cargo::Dependency {
        name: "objc",
        path: &crate_dir,
        features: &[&"exception"],
    };
    cargo::create_config(&build_dir, dep).unwrap();
    assert!(cargo::build(&build_dir).unwrap() == true);

    xcode::create_project(&build_dir).unwrap();
    assert!(xcode::run_tests(&build_dir).unwrap() == true);
}
