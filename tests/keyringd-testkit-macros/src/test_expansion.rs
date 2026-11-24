use proc_macro2::TokenStream as TokenStream2;
use syn::{Expr, LitStr};

pub(super) struct TestExpansion {
    pub(super) attributes: Vec<syn::Attribute>,
    pub(super) function_name: syn::Ident,
    pub(super) case_name: LitStr,
    pub(super) rationale: Option<LitStr>,
    pub(super) specifications: Vec<LitStr>,
    pub(super) source_start_line: u32,
    pub(super) source_end_line: u32,
    pub(super) source_column: u32,
    pub(super) asynchronous: bool,
    pub(super) runtime_arguments: TokenStream2,
    pub(super) timeout: Option<Expr>,
    pub(super) ignored: Option<LitStr>,
    pub(super) skip: Option<(Expr, LitStr)>,
    pub(super) example: Option<Expr>,
    pub(super) group_name: Option<LitStr>,
    pub(super) body: TokenStream2,
}
