use chunk::Chunk;
use rust_decimal::Decimal;
use value::Value;

mod chunk;
mod value;

fn main() {
    let mut chunk = Chunk::new();
    chunk.write(chunk::Operation::Return, 23);
    if let Some(constant_index) = chunk.add_constant(Value::Number(Decimal::from(4))) {
        chunk.write(chunk::Operation::Constant(constant_index), 145);
    }
    chunk.disassemble();
}
