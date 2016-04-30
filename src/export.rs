use std::os::raw::c_char;
use super::TESTS;

#[no_mangle]
pub extern fn tests_count() -> usize {
    TESTS.len()
}

#[no_mangle]
pub extern fn test_name(i: usize, len: &mut usize) -> *const c_char {
    let (name, _) = TESTS[i as usize];
    *len = name.len();
    name.as_ptr() as *const c_char
}

#[no_mangle]
pub extern fn run_test(i: usize) {
    let (_, test_fn) = TESTS[i as usize];
    test_fn();
}
