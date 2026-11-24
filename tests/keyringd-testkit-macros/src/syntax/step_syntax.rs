use proc_macro2::TokenStream as TokenStream2;

use super::*;
use syn::LitStr;

pub(crate) struct StepSyntax {
    pub(crate) kind: StepKindSyntax,
    pub(crate) description: LitStr,
    pub(crate) body: TokenStream2,
}
