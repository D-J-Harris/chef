use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use gc_arena::{Gc, Mutation};

use crate::objects::NativeFunction;
use crate::objects::NativeFunctionObject;
use crate::value::Value;

pub fn declare_native_functions<'gc>(
    mc: &'gc Mutation<'gc>,
    identifiers: &HashMap<Gc<'gc, String>, Value<'gc>>,
) {
    declare_native_function(mc, identifiers, "clock", current_time_s);
}

fn declare_native_function<'gc>(
    mc: &'gc Mutation,
    identifiers: &HashMap<Gc<'gc, String>, Value<'gc>>,
    name: &'static str,
    function: NativeFunction,
) {
    let key = Gc::new(mc, name.into());
    let value = NativeFunctionObject::new(name, function);
    identifiers.insert(key, Value::NativeFunction(Gc::new(mc, value)));
}

fn current_time() -> Duration {
    let start = SystemTime::now();
    start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
}

pub fn current_time_s<'gc>(_: u8, _: usize) -> Value<'gc> {
    Value::Number(current_time().as_secs_f64())
}
