use super::{
    CaseSyntax, ExampleSyntax, GroupSyntax, ScenarioSyntaxItem, StepKindSyntax, StepSyntax, keyword,
};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{Error, Expr, LitStr, Result, braced, bracketed};

pub(crate) struct ScenarioSyntax {
    pub(crate) items: Vec<ScenarioSyntaxItem>,
}

impl Parse for ScenarioSyntax {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut items = Vec::new();
        while !input.is_empty() {
            if input.peek(keyword::Case) || input.peek(syn::Token![#]) {
                items.push(ScenarioSyntaxItem::Case(Box::new(parse_case(input)?)));
            } else if input.peek(keyword::Group) {
                items.push(ScenarioSyntaxItem::Group(parse_group(input)?));
            } else {
                return Err(input.error("expected case or group"));
            }
        }
        if items.is_empty() {
            return Err(input.error("a scenario requires at least one case or group"));
        }
        Ok(Self { items })
    }
}

fn parse_case(input: ParseStream<'_>) -> Result<CaseSyntax> {
    let attributes = input.call(syn::Attribute::parse_outer)?;
    for attribute in &attributes {
        if !attribute.path().is_ident("cfg") {
            return Err(Error::new_spanned(
                attribute,
                "only #[cfg(...)] is supported before case",
            ));
        }
    }
    let case_keyword = input.parse::<keyword::Case>()?;
    let case_name = input.parse()?;
    let content;
    let brace = braced!(content in input);
    let start = attributes
        .first()
        .map(|attribute| attribute.span().start())
        .unwrap_or_else(|| case_keyword.span.start());
    let end = brace.span.close().end();
    let mut case = parse_case_body(
        &content,
        case_name,
        u32::try_from(start.line).expect("source line fits in u32"),
        u32::try_from(end.line).expect("source line fits in u32"),
        u32::try_from(start.column + 1).expect("source column fits in u32"),
    )?;
    case.attributes = attributes;
    Ok(case)
}

fn parse_group(input: ParseStream<'_>) -> Result<GroupSyntax> {
    input.parse::<keyword::Group>()?;
    let title: LitStr = input.parse()?;
    let content;
    braced!(content in input);
    let mut items = Vec::new();
    while !content.is_empty() {
        if content.peek(keyword::Case) || content.peek(syn::Token![#]) {
            items.push(ScenarioSyntaxItem::Case(Box::new(parse_case(&content)?)));
        } else if content.peek(keyword::Group) {
            items.push(ScenarioSyntaxItem::Group(parse_group(&content)?));
        } else {
            return Err(content.error("expected case or group"));
        }
    }
    if items.is_empty() {
        return Err(Error::new(
            title.span(),
            "a group requires at least one case or group",
        ));
    }
    Ok(GroupSyntax { title, items })
}

fn parse_case_body(
    input: ParseStream<'_>,
    case_name: LitStr,
    source_start_line: u32,
    source_end_line: u32,
    source_column: u32,
) -> Result<CaseSyntax> {
    let mut rationale = None;
    let mut specifications = Vec::new();
    while input.peek(keyword::Why) || input.peek(keyword::Spec) {
        if input.peek(keyword::Why) {
            let keyword = input.parse::<keyword::Why>()?;
            if rationale.is_some() {
                return Err(Error::new(keyword.span, "why may be specified only once"));
            }
            rationale = Some(input.parse()?);
        } else {
            input.parse::<keyword::Spec>()?;
            specifications.push(input.parse()?);
        }
    }
    let mut runtime_arguments = proc_macro2::TokenStream::new();
    let runtime = if input.peek(syn::Token![async]) {
        input.parse::<syn::Token![async]>()?;
        input.parse::<syn::Token![:]>()?;
        let runtime = input.parse()?;
        if input.peek(syn::token::Paren) {
            let arguments;
            syn::parenthesized!(arguments in input);
            runtime_arguments = arguments.parse()?;
        }
        input.parse::<syn::Token![;]>()?;
        Some(runtime)
    } else {
        None
    };

    let timeout = if input.peek(keyword::Timeout) {
        input.parse::<keyword::Timeout>()?;
        input.parse::<syn::Token![:]>()?;
        let timeout: Expr = input.parse()?;
        input.parse::<syn::Token![;]>()?;
        Some(timeout)
    } else {
        None
    };
    let ignored = if input.peek(keyword::Ignored) {
        input.parse::<keyword::Ignored>()?;
        input.parse::<syn::Token![:]>()?;
        let reason = input.parse()?;
        input.parse::<syn::Token![;]>()?;
        Some(reason)
    } else {
        None
    };

    let examples = if input.peek(keyword::Examples) {
        input.parse::<keyword::Examples>()?;
        input.parse::<syn::Token![:]>()?;
        let content;
        bracketed!(content in input);
        let mut examples = Vec::new();
        while !content.is_empty() {
            examples.push(ExampleSyntax {
                name: content.parse()?,
                value: {
                    content.parse::<syn::Token![=>]>()?;
                    content.parse()?
                },
            });
            if content.peek(syn::Token![,]) {
                content.parse::<syn::Token![,]>()?;
            }
        }
        if examples.is_empty() {
            return Err(Error::new(
                content.span(),
                "examples requires at least one named value",
            ));
        }
        input.parse::<syn::Token![;]>()?;
        examples
    } else {
        Vec::new()
    };

    let skip = if input.peek(keyword::SkipIf) {
        input.parse::<keyword::SkipIf>()?;
        input.parse::<syn::Token![:]>()?;
        let condition = input.parse()?;
        input.parse::<syn::Token![=>]>()?;
        let reason = input.parse()?;
        input.parse::<syn::Token![;]>()?;
        Some((condition, reason))
    } else {
        None
    };

    let mut steps = Vec::new();
    while !input.is_empty() {
        let kind = if input.peek(keyword::Given) {
            input.parse::<keyword::Given>()?;
            StepKindSyntax::Given
        } else if input.peek(keyword::When) {
            input.parse::<keyword::When>()?;
            StepKindSyntax::When
        } else if input.peek(keyword::Then) {
            input.parse::<keyword::Then>()?;
            StepKindSyntax::Then
        } else if input.peek(keyword::Verify) {
            input.parse::<keyword::Verify>()?;
            StepKindSyntax::Verify
        } else {
            return Err(input.error("expected verify, given, when, or then step"));
        };
        let description = input.parse()?;
        let content;
        braced!(content in input);
        let body = content.parse()?;
        steps.push(StepSyntax {
            kind,
            description,
            body,
        });
    }
    Ok(CaseSyntax {
        attributes: Vec::new(),
        case_name,
        rationale,
        specifications,
        source_start_line,
        source_end_line,
        source_column,
        runtime,
        runtime_arguments,
        timeout,
        ignored,
        examples,
        skip,
        steps,
    })
}
