use chunk::Chunk;
use rust_decimal::Decimal;
use value::Value;

mod chunk;
mod value;

fn main() {
    let mut chunk = Chunk::new();
    chunk.write(chunk::Operation::Return);
    let constant_index = chunk.add_constant(Value::Number(Decimal::from(4)));
    chunk.write(chunk::Operation::Constant(constant_index));
    let constant_index = chunk.add_constant(Value::Number(Decimal::from(5)));
    chunk.write(chunk::Operation::Constant(constant_index));
    let constant_index = chunk.add_constant(Value::Number(Decimal::from(4)));
    chunk.write(chunk::Operation::Constant(constant_index));
    chunk.disassemble();
}
