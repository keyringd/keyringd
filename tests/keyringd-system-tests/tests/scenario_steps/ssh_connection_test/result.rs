use keyringd_system_tests::application::features::ssh_connection_test::{
    SshConnectionIdentity, SshConnectionResult,
};

pub fn assert_success(result: &SshConnectionResult, host: &str, identity: SshConnectionIdentity) {
    assert_eq!(result.title, "SSH Connection Succeeded");
    assert_eq!(result.host, host);
    assert_eq!(
        result.diagnostic,
        format!(
            "OpenSSH authenticated with this profile’s {}.",
            identity.result_name()
        )
    );
    assert!(result.rejection.is_none());
}

pub fn assert_failure(result: &SshConnectionResult, host: &str, title: &str, diagnostic: &str) {
    assert_eq!(result.title, title);
    assert_eq!(result.host, host);
    assert_eq!(result.diagnostic, diagnostic);
}

pub fn assert_rejection_guidance(
    result: &SshConnectionResult,
    identity: SshConnectionIdentity,
    entry: &str,
) {
    let guidance = result
        .rejection
        .as_ref()
        .expect("identity rejection must offer registration guidance");
    assert_eq!(guidance.entry, entry);
    for required in [
        "authorized_keys",
        "PubkeyAuthentication yes",
        "AuthorizedKeysFile",
        "PubkeyAcceptedAlgorithms",
    ] {
        assert!(guidance.instructions.contains(required), "{guidance:?}");
    }
    if identity == SshConnectionIdentity::DeviceCertificate {
        assert!(
            guidance.instructions.contains("CASignatureAlgorithms"),
            "{guidance:?}"
        );
    } else {
        assert!(
            guidance
                .instructions
                .lines()
                .any(|line| line.trim() == "ecdsa-sha2-nistp256"),
            "{guidance:?}"
        );
    }
}
