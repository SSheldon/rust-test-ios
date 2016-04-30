use std::io::{Result as IoResult, Write};
use std::fs::{File, self};
use std::path::Path;
use std::process::Command;

static PROJECT: &'static str =
    include_str!("../RustTests.xcodeproj/project.pbxproj");

static PROJECT_WORKSPACE: &'static str =
    include_str!("../RustTests.xcodeproj/project.xcworkspace/contents.xcworkspacedata");

static PROJECT_SCHEME: &'static str =
    include_str!("../RustTests.xcodeproj/xcshareddata/xcschemes/RustTests.xcscheme");

static TEST_CASE: &'static str = include_str!("../RustTests.m");

pub fn create_project(dir: &Path) -> IoResult<()> {
    let proj_dir = dir.join("RustTests.xcodeproj");
    try!(fs::create_dir_all(&proj_dir));

    let mut proj_file = try!(File::create(proj_dir.join("project.pbxproj")));
    try!(proj_file.write(PROJECT.as_bytes()));

    let workspace_dir = proj_dir.join("project.xcworkspace");
    try!(fs::create_dir_all(&workspace_dir));

    let mut workspace_file = try!(File::create(workspace_dir.join("contents.xcworkspacedata")));
    try!(workspace_file.write(PROJECT_WORKSPACE.as_bytes()));

    let scheme_dir = proj_dir.join("xcshareddata").join("xcschemes");
    try!(fs::create_dir_all(&scheme_dir));

    let mut scheme_file = try!(File::create(scheme_dir.join("RustTests.xcscheme")));
    try!(scheme_file.write(PROJECT_SCHEME.as_bytes()));

    let mut test_file = try!(File::create(dir.join("RustTests.m")));
    try!(test_file.write(TEST_CASE.as_bytes()));

    Ok(())
}

pub fn run_tests(dir: &Path) -> IoResult<bool> {
    let result = Command::new("xcodebuild")
        .arg("-project").arg(&dir.join("RustTests.xcodeproj"))
        .arg("-scheme").arg("RustTests")
        .arg("-destination").arg("platform=iOS Simulator,name=iPhone 5")
        .arg("-destination").arg("platform=iOS Simulator,name=iPhone 5s")
        .arg("test")
        .status();
    result.map(|s| s.success())
}
