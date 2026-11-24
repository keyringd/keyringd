use std::fs::File;
use std::io::{self, Write};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::FromRawHandle;
use std::path::Path;

use windows::Win32::Foundation::GENERIC_WRITE;
use windows::Win32::Storage::FileSystem::{
    CREATE_NEW, CreateFileW, DELETE, FILE_ATTRIBUTE_SYSTEM, FILE_FLAG_DELETE_ON_CLOSE,
    FILE_SHARE_DELETE, FILE_SHARE_READ, MOVEFILE_WRITE_THROUGH, MoveFileExW,
};
use windows::core::PCWSTR;

use super::user_only_security_descriptor::UserOnlySecurityDescriptor;

/// MSYS AF_LOCAL rendezvous file, held open until the listener stops.
pub(super) struct GitBashSocketFile {
    _file: File,
}

impl GitBashSocketFile {
    pub(super) fn create(path: &Path, port: u16, secret: &[u8; 16]) -> io::Result<Self> {
        // Never overwrite an existing endpoint, including a symlink. Windows
        // deletes our file when the last handle closes, even after a crash.
        let descriptor = UserOnlySecurityDescriptor::new()?;
        let attributes = descriptor.attributes();
        let mut nonce = [0; 8];
        getrandom::fill(&mut nonce).map_err(io::Error::other)?;
        let temporary = path.with_extension(format!("sock.{:016x}.tmp", u64::from_le_bytes(nonce)));
        let wide_path = path
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>();
        let temporary = temporary
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>();
        let handle = unsafe {
            CreateFileW(
                PCWSTR(temporary.as_ptr()),
                GENERIC_WRITE.0 | DELETE.0,
                FILE_SHARE_READ | FILE_SHARE_DELETE,
                Some(&attributes),
                CREATE_NEW,
                FILE_ATTRIBUTE_SYSTEM | FILE_FLAG_DELETE_ON_CLOSE,
                None,
            )
        }
        .map_err(io::Error::other)?;
        let mut file = unsafe { File::from_raw_handle(handle.0) };
        // MSYS reads four native-endian 32-bit words from this text, then
        // exchanges their raw 16 bytes over a loopback TCP connection.
        // https://github.com/msys2/msys2-runtime/blob/msys2-3.6.11/winsup/cygwin/fhandler/socket_local.cc
        let words = secret
            .as_chunks::<4>()
            .0
            .iter()
            .map(|word| u32::from_le_bytes(*word))
            .collect::<Vec<_>>();
        write!(
            file,
            "!<socket >{port} s {:08x}-{:08x}-{:08x}-{:08x}\0",
            words[0], words[1], words[2], words[3]
        )?;
        file.flush()?;
        // Publish a complete file atomically, without replacing an endpoint
        // belonging to another listener. Its ACL was set at creation time.
        unsafe {
            MoveFileExW(
                PCWSTR(temporary.as_ptr()),
                PCWSTR(wide_path.as_ptr()),
                MOVEFILE_WRITE_THROUGH,
            )
        }
        .map_err(io::Error::other)?;
        Ok(Self { _file: file })
    }
}
