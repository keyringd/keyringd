use p256::Sec1Point;
use windows::Win32::Security::Cryptography::NCryptDeleteKey;
use windows::Win32::Security::Cryptography::{
    BCRYPT_ECCPUBLIC_BLOB, BCRYPT_ECDSA_PUBLIC_P256_MAGIC, CERT_KEY_SPEC,
    NCRYPT_ECDSA_P256_ALGORITHM, NCRYPT_FLAGS, NCRYPT_HANDLE, NCRYPT_KEY_HANDLE,
    NCRYPT_UI_FORCE_HIGH_PROTECTION_FLAG, NCRYPT_UI_POLICY, NCRYPT_UI_POLICY_PROPERTY,
    NCRYPT_WINDOW_HANDLE_PROPERTY, NCryptCreatePersistedKey, NCryptExportKey, NCryptFinalizeKey,
    NCryptFreeObject, NCryptGetProperty, NCryptOpenKey, NCryptSetProperty, NCryptSignHash,
};
use windows::core::PCWSTR;

use super::CngError;
use super::ProviderHandle;
use super::cng_device_key_service::wide_string;
use keyringd_core::model::device_key::LocalAuthentication;
use keyringd_core::service::DeviceKeyGenerationReason;
use keyringd_core::service::DeviceKeyGenerationRequest;

const ECC_P256_COORDINATE_SIZE: usize = 32;
const ECC_KEY_BLOB_HEADER_SIZE: usize = 8;

pub(super) struct KeyHandle(NCRYPT_KEY_HANDLE);

impl KeyHandle {
    pub(super) fn create(
        provider: &ProviderHandle,
        name: &str,
        protection: LocalAuthentication,
        ui_parent: isize,
        context: DeviceKeyGenerationRequest<'_>,
    ) -> Result<Self, CngError> {
        let wide_name = wide_string(name);
        Self::create_inner(
            provider,
            PCWSTR(wide_name.as_ptr()),
            protection,
            ui_parent,
            context,
        )
    }

    pub(super) fn create_ephemeral(provider: &ProviderHandle) -> Result<Self, CngError> {
        Self::create_inner(
            provider,
            PCWSTR::null(),
            LocalAuthentication::NotRequired,
            0,
            DeviceKeyGenerationRequest {
                profile_name: "Availability check",
                reason: DeviceKeyGenerationReason::InitialSetup,
                algorithm: keyringd_core::model::device_key::DeviceKeyAlgorithm::EcdsaP256,
                local_authentication: LocalAuthentication::NotRequired,
            },
        )
    }

    fn create_inner(
        provider: &ProviderHandle,
        name: PCWSTR,
        protection: LocalAuthentication,
        ui_parent: isize,
        context: DeviceKeyGenerationRequest<'_>,
    ) -> Result<Self, CngError> {
        let mut handle = NCRYPT_KEY_HANDLE::default();
        unsafe {
            NCryptCreatePersistedKey(
                provider.0,
                &mut handle,
                NCRYPT_ECDSA_P256_ALGORITHM,
                name,
                CERT_KEY_SPEC(0),
                NCRYPT_FLAGS(0),
            )?;
            let key = Self(handle);
            set_ui_parent(key.0, ui_parent)?;
            if protection == LocalAuthentication::Required {
                let creation_title = match context.reason {
                    DeviceKeyGenerationReason::InitialSetup => {
                        format!("Create an SSH key for \"{}\"", context.profile_name)
                    }
                    DeviceKeyGenerationReason::PasswordProtectionChange => format!(
                        "Create a replacement SSH key for \"{}\"",
                        context.profile_name
                    ),
                    DeviceKeyGenerationReason::Replacement => {
                        format!("Replace the SSH key for \"{}\"", context.profile_name)
                    }
                };
                let creation_title = wide_string(&creation_title);
                let friendly_name = wide_string(&format!("{} — Keyringd", context.profile_name));
                let description = wide_string(&format!(
                    "TPM-protected SSH key for the \"{}\" profile",
                    context.profile_name
                ));
                let policy = NCRYPT_UI_POLICY {
                    dwVersion: 1,
                    dwFlags: NCRYPT_UI_FORCE_HIGH_PROTECTION_FLAG,
                    pszCreationTitle: PCWSTR(creation_title.as_ptr()),
                    pszFriendlyName: PCWSTR(friendly_name.as_ptr()),
                    pszDescription: PCWSTR(description.as_ptr()),
                };
                NCryptSetProperty(
                    NCRYPT_HANDLE(handle.0),
                    NCRYPT_UI_POLICY_PROPERTY,
                    std::slice::from_raw_parts(
                        (&policy as *const NCRYPT_UI_POLICY).cast::<u8>(),
                        std::mem::size_of::<NCRYPT_UI_POLICY>(),
                    ),
                    NCRYPT_FLAGS(0),
                )?;
            }
            tracing::debug!(process_pid = windows::Win32::System::Threading::GetCurrentProcessId(),
                key_handle = handle.0, parent_hwnd = ui_parent,
                profile = context.profile_name, reason = ?context.reason,
                "CNG key finalization starting");
            NCryptFinalizeKey(key.0, NCRYPT_FLAGS(0))?;
            Ok(key)
        }
    }

