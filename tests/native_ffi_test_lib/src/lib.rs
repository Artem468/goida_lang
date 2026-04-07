use std::ffi::c_void;

#[no_mangle]
pub static mut COUNTER: i64 = 3;

#[no_mangle]
pub static mut RATIO: f64 = 1.5;

#[no_mangle]
pub static mut HANDLE: *mut c_void = std::ptr::null_mut();

#[no_mangle]
pub extern "C" fn add(a: i64, b: i64) -> i64 {
    a + b
}

#[no_mangle]
pub extern "C" fn minus(a: i64, b: i64) -> i64 {
    a - b
}

#[no_mangle]
pub extern "C" fn add_f64(a: f64, b: f64) -> f64 {
    a + b
}

#[no_mangle]
pub extern "C" fn identity_ptr(value: *mut c_void) -> *mut c_void {
    value
}

#[no_mangle]
pub extern "C" fn make_ptr() -> *mut c_void {
    0x1234usize as *mut c_void
}

