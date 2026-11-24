use super::{ProfileMutation, ProfilesWindowEvent};
use keyringd_core::application::KeyringdApplication;
use keyringd_core::service::{DeviceKeyService, PasskeyService};
use std::sync::Arc;

#[derive(Clone, PartialEq)]
pub struct ProfileMutationRequest {
    pub id: u64,
    pub mutation: ProfileMutation,
}

impl ProfileMutationRequest {
    pub async fn execute<D, F>(
        self,
        application: Arc<KeyringdApplication<D, F>>,
    ) -> ProfilesWindowEvent
    where
        D: DeviceKeyService + 'static,
        F: PasskeyService + 'static,
    {
        ProfilesWindowEvent::MutationFinished {
            request_id: self.id,
            result: self.mutation.execute(application).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profiles_window::test_support::BlockingProfileRepository;
    use keyringd_core::application::PasskeyChoice;
    use keyringd_core::application::ProfileSetupRequest;
    use keyringd_core::test_support::{
        MemoryProfileRepository, SoftwareDeviceKey, TestPasskeyService,
    };

    keyringd_testkit::scenario! {
        group "ProfileMutationRequest::execute" {
            case "Synchronous storage runs off the ui thread" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "run the repository mutation on a blocking worker" {
                    let initial_store = Arc::new(MemoryProfileRepository::default());
                    let application = KeyringdApplication::new(
                        SoftwareDeviceKey::generating(),
                        TestPasskeyService::default(),
                        initial_store.clone(),
                    )
                    .unwrap();
                    let profile = application
                        .setup_profile(
                            &Default::default(),
                            &Default::default(),
                            ProfileSetupRequest::new("Original".into(), PasskeyChoice::CreateNew),
                            |_| {},
                            keyringd_core::test_support::reject_credential_selection,
                        )
                        .await
                        .unwrap();
                    let (store, entered, resume) = BlockingProfileRepository::new(initial_store.snapshot().unwrap());
                    let application = KeyringdApplication::new(
                        SoftwareDeviceKey::generating(),
                        TestPasskeyService::default(),
                        Arc::new(store),
                    )
                    .unwrap();
                    let request = ProfileMutationRequest {
                        id: 1,
                        mutation: ProfileMutation::SetEnabled {
                            id: profile.id().clone(),
                            enabled: false,
                        },
                    };
                    let task = tokio::spawn(request.execute(application.clone()));
                    let saving_thread = tokio::time::timeout(std::time::Duration::from_secs(5), entered)
                        .await
                        .unwrap()
                        .unwrap();
                    let enabled_while_saving = application.profile(profile.id()).unwrap().is_enabled();
                    resume.send(()).unwrap();
                    assert_ne!(saving_thread, std::thread::current().id());
                    assert!(enabled_while_saving);
                    assert!(matches!(
                        task.await.unwrap(),
                        ProfilesWindowEvent::MutationFinished {
                            request_id: 1,
                            result: Ok(())
                        }
                    ));
                    assert!(!application.profile(profile.id()).unwrap().is_enabled());
                }
            }
        }
    }
}
