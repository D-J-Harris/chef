use std::rc::Rc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::objects::{NativeFunctionObject, Object};
use crate::{objects::NativeFunction, value::Value, vm::Vm};

impl Vm {
    pub fn declare_native_functions(&mut self) {
        self.declare_native_function("current_time_s", current_time_s);
        self.declare_native_function("current_time_ms", current_time_ms);
    }

    fn declare_native_function(&mut self, name: &str, function: NativeFunction) {
        let obj = NativeFunctionObject::new(name.into(), function);
        self.identifiers.insert(
            name.into(),
            Value::ObjectValue(Object::NativeFunction(Rc::new(obj))),
        );
    }
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

pub fn current_time_ms(_: u8, _: usize) -> Value {
    Value::Number(current_time().as_secs_f64() * 1000_f64)
}
