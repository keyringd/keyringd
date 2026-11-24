use super::SshConnectionTestResult;
use crate::model::{Profile, ProfileSshIdentity, ProfileSshIdentityKind};

/// The result and exact public identity selected immediately before the test.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SshConnectionTestReport {
    pub result: SshConnectionTestResult,
    pub identity: ProfileSshIdentityKind,
    pub trust_entry: String,
    pub public_key_algorithm: String,
    pub openssh_algorithm_directives: Vec<String>,
}

impl SshConnectionTestReport {
    pub(crate) fn new(
        profile: &Profile,
        identity: &ProfileSshIdentity,
        result: SshConnectionTestResult,
    ) -> Self {
        let algorithm = identity.key_blob().algorithm();
        let mut openssh_algorithm_directives =
            vec![format!("PubkeyAcceptedAlgorithms +{algorithm}")];
        if identity.kind() == ProfileSshIdentityKind::DeviceCertificate {
            openssh_algorithm_directives.push(format!(
                "CASignatureAlgorithms +{}",
                profile.device_key().certificate().signature().algorithm()
            ));
        }
        Self {
            result,
            identity: identity.kind(),
            trust_entry: identity.kind().trust_entry(profile),
            public_key_algorithm: algorithm.as_str().to_owned(),
            openssh_algorithm_directives,
        }
    }
}
