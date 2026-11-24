use objc2_service_management::{SMAppService, SMAppServiceStatus};

pub(crate) struct MacOsLoginItem;

impl MacOsLoginItem {
    /// Register once. Respect a user who later disables login startup in Settings.
    pub(crate) fn register() -> anyhow::Result<()> {
        let service = unsafe { SMAppService::mainAppService() };
        Self::register_with(
            || unsafe { service.status() },
            || {
                unsafe { service.registerAndReturnError() }.map_err(|error| {
                    anyhow::anyhow!("Could not register Keyringd to start at login: {error}")
                })
            },
        )
    }

    fn register_with(
        mut status: impl FnMut() -> SMAppServiceStatus,
        register: impl FnOnce() -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        let before = status();
        tracing::info!(
            state = %Self::status_name(before),
            raw_status = before.0,
            "Checking macOS login item registration"
        );
        match before {
            SMAppServiceStatus::Enabled => return Ok(()),
            SMAppServiceStatus::RequiresApproval => {
                Self::log_approval_required();
                return Ok(());
            }
            SMAppServiceStatus::NotRegistered => {}
            SMAppServiceStatus::NotFound => {
                // A missing status record does not mean registration is impossible.
                tracing::warn!("macOS login item status is NotFound; attempting registration");
            }
            other => {
                anyhow::bail!("Unknown macOS login item status before registration: {other:?}")
            }
        }

        tracing::info!("Registering Keyringd to start at login");
        register()?;
        let after = status();
        tracing::info!(
            state = %Self::status_name(after),
            raw_status = after.0,
            "Checked macOS login item status after registration"
        );
        match after {
            SMAppServiceStatus::Enabled => Ok(()),
            SMAppServiceStatus::RequiresApproval => {
                Self::log_approval_required();
                Ok(())
            }
            other => {
                anyhow::bail!("Keyringd login item is not enabled after registration: {other:?}")
            }
        }
    }

    fn log_approval_required() {
        tracing::warn!(
            "Keyringd login startup requires approval in System Settings > General > Login Items"
        );
    }

    fn status_name(status: SMAppServiceStatus) -> &'static str {
        match status {
            SMAppServiceStatus::NotRegistered => "NotRegistered",
            SMAppServiceStatus::Enabled => "Enabled",
            SMAppServiceStatus::RequiresApproval => "RequiresApproval",
            SMAppServiceStatus::NotFound => "NotFound",
            _ => "Unknown",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn missing_or_unregistered_service_is_registered_and_checked_again() {
        for before in [
            SMAppServiceStatus::NotRegistered,
            SMAppServiceStatus::NotFound,
        ] {
            let mut statuses = [before, SMAppServiceStatus::Enabled].into_iter();
            let calls = Cell::new(0);
            MacOsLoginItem::register_with(
                || statuses.next().expect("unexpected status query"),
                || {
                    calls.set(calls.get() + 1);
                    Ok(())
                },
            )
            .unwrap();
            assert_eq!(calls.get(), 1);
            assert!(statuses.next().is_none());
        }
    }

    #[test]
    fn enabled_or_pending_service_is_not_registered_again() {
        for before in [
            SMAppServiceStatus::Enabled,
            SMAppServiceStatus::RequiresApproval,
        ] {
            let mut statuses = [before].into_iter();
            MacOsLoginItem::register_with(
                || statuses.next().expect("unexpected status query"),
                || panic!("must respect the existing login item"),
            )
            .unwrap();
        }
    }

    #[test]
    fn registration_failure_is_propagated_without_another_status_query() {
        let mut statuses = [SMAppServiceStatus::NotFound].into_iter();
        let error = MacOsLoginItem::register_with(
            || {
                statuses
                    .next()
                    .expect("must stop after registration failure")
            },
            || anyhow::bail!("registration denied"),
        )
        .unwrap_err();
        assert_eq!(error.to_string(), "registration denied");
    }

    #[test]
    fn successful_registration_can_require_approval() {
        let mut statuses = [
            SMAppServiceStatus::NotFound,
            SMAppServiceStatus::RequiresApproval,
        ]
        .into_iter();
        MacOsLoginItem::register_with(|| statuses.next().unwrap(), || Ok(())).unwrap();
        assert!(statuses.next().is_none());
    }

    #[test]
    fn successful_api_call_without_a_registered_state_is_an_error() {
        for after in [
            SMAppServiceStatus::NotRegistered,
            SMAppServiceStatus::NotFound,
            SMAppServiceStatus(99),
        ] {
            let mut statuses = [SMAppServiceStatus::NotRegistered, after].into_iter();
            let error =
                MacOsLoginItem::register_with(|| statuses.next().unwrap(), || Ok(())).unwrap_err();
            let message = error.to_string();
            assert!(message.contains("not enabled after registration"));
            assert!(message.contains(&format!("{after:?}")));
        }
    }

    #[test]
    fn unknown_initial_status_is_an_error_without_registering() {
        let unknown = SMAppServiceStatus(99);
        let error = MacOsLoginItem::register_with(
            || unknown,
            || panic!("must not guess how to handle an unknown status"),
        )
        .unwrap_err();
        assert!(error.to_string().contains("before registration"));
        assert!(error.to_string().contains(&format!("{unknown:?}")));
    }
}
