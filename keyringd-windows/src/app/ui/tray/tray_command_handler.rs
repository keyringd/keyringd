use super::TrayMenuView;
use keyringd_ui_core::app_menu::AppMenuCommand;
use keyringd_ui_core::app_menu::AppMenuEvent;
use tokio::sync::mpsc::UnboundedSender;
use tray_icon::TrayIcon;

pub(super) struct TrayCommandHandler {
    events: UnboundedSender<AppMenuEvent>,
    tray: Option<TrayIcon>,
    actions: Vec<(String, AppMenuEvent)>,
}

impl TrayCommandHandler {
    pub(super) fn new(events: UnboundedSender<AppMenuEvent>) -> Self {
        Self {
            events,
            tray: None,
            actions: Vec::new(),
        }
    }

    pub(super) fn handle(&mut self, command: AppMenuCommand) -> bool {
        match command {
            AppMenuCommand::RebuildMenu(profiles) => match TrayMenuView::render(profiles) {
                Ok((tray, actions)) => {
                    self.tray = Some(tray);
                    self.actions = actions;
                    true
                }
                Err(error) => {
                    keyringd_core::util::FailureDiagnostics::record_error(
                        "Tray menu thread",
                        &error,
                    );
                    false
                }
            },
            AppMenuCommand::Emit(event) => {
                let delivered = self.events.send(event).is_ok();
                if !delivered {
                    self.stop();
                }
                delivered
            }
            AppMenuCommand::Stop { action } => {
                self.stop();
                if let Some(action) = action {
                    let _ = self.events.send(action);
                }
                false
            }
        }
    }

    fn stop(&mut self) {
        self.actions.clear();
        self.tray = None;
    }

    pub(super) fn action(&self, menu_id: &str) -> Option<AppMenuEvent> {
        self.actions
            .iter()
            .find(|(id, _)| id == menu_id)
            .map(|(_, action)| action.clone())
    }
}
