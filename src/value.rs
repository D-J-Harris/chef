use rust_decimal::Decimal;

/// Value derives Copy to enable value copy across from chunks to the VM
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Value {
    Number(Decimal),
}

pub enum ValueNegationResult {
    Ok,
    Error,
}

impl Value {
    pub fn negate(&mut self) -> ValueNegationResult {
        match self {
            Value::Number(decimal) => match decimal.is_sign_positive() {
                true => decimal.set_sign_negative(true),
                false => decimal.set_sign_positive(true),
            },
        }
        ValueNegationResult::Ok
    }
}
