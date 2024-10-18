use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::value::Value;

fn current_time() -> Duration {
    let start = SystemTime::now();
    start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
}

pub fn current_time_s<'gc>(_: u8, _: usize) -> Value<'gc> {
    Value::Number(current_time().as_secs_f64())
}
