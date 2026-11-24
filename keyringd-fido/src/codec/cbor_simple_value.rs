#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CborSimpleValue {
    Other(u8),
    FalseValue,
    TrueValue,
    NullValue,
    Undefined,
}
