use std::collections::HashMap;

use crate::Error;

use chrono::NaiveDateTime;
use windows::{
    self,
    Win32::{Foundation::GetLastError, Storage::FileSystem},
};
use winreg::{enums::*, RegKey, RegValue};

const HKLM: RegKey = RegKey::predef(HKEY_LOCAL_MACHINE);
const UNINSTALL_LOCATIONS: &'static [&'static str] = &[
    "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
    "SOFTWARE\\Wow6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
];

// modified from windows::core::w! to process non literals
fn str_to_pcwstr(s: &str) -> windows::core::PCWSTR {
    let input: &[u8] = s.as_bytes();
    let output_len: usize = ::windows::core::utf16_len(input) + 1;
    let as_utf16: Vec<u16> = {
        if output_len == 1 {
            vec![0]
        } else {
            let output: Vec<u16> = {
                let mut buffer = vec![0; output_len];
                let mut input_pos = 0;
                let mut output_pos = 0;
                while let Some((mut code_point, new_pos)) =
                    ::windows::core::decode_utf8_char(input, input_pos)
                {
                    input_pos = new_pos;
                    if code_point <= 0xffff {
                        buffer[output_pos] = code_point as u16;
                        output_pos += 1;
                    } else {
                        code_point -= 0x10000;
                        buffer[output_pos] = 0xd800 + (code_point >> 10) as u16;
                        output_pos += 1;
                        buffer[output_pos] = 0xdc00 + (code_point & 0x3ff) as u16;
                        output_pos += 1;
                    }
                }
                buffer
            };
            output
        }
    };
    windows::core::PCWSTR::from_raw(as_utf16.as_ptr())
}

