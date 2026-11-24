use super::TrayComponent;
use keyringd_core::model::Profile;
use keyringd_ui_core::app_menu::{AppMenuEvent, AppMenuLifecycleEvent};
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::sync::{Arc, mpsc};
use tokio::sync::mpsc::UnboundedReceiver;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::Threading::{CreateEventW, SetEvent};

pub(crate) struct TrayHost {
    updates: mpsc::Sender<AppMenuLifecycleEvent>,
    events: UnboundedReceiver<AppMenuEvent>,
    wake: Arc<OwnedHandle>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl TrayHost {
    pub(crate) fn start(profiles: Vec<Profile>) -> anyhow::Result<Self> {
        let (updates, update_receiver) = mpsc::channel();
        let (event_sender, events) = tokio::sync::mpsc::unbounded_channel();
        let wake = Arc::new(unsafe {
            OwnedHandle::from_raw_handle(CreateEventW(None, false, false, None)?.0)
        });
        let thread_wake = Arc::clone(&wake);
        let thread = std::thread::Builder::new()
            .name("keyringd-tray".to_owned())
            .spawn(move || {
                TrayComponent::run(profiles, update_receiver, event_sender, &thread_wake)
            })?;
        Ok(Self {
            updates,
            events,
            wake,
            thread: Some(thread),
        })
    }

    pub(crate) fn post_profiles_update(&self, profiles: Vec<Profile>) -> anyhow::Result<()> {
        self.post(AppMenuLifecycleEvent::ProfilesChanged(profiles))
    }

    fn post(&self, event: AppMenuLifecycleEvent) -> anyhow::Result<()> {
        self.updates
            .send(event)
            .map_err(|_| anyhow::anyhow!("tray thread stopped"))?;
        unsafe { SetEvent(HANDLE(self.wake.as_raw_handle()))? };
        Ok(())
    }

    pub(crate) async fn recv_event(&mut self) -> Option<AppMenuEvent> {
        self.events.recv().await
    }
}

impl Drop for TrayHost {
    fn drop(&mut self) {
        let _ = self.post(AppMenuLifecycleEvent::StopRequested);
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            tracing::warn!("tray thread panicked while stopping");
        }
    }
}
