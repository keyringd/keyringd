use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, quote_spanned};
use syn::{LitStr, Result};

use crate::syntax::{
    CaseSyntax, GroupSyntax, ScenarioSyntax, ScenarioSyntaxItem, StepKindSyntax, StepSyntax,
};
use crate::test_function_name::test_function_name;

pub(crate) fn expand_suite(input: ScenarioSyntax) -> Result<TokenStream2> {
    let items = expand_items(input.items, Vec::new())?;
    Ok(quote!(#(#items)*))
}

fn expand_items(
    items: Vec<ScenarioSyntaxItem>,
    group_path: Vec<LitStr>,
) -> Result<Vec<TokenStream2>> {
    items
        .into_iter()
        .map(|item| match item {
            ScenarioSyntaxItem::Case(case) => expand(*case, group_name(&group_path)),
            ScenarioSyntaxItem::Group(group) => expand_group(group, &group_path),
        })
        .collect()
}

fn expand_group(input: GroupSyntax, parent_path: &[LitStr]) -> Result<TokenStream2> {
    let mut group_path = parent_path.to_vec();
    group_path.push(input.title);
    let items = expand_items(input.items, group_path)?;
    Ok(quote!(#(#items)*))
}

fn group_name(group_path: &[LitStr]) -> Option<LitStr> {
    let first = group_path.first()?;
    Some(LitStr::new(
        &group_path
            .iter()
            .map(LitStr::value)
            .collect::<Vec<_>>()
            .join(" / "),
        first.span(),
    ))
}

fn expand(input: CaseSyntax, group_name: Option<LitStr>) -> Result<TokenStream2> {
    let asynchronous = input.runtime.is_some();
    let base_function_name = test_function_name(&input.case_name)?;
    let source_start_line = input.source_start_line;
    let source_end_line = input.source_end_line;
    let source_column = input.source_column;
    let body = expand_steps(input.steps);
    if input.examples.is_empty() {
        Ok(expand_test(TestExpansion {
            attributes: input.attributes,
            function_name: base_function_name,
            case_name: input.case_name,
            rationale: input.rationale,
            specifications: input.specifications,
            source_start_line,
            source_end_line,
            source_column,
            asynchronous,
            runtime_arguments: input.runtime_arguments,
            timeout: input.timeout,
            ignored: input.ignored,
            skip: input.skip,
            example: None,
            group_name,
            body,
        }))
    } else {
        let tests = input.examples.into_iter().map(|example| {
            let example_function_name = test_function_name(&example.name)?;
            let function_name = format_ident!(
                "{}_{}",
                base_function_name,
                example_function_name,
                span = example.name.span()
            );
            let title = LitStr::new(
                &format!("{} [{}]", input.case_name.value(), example.name.value()),
                input.case_name.span(),
            );
            Ok(expand_test(TestExpansion {
                attributes: input.attributes.clone(),
                function_name,
                case_name: title,
                rationale: input.rationale.clone(),
                specifications: input.specifications.clone(),
                source_start_line,
                source_end_line,
                source_column,
                asynchronous,
                runtime_arguments: input.runtime_arguments.clone(),
                timeout: input.timeout.clone(),
                ignored: input.ignored.clone(),
                skip: input.skip.clone(),
                example: Some(example.value),
                group_name: group_name.clone(),
                body: body.clone(),
            }))
        });
        let tests = tests.collect::<Result<Vec<_>>>()?;
        Ok(quote!(#(#tests)*))
    }
}

use crate::TestExpansion;

fn expand_test(input: TestExpansion) -> TokenStream2 {
    let TestExpansion {
        attributes,
        function_name,
        case_name,
        rationale,
        specifications,
        source_start_line,
        source_end_line,
        source_column,
        asynchronous,
        runtime_arguments,
        timeout,
        ignored,
        skip,
        example,
        group_name,
        body,
    } = input;
    let group_name = group_name
        .map(|name| quote!(Some(#name)))
        .unwrap_or_else(|| quote!(None));
    let rationale = rationale
        .map(|rationale| quote!(Some(#rationale)))
        .unwrap_or_else(|| quote!(None));
    let ignored_metadata = ignored.as_ref().map(|reason| {
        let metadata_function_name = format_ident!("{}_ignored_case_metadata", function_name);
        quote! {
            #(#attributes)*
            #[test]
            fn #metadata_function_name() {
                let mut case_execution = ::keyringd_testkit::__private::begin_case(
                    ::keyringd_testkit::__private::CaseMetadata {
                        id: concat!(env!("CARGO_PKG_NAME"), "::", module_path!(), "::", stringify!(#function_name)),
                        title: #case_name,
                        rationale: #rationale,
                        specifications: &[#(#specifications),*],
                        source_end_line: #source_end_line,
                        crate_name: env!("CARGO_PKG_NAME"),
                        module_path: module_path!(),
                        group: #group_name,
                        source: ::keyringd_testkit::__private::SourceLocation::from_manifest(
                            env!("CARGO_MANIFEST_DIR"),
                            file!(),
                            #source_start_line,
                            #source_column,
                        ),
                    },
                );
                case_execution.skip(#reason);
                ::keyringd_testkit::__private::complete_case(case_execution, Ok(()));
            }
        }
    });
    let ignore_attribute = ignored.map(|reason| quote!(#[ignore = #reason]));
    let case_initialization = quote! {
        let mut case_execution = ::keyringd_testkit::__private::begin_case(
            ::keyringd_testkit::__private::CaseMetadata {
                id: concat!(env!("CARGO_PKG_NAME"), "::", module_path!(), "::", stringify!(#function_name)),
                title: #case_name,
                rationale: #rationale,
                specifications: &[#(#specifications),*],
                source_end_line: #source_end_line,
                crate_name: env!("CARGO_PKG_NAME"),
                module_path: module_path!(),
                group: #group_name,
                source: ::keyringd_testkit::__private::SourceLocation::from_manifest(
                    env!("CARGO_MANIFEST_DIR"),
                    file!(),
                    #source_start_line,
                    #source_column,
                ),
            },
        );
    };
    let example = example.map(|value| quote!(let example = #value;));
    let skip = skip.map(|(condition, reason)| {
        let captured_condition = if asynchronous {
            quote!(::keyringd_testkit::__private::capture_async(async { #condition }).await)
        } else {
            quote!(::keyringd_testkit::__private::capture(|| #condition))
        };
        quote! {
            match #captured_condition {
                Ok(true) => {
                    case_execution.skip(#reason);
                    ::keyringd_testkit::__private::complete_case(case_execution, Ok(()));
                    return;
                }
                Ok(false) => {}
                Err(payload) => {
                    ::keyringd_testkit::__private::complete_case(case_execution, Err(payload));
                    return;
                }
            }
        }
    });
    if asynchronous {
        let outcome = if let Some(timeout) = timeout {
            quote! {
                ::keyringd_testkit::__private::capture_async_with_timeout(
                    #timeout,
                    async { #example #body },
                )
                .await
            }
        } else {
            quote! {
                ::keyringd_testkit::__private::capture_async(async { #example #body }).await
            }
        };
        quote! {
            #(#attributes)*
            #[::tokio::test(#runtime_arguments)]
            #ignore_attribute
            async fn #function_name() {
                #case_initialization
                #skip
                let outcome = ::keyringd_testkit::__private::EvidenceScope::asynchronous(async { #outcome }).await;
                ::keyringd_testkit::__private::complete_case(case_execution, outcome);
            }
            #ignored_metadata
        }
    } else {
        quote! {
            #(#attributes)*
            #[test]
            #ignore_attribute
            fn #function_name() {
                #case_initialization
                #skip
                let evidence_scope = ::keyringd_testkit::__private::EvidenceScope::enter();
                let outcome = ::keyringd_testkit::__private::capture(|| { #example #body });
                drop(evidence_scope);
                ::keyringd_testkit::__private::complete_case(case_execution, outcome);
            }
            #ignored_metadata
        }
    }
}

fn expand_steps(steps: Vec<StepSyntax>) -> TokenStream2 {
    let steps = steps.into_iter().map(|step| {
        let kind = match step.kind {
            StepKindSyntax::Given => quote!(Given),
            StepKindSyntax::When => quote!(When),
            StepKindSyntax::Then => quote!(Then),
            StepKindSyntax::Verify => quote!(Verify),
        };
        let description = step.description;
        let body = step.body;
        let source = quote_spanned! {description.span()=>
            ::keyringd_testkit::__private::SourceLocation::from_manifest(
                env!("CARGO_MANIFEST_DIR"),
                file!(),
                line!(),
                column!(),
            )
        };
        quote_spanned! {description.span()=>
            case_execution.begin_step(
                ::keyringd_testkit::__private::StepKind::#kind,
                #description,
                #source,
            );
            #body
            case_execution.complete_step();
        }
    });
    quote!(#(#steps)*)
}

#[cfg(test)]
mod tests {
    use super::expand_suite;
    use crate::syntax::ScenarioSyntax;

    keyringd_testkit::scenario! {
        group "expand_suite" {
            case "Case cfg applies to examples and ignored metadata" {
                verify "carry the case's cfg condition into expanded examples and ignored tests" {
                    for condition in ["all()", "any()"] {
                        let input: ScenarioSyntax = syn::parse_str(&format!(
                            r#"#[cfg({condition})]
                case "platform case" {{
                    ignored: "platform requirement";
                    examples: ["one" => 1, "two" => 2];
                    verify "value" {{ assert!(example > 0); }}
                }}"#
                        ))
                        .unwrap();
                        let expanded: syn::File = syn::parse2(expand_suite(input).unwrap()).unwrap();
                        assert_eq!(expanded.items.len(), 4, "two tests and their metadata");
                        for item in expanded.items {
                            let syn::Item::Fn(function) = item else {
                                panic!("expected generated test or metadata function");
                            };
                            let cfg = function
                                .attrs
                                .iter()
                                .find(|attr| attr.path().is_ident("cfg"))
                                .expect("each generated entry must retain the case cfg");
                            assert_eq!(
                                cfg.parse_args::<syn::Meta>()
                                    .unwrap()
                                    .path()
                                    .get_ident()
                                    .unwrap(),
                                condition.trim_end_matches("()")
                            );
                        }
                    }
                }
            }
        }
    }
}
