use std::fs::{File, Metadata, self};
use std::io::{ErrorKind, Read, Result as IoResult, Write};
use std::iter::FromIterator;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use regex::Regex;
use walkdir::{DirEntry, WalkDir};

static TEST_REGEX: &'static str =
    "#\\[test\\]\n(    fn ([^\\{]*)\\(\\) \\{(?s:.)*?\n    \\}\n)";

static TEMPLATE: &'static str = r##"
#[macro_use]
extern crate objc;

pub use objc::*;
use objc::runtime::*;

#[path = "../src/test_utils.rs"]
mod test_utils;
"##;

static EXPORT_MOD: &'static str = include_str!("export.rs");

struct TestModule {
    output: String,
    test_names: Vec<String>,
    re: Regex,
}

impl TestModule {
    fn new() -> TestModule {
        TestModule {
            output: TEMPLATE.to_owned(),
            test_names: Vec::new(),
            re: Regex::new(TEST_REGEX).unwrap(),
        }
    }

    fn add_tests(&mut self, src_file: &str) {
        for capture in self.re.captures_iter(src_file) {
            self.output.push_str("\n");
            self.output.push_str(&capture[1]);

            self.test_names.push(capture[2].to_owned());
        }
    }

    fn finish(self) -> String {
        use std::fmt::Write;

        let TestModule { mut output, test_names, .. } = self;

        output.push_str("\npub static TESTS: &'static [(&'static str, fn())] = &[\n");
        for test_name in test_names {
            write!(&mut output, "(\"{0}\", {0}),\n", test_name).unwrap();
        }
        output.push_str("];\n");

        output.push_str("pub mod export {\n");
        output.push_str(EXPORT_MOD);
        output.push_str("}\n");

        output
    }
}

impl FromIterator<String> for TestModule {
    fn from_iter<I>(iterator: I) -> Self where I: IntoIterator<Item=String> {
        let mut test_mod = TestModule::new();
        for file in iterator {
            test_mod.add_tests(&file);
        }
        test_mod
    }
}

fn has_rs_ext(path: &Path) -> bool {
    path.extension().and_then(|x| x.to_str()).map_or(false, |x| x == "rs")
}

fn modified_more_recently(m1: &Metadata, m2: &Metadata) -> bool {
    m1.mtime() > m2.mtime() ||
        (m1.mtime() == m2.mtime() && m1.mtime_nsec() > m2.mtime_nsec())
}

fn should_build(output: &Path, src_files: &[DirEntry]) -> bool {
    let output_metadata = match fs::metadata(output) {
        Ok(m) => m,
        Err(ref e) if e.kind() == ErrorKind::NotFound => return true,
        Err(e) => panic!("Error getting output file metadata: {:?}", e),
    };
    src_files.iter()
        .map(|e| e.metadata().unwrap())
        .any(|m| modified_more_recently(&m, &output_metadata))
}

fn read_file(path: &Path) -> IoResult<String> {
    let mut f = try!(File::open(path));
    let mut buf = String::new();
    try!(f.read_to_string(&mut buf));
    Ok(buf)
}

pub fn create_test_module(dir: &Path, src_dir: &Path) -> IoResult<()> {
    let output_path = dir.join("lib.rs");

    let mut src_files: Vec<_> = try!(WalkDir::new(src_dir).into_iter().collect());
    src_files.retain(|e| e.file_type().is_file() && has_rs_ext(e.path()));

    if !should_build(&output_path, &src_files) {
        return Ok(());
    }

    let src_contents = src_files.iter().map(|e| read_file(e.path()));
    let test_mod: TestModule = try!(src_contents.collect());
    let output = test_mod.finish();

    let mut output_file = try!(File::create(output_path));
    try!(output_file.write(output.as_bytes()));

    Ok(())
}
