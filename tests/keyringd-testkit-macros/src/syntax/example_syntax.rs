use syn::{Expr, LitStr};

pub(crate) struct ExampleSyntax {
    pub(crate) name: LitStr,
    pub(crate) value: Expr,
}
