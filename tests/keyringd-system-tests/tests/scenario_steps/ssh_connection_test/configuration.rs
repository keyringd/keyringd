use anyhow::Result;
use keyringd_system_tests::environment::NativeSystemTestEnvironment;
#[cfg(target_os = "macos")]
use keyringd_system_tests::environment::SystemTestEnvironment;

#[cfg(target_os = "linux")]
pub fn expected_guidance(_environment: &mut NativeSystemTestEnvironment) -> Result<String> {
    Ok("SSH is not using Keyringd. Configure SSH_AUTH_SOCK and sign out and back in. Check ~/.ssh/config for an IdentityAgent override.".into())
}

#[cfg(target_os = "windows")]
pub fn expected_guidance(_environment: &mut NativeSystemTestEnvironment) -> Result<String> {
    Ok("OpenSSH is not configured to use keyringd. Use Configure Windows SSH, or check IdentityAgent in your SSH configuration.".into())
}

#[cfg(target_os = "macos")]
pub fn expected_guidance(environment: &mut NativeSystemTestEnvironment) -> Result<String> {
    let ssh_auth_sock = environment
        .application()
        .original_agent_socket()
        .display()
        .to_string();
    Ok(format!(
        "SSH is not using Keyringd. Open Configure SSH Clients to select Keyringd. Check ~/.ssh/config for an IdentityAgent override.\n\nLogin session SSH_AUTH_SOCK: {}\nIdentityAgent: none\nkeyringd socket: {}",
        if ssh_auth_sock.is_empty() {
            "(not set)"
        } else {
            &ssh_auth_sock
        },
        environment.application().agent_socket()?
    ))
}
