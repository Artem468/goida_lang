use std::collections::HashMap;
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

struct NativeText {
    bytes: Vec<u8>,
}

struct NativeList {
    items: Vec<i64>,
}

struct NativeArray {
    items: Box<[i64]>,
}

enum NativeDictValue {
    I64(i64),
    Text(String),
}

struct NativeDict {
    map: HashMap<String, NativeDictValue>,
}

unsafe fn as_ref<'a, T>(ptr: *mut c_void) -> Option<&'a T> {
    if ptr.is_null() {
        None
    } else {
        (ptr as *const T).as_ref()
    }
}

#[no_mangle]
pub extern "C" fn ffi_create_demo_text() -> *mut c_void {
    let data = NativeText {
        bytes: "привет из dll".as_bytes().to_vec(),
    };
    Box::into_raw(Box::new(data)) as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn ffi_text_len(ptr: *mut c_void) -> i64 {
    as_ref::<NativeText>(ptr)
        .map(|value| value.bytes.len() as i64)
        .unwrap_or(-1)
}

#[no_mangle]
pub unsafe extern "C" fn ffi_text_byte_at(ptr: *mut c_void, index: i64) -> i64 {
    if index < 0 {
        return i64::MIN;
    }
    as_ref::<NativeText>(ptr)
        .and_then(|value| value.bytes.get(index as usize))
        .map(|value| *value as i64)
        .unwrap_or(i64::MIN)
}

#[no_mangle]
pub unsafe extern "C" fn ffi_text_free(ptr: *mut c_void) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr as *mut NativeText));
    }
}

#[no_mangle]
pub extern "C" fn ffi_create_list3(a: i64, b: i64, c: i64) -> *mut c_void {
    let data = NativeList {
        items: vec![a, b, c],
    };
    Box::into_raw(Box::new(data)) as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn ffi_list_len(ptr: *mut c_void) -> i64 {
    as_ref::<NativeList>(ptr)
        .map(|value| value.items.len() as i64)
        .unwrap_or(-1)
}

#[no_mangle]
pub unsafe extern "C" fn ffi_list_i64_at(ptr: *mut c_void, index: i64) -> i64 {
    if index < 0 {
        return i64::MIN;
    }
    as_ref::<NativeList>(ptr)
        .and_then(|value| value.items.get(index as usize))
        .copied()
        .unwrap_or(i64::MIN)
}

#[no_mangle]
pub unsafe extern "C" fn ffi_list_free(ptr: *mut c_void) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr as *mut NativeList));
    }
}

#[no_mangle]
pub extern "C" fn ffi_create_array3(a: i64, b: i64, c: i64) -> *mut c_void {
    let data = NativeArray {
        items: vec![a, b, c].into_boxed_slice(),
    };
    Box::into_raw(Box::new(data)) as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn ffi_array_len(ptr: *mut c_void) -> i64 {
    as_ref::<NativeArray>(ptr)
        .map(|value| value.items.len() as i64)
        .unwrap_or(-1)
}

#[no_mangle]
pub unsafe extern "C" fn ffi_array_i64_at(ptr: *mut c_void, index: i64) -> i64 {
    if index < 0 {
        return i64::MIN;
    }
    as_ref::<NativeArray>(ptr)
        .and_then(|value| value.items.get(index as usize))
        .copied()
        .unwrap_or(i64::MIN)
}

#[no_mangle]
pub unsafe extern "C" fn ffi_array_free(ptr: *mut c_void) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr as *mut NativeArray));
    }
}

#[no_mangle]
pub extern "C" fn ffi_create_demo_dict() -> *mut c_void {
    let mut map = HashMap::new();
    map.insert("ключ".to_string(), NativeDictValue::I64(42));
    map.insert(
        "текст".to_string(),
        NativeDictValue::Text("значение".to_string()),
    );
    Box::into_raw(Box::new(NativeDict { map })) as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn ffi_dict_len(ptr: *mut c_void) -> i64 {
    as_ref::<NativeDict>(ptr)
        .map(|value| value.map.len() as i64)
        .unwrap_or(-1)
}

#[no_mangle]
pub unsafe extern "C" fn ffi_dict_get_i64_known_key(ptr: *mut c_void, key_id: i64) -> i64 {
    let key = match key_id {
        1 => "ключ",
        _ => return i64::MIN,
    };
    match as_ref::<NativeDict>(ptr).and_then(|value| value.map.get(key)) {
        Some(NativeDictValue::I64(number)) => *number,
        _ => i64::MIN,
    }
}

#[no_mangle]
pub unsafe extern "C" fn ffi_dict_get_text_len_known_key(ptr: *mut c_void, key_id: i64) -> i64 {
    let key = match key_id {
        2 => "текст",
        _ => return i64::MIN,
    };
    match as_ref::<NativeDict>(ptr).and_then(|value| value.map.get(key)) {
        Some(NativeDictValue::Text(text)) => text.len() as i64,
        _ => i64::MIN,
    }
}

#[no_mangle]
pub unsafe extern "C" fn ffi_dict_free(ptr: *mut c_void) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr as *mut NativeDict));
    }
}
