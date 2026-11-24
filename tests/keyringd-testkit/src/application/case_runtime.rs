use std::any::Any;

use crate::execution::CaseExecution;
use crate::infrastructure::{ResultStore, SourceSnippetLoader};
use crate::model::CaseDefinition;

use super::CaseMetadata;

pub fn begin_case(metadata: CaseMetadata) -> CaseExecution {
    let source_snippet = SourceSnippetLoader::load(&metadata.source, metadata.source_end_line);
    let definition = CaseDefinition {
        id: metadata.id,
        title: metadata.title,
        rationale: metadata.rationale,
        specifications: metadata.specifications,
        crate_name: metadata.crate_name,
        module_path: metadata.module_path,
        group: metadata.group,
        source: metadata.source,
    };
    if let Some(checkpoint) = std::env::var_os("KEYRINGD_TEST_CASE_CHECKPOINT") {
        let pending = CaseExecution::start(definition.clone(), source_snippet.clone())
            .finish(&Err(Box::new("case did not finish".to_owned())));
        std::fs::write(
            checkpoint,
            serde_json::to_vec_pretty(&pending).expect("serializable case checkpoint"),
        )
        .expect("failed to write supervised case checkpoint");
    }
    CaseExecution::start(definition, source_snippet)
}

pub fn complete_case(execution: CaseExecution, outcome: Result<(), Box<dyn Any + Send + 'static>>) {
    let result = execution.finish(&outcome);
    let framework_failure = outcome.is_ok().then(|| result.failure.clone()).flatten();
    let store_outcome = ResultStore::for_test_execution()
        .write(&result)
        .map_err(|error| Box::new(error) as Box<dyn std::error::Error>);

    match (outcome, store_outcome, framework_failure) {
        (Ok(()), Ok(()), None) => {}
        (Ok(()), Ok(()), Some(failure)) => panic!("{failure}"),
        (Ok(()), Err(error), _) => panic!("failed to write test report result: {error}"),
        (Err(payload), Ok(()), _) => std::panic::resume_unwind(payload),
        (Err(payload), Err(error), _) => {
            eprintln!("failed to write test report result after test failure: {error}");
            std::panic::resume_unwind(payload);
        }
    }
}

pub fn capture<T>(operation: impl FnOnce() -> T) -> Result<T, Box<dyn Any + Send + 'static>> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(operation))
}

pub async fn capture_async<F, T>(future: F) -> Result<T, Box<dyn Any + Send + 'static>>
where
    F: std::future::Future<Output = T>,
{
    use futures_util::FutureExt;
    std::panic::AssertUnwindSafe(future).catch_unwind().await
}

pub async fn capture_async_with_timeout<F, T>(
    timeout: std::time::Duration,
    future: F,
) -> Result<T, Box<dyn Any + Send + 'static>>
where
    F: std::future::Future<Output = T>,
{
    match tokio::time::timeout(timeout, capture_async(future)).await {
        Ok(outcome) => outcome,
        Err(_) => Err(Box::new(format!("scenario timed out after {timeout:?}"))),
    }
}

#[cfg(test)]
mod tests {
    keyringd_testkit::scenario! {
        group "capture_async_with_timeout" {
            case "Reports an asynchronous scenario timeout" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "produce a failed result that records the async deadline" {
                    std::thread::spawn(|| {
                        tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                            .unwrap()
                            .block_on(async {
                                let timeout = std::time::Duration::from_millis(1);
                                let outcome =
                                    super::capture_async_with_timeout(timeout, std::future::pending::<()>()).await;

                                let message = outcome
                                    .unwrap_err()
                                    .downcast::<String>()
                                    .expect("timeout failure should contain a string message");
                                assert_eq!(*message, "scenario timed out after 1ms");
                            })
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }
}