    pub(super) fn open(
        provider: &ProviderHandle,
        name: &str,
        ui_parent: isize,
    ) -> Result<Self, CngError> {
        let wide_name = wide_string(name);
        let mut handle = NCRYPT_KEY_HANDLE::default();
        unsafe {
            NCryptOpenKey(
                provider.0,
                &mut handle,
                PCWSTR(wide_name.as_ptr()),
                CERT_KEY_SPEC(0),
                NCRYPT_FLAGS(0),
            )?;
            let key = Self(handle);
            set_ui_parent(key.0, ui_parent)?;
            Ok(key)
        }
    }

    pub(super) fn local_authentication(
        &self,
    ) -> Result<keyringd_core::model::device_key::LocalAuthentication, CngError> {
        let mut length = 0;
        // CNG reports an unset UI policy as NTE_NOT_FOUND; other read errors are fatal.
        let read = unsafe {
            NCryptGetProperty(
                NCRYPT_HANDLE(self.0.0),
                NCRYPT_UI_POLICY_PROPERTY,
                None,
                &mut length,
                Default::default(),
            )
        };
        if let Err(error) = read {
            if error.code() == windows::Win32::Foundation::NTE_NOT_FOUND {
                return Ok(keyringd_core::model::device_key::LocalAuthentication::NotRequired);
            }
            return Err(error.into());
        }
        let mut data = vec![0u8; length as usize];
        unsafe {
            NCryptGetProperty(
                NCRYPT_HANDLE(self.0.0),
                NCRYPT_UI_POLICY_PROPERTY,
                Some(&mut data),
                &mut length,
                Default::default(),
            )?;
        }
        if (length as usize) < std::mem::size_of::<NCRYPT_UI_POLICY>() {
            return Err(CngError::InvalidKeyHandle(None));
        }
        let policy = unsafe { std::ptr::read_unaligned(data.as_ptr().cast::<NCRYPT_UI_POLICY>()) };
        if policy.dwVersion != 1 {
            return Err(CngError::InvalidKeyHandle(None));
        }
        // A fresh key handle with forced high protection requires authentication for use.
        Ok(
            if policy.dwFlags & NCRYPT_UI_FORCE_HIGH_PROTECTION_FLAG != 0 {
                keyringd_core::model::device_key::LocalAuthentication::Required
            } else {
                keyringd_core::model::device_key::LocalAuthentication::NotRequired
            },
        )
    }

    pub(super) fn delete(self) -> Result<(), CngError> {
        let handle = self.0;
        std::mem::forget(self);
        unsafe { NCryptDeleteKey(handle, 0)? };
        Ok(())
    }
}