/*
    FIXME: for reasons i failed to understand so far, this is not always working.
    Sometimes one of these API fail with last_error=2 (fail not found) even when
    the file does indeed exist.

    WARN  unisbom::windows > Some("GetFileVersionInfoW failed for C:\\Windows\\system32\\drivers\\amdi2c.sys with 1812")
    WARN  unisbom::windows > Some("GetFileVersionInfoSizeW failed for C:\\Windows\\system32\\drivers\\applockerfltr.sys with 2")
    WARN  unisbom::windows > Some("GetFileVersionInfoSizeW failed for C:\\Windows\\system32\\drivers\\atapi.sys with 2")
    WARN  unisbom::windows > Some("GetFileVersionInfoW failed for C:\\Windows\\system32\\drivers\\BthA2dp.sys with 2")
    WARN  unisbom::windows > Some("GetFileVersionInfoSizeW failed for C:\\Windows\\system32\\drivers\\Microsoft.Bluetooth.Legacy.LEEnumerator.sys with 2")
    WARN  unisbom::windows > Some("GetFileVersionInfoW failed for C:\\Windows\\system32\\DRIVERS\\cdfs.sys with 2")
    WARN  unisbom::windows > Some("GetFileVersionInfoSizeW failed for C:\\Windows\\system32\\drivers\\cdrom.sys with 2")
    WARN  unisbom::windows > Some("GetFileVersionInfoSizeW failed for C:\\Windows\\system32\\drivers\\CimFS.sys with 1813")
    WARN  unisbom::windows > Some("GetFileVersionInfoW failed for C:\\Windows\\system32\\DriverStore\\FileRepository\\compositebus.inf_amd64_7500cffa210c6946\\CompositeBus.sys with 2")
    WARN  unisbom::windows > Some("GetFileVersionInfoW failed for C:\\Windows\\system32\\drivers\\EhStorTcgDrv.sys with 1812")
    WARN  unisbom::windows > Some("GetFileVersionInfoW failed for C:\\Windows\\system32\\drivers\\fltmgr.sys with 2")
    WARN  unisbom::windows > Some("GetFileVersionInfoW failed for C:\\Windows\\system32\\drivers\\vmgencounter.sys with 1812")
    WARN  unisbom::windows > Some("GetFileVersionInfoSizeW failed for C:\\Windows\\system32\\DriverStore\\FileRepository\\genericusbfn.inf_amd64_53931f0ae21d6d2c\\genericusbfn.sys with 2")
    WARN  unisbom::windows > Some("GetFileVersionInfoW failed for C:\\Windows\\system32\\drivers\\gpuenergydrv.sys with 1812")
    WARN  unisbom::windows > Some("GetFileVersionInfoW failed for C:\\Windows\\system32\\drivers\\hidinterrupt.sys with 2")
    WARN  unisbom::windows > Some("GetFileVersionInfoW failed for C:\\Windows\\system32\\drivers\\hvcrash.sys with 2")
    WARN  unisbom::windows > Some("GetFileVersionInfoW failed for C:\\Windows\\system32\\drivers\\iaLPSS2i_GPIO2.sys with 2")
    WARN  unisbom::windows > Some("GetFileVersionInfoSizeW failed for C:\\Windows\\system32\\drivers\\iaLPSS2i_GPIO2_BXT_P.sys with 2")
    WARN  unisbom::windows > Some("GetFileVersionInfoSizeW failed for C:\\Windows\\system32\\drivers\\iaLPSS2i_I2C.sys with 2")
    WARN  unisbom::windows > Some("GetFileVersionInfoW failed for C:\\Windows\\system32\\drivers\\IPMIDrv.sys with 2")
    WARN  unisbom::windows > Some("GetFileVersionInfoW failed for C:\\Windows\\system32\\drivers\\luafv.sys with 2")
    WARN  unisbom::windows > Some("GetFileVersionInfoW failed for C:\\Windows\\system32\\DRIVERS\\NDProxy.sys with 2")
    WARN  unisbom::windows > Some("GetFileVersionInfoW failed for C:\\Windows\\system32\\drivers\\NetAdapterCx.sys with 1812")
    WARN  unisbom::windows > Some("GetFileVersionInfoW failed for C:\\Windows\\system32\\drivers\\nvraid.sys with 1812")
    WARN  unisbom::windows > Some("GetFileVersionInfoW failed for C:\\Windows\\system32\\drivers\\nvstor.sys with 2")
    WARN  unisbom::windows > Some("GetFileVersionInfoSizeW failed for C:\\Windows\\system32\\drivers\\parport.sys with 2")
    WARN  unisbom::windows > Some("GetFileVersionInfoSizeW failed for C:\\Windows\\system32\\drivers\\passthruparser.sys with 2")
    WARN  unisbom::windows > Some("GetFileVersionInfoSizeW failed for C:\\Windows\\system32\\drivers\\pciide.sys with 2")
    WARN  unisbom::windows > Some("GetFileVersionInfoW failed for C:\\Windows\\system32\\drivers\\rassstp.sys with 2")
    WARN  unisbom::windows > Some("GetFileVersionInfoSizeW failed for C:\\Windows\\system32\\drivers\\scmbus.sys with 2")
    WARN  unisbom::windows > Some("GetFileVersionInfoSizeW failed for C:\\Windows\\system32\\DriverStore\\FileRepository\\swenum.inf_amd64_16a14542b63c02af\\swenum.sys with 2")
    WARN  unisbom::windows > Some("GetFileVersionInfoSizeW failed for C:\\Windows\\system32\\drivers\\tcpip.sys with 2")
    WARN  unisbom::windows > Some("GetFileVersionInfoW failed for C:\\Windows\\system32\\drivers\\tunnel.sys with 2")
    WARN  unisbom::windows > Some("GetFileVersionInfoSizeW failed for C:\\Windows\\system32\\DriverStore\\FileRepository\\uefi.inf_amd64_c1628ffa62c8e54c\\UEFI.sys with 2")
    WARN  unisbom::windows > Some("GetFileVersionInfoSizeW failed for C:\\Windows\\system32\\DriverStore\\FileRepository\\umbus.inf_amd64_b78a9c5b6fd62c27\\umbus.sys with 2")
    WARN  unisbom::windows > Some("GetFileVersionInfoW failed for C:\\Windows\\system32\\drivers\\umpass.sys with 1812")
    WARN  unisbom::windows > Some("GetFileVersionInfoSizeW failed for C:\\Windows\\system32\\DriverStore\\FileRepository\\urschipidea.inf_amd64_78ad1c14e33df968\\urschipidea.sys with 2")
    WARN  unisbom::windows > Some("GetFileVersionInfoW failed for C:\\Windows\\system32\\DriverStore\\FileRepository\\urssynopsys.inf_amd64_057fa37902020500\\urssynopsys.sys with 1812")
    WARN  unisbom::windows > Some("GetFileVersionInfoW failed for C:\\Windows\\system32\\drivers\\usbuhci.sys with 1812")
    WARN  unisbom::windows > Some("GetFileVersionInfoSizeW failed for C:\\Windows\\system32\\DriverStore\\FileRepository\\vrd.inf_amd64_81fbd405ff2470fc\\vrd.sys with 2")
    WARN  unisbom::windows > Some("GetFileVersionInfoW failed for C:\\Windows\\system32\\drivers\\wcifs.sys with 1812")
    WARN  unisbom::windows > Some("GetFileVersionInfoW failed for C:\\Windows\\system32\\DRIVERS\\wdiwifi.sys with 2")
    WARN  unisbom::windows > Some("GetFileVersionInfoSizeW failed for C:\\Windows\\system32\\drivers\\WindowsTrustedRTProxy.sys with 2")
    WARN  unisbom::windows > Some("GetFileVersionInfoW failed for C:\\Windows\\system32\\drivers\\ws2ifsl.sys with 1812")
*/
pub(crate) fn parse_file_version(path: &str) -> Result<String, Error> {
    let filename = str_to_pcwstr(path);
    let mut handle: u32 = 0;
    let size = unsafe { FileSystem::GetFileVersionInfoSizeW(filename, &mut handle) };
    if size == 0 {
        return Err(format!(
            "GetFileVersionInfoSizeW failed for {} with {}",
            path,
            unsafe { GetLastError() }.0
        ));
    }

    log::debug!("GetFileVersionInfoSizeW({}) -> {}", path, size);

    let mut buffer: Vec<u16> = vec![0; size as usize];
    let mut ret = unsafe {
        FileSystem::GetFileVersionInfoW(
            filename,
            handle,
            size as u32,
            buffer.as_mut_ptr() as *mut core::ffi::c_void,
        )
    };
    if !ret.as_bool() {
        return Err(format!(
            "GetFileVersionInfoW failed for {} with {}",
            path,
            unsafe { GetLastError() }.0
        ));
    }

    let mut info: *mut core::ffi::c_void = std::ptr::null_mut();
    let info_ptr: *mut *mut core::ffi::c_void = &mut info;
    let mut size: u32 = std::mem::size_of::<FileSystem::VS_FIXEDFILEINFO>() as u32;
    let filter = windows::core::w!("\\\\");
    ret = unsafe {
        FileSystem::VerQueryValueW(
            buffer.as_ptr() as *mut core::ffi::c_void,
            filter,
            info_ptr,
            &mut size,
        )
    };
    if !ret.as_bool() {
        return Err(format!(
            "VerQueryValueW failed for {} with {}",
            path,
            unsafe { GetLastError() }.0
        ));
    }

    let pinfo = unsafe { &mut *(info as *mut FileSystem::VS_FIXEDFILEINFO) };

    log::debug!("VerQueryValueW({}) -> {{", path);
    log::debug!("  .dwSignature = {}", pinfo.dwSignature);
    log::debug!("  .dwStrucVersion = {}", pinfo.dwStrucVersion);
    log::debug!("  .dwFileVersionMS = {}", pinfo.dwFileVersionMS);
    log::debug!("  .dwFileVersionLS = {}", pinfo.dwFileVersionLS);
    log::debug!("  .dwProductVersionMS = {}", pinfo.dwProductVersionMS);
    log::debug!("  .dwProductVersionLS = {}", pinfo.dwProductVersionLS);
    log::debug!("  .dwFileFlagsMask = {}", pinfo.dwFileFlagsMask);
    log::debug!("  .dwFileFlags = {}", pinfo.dwFileFlags.0);
    log::debug!("  .dwFileOS = {}", pinfo.dwFileOS.0);
    log::debug!("  .dwFileType = {}", pinfo.dwFileType.0);
    log::debug!("  .dwFileSubtype = {}", pinfo.dwFileSubtype.0);
    log::debug!("  .dwFileDateMS = {}", pinfo.dwFileDateMS);
    log::debug!("  .dwFileDateLS = {}", pinfo.dwFileDateLS);
    log::debug!("}}");

    Ok(format!(
        "{}.{}.{}.{}",
        pinfo.dwProductVersionMS >> 16,
        pinfo.dwProductVersionMS & 0xFFFF,
        pinfo.dwProductVersionLS >> 16,
        pinfo.dwProductVersionLS & 0xFFFF,
    ))
}

