use super::*;
use syn::LitStr;

pub(crate) struct GroupSyntax {
    pub(crate) title: LitStr,
    pub(crate) items: Vec<ScenarioSyntaxItem>,
}
