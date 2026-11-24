#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum StepKindSyntax {
    Given,
    When,
    Then,
    Verify,
}
