use crate::system::device_key::{CngDeviceKeyService, CngOperationContext};
use keyringd_core::model::device_key::DevicePrivateKeyHandle;
use keyringd_core::service::DeviceKeyGenerationRequest;
use keyringd_core::service::DeviceKeyMaterial;
use keyringd_core::service::DeviceKeyProtectionChangeRequest;
use keyringd_core::service::DeviceKeyProtectionChangeResult;
use keyringd_core::service::DeviceKeyService;
use keyringd_core::service::DeviceKeySigningRequest;
use keyringd_core::test_support::SoftwareDeviceKey;

pub(crate) enum WindowsDeviceKeyBackend {
    Native {
        service: CngDeviceKeyService,
        ledger: Option<std::path::PathBuf>,
    },
    Test(SoftwareDeviceKey),
}
use super::DeviceKeyBackendError;

impl From<CngDeviceKeyService> for WindowsDeviceKeyBackend {
    fn from(service: CngDeviceKeyService) -> Self {
        Self::Native {
            service,
            ledger: None,
        }
    }
}

impl DeviceKeyService for WindowsDeviceKeyBackend {
    type Error = DeviceKeyBackendError;
    type OperationContext = CngOperationContext;

    async fn check_availability(&self) -> Result<(), Self::Error> {
        match self {
            Self::Native { service, .. } => Ok(service.check_availability().await?),
            Self::Test(service) => Ok(service.check_availability().await?),
        }
    }

    async fn generate_key(
        &self,
        context: &CngOperationContext,
        request: DeviceKeyGenerationRequest<'_>,
    ) -> Result<DeviceKeyMaterial, Self::Error> {
        match self {
            Self::Native {
                service,
                ledger: Some(ledger),
            } => {
                // This intent survives a crash during CNG creation and a Profile reset.
                let file = tempfile::Builder::new()
                    .prefix(KEY_PREFIX)
                    .tempfile_in(ledger)
                    .map_err(anyhow::Error::from)?;
                let (file, path) = file.keep().map_err(anyhow::Error::from)?;
                file.sync_all().map_err(anyhow::Error::from)?;
                let name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .ok_or_else(|| anyhow::anyhow!("invalid fixture key name"))?;
                Ok(service.generate_named_key(context, request, name)?)
            }
            Self::Native { service, .. } => Ok(service.generate_key(context, request).await?),
            Self::Test(service) => Ok(service.generate_key(&(), request).await?),
        }
    }

    async fn change_key_protection(
        &self,
        context: &CngOperationContext,
        request: DeviceKeyProtectionChangeRequest<'_>,
    ) -> Result<DeviceKeyProtectionChangeResult, Self::Error> {
        match self {
            Self::Native { .. } => {
                let key = self.generate_key(context, DeviceKeyGenerationRequest { profile_name: request.profile_name, reason: keyringd_core::service::DeviceKeyGenerationReason::PasswordProtectionChange, algorithm: request.algorithm, local_authentication: request.change.requested() }).await?;
                Ok(DeviceKeyProtectionChangeResult::replacement(
                    key,
                    request.handle.clone(),
                ))
            }
            Self::Test(service) => Ok(service.change_key_protection(&(), request).await?),
        }
    }
    async fn sign(
        &self,
        context: &CngOperationContext,
        request: DeviceKeySigningRequest<'_>,
    ) -> Result<keyringd_core::service::DeviceKeySignature, Self::Error> {
        match self {
            Self::Native { service, .. } => Ok(service.sign(context, request).await?),
            Self::Test(service) => Ok(service.sign(&(), request).await?),
        }
    }
    async fn delete_key(
        &self,
        context: &CngOperationContext,
        handle: &DevicePrivateKeyHandle,
    ) -> Result<(), Self::Error> {
        match self {
            Self::Native { service, ledger } => {
                DeviceKeyService::delete_key(service, context, handle).await?;
                if let Some(ledger) = ledger {
                    let name = CngDeviceKeyService::key_name(handle)?;
                    let path = ledger.join(name);
                    match std::fs::remove_file(path) {
                        Ok(()) => {}
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                        Err(error) => return Err(anyhow::Error::from(error).into()),
                    }
                }
                Ok(())
            }
            Self::Test(service) => Ok(service.delete_key(&(), handle).await?),
        }
    }
    fn is_user_cancellation(&self, error: &Self::Error) -> bool {
        match (self, error) {
            (Self::Native { service, .. }, DeviceKeyBackendError::Native(error)) => {
                service.is_user_cancellation(error)
            }
            (Self::Test(service), DeviceKeyBackendError::Test(error)) => {
                service.is_user_cancellation(error)
            }
            _ => false,
        }
    }
}

