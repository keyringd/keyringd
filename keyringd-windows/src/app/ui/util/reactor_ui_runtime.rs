use anyhow::Context;
use std::sync::mpsc;

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{SW_HIDE, ShowWindow};
use windows_reactor::{App, ReactorWindow, UiMarshaller, WinUIDispatcher, border, text_block};

use super::CreatedReactorWindowLocator;

const UI_RESPONSE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

#[derive(Clone)]
pub(crate) struct ReactorUiRuntime {
    dispatcher: UiMarshaller,
}

impl ReactorUiRuntime {
    pub(crate) fn start() -> anyhow::Result<Self> {
        let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("keyringd-reactor-ui".to_string())
            .spawn(move || Self::run(ready_sender))?;
        ready_receiver
            .recv()
            .context("Reactor UI thread stopped during startup")?
    }

    fn run(ready: mpsc::SyncSender<anyhow::Result<Self>>) {
        let report = ready.clone();
        let result = App::new().run_custom(move |_| {
            let dispatcher = WinUIDispatcher::for_current_thread()?.marshaller();
            let keepalive = Self::create_keepalive_window()?.0 as usize;
            let _ = ready.send(Ok(Self {
                dispatcher: dispatcher.clone(),
            }));
            dispatcher.dispatch(move || unsafe {
                let keepalive = HWND(keepalive as *mut core::ffi::c_void);
                let _ = ShowWindow(keepalive, SW_HIDE);
            });
            Ok(())
        });
        if let Err(error) = result {
            keyringd_core::util::FailureDiagnostics::record_error("Reactor UI thread", &error);
            let _ = report.send(Err(anyhow::Error::new(error)));
        }
    }

    fn create_keepalive_window() -> windows_reactor::Result<HWND> {
        let created_window = CreatedReactorWindowLocator::begin()?;
        ReactorWindow::new()
            .title("Keyringd UI runtime")
            .inner_size(1.0, 1.0)
            .render(|_| border(text_block("")).into())?;
        Ok(created_window.finish()?)
    }

    pub(crate) fn dispatcher(&self) -> UiMarshaller {
        self.dispatcher.clone()
    }

    pub(crate) fn invoke<T>(
        &self,
        operation: impl FnOnce() -> anyhow::Result<T> + Send + 'static,
    ) -> anyhow::Result<T>
    where
        T: Send + 'static,
    {
        let (sender, receiver) = mpsc::sync_channel(1);
        anyhow::ensure!(
            self.dispatcher.dispatch(move || {
                let _ = sender.send(operation());
            }),
            "Reactor UI is no longer available"
        );
        receiver
            .recv_timeout(UI_RESPONSE_TIMEOUT)
            .context("Reactor UI did not respond")?
    }
}
