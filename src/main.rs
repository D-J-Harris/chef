use chunk::Chunk;
use chunk::Operation;
use value::Value;
use vm::Vm;

mod chunk;
mod value;
mod vm;

fn main() {
    let mut vm = Vm::new();
    let mut chunk = Chunk::new();
    let constant_index = chunk.add_constant(Value::Number(1.2_f64)).unwrap();
    chunk.write(Operation::Constant(constant_index), 1);
    let constant_index = chunk.add_constant(Value::Number(3.4_f64)).unwrap();
    chunk.write(Operation::Constant(constant_index), 1);
    chunk.write(Operation::Add, 1);

    let constant_index = chunk.add_constant(Value::Number(5.6_f64)).unwrap();
    chunk.write(Operation::Constant(constant_index), 1);
    chunk.write(Operation::Divide, 1);

    chunk.write(Operation::Negate, 146);
    chunk.write(Operation::Return, 146);
    let result = vm.run(&chunk);
    print!("{result:?}");
}
