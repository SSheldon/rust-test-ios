extern crate regex;
extern crate walkdir;

mod cargo;
mod tests;
mod xcode;

use std::env;

static TESTS_PRELUDE: &'static str = r##"
#[macro_use]
extern crate objc;

pub use objc::*;
use objc::runtime::*;

#[path = "../src/test_utils.rs"]
mod test_utils;
"##;

fn main() {
    let cwd = env::current_dir().unwrap();
    let src_dir = cwd.parent().unwrap().join("src");

    let prelude = TESTS_PRELUDE.to_owned();
    tests::create_test_module(&cwd, &src_dir, prelude).unwrap();

    cargo::create_config(&cwd).unwrap();
    assert!(cargo::build(&cwd).unwrap() == true);

    xcode::create_project(&cwd).unwrap();
    assert!(xcode::run_tests(&cwd).unwrap() == true);
}
