#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CoseEllipticCurve(i64);

impl CoseEllipticCurve {
    pub const P256: Self = Self(1);

    pub const fn new(value: i64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> i64 {
        self.0
    }
}
