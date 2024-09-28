use chunk::Chunk;
use chunk::Operation;
use rust_decimal::Decimal;
use value::Value;
use vm::Vm;

mod chunk;
mod value;
mod vm;

fn main() {
    let mut vm = Vm::new();
    let mut chunk = Chunk::new();
    if let Some(constant_index) = chunk.add_constant(Value::Number(Decimal::from(4))) {
        chunk.write(Operation::Constant(constant_index), 145);
    }
    chunk.write(Operation::Negation, 146);
    chunk.write(Operation::Return, 146);
    let result = vm.run(&chunk);
    print!("{result:?}");
}
