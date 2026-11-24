use super::*;

pub(crate) enum ScenarioSyntaxItem {
    Case(Box<CaseSyntax>),
    Group(GroupSyntax),
}
