use keyringd_core::application::{ActionId, KeyringdApplication};
use keyringd_core::model::ProfileId;
use keyringd_core::service::{DeviceKeyService, PasskeyService};
use std::sync::Arc;

#[derive(Clone, PartialEq)]
pub enum ProfileMutation {
    Rename { id: ProfileId, name: String },
    SetEnabled { id: ProfileId, enabled: bool },
    Reorder(Vec<ProfileId>),
}

impl ProfileMutation {
    /// Runs synchronous persistence on the blocking pool. The save outlives its window.
    pub async fn execute<D, F>(
        self,
        application: Arc<KeyringdApplication<D, F>>,
    ) -> Result<(), String>
    where
        D: DeviceKeyService + 'static,
        F: PasskeyService + 'static,
    {
        let action_id = ActionId::current().unwrap_or_else(ActionId::generate);
        tokio::task::spawn_blocking(move || {
            action_id.sync_scope(|| {
                match self {
                    Self::Rename { id, name } => application.rename_profile(&id, name),
                    Self::SetEnabled { id, enabled } => {
                        application.set_profile_enabled(&id, enabled)
                    }
                    Self::Reorder(ids) => application.set_profile_order(&ids).map(|_| ()),
                }
                .map_err(|error| error.report_user_error(&error.user_message()))
            })
        })
        .await
        .map_err(|error| {
            keyringd_core::util::report_user_error(
                &error,
                "Could not update the profiles. Try again.",
            )
        })?
    }
}
