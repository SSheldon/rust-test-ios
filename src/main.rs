#[macro_use]
extern crate lazy_static;
extern crate regex;
extern crate serde_json;
extern crate toml;
extern crate walkdir;

macro_rules! err {
    ($($e:expr),*) => ({
        return Err(From::from(format!($($e),*)));
    })
}

mod cargo;
mod tests;
mod xcode;

use std::env;
use std::error::Error;
use std::fs;

pub type BuildResult<T = ()> = Result<T, Box<Error>>;

fn main() {
    let crate_dir = env::current_dir().unwrap();

    let build_dir = crate_dir.join("tests-ios");
    fs::create_dir_all(&build_dir).unwrap();

    let src_dir = crate_dir.join("src");
    tests::create_test_module(&build_dir, &src_dir).unwrap();

    cargo::create_config(&build_dir, &crate_dir).unwrap();
    assert!(cargo::build(&build_dir).unwrap() == true);

    xcode::create_project(&build_dir).unwrap();
    assert!(xcode::run_tests(&build_dir).unwrap() == true);
}
