use keyringd_system_tests::application::SystemTestApplication;
use keyringd_system_tests::application::features::profile_setup::ProfileSetupWindowDriver;
#[cfg(unix)]
use keyringd_system_tests::authenticator::UsbSecurityKey;
use keyringd_system_tests::environment::SystemTestEnvironment;
use std::time::{Duration, Instant};

use keyringd_system_tests::application::features::profiles::TrustEntries;
use keyringd_testkit::agent::AgentIdentity;

pub struct RecoverySources {
    pub passkeys: [serde_json::Value; 2],
    pub trust: [TrustEntries; 2],
}

impl RecoverySources {
    #[cfg(unix)]
    pub fn create(environment: &mut impl SystemTestEnvironment, key: &UsbSecurityKey) -> Self {
        Self::create_using(environment, |environment, name| {
            super::profile_fixture::create_recovery_source(environment, key, name).unwrap();
        })
    }

    pub fn create_hybrid(
        environment: &mut impl SystemTestEnvironment,
        phone: &keyringd_system_tests::authenticator::HybridPhone,
    ) -> Self {
        Self::create_using(environment, |environment, name| {
            super::hybrid_profiles::create(environment, phone, name).unwrap();
        })
    }

    fn create_using<E: SystemTestEnvironment>(
        environment: &mut E,
        mut create: impl FnMut(&mut E, &str),
    ) -> Self {
        create(environment, "first-passkey");
        let first_end = format!(
            "keyringd — {}",
            environment.application().local_timestamp().unwrap()
        );
        let first = crate::scenario_steps::profile_management::public_keys::read(
            environment,
            "first-passkey",
        )
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let label = format!(
                "keyringd — {}",
                environment.application().local_timestamp().unwrap()
            );
            if label > first_end {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "clock did not advance beyond first Passkey creation"
            );
            std::thread::sleep(Duration::from_millis(25));
        }
        environment
            .application()
            .profile_setup_window()
            .open()
            .unwrap();
        create(environment, "second-passkey");
        let second = crate::scenario_steps::profile_management::public_keys::read(
            environment,
            "second-passkey",
        )
        .unwrap();
        assert_ne!(first.ca, second.ca);
        assert_ne!(
            AgentIdentity::from_openssh(&first.passkey)
                .unwrap()
                .public_key,
            AgentIdentity::from_openssh(&second.passkey)
                .unwrap()
                .public_key
        );
        let stored = environment.application().stored_profiles().unwrap();
        assert_eq!(stored.len(), 2);
        let passkeys = ["first-passkey", "second-passkey"].map(|name| {
            stored.iter().find(|profile| profile["displayName"] == name)
                .unwrap_or_else(|| panic!("missing recovery source {name:?}: {stored:?}"))["passkey"].clone()
        });
        assert_ne!(passkeys[0], passkeys[1]);
        Self {
            passkeys,
            trust: [first, second],
        }
    }

    pub fn match_candidates(&self, actual: &[String]) -> [String; 2] {
        assert_eq!(actual.len(), 2, "{actual:?}");
        // Use labels only to identify the two creation-ordered candidates.
        // The scenario verifies the recovered credential and CA, not timestamp formatting.
        let mut labels = [actual[0].clone(), actual[1].clone()];
        labels.sort();
        assert_ne!(
            labels[0], labels[1],
            "recovery candidates must be distinguishable"
        );
        labels
    }
}
