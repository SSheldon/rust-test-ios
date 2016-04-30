extern crate regex;
extern crate walkdir;

mod cargo;
mod tests;
mod xcode;

use std::env;

fn main() {
    let cwd = env::current_dir().unwrap();
    let src_dir = cwd.parent().unwrap().join("src");

    tests::create_test_module(&cwd, &src_dir).unwrap();

    cargo::create_config(&cwd).unwrap();
    assert!(cargo::build(&cwd).unwrap() == true);

    xcode::create_project(&cwd).unwrap();
    assert!(xcode::run_tests(&cwd).unwrap() == true);
}
