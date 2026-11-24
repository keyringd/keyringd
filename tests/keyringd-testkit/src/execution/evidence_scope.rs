use std::cell::RefCell;
use std::future::Future;
use std::marker::PhantomData;
use std::path::Path;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use super::EvidenceCollector;
use crate::model::Evidence;
use anyhow::{Context, Result, ensure};

type Collector = Arc<Mutex<EvidenceCollector>>;
type Scope = Arc<Mutex<ScopeState>>;
#[derive(Default)]
struct ScopeState {
    step: Option<Collector>,
    closed: bool,
}
thread_local! { static CURRENT_EVIDENCE: RefCell<Option<Scope>> = const { RefCell::new(None) }; }
tokio::task_local! { static ASYNC_EVIDENCE: Scope; }

/// Keeps synchronous evidence local to this thread, and async evidence local to its task.
#[doc(hidden)]
pub struct EvidenceScope {
    state: Scope,
    previous: Option<Scope>,
    installed_thread: bool,
    _same_thread: PhantomData<Rc<()>>,
}
impl EvidenceScope {
    pub fn enter() -> Self {
        let state = Arc::new(Mutex::new(ScopeState::default()));
        let previous = CURRENT_EVIDENCE.with(|slot| slot.replace(Some(Arc::clone(&state))));
        Self {
            state,
            previous,
            installed_thread: true,
            _same_thread: PhantomData,
        }
    }
    pub async fn asynchronous<F: Future>(future: F) -> F::Output {
        let state = Arc::new(Mutex::new(ScopeState::default()));
        let guard = Self {
            state: Arc::clone(&state),
            previous: None,
            installed_thread: false,
            _same_thread: PhantomData,
        };
        ASYNC_EVIDENCE
            .scope(state, async move {
                let _guard = guard;
                future.await
            })
            .await
    }
}
impl Drop for EvidenceScope {
    fn drop(&mut self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.closed = true;
        if let Some(step) = state.step.take() {
            step.lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .closed = true;
        }
        drop(state);
        if self.installed_thread {
            CURRENT_EVIDENCE.with(|slot| {
                let mut slot = slot.borrow_mut();
                if slot
                    .as_ref()
                    .is_some_and(|current| Arc::ptr_eq(current, &self.state))
                {
                    *slot = self.previous.take();
                }
            });
        }
    }
}
fn current_scope() -> Option<Scope> {
    match ASYNC_EVIDENCE.try_with(Arc::clone) {
        Ok(scope) => Some(scope),
        // A spawned task must opt into its own scenario scope. It must not
        // borrow a synchronous case's thread-local recorder while being polled.
        Err(_) if tokio::task::try_id().is_some() => None,
        Err(_) => CURRENT_EVIDENCE.with(|slot| slot.borrow().clone()),
    }
}
pub(crate) fn activate_step(step: &Collector) {
    if let Some(scope) = current_scope() {
        let mut state = scope.lock().expect("evidence scope lock poisoned");
        assert!(
            !state.closed && state.step.is_none(),
            "an evidence scope already has an active step"
        );
        state.step = Some(Arc::clone(step));
    }
}
pub(crate) fn deactivate_step(step: &Collector) {
    if let Some(scope) = current_scope() {
        let mut state = scope
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state
            .step
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, step))
        {
            state.step = None;
        }
    }
}
/// Register an already captured image with the current case step.
/// A caller outside an active scenario step receives an error.
pub fn record_image(name: impl Into<String>, path: impl AsRef<Path>) -> Result<()> {
    let scope =
        current_scope().context("cannot record an image outside an executing scenario case")?;
    let state = scope
        .lock()
        .map_err(|_| anyhow::anyhow!("evidence scope lock poisoned"))?;
    ensure!(
        !state.closed,
        "cannot record an image after its scenario case finished"
    );
    let step = state
        .step
        .as_ref()
        .context("cannot record an image outside an active scenario step")?;
    let mut collector = step
        .lock()
        .map_err(|_| anyhow::anyhow!("step evidence lock poisoned"))?;
    ensure!(
        !collector.closed,
        "cannot record an image after its scenario step finished"
    );
    collector.evidence.push(Evidence::Image {
        name: name.into(),
        path: path.as_ref().display().to_string(),
    });
    Ok(())
}
