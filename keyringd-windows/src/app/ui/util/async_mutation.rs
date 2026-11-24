use keyringd_ui_core::runtime::UiEventTarget;
use std::future::Future;
use std::sync::{Arc, Mutex};

use super::UiEventSink;
use tokio::runtime::Handle;
use windows_reactor::{MutationState, RenderCx};

fn start_task(
    active: &Mutex<Option<tokio::task::AbortHandle>>,
    spawn: impl FnOnce() -> tokio::task::AbortHandle,
) -> tokio::task::AbortHandle {
    let mut active = active.lock().unwrap();
    if let Some(task) = active.as_ref().filter(|task| !task.is_finished()) {
        return task.clone();
    }
    let task = spawn();
    *active = Some(task.clone());
    task
}

#[derive(Clone)]
pub(crate) struct AsyncMutation<T>
where
    T: Send + Clone + 'static,
{
    runtime: Handle,
    task: Arc<Mutex<Option<tokio::task::AbortHandle>>>,
    events: UiEventSink<MutationState<T>>,
}

impl<T> AsyncMutation<T>
where
    T: Send + Clone + 'static,
{
    pub(crate) fn use_in(cx: &mut RenderCx, runtime: Handle) -> Self {
        let events = UiEventSink::use_in(cx);
        let task = cx
            .use_ref(Arc::new(Mutex::new(None::<tokio::task::AbortHandle>)))
            .get_cloned();
        Self {
            runtime,
            task,
            events,
        }
    }

    pub(crate) fn on_change(&self, handler: impl Fn(MutationState<T>) + 'static) {
        self.events.bind(UiEventTarget::new(handler));
    }

    /// The error is converted to the user's guide by the caller before handing it over.
    pub(crate) fn start<F>(&self, operation: F)
    where
        F: Future<Output = Result<T, String>> + Send + 'static,
    {
        let _ = self.start_cancellable(operation);
    }

    /// Only for operations that are safe to drop, such as an SSH subprocess.
    /// Persistent key operations must run to completion so their cleanup is not skipped.
    pub(crate) fn start_cancellable<F>(&self, operation: F) -> tokio::task::AbortHandle
    where
        F: Future<Output = Result<T, String>> + Send + 'static,
    {
        let mut started = false;
        let handle = start_task(&self.task, || {
            started = true;
            let events = self.events.sender();
            let task = Arc::clone(&self.task);
            self.runtime
                .spawn(async move {
                    let state = match operation.await {
                        Ok(value) => MutationState::Success(value),
                        Err(error) => MutationState::Error(error),
                    };
                    // Release the operation slot before making a retry available
                    // on the UI thread, even if this task has not returned yet.
                    *task.lock().unwrap() = None;
                    events.send(state);
                })
                .abort_handle()
        });
        if started {
            // No task-slot lock is held while the reducer handles the event.
            self.events.target().send(MutationState::Loading);
        }
        handle
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    use crate::app::ui::util::test_support::Cleanup;

    keyringd_testkit::scenario! {
        group "AsyncMutation::use_in" {
            case "Immediate retries deliver every terminal event without rendering" {
                verify "deliver each retry outcome without waiting for the view to render" {
                    use std::cell::RefCell;
                    use std::rc::Rc;
                    use windows_reactor::ChannelDispatcher;

                    let runtime = tokio::runtime::Runtime::new().unwrap();
                    let dispatcher = ChannelDispatcher::new();
                    let mut cx = RenderCx::new(Rc::new(|| {}));
                    cx.set_marshaller(Some(dispatcher.marshaller()));
                    let mutation = AsyncMutation::<()>::use_in(&mut cx, runtime.handle().clone());
                    let received = Rc::new(RefCell::new(Vec::new()));
                    let captured = received.clone();
                    mutation.on_change(move |event| captured.borrow_mut().push(event));
                    cx.flush_effects();

                    for outcome in [Ok(()), Ok(()), Err("same error"), Err("same error")] {
                        let before = received.borrow().len();
                        let task = mutation.start_cancellable(async move { outcome.map_err(str::to_owned) });
                        assert_eq!(received.borrow()[before], MutationState::Loading);
                        runtime.block_on(async {
                            tokio::time::timeout(Duration::from_secs(2), async {
                                while !task.is_finished() {
                                    tokio::task::yield_now().await;
                                }
                            })
                            .await
                            .unwrap();
                        });
                        dispatcher.drain();
                    }
                    assert_eq!(received.borrow().len(), 8);
                    assert_eq!(received.borrow()[1], MutationState::Success(()));
                    assert_eq!(received.borrow()[3], MutationState::Success(()));
                    assert_eq!(
                        received.borrow()[5],
                        MutationState::Error("same error".into())
                    );
                    assert_eq!(
                        received.borrow()[7],
                        MutationState::Error("same error".into())
                    );
                    cx.run_cleanups();
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "start_task" {
            case "Cancel the Asynchronous Mutation, release the Resource and not call the completion process" {
                async: tokio;
                timeout: std::time::Duration::from_secs(5);
                given "Prepare SLOT, CLEANED, COMPLETED" {
                    let slot = Mutex::new(None);
                    let cleaned = Arc::new(AtomicBool::new(false));
                    let completed = Arc::new(AtomicBool::new(false));
                    let (started, ready) = tokio::sync::oneshot::channel();
                    let cleanup = Cleanup(cleaned.clone());
                    let completion = completed.clone();
                    let handle = start_task(&slot, || {
                        tokio::spawn(async move {
                            let _cleanup = cleanup;
                            let _ = started.send(());
                            std::future::pending::<()>().await;
                            completion.store(true, Ordering::SeqCst);
                        })
                        .abort_handle()
                    });
                    ready.await.unwrap();
                    start_task(&slot, || {
                        panic!("a second operation must not start while one is running")
                    });
                    handle.abort();
                }
                when "Wait until the Abort Task ends" {
                    tokio::time::timeout(Duration::from_secs(1), async {
                        while !handle.is_finished() {
                            tokio::task::yield_now().await;
                        }
                    })
                    .await
                    .unwrap();
                }
                then "Cancel the Asynchronous Mutation, release the Resource and not call the completion process" {
                    assert!(cleaned.load(Ordering::SeqCst));
                    assert!(!completed.load(Ordering::SeqCst));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "start_task" {
            case "Discarding the Window's Task Slot does not interrupt the persistent operation" {
                async: tokio;
                timeout: std::time::Duration::from_secs(5);
                given "Prepare a Window-owned Task Slot and Completion Notification Channel" {
                    let slot = Mutex::new(None);
                    let (finish, wait) = tokio::sync::oneshot::channel();
                    let (done, completed) = tokio::sync::oneshot::channel();
                }
                when "Start Persistent Task to complete Task after discarding Slots" {
                    start_task(&slot, || {
                        tokio::spawn(async move {
                            wait.await.unwrap();
                            done.send(()).unwrap();
                        })
                        .abort_handle()
                    });
                    drop(slot);
                    finish.send(()).unwrap();
                    let result = tokio::time::timeout(Duration::from_secs(1), completed).await;
                }
                then "Task completes independently of SLOT's Lifetime" {
                    result.unwrap().unwrap();
                }
            }
        }
    }
}
