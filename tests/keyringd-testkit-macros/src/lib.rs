use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Expr, LitStr, Result, parse_macro_input};

mod expansion;
mod syntax;
mod test_function_name;
mod validation;

use syntax::ScenarioSyntax;

mod evidence_syntax;
use evidence_syntax::EvidenceSyntax;

#[proc_macro]
pub fn scenario(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ScenarioSyntax);
    validation::validate(&input)
        .and_then(|()| expansion::expand_suite(input))
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro]
pub fn evidence_text(input: TokenStream) -> TokenStream {
    expand_evidence(input, quote!(attach_text))
}

#[proc_macro]
pub fn evidence_log(input: TokenStream) -> TokenStream {
    expand_evidence(input, quote!(attach_log))
}

#[proc_macro]
pub fn evidence_bytes(input: TokenStream) -> TokenStream {
    expand_evidence(input, quote!(attach_bytes))
}

#[proc_macro]
pub fn evidence_file(input: TokenStream) -> TokenStream {
    expand_evidence(input, quote!(attach_file))
}

#[proc_macro]
pub fn evidence_image(input: TokenStream) -> TokenStream {
    expand_evidence(input, quote!(attach_image))
}

fn expand_evidence(input: TokenStream, method: TokenStream2) -> TokenStream {
    let input = parse_macro_input!(input as EvidenceSyntax);
    let name = input.name;
    let value = input.value;
    quote!(case_execution.#method(#name, #value)).into()
}

mod test_expansion;
use test_expansion::TestExpansion;
