use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::value::Value;

pub type NativeFunction = fn(arg_count: u8, ip: usize) -> Value;

const NATIVE_FUNCTION_COUNT: usize = 2;

pub fn declare_native_functions() -> [(&'static str, NativeFunction); NATIVE_FUNCTION_COUNT] {
    [("time", current_time_s), ("stir", do_nothing)]
}

fn current_time() -> Duration {
    let start = SystemTime::now();
    start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
}

fn current_time_s(_: u8, _: usize) -> Value {
    Value::Number(current_time().as_secs_f64().floor())
}

fn do_nothing(_: u8, _: usize) -> Value {
    Value::Nil
}
