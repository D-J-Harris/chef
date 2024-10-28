use std::{
    collections::HashMap,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use crate::value::Value;

pub type NativeFunction = fn(arg_count: u8, ip: usize) -> Value;

pub const NATIVE_FUNCTION_COUNT: usize = 1;

pub fn declare_native_functions(
    globals: &mut HashMap<String, Value>,
) -> [NativeFunction; NATIVE_FUNCTION_COUNT] {
    let name = "clock";
    let native_function_clock = current_time_s;
    let value = Value::NativeFunction(native_function_clock);
    globals.insert(name.into(), value);

    [native_function_clock]
}
fn current_time() -> Duration {
    let start = SystemTime::now();
    start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
}

pub fn current_time_s(_: u8, _: usize) -> Value {
    Value::Number(current_time().as_secs_f64())
}