#[derive(Debug)]
pub(crate) struct UninstallEntry {
    pub key_name: String,
    pub modified: NaiveDateTime,
    pub properties: HashMap<String, String>,
}

fn regvalue_to_string(v: &RegValue) -> String {
    match v.vtype {
        REG_SZ | REG_EXPAND_SZ | REG_MULTI_SZ => {
            let words = unsafe {
                #[allow(clippy::cast_ptr_alignment)]
                std::slice::from_raw_parts(v.bytes.as_ptr() as *const u16, v.bytes.len() / 2)
            };
            let mut s = String::from_utf16_lossy(words);
            while s.ends_with('\u{0}') {
                s.pop();
            }
            if v.vtype == REG_MULTI_SZ {
                s.replace("\u{0}", "\n")
            } else {
                s
            }
        }
        _ => format!("{:?}", v.bytes),
    }
}

pub(crate) fn enum_registry_uninstall_locations() -> Result<Vec<UninstallEntry>, Error> {
    let mut found = vec![];

    for location in UNINSTALL_LOCATIONS {
        let uninstall = HKLM
            .open_subkey(location)
            .map_err(|e| format!("can't open {}: {:?}", location, e))?;

        for sub_key_name in uninstall.enum_keys().map(|x| x.unwrap()) {
            let sub_key = uninstall
                .open_subkey(&sub_key_name)
                .map_err(|e| format!("can't open {}/{}: {:?}", location, &sub_key_name, e))?;

            let sub_key_info = sub_key.query_info().map_err(|e| {
                format!(
                    "can't query info for {}/{}: {:?}",
                    location, &sub_key_name, e
                )
            })?;

            let mut properties = HashMap::new();
            for (name, value) in sub_key.enum_values().map(|x| x.unwrap()) {
                properties.insert(name, regvalue_to_string(&value));
            }

            found.push(UninstallEntry {
                key_name: sub_key_name,
                modified: sub_key_info.get_last_write_time_chrono(),
                properties,
            })
        }
    }

    Ok(found)
}
