use dispatch2::DispatchQueue;
use objc2::MainThreadMarker;

use keyringd_ui_core::runtime::UiEventReceiver;

pub(crate) trait MainThreadEventReceiverExt<T> {
    fn spawn_on_main(
        self,
        runtime: &tokio::runtime::Handle,
        handler: impl Fn(T, MainThreadMarker) + Send + Sync + 'static,
    );
}

impl<T: Send + 'static> MainThreadEventReceiverExt<T> for UiEventReceiver<T> {
    fn spawn_on_main(
        mut self,
        runtime: &tokio::runtime::Handle,
        handler: impl Fn(T, MainThreadMarker) + Send + Sync + 'static,
    ) {
        let handler = std::sync::Arc::new(handler);
        runtime.spawn(async move {
            while let Some(event) = self.receive().await {
                let handler = std::sync::Arc::clone(&handler);
                DispatchQueue::main().exec_async(move || {
                    let mtm = MainThreadMarker::new()
                        .expect("the callback should run on the AppKit thread because it was dispatched to the GCD main queue");
                    handler(event, mtm);
                });
            }
        });
    }
}
