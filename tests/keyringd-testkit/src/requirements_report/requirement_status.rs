#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RequirementStatus {
    Covered,
    Uncovered,
    Failed,
    Skipped,
    MissingResult,
    OtherPlatform,
    OtherComponent,
    OutOfScope,
    Deferred,
}

impl RequirementStatus {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Covered => "COVERED",
            Self::Uncovered => "UNCOVERED",
            Self::Failed => "FAILED",
            Self::Skipped => "SKIPPED",
            Self::MissingResult => "MISSING RESULT",
            Self::OtherPlatform => "OTHER PLATFORM",
            Self::OtherComponent => "OTHER COMPONENT",
            Self::OutOfScope => "OUT OF SCOPE",
            Self::Deferred => "DEFERRED",
        }
    }

    pub(super) fn css_class(self) -> &'static str {
        match self {
            Self::Covered => "covered",
            Self::Uncovered => "uncovered",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
            Self::MissingResult => "missing-result",
            Self::OtherPlatform => "other-platform",
            Self::OtherComponent => "other-component",
            Self::OutOfScope => "out-of-scope",
            Self::Deferred => "deferred",
        }
    }
}
