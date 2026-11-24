use super::*;

pub(super) struct EvidenceSyntax {
    pub(super) name: LitStr,
    pub(super) value: Expr,
}

impl Parse for EvidenceSyntax {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let name = input.parse()?;
        input.parse::<syn::Token![,]>()?;
        let value = input.parse()?;
        Ok(Self { name, value })
    }
}
