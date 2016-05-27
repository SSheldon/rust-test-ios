use std::os::raw::{c_char, c_int};
use std::panic;
use super::TESTS;

#[no_mangle]
pub extern fn tests_count() -> usize {
    TESTS.len()
}

#[no_mangle]
pub extern fn test_name(i: usize, len: &mut usize) -> *const c_char {
    let (name, _) = TESTS[i];
    *len = name.len();
    name.as_ptr() as *const c_char
}

#[no_mangle]
pub extern fn run_test(i: usize) -> c_int {
    let (_, test_fn) = TESTS[i];
    let result = panic::catch_unwind(|| {
        test_fn();
    });
    if result.is_ok() { 1 } else { 0 }
}
