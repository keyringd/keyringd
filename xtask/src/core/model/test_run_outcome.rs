use anyhow::Result;

#[derive(Debug)]
pub(crate) struct TestRunOutcome {
    stages: Vec<(&'static str, Result<()>)>,
    report: Result<()>,
}

impl TestRunOutcome {
    pub(crate) fn new(stages: Vec<(&'static str, Result<()>)>, report: Result<()>) -> Self {
        Self { stages, report }
    }

    pub(crate) fn into_result(self) -> Result<()> {
        keyringd_testkit::runner::StageFailures::finish(
            self.stages
                .into_iter()
                .chain(std::iter::once(("test report", self.report))),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "TestRunOutcome::into_result" {
            case "Preserves all test and rendering errors together" {
                verify "return the individual test and report-rendering failures in one error" {
                    let outcome = TestRunOutcome::new(
                        vec![
                            ("cargo test", Err(anyhow::anyhow!("test failure"))),
                            ("UI tests", Err(anyhow::anyhow!("UI failure"))),
                        ],
                        Err(anyhow::anyhow!("render failure")),
                    );
                    assert_eq!(
                        outcome.into_result().unwrap_err().to_string(),
                        "cargo test: test failure\nUI tests: UI failure\ntest report: render failure"
                    );
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "TestRunOutcome::into_result" {
            case "Reports rendering failure even when tests succeed" {
                verify "return the report-rendering error after successful test stages" {
                    let outcome = TestRunOutcome::new(
                        vec![("cargo test", Ok(()))],
                        Err(anyhow::anyhow!("render failure")),
                    );
                    assert_eq!(
                        outcome.into_result().unwrap_err().to_string(),
                        "test report: render failure"
                    );
                    assert!(
                        TestRunOutcome::new(vec![("cargo test", Ok(()))], Ok(()))
                            .into_result()
                            .is_ok()
                    );
                }
            }
        }
    }
}
