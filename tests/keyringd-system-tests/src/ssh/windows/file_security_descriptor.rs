use anyhow::{Result, ensure};
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows::Win32::Foundation::{HLOCAL, LocalFree};
use windows::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1, SE_FILE_OBJECT,
    SetNamedSecurityInfoW,
};
use windows::Win32::Security::{
    ACL, DACL_SECURITY_INFORMATION, GetSecurityDescriptorDacl, GetSecurityDescriptorOwner,
    OWNER_SECURITY_INFORMATION, PROTECTED_DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID,
};
use windows::core::{BOOL, PCWSTR};

pub(super) struct FileSecurityDescriptor(PSECURITY_DESCRIPTOR);

impl FileSecurityDescriptor {
    pub(super) fn user_and_system(sid: &str) -> Result<Self> {
        let text = format!("O:{sid}D:P(A;;FA;;;{sid})(A;;FA;;;SY)")
            .encode_utf16()
            .chain(Some(0))
            .collect::<Vec<_>>();
        let mut descriptor = PSECURITY_DESCRIPTOR::default();
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                PCWSTR(text.as_ptr()),
                SDDL_REVISION_1,
                &mut descriptor,
                None,
            )
        }?;
        Ok(Self(descriptor))
    }

    pub(super) fn apply(&self, path: &Path) -> Result<()> {
        let path = path
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>();
        let mut owner = PSID::default();
        let mut defaulted = BOOL::default();
        let mut present = BOOL::default();
        let mut dacl = std::ptr::null_mut::<ACL>();
        unsafe {
            GetSecurityDescriptorOwner(self.0, &mut owner, &mut defaulted)?;
            GetSecurityDescriptorDacl(self.0, &mut present, &mut dacl, &mut defaulted)?;
        }
        ensure!(
            present.as_bool() && !dacl.is_null() && !owner.0.is_null(),
            "private file security descriptor is incomplete"
        );
        unsafe {
            SetNamedSecurityInfoW(
                PCWSTR(path.as_ptr()),
                SE_FILE_OBJECT,
                OWNER_SECURITY_INFORMATION
                    | DACL_SECURITY_INFORMATION
                    | PROTECTED_DACL_SECURITY_INFORMATION,
                Some(owner),
                None,
                Some(dacl),
                None,
            )
        }
        .ok()?;
        Ok(())
    }
}

impl Drop for FileSecurityDescriptor {
    fn drop(&mut self) {
        unsafe {
            let _ = LocalFree(Some(HLOCAL(self.0.0)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ssh::windows::current_user::CurrentUser;
    use windows::Win32::Security::Authorization::GetNamedSecurityInfoW;
    use windows::Win32::Security::{
        ACCESS_ALLOWED_ACE, EqualSid, GetAce, GetSecurityDescriptorControl, INHERITED_ACE,
        SE_DACL_PROTECTED,
    };

    keyringd_testkit::scenario! {
        group "FileSecurityDescriptor::user_and_system" {
            case "Authorized keys is owned by the user and accessible only to user and system" {
                verify "give only the current user and SYSTEM access to the authorized_keys file" {
                    (|| -> Result<()> {
                        let directory = tempfile::tempdir()?;
                        let path = directory.path().join("authorized_keys");
                        std::fs::write(&path, "test key")?;
                        let sid = CurrentUser::new()?.sid_string()?;
                        let expected = FileSecurityDescriptor::user_and_system(&sid)?;
                        expected.apply(&path)?;

                        let path = path
                            .as_os_str()
                            .encode_wide()
                            .chain(Some(0))
                            .collect::<Vec<_>>();
                        let mut descriptor = PSECURITY_DESCRIPTOR::default();
                        unsafe {
                            GetNamedSecurityInfoW(
                                PCWSTR(path.as_ptr()),
                                SE_FILE_OBJECT,
                                OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
                                None,
                                None,
                                None,
                                None,
                                &mut descriptor,
                            )
                        }
                        .ok()?;
                        let actual = FileSecurityDescriptor(descriptor);
                        let mut expected_owner = PSID::default();
                        let mut actual_owner = PSID::default();
                        let mut defaulted = BOOL::default();
                        let mut present = BOOL::default();
                        let mut expected_dacl = std::ptr::null_mut::<ACL>();
                        let mut actual_dacl = std::ptr::null_mut::<ACL>();
                        let mut control = 0;
                        let mut revision = 0;
                        unsafe {
                            GetSecurityDescriptorOwner(expected.0, &mut expected_owner, &mut defaulted)?;
                            GetSecurityDescriptorOwner(actual.0, &mut actual_owner, &mut defaulted)?;
                            EqualSid(expected_owner, actual_owner)?;
                            GetSecurityDescriptorDacl(
                                expected.0,
                                &mut present,
                                &mut expected_dacl,
                                &mut defaulted,
                            )?;
                            GetSecurityDescriptorDacl(actual.0, &mut present, &mut actual_dacl, &mut defaulted)?;
                            GetSecurityDescriptorControl(actual.0, &mut control, &mut revision)?;
                        }
                        ensure!(
                            present.as_bool() && !actual_dacl.is_null(),
                            "file must have an explicit DACL"
                        );
                        assert_ne!(
                            control & SE_DACL_PROTECTED.0,
                            0,
                            "DACL must block inherited access"
                        );
                        assert_eq!(
                            unsafe { (*actual_dacl).AceCount },
                            2,
                            "only user and SYSTEM may have access"
                        );
                        // Windows may serialize the same SID as LA and add the AI control flag.
                        // Compare the actual SIDs and access masks instead of SDDL spelling.
                        for expected_index in 0..2 {
                            let mut expected_ace = std::ptr::null_mut();
                            unsafe {
                                GetAce(expected_dacl, expected_index, &mut expected_ace)?;
                            }
                            let expected_ace = unsafe { &*expected_ace.cast::<ACCESS_ALLOWED_ACE>() };
                            let mut found = false;
                            for actual_index in 0..2 {
                                let mut actual_ace = std::ptr::null_mut();
                                unsafe {
                                    GetAce(actual_dacl, actual_index, &mut actual_ace)?;
                                }
                                let actual_ace = unsafe { &*actual_ace.cast::<ACCESS_ALLOWED_ACE>() };
                                assert_eq!(
                                    actual_ace.Header.AceType, expected_ace.Header.AceType,
                                    "access must be allowed"
                                );
                                assert_eq!(
                                    u32::from(actual_ace.Header.AceFlags) & INHERITED_ACE.0,
                                    0,
                                    "ACE must not be inherited"
                                );
                                found |= actual_ace.Mask == expected_ace.Mask
                                    && unsafe {
                                        EqualSid(
                                            PSID(std::ptr::addr_of!(actual_ace.SidStart).cast_mut().cast()),
                                            PSID(std::ptr::addr_of!(expected_ace.SidStart).cast_mut().cast()),
                                        )
                                        .is_ok()
                                    };
                            }
                            assert!(found, "required user or SYSTEM access is missing");
                        }
                        Ok(())
                    })()
                    .unwrap();
                }
            }
        }
    }
}
