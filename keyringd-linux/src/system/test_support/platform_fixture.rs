use std::path::Path;
use std::sync::{Arc, MutexGuard};

use keyringd_core::application::KeyringdApplication;
use keyringd_core::application::PasskeyChoice;
use keyringd_core::application::ProfileSetupRequest;
use keyringd_core::model::ProfileId;
use keyringd_core::persistence::ProfileRepository;
use tempfile::TempDir;

use crate::system::device_key::EsapiDeviceKeyService;
use crate::system::device_key::test_support::TPM_LOCK;
use crate::system::device_key::test_support::ensure_swtpm;
use keyringd_core::persistence::FileProfileRepository;
use keyringd_core::test_support::TestPasskeyService;

use super::TestApplication;

pub(crate) struct PlatformFixture {
    _tpm_guard: MutexGuard<'static, ()>,
    data_dir: TempDir,
    pub application: Arc<TestApplication>,
}

impl PlatformFixture {
    // Keep exclusive access to the shared TPM for the fixture's entire lifetime.
    #[allow(clippy::await_holding_lock)]
    pub(crate) async fn new() -> Self {
        init_tracing();
        let tpm_guard = TPM_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        ensure_swtpm();
        let data_dir = tempfile::tempdir().unwrap();
        let application = Self::open_application(data_dir.path()).await;
        Self {
            _tpm_guard: tpm_guard,
            data_dir,
            application,
        }
    }

    pub(crate) async fn complete_initial_profile(&self) -> ProfileId {
        let profile = self
            .application
            .setup_profile(
                &Default::default(),
                &Default::default(),
                ProfileSetupRequest::new("Default".to_owned(), PasskeyChoice::CreateNew),
                |_| {},
                keyringd_core::test_support::reject_credential_selection,
            )
            .await
            .unwrap();
        profile.id().clone()
    }

    pub(crate) async fn reopen_application(&self) -> Arc<TestApplication> {
        Self::open_application(self.data_dir.path()).await
    }

    pub(crate) fn data_dir(&self) -> &Path {
        self.data_dir.path()
    }

    async fn open_application(data_dir: &Path) -> Arc<TestApplication> {
        let store: Arc<dyn ProfileRepository> = Arc::new(FileProfileRepository::new(Arc::new(
            crate::system::persistence::LinuxAtomicFile::new(&data_dir.join("profiles.json")),
        )));
        KeyringdApplication::new(
            EsapiDeviceKeyService::new(tss_esapi::TctiNameConf::Swtpm(Default::default()), None),
            TestPasskeyService::new(),
            store,
        )
        .unwrap()
    }
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_test_writer()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env().add_directive(
                "keyringd_context=info"
                    .parse()
                    .expect("valid context log directive"),
            ),
        )
        .try_init();
}
