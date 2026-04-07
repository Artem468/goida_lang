use goida_core::ffi::GoidaFfiValue;
use goida_core::interpreter::prelude::Value;

#[no_mangle]
pub static mut counter: GoidaFfiValue = GoidaFfiValue::number(3);

#[no_mangle]
pub extern "C" fn add(a: *const GoidaFfiValue, b: *const GoidaFfiValue) -> *mut GoidaFfiValue {
    let left = unsafe { (*a).clone_value() };
    let right = unsafe { (*b).clone_value() };

    let Value::Number(left) = left else {
        panic!("left argument must be a number");
    };
    let Value::Number(right) = right else {
        panic!("right argument must be a number");
    };

    GoidaFfiValue::boxed_result(Value::Number(left + right))
}

#[no_mangle]
pub extern "C" fn identity(value: *const GoidaFfiValue) -> *mut GoidaFfiValue {
    let value = unsafe { (*value).clone_value() };
    GoidaFfiValue::boxed_result(value)
}


#[no_mangle]
pub extern "C" fn minus(a: *const GoidaFfiValue, b: *const GoidaFfiValue) -> *mut GoidaFfiValue {
    let left = unsafe { (*a).clone_value() };
    let right = unsafe { (*b).clone_value() };

    let Value::Number(left) = left else {
        panic!("left argument must be a number");
    };
    let Value::Number(right) = right else {
        panic!("right argument must be a number");
    };

    GoidaFfiValue::boxed_result(Value::Number(left - right))
}