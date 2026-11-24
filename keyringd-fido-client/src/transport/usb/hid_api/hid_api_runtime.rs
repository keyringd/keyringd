use std::sync::{Arc, LazyLock, mpsc};

use super::{HidApiIoError, hid_api_task::HidApiTask};

// hidapi's native context is process-global and is never deinitialized by the
// Rust binding. Its initializing thread must therefore also live until exit.
static RUNTIME: LazyLock<Result<HidApiRuntime, Arc<std::io::Error>>> =
    LazyLock::new(|| HidApiRuntime::start().map_err(Arc::new));

pub(super) struct HidApiRuntime {
    tasks: mpsc::Sender<HidApiTask>,
}

impl HidApiRuntime {
    pub(super) fn shared() -> Result<&'static Self, HidApiIoError> {
        RUNTIME
            .as_ref()
            .map_err(|error| HidApiIoError::StartRuntime(Arc::clone(error)))
    }

    fn start() -> std::io::Result<Self> {
        let (tasks, receiver) = mpsc::channel::<HidApiTask>();
        std::thread::Builder::new()
            .name("keyringd-hid-manager".to_owned())
            .spawn(move || {
                let mut api = None;
                for task in receiver {
                    // The panic hook records failures; the dropped response
                    // reports them to the caller. Keep the owning RunLoop alive
                    // even if one management operation panics.
                    let _ =
                        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| task(&mut api)));
                }
            })?;
        Ok(Self { tasks })
    }

    pub(super) async fn execute<T: Send + 'static>(
        &'static self,
        operation: impl FnOnce(&mut hidapi::HidApi) -> Result<T, HidApiIoError> + Send + 'static,
    ) -> Result<T, HidApiIoError> {
        let (response, receiver) = tokio::sync::oneshot::channel();
        let span = tracing::Span::current();
        self.tasks
            .send(Box::new(move |api| {
                span.in_scope(|| {
                    let result = (|| {
                        if api.is_none() {
                            *api = Some(hidapi::HidApi::new().map_err(HidApiIoError::Initialize)?);
                        }
                        operation(
                            api.as_mut()
                                .expect("HID context was initialized on its owner thread"),
                        )
                    })();
                    // If the caller cancelled, the result is dropped here, on the
                    // owner thread, rather than leaking an opened device.
                    let _ = response.send(result);
                })
            }))
            .map_err(|_| HidApiIoError::RuntimeStopped)?;
        receiver
            .await
            .map_err(|_| HidApiIoError::ManagementTaskAborted)?
    }

    pub(super) fn close(&self, device: hidapi::HidDevice) {
        if let Err(error) = self.tasks.send(Box::new(move |_| drop(device))) {
            // Never call hid_close on an arbitrary thread if the owner has
            // unexpectedly disappeared. Leave the OS handle until process exit.
            std::mem::forget(error.0);
            tracing::error!("HID manager stopped before the device could be closed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use super::super::DropProbe;

    fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
    }

    keyringd_testkit::scenario! {
        group "HidApiRuntime::shared" {
            case "Hid owner outlives async runtimes and is shared by callers" {
                verify "share one HID owner across callers and successive async runtimes" {
                    let owner = HidApiRuntime::shared().unwrap();
                    let first = runtime()
                        .block_on(owner.execute(|_| Ok(std::thread::current().id())))
                        .unwrap();
                    assert_ne!(first, std::thread::current().id());
                    let second = std::thread::spawn(|| {
                        runtime()
                            .block_on(
                                HidApiRuntime::shared()
                                    .unwrap()
                                    .execute(|_| Ok(std::thread::current().id())),
                            )
                            .unwrap()
                    })
                    .join()
                    .unwrap();
                    assert_eq!(first, second);
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "HidApiRuntime::execute" {
            case "A panicking operation does not end the hid owner" {
                verify "service another request on the same HID owner after a panic" {
                    let owner = HidApiRuntime::shared().unwrap();
                    let runtime = runtime();
                    let before = runtime
                        .block_on(owner.execute(|_| Ok(std::thread::current().id())))
                        .unwrap();
                    let error = runtime
                        .block_on(owner.execute::<()>(|_| panic!("HID management test panic")))
                        .unwrap_err();
                    assert!(matches!(error, HidApiIoError::ManagementTaskAborted));
                    let after = runtime
                        .block_on(owner.execute(|_| Ok(std::thread::current().id())))
                        .unwrap();
                    assert_eq!(before, after);
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "HidApiRuntime::execute" {
            case "A cancelled request drops its result on the hid owner" {
                verify "drop the undelivered result on the HID owner thread" {
                    let owner = HidApiRuntime::shared().unwrap();
                    let (release, released) = mpsc::channel();
                    let (dropped, observed) = mpsc::channel();
                    let mut operation = Box::pin(owner.execute(move |_| {
                        released.recv().unwrap();
                        Ok(DropProbe(dropped))
                    }));
                    runtime().block_on(async {
                        assert!(futures_util::poll!(operation.as_mut()).is_pending());
                    });
                    drop(operation);
                    release.send(()).unwrap();
                    let dropping_thread = observed
                        .recv_timeout(std::time::Duration::from_secs(5))
                        .unwrap();
                    let owner_thread = runtime()
                        .block_on(owner.execute(|_| Ok(std::thread::current().id())))
                        .unwrap();
                    assert_eq!(dropping_thread, owner_thread);
                }
            }
        }
    }
}
