use std::any::Any;
use std::path::Path;
use std::time::Instant;

use crate::execution::{ActiveStep, RecordedStep};
use crate::model::{
    CaseDefinition, CaseOutcome, CaseResult, Evidence, SourceLocation, StepKind, StepOutcome,
};

pub struct CaseExecution {
    definition: CaseDefinition,
    source_code: Option<String>,
    source_error: Option<String>,
    started_at: Instant,
    completed_steps: Vec<RecordedStep>,
    state: CaseExecutionState,
    evidence: Vec<Evidence>,
}

use super::CaseExecutionState;

impl CaseExecution {
    pub(crate) fn start(
        definition: CaseDefinition,
        source_snippet: Result<String, String>,
    ) -> Self {
        let (source_code, source_error) = match source_snippet {
            Ok(source_code) => (Some(source_code), None),
            Err(error) => (None, Some(error)),
        };
        Self {
            definition,
            source_code,
            source_error,
            started_at: Instant::now(),
            completed_steps: Vec::new(),
            state: CaseExecutionState::Ready,
            evidence: Vec::new(),
        }
    }

    pub fn begin_step(
        &mut self,
        kind: StepKind,
        description: &'static str,
        source: SourceLocation,
    ) {
        assert!(
            matches!(self.state, CaseExecutionState::Ready),
            "a case can start a step only while it is ready"
        );
        self.state = CaseExecutionState::RunningStep(ActiveStep::start(kind, description, source));
    }

    pub fn complete_step(&mut self) {
        let CaseExecutionState::RunningStep(step) =
            std::mem::replace(&mut self.state, CaseExecutionState::Ready)
        else {
            panic!("no case step is active");
        };
        self.completed_steps.push(step.finish(StepOutcome::Passed));
    }

    pub fn skip(&mut self, reason: impl Into<String>) {
        assert!(
            matches!(self.state, CaseExecutionState::Ready),
            "a case can be skipped only before a step starts"
        );
        self.state = CaseExecutionState::Skipped {
            reason: reason.into(),
        };
    }

    pub fn attach_text(&mut self, name: impl Into<String>, content: impl Into<String>) {
        self.attach(Evidence::Text {
            name: name.into(),
            content: content.into(),
        });
    }

    pub fn attach_log(&mut self, name: impl Into<String>, content: impl Into<String>) {
        self.attach(Evidence::Log {
            name: name.into(),
            content: content.into(),
        });
    }

    pub fn attach_bytes(&mut self, name: impl Into<String>, bytes: &[u8]) {
        let hexadecimal = bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<Vec<_>>()
            .join(" ");
        self.attach(Evidence::Bytes {
            name: name.into(),
            hexadecimal,
        });
    }

    pub fn attach_file(&mut self, name: impl Into<String>, path: impl AsRef<Path>) {
        self.attach(Evidence::File {
            name: name.into(),
            path: path.as_ref().display().to_string(),
        });
    }

    pub fn attach_image(&mut self, name: impl Into<String>, path: impl AsRef<Path>) {
        self.attach(Evidence::Image {
            name: name.into(),
            path: path.as_ref().display().to_string(),
        });
    }

    fn attach(&mut self, evidence: Evidence) {
        match &mut self.state {
            CaseExecutionState::RunningStep(step) => step.attach(evidence),
            CaseExecutionState::Ready | CaseExecutionState::Skipped { .. } => {
                self.evidence.push(evidence);
            }
        }
    }

    pub(crate) fn finish(
        mut self,
        body_outcome: &Result<(), Box<dyn Any + Send + 'static>>,
    ) -> CaseResult {
        let state = std::mem::replace(&mut self.state, CaseExecutionState::Ready);
        let (outcome, failure, skipped_reason) = match (body_outcome, state) {
            (Ok(()), CaseExecutionState::Ready) => (CaseOutcome::Passed, None, None),
            (Ok(()), CaseExecutionState::Skipped { reason }) => {
                (CaseOutcome::Skipped, None, Some(reason))
            }
            (Ok(()), CaseExecutionState::RunningStep(step)) => {
                let description = step.description();
                self.completed_steps.push(step.finish(StepOutcome::Failed));
                (
                    CaseOutcome::Failed,
                    Some(format!(
                        "case returned before completing the active step: {description}"
                    )),
                    None,
                )
            }
            (Err(payload), CaseExecutionState::RunningStep(step)) => {
                self.completed_steps.push(step.finish(StepOutcome::Failed));
                (
                    CaseOutcome::Failed,
                    Some(panic_message(payload.as_ref())),
                    None,
                )
            }
            (Err(payload), _) => (
                CaseOutcome::Failed,
                Some(panic_message(payload.as_ref())),
                None,
            ),
        };

        CaseResult {
            id: self.definition.id.to_owned(),
            title: self.definition.title.to_owned(),
            rationale: self.definition.rationale.map(str::to_owned),
            specifications: self
                .definition
                .specifications
                .iter()
                .map(|url| (*url).to_owned())
                .collect(),
            source_code: self.source_code,
            source_error: self.source_error,
            crate_name: self.definition.crate_name.to_owned(),
            module_path: self.definition.module_path.to_owned(),
            group: self.definition.group.map(str::to_owned),
            source: self.definition.source,
            outcome,
            duration_ms: self.started_at.elapsed().as_millis(),
            failure,
            skipped_reason,
            steps: self.completed_steps.iter().map(Into::into).collect(),
            evidence: self.evidence,
        }
    }
}