unsafe fn set_ui_parent(handle: NCRYPT_KEY_HANDLE, ui_parent: isize) -> Result<(), CngError> {
    use windows::Win32::UI::WindowsAndMessaging::{GetWindowThreadProcessId, IsWindow};
    let window = windows::Win32::Foundation::HWND(ui_parent as *mut core::ffi::c_void);
    let mut owner_pid = 0;
    let thread_id = unsafe { GetWindowThreadProcessId(window, Some(&mut owner_pid)) };
    tracing::debug!(
        process_pid = unsafe { windows::Win32::System::Threading::GetCurrentProcessId() },
        key_handle = handle.0,
        parent_hwnd = ui_parent,
        owner_pid,
        thread_id,
        parent_valid = unsafe { IsWindow(Some(window)) }.as_bool(),
        "CNG UI parent requested"
    );
    if ui_parent == 0 {
        return Ok(());
    }
    unsafe {
        NCryptSetProperty(
            NCRYPT_HANDLE(handle.0),
            NCRYPT_WINDOW_HANDLE_PROPERTY,
            std::slice::from_raw_parts(
                (&ui_parent as *const isize).cast::<u8>(),
                std::mem::size_of::<isize>(),
            ),
            NCRYPT_FLAGS(0),
        )?;
    }
    let mut actual = [0u8; std::mem::size_of::<isize>()];
    let mut length = 0;
    let readback = unsafe {
        NCryptGetProperty(
            NCRYPT_HANDLE(handle.0),
            NCRYPT_WINDOW_HANDLE_PROPERTY,
            Some(&mut actual),
            &mut length,
            Default::default(),
        )
    };
    tracing::debug!(key_handle = handle.0, parent_hwnd = ui_parent,
        actual_parent_hwnd = isize::from_ne_bytes(actual), returned_bytes = length,
        result = ?readback, "CNG UI parent readback");
    // Readback is diagnostic only; it must not change the operation's outcome.
    Ok(())
}

impl Drop for KeyHandle {
    fn drop(&mut self) {
        let _ = unsafe { NCryptFreeObject(NCRYPT_HANDLE(self.0.0)) };
    }
}

pub(super) fn export_ec_point(key: &KeyHandle) -> Result<Sec1Point, CngError> {
    let mut size = 0;
    unsafe {
        NCryptExportKey(
            key.0,
            None,
            BCRYPT_ECCPUBLIC_BLOB,
            None,
            None,
            &mut size,
            NCRYPT_FLAGS(0),
        )?;
    }
    let mut blob = vec![0u8; size as usize];
    unsafe {
        NCryptExportKey(
            key.0,
            None,
            BCRYPT_ECCPUBLIC_BLOB,
            None,
            Some(&mut blob),
            &mut size,
            NCRYPT_FLAGS(0),
        )?;
    }
    blob.truncate(size as usize);
    parse_ec_public_blob(&blob)
}

pub(super) fn sign_with_key(key: &KeyHandle, data: &[u8]) -> Result<Vec<u8>, CngError> {
    let mut size = 0;
    unsafe {
        NCryptSignHash(key.0, None, data, None, &mut size, NCRYPT_FLAGS(0))?;
    }
    let mut signature = vec![0u8; size as usize];
    unsafe {
        NCryptSignHash(
            key.0,
            None,
            data,
            Some(&mut signature),
            &mut size,
            NCRYPT_FLAGS(0),
        )?;
    }
    signature.truncate(size as usize);

    if signature.len() != ECC_P256_COORDINATE_SIZE * 2 {
        return Err(CngError::InvalidSignature);
    }

    Ok(signature)
}

pub(super) fn parse_ec_public_blob(blob: &[u8]) -> Result<Sec1Point, CngError> {
    if blob.len() < ECC_KEY_BLOB_HEADER_SIZE {
        return Err(CngError::InvalidDevicePublicKey(None));
    }
    let magic = u32::from_le_bytes(blob[0..4].try_into().unwrap());
    let coordinate_size = u32::from_le_bytes(blob[4..8].try_into().unwrap()) as usize;
    if magic != BCRYPT_ECDSA_PUBLIC_P256_MAGIC
        || coordinate_size != ECC_P256_COORDINATE_SIZE
        || blob.len() != ECC_KEY_BLOB_HEADER_SIZE + coordinate_size * 2
    {
        return Err(CngError::InvalidDevicePublicKey(None));
    }

    const EC_UNCOMPRESSED_PREFIX: u8 = 0x04;
    const P256_UNCOMPRESSED_POINT_LEN: usize = 65;
    let mut point = [0u8; P256_UNCOMPRESSED_POINT_LEN];
    point[0] = EC_UNCOMPRESSED_PREFIX;
    point[1..].copy_from_slice(&blob[ECC_KEY_BLOB_HEADER_SIZE..]);
    Sec1Point::from_bytes(point)
        .map_err(|error| CngError::InvalidDevicePublicKey(Some(error.into())))
}