const KEY_PREFIX: &str = "keyringd-test-device-key-";

impl WindowsDeviceKeyBackend {
    pub(super) fn for_case(ledger: std::path::PathBuf) -> anyhow::Result<Self> {
        std::fs::create_dir_all(&ledger)?;
        Ok(Self::Native {
            service: CngDeviceKeyService::software(),
            ledger: Some(ledger),
        })
    }

    /// Only keys explicitly registered by this case may be removed.
    pub(super) fn cleanup_case_keys(ledger: &std::path::Path) -> anyhow::Result<()> {
        use anyhow::ensure;
        if !ledger.try_exists()? {
            return Ok(());
        }
        let service = CngDeviceKeyService::software();
        let context = CngOperationContext::for_window(0);
        let mut failures = Vec::new();
        for entry in std::fs::read_dir(ledger)? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("invalid fixture key name"))?;
            ensure!(
                entry.file_type()?.is_file()
                    && name.starts_with(KEY_PREFIX)
                    && name
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.')),
                "invalid fixture key intent: {name}"
            );
            let result = service.delete_named_key(&context, name);
            // A write-ahead intent can refer to a key that was never created or was already deleted.
            let absent = matches!(&result, Err(crate::system::device_key::CngError::Windows(error)) if matches!(error.code(), windows::Win32::Foundation::NTE_BAD_KEYSET | windows::Win32::Foundation::NTE_NOT_FOUND));
            if result.is_ok() || absent {
                if let Err(error) = std::fs::remove_file(entry.path()) {
                    failures.push(format!("{name}: {error}"));
                }
            } else if let Err(error) = result {
                failures.push(format!("{name}: {error}"));
            }
        }
        ensure!(
            failures.is_empty(),
            "case CNG key cleanup failed: {}",
            failures.join("; ")
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    keyringd_testkit::scenario! {
        group "WindowsDeviceKeyBackend case cleanup" {
            case "Persisted keys and unfinished creation intents are recovered without a running fixture or profiles" {
                async: tokio;
                verify "cleanup uses the case ledger after its backend was dropped" {
                    let directory = tempfile::tempdir().unwrap().keep();
                    let backend = WindowsDeviceKeyBackend::for_case(directory.clone()).unwrap();
                    let context = CngOperationContext::for_window(0);
                    let material = backend.generate_key(&context, DeviceKeyGenerationRequest {
                        profile_name: "case-cleanup", reason: keyringd_core::service::DeviceKeyGenerationReason::InitialSetup,
                        algorithm: keyringd_core::model::device_key::DeviceKeyAlgorithm::EcdsaP256,
                        local_authentication: keyringd_core::model::device_key::LocalAuthentication::NotRequired,
                    }).await.unwrap();
                    drop(backend);
                    tempfile::Builder::new().prefix(KEY_PREFIX).tempfile_in(&directory).unwrap().keep().unwrap();
                    WindowsDeviceKeyBackend::cleanup_case_keys(&directory).unwrap();
                    let error = CngDeviceKeyService::software().delete_key(&context, &material.handle).unwrap_err();
                    assert!(matches!(error, crate::system::device_key::CngError::Windows(error) if matches!(error.code(), windows::Win32::Foundation::NTE_BAD_KEYSET | windows::Win32::Foundation::NTE_NOT_FOUND)));
                    assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 0);
                    WindowsDeviceKeyBackend::cleanup_case_keys(&directory).unwrap();
                    std::fs::remove_dir(directory).unwrap();
                }
            }
        }
    }
}
