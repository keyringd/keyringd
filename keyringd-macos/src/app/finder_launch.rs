use objc2_foundation::{NSDistributedNotificationCenter, NSString};
use std::path::{Path, PathBuf};

pub(super) const OPEN_PROFILES: &str = "com.keyringd.open-profiles";

pub(super) fn request_file(home: &Path) -> PathBuf {
    home.join("Library/Caches/com.keyringd.Keyringd/open-profiles")
}

pub(super) fn open(home: &Path) -> anyhow::Result<()> {
    let request = request_file(home);
    std::fs::create_dir_all(request.parent().unwrap())?;
    match std::fs::File::create_new(&request) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error.into()),
    }
    unsafe {
        NSDistributedNotificationCenter::defaultCenter()
            .postNotificationName_object(&NSString::from_str(OPEN_PROFILES), None);
    }
    Ok(())
}