fn panic_message(payload: &(dyn Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else if let Some(message) = payload.downcast_ref::<&'static str>() {
        (*message).to_owned()
    } else {
        "non-string panic payload".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn execution() -> CaseExecution {
        CaseExecution::start(
            CaseDefinition {
                id: "crate::case",
                title: "case title",
                rationale: None,
                specifications: &[],
                crate_name: "crate",
                module_path: "module",
                group: None,
                source: SourceLocation::new("tests/case.rs", 1, 1),
            },
            Ok("case \"case title\" {}".to_owned()),
        )
    }

    keyringd_testkit::scenario! {
        group "CaseExecution::begin_step" {
            case "Marks a case failed when it returns during an active step" {
                verify "record an unfinished active step and fail the returned case" {
                    std::thread::spawn(|| {
                        let mut execution = execution();
                        execution.begin_step(
                            StepKind::When,
                            "operation is still running",
                            SourceLocation::new("tests/case.rs", 2, 1),
                        );

                        let result = execution.finish(&Ok(()));

                        assert_eq!(result.outcome, CaseOutcome::Failed);
                        assert_eq!(result.steps[0].outcome, StepOutcome::Failed);
                        assert_eq!(
                            result.failure.as_deref(),
                            Some("case returned before completing the active step: operation is still running")
                        );
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "CaseExecution::begin_step" {
            case "Attaches evidence to the active step" {
                verify "store the evidence on the step that was active when it was attached" {
                    std::thread::spawn(|| {
                        let mut execution = execution();
                        execution.begin_step(
                            StepKind::Given,
                            "state is visible",
                            SourceLocation::new("tests/case.rs", 2, 1),
                        );
                        execution.attach_text("state", "ready");
                        execution.complete_step();

                        let result = execution.finish(&Ok(()));

                        assert!(result.evidence.is_empty());
                        assert_eq!(result.steps[0].evidence.len(), 1);
                        assert!(matches!(
                            &result.steps[0].evidence[0],
                            Evidence::Text { name, content } if name == "state" && content == "ready"
                        ));
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }
    fn begin_image_step(execution: &mut CaseExecution) {
        execution.begin_step(
            StepKind::When,
            "capture",
            SourceLocation::new("tests/case.rs", 2, 1),
        );
    }
    fn only_image(result: &CaseResult) -> &str {
        assert!(result.evidence.is_empty());
        assert_eq!(result.steps.len(), 1);
        assert_eq!(result.steps[0].evidence.len(), 1);
        let Evidence::Image { name, .. } = &result.steps[0].evidence[0] else {
            panic!("expected an image")
        };
        name
    }
    keyringd_testkit::scenario! {
        group "CaseExecution::complete_step" {
            case "Recorded images require an active step and do not flow into the next step" {
                verify "attach the image only within its active step scope" {
                    std::thread::spawn(|| {
                        use crate::execution::{EvidenceScope, record_image};
                        assert!(record_image("outside", "outside.png").is_err());
                        let _scope = EvidenceScope::enter();
                        let mut execution = execution();
                        assert!(record_image("before", "before.png").is_err());
                        begin_image_step(&mut execution);
                        record_image("first", "first.png").unwrap();
                        execution.complete_step();
                        assert!(record_image("between", "between.png").is_err());
                        begin_image_step(&mut execution);
                        record_image("second", "second.png").unwrap();
                        execution.complete_step();
                        let result = execution.finish(&Ok(()));
                        assert_eq!(result.steps.len(), 2);
                        assert!(
                            matches!(&result.steps[0].evidence[..],[Evidence::Image {name,..}] if name=="first")
                        );
                        assert!(
                            matches!(&result.steps[1].evidence[..],[Evidence::Image {name,..}] if name=="second")
                        );
                        assert!(record_image("after", "after.png").is_err());
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }
    keyringd_testkit::scenario! {
        group "CaseExecution::finish" {
            case "A panicking case preserves its image and leaves the next case isolated" {
                verify "retain the panicking case's image and leave the following case's evidence empty" {
                    std::thread::spawn(|| {
                        use crate::execution::{EvidenceScope, record_image};
                        let mut failed = execution();
                        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            let _scope = EvidenceScope::enter();
                            begin_image_step(&mut failed);
                            record_image("failed", "failed.png").unwrap();
                            panic!("case failed");
                        }));
                        assert!(record_image("leaked", "leaked.png").is_err());
                        let failed = failed.finish(&outcome);
                        assert_eq!(failed.outcome, CaseOutcome::Failed);
                        assert_eq!(only_image(&failed), "failed");
                        let _scope = EvidenceScope::enter();
                        let mut next = execution();
                        begin_image_step(&mut next);
                        record_image("next", "next.png").unwrap();
                        next.complete_step();
                        assert_eq!(only_image(&next.finish(&Ok(()))), "next");
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }
    keyringd_testkit::scenario! {
        group "CaseExecution::complete_step" {
            case "Simultaneous case threads record only their own images" {
                verify "attribute each recorded image to the thread's active case" {
                    std::thread::spawn(|| {
                        use crate::execution::{EvidenceScope, record_image};
                        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
                        let threads = ["left", "right"].map(|name| {
                            let barrier = std::sync::Arc::clone(&barrier);
                            std::thread::spawn(move || {
                                let _scope = EvidenceScope::enter();
                                let mut execution = execution();
                                begin_image_step(&mut execution);
                                barrier.wait();
                                record_image(name, "image.png").unwrap();
                                execution.complete_step();
                                let result = execution.finish(&Ok(()));
                                assert_eq!(only_image(&result), name);
                            })
                        });
                        for thread in threads {
                            thread.join().unwrap();
                        }
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }
    keyringd_testkit::scenario! {
        group "CaseExecution::complete_step" {
            case "Interleaved async cases have task local step evidence" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "attribute images to the active step of each interleaved task" {
                    std::thread::spawn(|| {
                        tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                            .unwrap()
                            .block_on(async {
                                use crate::execution::{EvidenceScope, record_image};
                                async fn run(name: &'static str) -> CaseResult {
                                    EvidenceScope::asynchronous(async {
                                        let mut execution = execution();
                                        begin_image_step(&mut execution);
                                        tokio::task::yield_now().await;
                                        record_image(name, "image.png").unwrap();
                                        tokio::task::yield_now().await;
                                        execution.complete_step();
                                        execution.finish(&Ok(()))
                                    })
                                    .await
                                }
                                let (left, right) = futures_util::future::join(run("left"), run("right")).await;
                                assert_eq!(only_image(&left), "left");
                                assert_eq!(only_image(&right), "right");
                                assert!(record_image("outside", "outside.png").is_err());
                            })
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }
    keyringd_testkit::scenario! {
        group "CaseExecution::finish" {
            case "Cancelling an async case closes its step context" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "clear the task-local evidence scope when the case future is cancelled" {
                    std::thread::spawn(|| {
                        tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                            .unwrap()
                            .block_on(async {
                                use crate::execution::{EvidenceScope, record_image};
                                let mut execution = execution();
                                let cancelled = tokio::time::timeout(
                                    std::time::Duration::from_millis(1),
                                    EvidenceScope::asynchronous(async {
                                        begin_image_step(&mut execution);
                                        record_image("before cancellation", "image.png").unwrap();
                                        std::future::pending::<()>().await;
                                    }),
                                )
                                .await;
                                assert!(cancelled.is_err());
                                assert!(record_image("after cancellation", "leaked.png").is_err());
                                let result = execution.finish(&Ok(()));
                                assert_eq!(result.outcome, CaseOutcome::Failed);
                                assert_eq!(only_image(&result), "before cancellation");
                            })
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }
    keyringd_testkit::scenario! {
        group "CaseExecution::complete_step" {
            case "A spawned task cannot borrow a synchronous case scope" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "keep the synchronous evidence scope inaccessible from the spawned task" {
                    std::thread::spawn(|| {
                        tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                            .unwrap()
                            .block_on(async {
                                use crate::execution::{EvidenceScope, record_image};
                                let _scope = EvidenceScope::enter();
                                let mut execution = execution();
                                begin_image_step(&mut execution);
                                let child = tokio::spawn(async { record_image("background", "leaked.png") });
                                assert!(child.await.unwrap().is_err());
                                record_image("owner", "owner.png").unwrap();
                                execution.complete_step();
                                assert_eq!(only_image(&execution.finish(&Ok(()))), "owner");
                            })
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }
}
