use rust_decimal::Decimal;

#[derive(Debug, PartialEq, Eq)]
pub enum Value {
    Number(Decimal),
}
