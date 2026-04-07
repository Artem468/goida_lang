use crate::interpreter::prelude::Value;
use std::ffi::c_void;
use std::ptr;

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoidaFfiTag {
    Empty = 0,
    Number = 1,
    Float = 2,
    Boolean = 3,
    Boxed = 4,
}

#[repr(C)]
#[derive(Debug)]
pub struct GoidaFfiValue {
    pub tag: u32,
    pub int_value: i64,
    pub float_value: f64,
    pub ptr_value: *mut c_void,
}

impl GoidaFfiValue {
    pub const fn empty() -> Self {
        Self {
            tag: GoidaFfiTag::Empty as u32,
            int_value: 0,
            float_value: 0.0,
            ptr_value: ptr::null_mut(),
        }
    }

    pub const fn number(value: i64) -> Self {
        Self {
            tag: GoidaFfiTag::Number as u32,
            int_value: value,
            float_value: 0.0,
            ptr_value: ptr::null_mut(),
        }
    }

    pub const fn float(value: f64) -> Self {
        Self {
            tag: GoidaFfiTag::Float as u32,
            int_value: 0,
            float_value: value,
            ptr_value: ptr::null_mut(),
        }
    }

    pub const fn boolean(value: bool) -> Self {
        Self {
            tag: GoidaFfiTag::Boolean as u32,
            int_value: value as i64,
            float_value: 0.0,
            ptr_value: ptr::null_mut(),
        }
    }

    pub fn from_value(value: Value) -> Self {
        match value {
            Value::Empty => Self::empty(),
            Value::Number(value) => Self::number(value),
            Value::Float(value) => Self::float(value),
            Value::Boolean(value) => Self::boolean(value),
            other => Self {
                tag: GoidaFfiTag::Boxed as u32,
                int_value: 0,
                float_value: 0.0,
                ptr_value: Box::into_raw(Box::new(other)) as *mut c_void,
            },
        }
    }

    pub fn boxed_result(value: Value) -> *mut GoidaFfiValue {
        Box::into_raw(Box::new(Self::from_value(value)))
    }

    pub unsafe fn clone_value(&self) -> Value {
        match self.tag {
            x if x == GoidaFfiTag::Empty as u32 => Value::Empty,
            x if x == GoidaFfiTag::Number as u32 => Value::Number(self.int_value),
            x if x == GoidaFfiTag::Float as u32 => Value::Float(self.float_value),
            x if x == GoidaFfiTag::Boolean as u32 => Value::Boolean(self.int_value != 0),
            x if x == GoidaFfiTag::Boxed as u32 => {
                let value = self.ptr_value as *const Value;
                assert!(!value.is_null(), "boxed ffi value pointer must not be null");
                (*value).clone()
            }
            _ => panic!("unknown ffi tag {}", self.tag),
        }
    }

    pub unsafe fn into_value(self) -> Value {
        match self.tag {
            x if x == GoidaFfiTag::Empty as u32 => Value::Empty,
            x if x == GoidaFfiTag::Number as u32 => Value::Number(self.int_value),
            x if x == GoidaFfiTag::Float as u32 => Value::Float(self.float_value),
            x if x == GoidaFfiTag::Boolean as u32 => Value::Boolean(self.int_value != 0),
            x if x == GoidaFfiTag::Boxed as u32 => {
                let value = self.ptr_value as *mut Value;
                assert!(!value.is_null(), "boxed ffi value pointer must not be null");
                *Box::from_raw(value)
            }
            _ => panic!("unknown ffi tag {}", self.tag),
        }
    }

    pub unsafe fn write_value(&mut self, value: Value) {
        self.release_boxed();
        *self = Self::from_value(value);
    }

    pub unsafe fn release_boxed(&mut self) {
        if self.tag == GoidaFfiTag::Boxed as u32 && !self.ptr_value.is_null() {
            drop(Box::from_raw(self.ptr_value as *mut Value));
            self.ptr_value = ptr::null_mut();
        }
    }
}
