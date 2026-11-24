use super::*;
use syn::{Expr, Ident, LitStr};

pub(crate) struct CaseSyntax {
    pub(crate) attributes: Vec<syn::Attribute>,
    pub(crate) case_name: LitStr,
    pub(crate) rationale: Option<LitStr>,
    pub(crate) specifications: Vec<LitStr>,
    pub(crate) source_start_line: u32,
    pub(crate) source_end_line: u32,
    pub(crate) source_column: u32,
    pub(crate) runtime: Option<Ident>,
    pub(crate) runtime_arguments: proc_macro2::TokenStream,
    pub(crate) timeout: Option<Expr>,
    pub(crate) ignored: Option<LitStr>,
    pub(crate) examples: Vec<ExampleSyntax>,
    pub(crate) skip: Option<(Expr, LitStr)>,
    pub(crate) steps: Vec<StepSyntax>,
}
