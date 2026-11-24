use windows::Win32::Foundation::{HLOCAL, LocalFree};

use windows::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
    ConvertStringSidToSidW, SDDL_REVISION_1,
};

use windows::Win32::Security::{
    ACCESS_ALLOWED_ACE, ACE_HEADER, ACL, EqualSid, GetAce, GetSecurityDescriptorControl,
    GetSecurityDescriptorDacl, PSECURITY_DESCRIPTOR, PSID, SE_DACL_PROTECTED,
};

use windows::Win32::Storage::FileSystem::FILE_ALL_ACCESS;

use windows::core::{BOOL, PCWSTR, PWSTR, w};

pub(crate) fn assert_user_only_access(sddl: &str, user_sid: &str) {
    let encoded = sddl.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let user_sid = user_sid.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    let mut expected_sid = PSID::default();
    let mut dacl = std::ptr::null_mut::<ACL>();
    let mut present = BOOL::default();
    let mut defaulted = BOOL::default();
    let mut control = 0;
    let mut revision = 0;
    unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            PCWSTR(encoded.as_ptr()),
            SDDL_REVISION_1,
            &mut descriptor,
            None,
        )
        .unwrap();
        ConvertStringSidToSidW(PCWSTR(user_sid.as_ptr()), &mut expected_sid).unwrap();
        GetSecurityDescriptorControl(descriptor, &mut control, &mut revision).unwrap();
        GetSecurityDescriptorDacl(descriptor, &mut present, &mut dacl, &mut defaulted).unwrap();
        assert!(
            present.as_bool() && !dacl.is_null(),
            "missing DACL in {sddl}"
        );
        assert_eq!((*dacl).AceCount, 1, "unexpected access entries in {sddl}");
        let mut entry = std::ptr::null_mut();
        GetAce(dacl, 0, &mut entry).unwrap();
        // ACCESS_ALLOWED_ACE_TYPE is zero. Validate the header before reading
        // the remaining fields as an access-allowed ACE.
        assert_eq!(
            (*entry.cast::<ACE_HEADER>()).AceType,
            0,
            "unexpected ACE type in {sddl}"
        );
        let entry = &*entry.cast::<ACCESS_ALLOWED_ACE>();
        let mask = entry.Mask;
        let authorized = EqualSid(
            PSID((&entry.SidStart as *const u32).cast_mut().cast()),
            expected_sid,
        )
        .is_ok();
        let _ = LocalFree(Some(HLOCAL(expected_sid.0)));
        let _ = LocalFree(Some(HLOCAL(descriptor.0)));
        assert_ne!(
            control & SE_DACL_PROTECTED.0,
            0,
            "DACL is not protected in {sddl}"
        );
        assert_eq!(mask, FILE_ALL_ACCESS.0, "unexpected permissions in {sddl}");
        assert!(authorized, "process user is not authorized in {sddl}");
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "assert_user_only_access" {
            case "Compares aliased and numeric sids by identity" {
                verify "resolve the alias and numeric SID to the same identity" {
                    assert_user_only_access("O:SYD:P(A;;FA;;;SY)", "S-1-5-18");
                    // Hosted Windows runners run as the local Administrator. Windows emits
                    // its LA alias rather than its machine-specific numeric SID in SDDL.
                    let mut sid = PSID::default();
                    let mut encoded = PWSTR::null();
                    unsafe {
                        ConvertStringSidToSidW(w!("LA"), &mut sid).unwrap();
                        ConvertSidToStringSidW(sid, &mut encoded).unwrap();
                    }
                    let numeric_sid = unsafe { encoded.to_string().unwrap() };
                    unsafe {
                        let _ = LocalFree(Some(HLOCAL(encoded.0.cast())));
                        let _ = LocalFree(Some(HLOCAL(sid.0)));
                    }
                    assert_user_only_access("D:P(A;;FA;;;LA)", &numeric_sid);
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "assert_user_only_access" {
            case "Rejects access granted to a different sid" {
                verify "detect an access entry belonging to an unrelated SID" {
                    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        assert_user_only_access("O:SYD:P(A;;FA;;;SY)", "S-1-5-19");
                    }))
                    .expect_err("expected the operation to panic");
                    let message = panic
                        .downcast_ref::<String>()
                        .map(String::as_str)
                        .or_else(|| panic.downcast_ref::<&str>().copied())
                        .unwrap_or("");
                    assert!(
                        message.contains("process user is not authorized"),
                        "{message}"
                    );
                }
            }
        }
    }
}
