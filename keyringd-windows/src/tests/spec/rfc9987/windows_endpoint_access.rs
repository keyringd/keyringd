use std::os::windows::io::AsRawHandle;
use std::time::Duration;

use keyringd_ssh_agent_protocol::SshAgentListener;
use tokio::net::windows::named_pipe::ClientOptions;
use windows::Win32::Foundation::{HANDLE, HLOCAL, LocalFree};
use windows::Win32::Security::Authorization::{
    ConvertSecurityDescriptorToStringSecurityDescriptorW, GetSecurityInfo, SDDL_REVISION_1,
    SE_KERNEL_OBJECT,
};
use windows::Win32::Security::{
    DACL_SECURITY_INFORMATION, OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR,
};

use windows::core::PWSTR;

use crate::app::handler::WindowsNamedPipeListener;
use crate::app::handler::assert_user_only_access;
use crate::system::current_user_sid;

fn security_descriptor(server: &tokio::net::windows::named_pipe::NamedPipeServer) -> String {
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    let information = DACL_SECURITY_INFORMATION | OWNER_SECURITY_INFORMATION;
    let status = unsafe {
        GetSecurityInfo(
            HANDLE(server.as_raw_handle()),
            SE_KERNEL_OBJECT,
            information,
            None,
            None,
            None,
            None,
            Some(&mut descriptor),
        )
    };
    assert_eq!(status.0, 0, "GetSecurityInfo failed with {status:?}");

    let mut encoded = PWSTR::null();
    unsafe {
        ConvertSecurityDescriptorToStringSecurityDescriptorW(
            descriptor,
            SDDL_REVISION_1,
            information,
            &mut encoded,
            None,
        )
        .unwrap();
    }
    let result = unsafe { encoded.to_string().unwrap() };
    unsafe {
        let _ = LocalFree(Some(HLOCAL(encoded.0.cast())));
        let _ = LocalFree(Some(HLOCAL(descriptor.0)));
    }
    result
}

keyringd_testkit::scenario! {
    case "Restricts the Windows Agent pipe to authorized local principals" {
        spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-6"
        spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-10"
        async: tokio;
        timeout: Duration::from_secs(10);

        given "a uniquely named production Windows Agent pipe listener" {
            let pipe = format!(r"\\.\pipe\keyringd-rfc9987-acl-{}", std::process::id());
            let mut listener = WindowsNamedPipeListener::new(pipe.clone());
        }
        when "a local client connects and the server pipe security descriptor is read" {
            let accepted = tokio::spawn(async move {
                let server = listener.accept().await.unwrap();
                (listener, server)
            });
            let client = loop {
                match ClientOptions::new().open(&pipe) {
                    Ok(client) => break client,
                    Err(error) if error.raw_os_error() == Some(2) => tokio::task::yield_now().await,
                    Err(error) => panic!("failed to connect to test Agent pipe: {error}"),
                }
            };
            let (mut listener, server) = accepted.await.unwrap();
            let descriptor = security_descriptor(&server);
        }
        when "the same user connects to a subsequent pipe instance" {
            let accepted = tokio::spawn(async move { listener.accept().await.unwrap() });
            let second_client = loop {
                match ClientOptions::new().open(&pipe) {
                    Ok(client) => break client,
                    Err(error) if matches!(error.raw_os_error(), Some(2 | 231)) => tokio::task::yield_now().await,
                    Err(error) => panic!("failed to connect to subsequent Agent pipe: {error}"),
                }
            };
            let second_server = accepted.await.unwrap();
            let second_descriptor = security_descriptor(&second_server);
        }
        then "both protected DACLs grant access only to the process user" {
            let sid = current_user_sid().unwrap();
            for descriptor in [&descriptor, &second_descriptor] {
                assert_user_only_access(descriptor, &sid);
            }
            drop(client);
            drop(server);
            drop(second_client);
            drop(second_server);
        }
    }
}
