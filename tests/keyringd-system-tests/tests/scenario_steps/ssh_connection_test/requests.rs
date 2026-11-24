use keyringd_system_tests::application::features::ssh_connection_test::{
    SshConnectionIdentity, SshConnectionRequest,
};

// Only the Windows service fixture records these requests; real SSH is verified on all OSes.
pub fn assert_identity_when_recorded(
    requests: &Option<Vec<SshConnectionRequest>>,
    host: &str,
    identity: SshConnectionIdentity,
    count: usize,
) {
    if let Some(requests) = requests {
        assert_eq!(requests.len(), count, "{requests:?}");
        let algorithm = match identity {
            SshConnectionIdentity::DeviceCertificate => "ecdsa-sha2-nistp256-cert-v01@openssh.com",
            SshConnectionIdentity::DeviceKey => "ecdsa-sha2-nistp256",
            SshConnectionIdentity::Passkey => "sk-ecdsa-sha2-nistp256@openssh.com",
        };
        for request in requests {
            assert_eq!(request.host, host);
            assert_eq!(request.public_identity, requests[0].public_identity);
            assert!(
                request
                    .public_identity
                    .starts_with(&format!("{algorithm} ")),
                "{request:?}"
            );
        }
    }
}
