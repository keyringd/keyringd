use super::device_info_set::DeviceInfoSet;
use std::io;
use windows::Win32::Devices::DeviceAndDriverInstallation::*;
use windows::Win32::Devices::Properties::*;
use windows::Win32::Foundation::{DEVPROPKEY, ERROR_GEN_FAILURE};
use windows::core::PCWSTR;

fn check(status: CONFIGRET) -> io::Result<()> {
    if status == CR_SUCCESS {
        return Ok(());
    }
    Err(io::Error::from_raw_os_error(
        unsafe { CM_MapCrToWin32Err(status, ERROR_GEN_FAILURE.0) } as i32,
    ))
}

fn property(node: u32, key: &DEVPROPKEY) -> io::Result<Option<(DEVPROPTYPE, Vec<u8>)>> {
    let mut kind = DEVPROPTYPE::default();
    let mut size = 0;
    let status = unsafe { CM_Get_DevNode_PropertyW(node, key, &mut kind, None, &mut size, 0) };
    if status == CR_NO_SUCH_VALUE || status == CR_NO_SUCH_DEVNODE {
        return Ok(None);
    }
    if status != CR_BUFFER_SMALL {
        check(status)?;
    }
    let mut bytes = vec![0; size as usize];
    let status = unsafe {
        CM_Get_DevNode_PropertyW(node, key, &mut kind, Some(bytes.as_mut_ptr()), &mut size, 0)
    };
    if status == CR_NO_SUCH_VALUE || status == CR_NO_SUCH_DEVNODE {
        return Ok(None);
    }
    check(status)?;
    bytes.truncate(size as usize);
    Ok(Some((kind, bytes)))
}

fn service(node: u32) -> io::Result<Option<String>> {
    let Some((kind, bytes)) = property(node, &DEVPKEY_Device_Service)? else {
        return Ok(None);
    };
    if kind != DEVPROP_TYPE_STRING || !bytes.len().is_multiple_of(2) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid device service property",
        ));
    }
    let text = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|word| u16::from_le_bytes([word[0], word[1]]))
        .take_while(|word| *word != 0)
        .collect::<Vec<_>>();
    String::from_utf16(&text)
        .map(Some)
        .map_err(io::Error::other)
}

fn address(node: u32) -> io::Result<Option<u32>> {
    let Some((kind, bytes)) = property(node, &DEVPKEY_Device_Address)? else {
        return Ok(None);
    };
    if kind != DEVPROP_TYPE_UINT32 || bytes.len() != 4 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid device address property",
        ));
    }
    Ok(Some(u32::from_le_bytes(bytes.try_into().unwrap())))
}

fn instance_id(node: u32) -> io::Result<Option<String>> {
    let mut size = 0;
    let status = unsafe { CM_Get_Device_ID_Size(&mut size, node, 0) };
    if status == CR_NO_SUCH_DEVNODE {
        return Ok(None);
    }
    check(status)?;
    let mut text = vec![0; size as usize + 1];
    let status = unsafe { CM_Get_Device_IDW(node, &mut text, 0) };
    if status == CR_NO_SUCH_DEVNODE {
        return Ok(None);
    }
    check(status)?;
    String::from_utf16(&text[..size as usize])
        .map(Some)
        .map_err(io::Error::other)
}

fn matches_port(mut node: u32, port: u16) -> io::Result<bool> {
    let mut port_matches = false;
    for _ in 0..5 {
        match service(node)?.as_deref() {
            Some(name) if name.eq_ignore_ascii_case("HidUsb") => {
                port_matches = address(node)? == Some(port as u32)
            }
            Some(name) if name.eq_ignore_ascii_case("usbip2_ude") => return Ok(port_matches),
            _ => {}
        }
        let mut parent = 0;
        let status = unsafe { CM_Get_Parent(&mut parent, node, 0) };
        if status == CR_NO_SUCH_DEVNODE {
            break;
        }
        check(status)?;
        node = parent;
    }
    Ok(false)
}

pub(super) fn ready_hid(port: u16) -> io::Result<Option<String>> {
    let devices = DeviceInfoSet::present_hid()?;
    let mut found = Vec::new();
    for node in devices.nodes()? {
        let Some(id) = instance_id(node)? else {
            continue;
        };
        if !id
            .to_ascii_uppercase()
            .starts_with(r"HID\VID_1234&PID_F1D0")
        {
            continue;
        }
        let mut status = CM_DEVNODE_STATUS_FLAGS::default();
        let mut problem = CM_PROB::default();
        let result = unsafe { CM_Get_DevNode_Status(&mut status, &mut problem, node, 0) };
        if result == CR_NO_SUCH_DEVNODE {
            continue;
        }
        check(result)?;
        if problem.0 == 0 && (status & DN_HAS_PROBLEM).0 == 0 && matches_port(node, port)? {
            found.push(id);
        }
    }
    if found.len() > 1 {
        return Err(io::Error::other(format!(
            "multiple test HID devices on USB/IP port {port}: {found:?}"
        )));
    }
    Ok(found.pop())
}

pub(super) fn present(instance: &str) -> io::Result<bool> {
    let instance = instance.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let mut node = 0;
    let status = unsafe {
        CM_Locate_DevNodeW(
            &mut node,
            PCWSTR(instance.as_ptr()),
            CM_LOCATE_DEVNODE_NORMAL,
        )
    };
    if status == CR_NO_SUCH_DEVNODE {
        return Ok(false);
    }
    check(status)?;
    Ok(true)
}
