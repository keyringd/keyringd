use std::io;
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    DIGCF_PRESENT, GUID_DEVCLASS_HIDCLASS, HDEVINFO, SP_DEVINFO_DATA, SetupDiDestroyDeviceInfoList,
    SetupDiEnumDeviceInfo, SetupDiGetClassDevsW,
};
use windows::Win32::Foundation::ERROR_NO_MORE_ITEMS;
use windows::core::{HRESULT, PCWSTR};

pub(super) struct DeviceInfoSet(HDEVINFO);

impl DeviceInfoSet {
    pub(super) fn present_hid() -> io::Result<Self> {
        unsafe {
            SetupDiGetClassDevsW(
                Some(&GUID_DEVCLASS_HIDCLASS),
                PCWSTR::null(),
                None,
                DIGCF_PRESENT,
            )
        }
        .map(Self)
        .map_err(io::Error::other)
    }

    pub(super) fn nodes(&self) -> io::Result<Vec<u32>> {
        let mut nodes = Vec::new();
        for index in 0.. {
            let mut data = SP_DEVINFO_DATA {
                cbSize: size_of::<SP_DEVINFO_DATA>() as u32,
                ..Default::default()
            };
            match unsafe { SetupDiEnumDeviceInfo(self.0, index, &mut data) } {
                Ok(()) => nodes.push(data.DevInst),
                Err(error) if error.code() == HRESULT::from_win32(ERROR_NO_MORE_ITEMS.0) => break,
                Err(error) => return Err(io::Error::other(error)),
            }
        }
        Ok(nodes)
    }
}

impl Drop for DeviceInfoSet {
    fn drop(&mut self) {
        unsafe {
            let _ = SetupDiDestroyDeviceInfoList(self.0);
        }
    }
}
