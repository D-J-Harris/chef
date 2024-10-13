use std::rc::Rc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::objects::NativeFunctionObject;
use crate::value::Value;
use crate::{objects::NativeFunction, vm::Vm};

impl Vm {
    pub fn declare_native_functions(&mut self) {
        self.declare_native_function("clock", current_time_s);
    }

    fn declare_native_function(&mut self, name: &str, function: NativeFunction) {
        let obj = NativeFunctionObject::new(name, function);
        self.identifiers
            .insert(name.into(), Value::NativeFunction(Rc::new(obj)));
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
