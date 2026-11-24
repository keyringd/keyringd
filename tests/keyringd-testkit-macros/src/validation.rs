use syn::spanned::Spanned;
use syn::{Error, LitStr, Result};

use crate::syntax::{CaseSyntax, ScenarioSyntax, ScenarioSyntaxItem, StepKindSyntax, StepSyntax};

pub(crate) fn validate(scenario: &ScenarioSyntax) -> Result<()> {
    validate_items(&scenario.items)
}

fn validate_items(items: &[ScenarioSyntaxItem]) -> Result<()> {
    for item in items {
        match item {
            ScenarioSyntaxItem::Case(case) => validate_case(case)?,
            ScenarioSyntaxItem::Group(group) => validate_items(&group.items)?,
        }
    }
    Ok(())
}

fn validate_case(case: &CaseSyntax) -> Result<()> {
    if let Some(rationale) = &case.rationale
        && rationale.value().trim().is_empty()
    {
        return Err(Error::new(
            rationale.span(),
            "case rationale must explain why the behavior matters",
        ));
    }
    for specification in &case.specifications {
        let url = specification.value();
        let anchored_http_url = (url.starts_with("https://") || url.starts_with("http://"))
            && url
                .split_once('#')
                .is_some_and(|(_, fragment)| !fragment.trim().is_empty());
        if !anchored_http_url {
            return Err(Error::new(
                specification.span(),
                "spec must be an HTTP(S) URL with a non-empty fragment anchor",
            ));
        }
    }
    if let Some(runtime) = &case.runtime
        && runtime != "tokio"
    {
        return Err(Error::new(
            runtime.span(),
            "only the tokio runtime is supported",
        ));
    }
    if case.runtime.is_none()
        && let Some(timeout) = &case.timeout
    {
        return Err(Error::new(
            timeout.span(),
            "timeout is supported only for async: tokio scenarios",
        ));
    }

    let has_given = case
        .steps
        .iter()
        .any(|step| step.kind == StepKindSyntax::Given);
    let has_when = case
        .steps
        .iter()
        .any(|step| step.kind == StepKindSyntax::When);
    let has_then = case
        .steps
        .iter()
        .any(|step| step.kind == StepKindSyntax::Then);
    let has_verify = case
        .steps
        .iter()
        .any(|step| step.kind == StepKindSyntax::Verify);
    let bdd_complete = !has_verify && has_given && has_when && has_then;
    let verification_only = has_verify
        && case
            .steps
            .iter()
            .all(|step| step.kind == StepKindSyntax::Verify);
    if !bdd_complete && !verification_only {
        return Err(Error::new(
            case.case_name.span(),
            "a scenario requires either verify steps or at least one given, when, and then step",
        ));
    }

    for step in &case.steps {
        validate_step(step)?;
    }
    Ok(())
}

fn validate_step(step: &StepSyntax) -> Result<()> {
    let description: &LitStr = &step.description;
    let value = description.value();
    let is_vague = matches!(
        value.as_str(),
        "Prepare the initial state and dependencies"
            | "Run the operation under test"
            | "Check the resulting state and side effects"
            | "Apply the input or state change under test"
            | "Prepare the state and dependencies used by the operation"
            | "Verify the return value, retained state, and emitted command"
    ) || value.starts_with('"')
        && (value.ends_with("\" has the required inputs and initial state")
            || value.ends_with("\" behaves as expected"))
        || value.ends_with(" has the expected values and state");
    if is_vague {
        return Err(Error::new(
            description.span(),
            "step description must name the concrete setup, operation, or expected result",
        ));
    }
    Ok(())
}
