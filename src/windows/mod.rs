use std::collections::HashMap;
use std::process::Command;

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use windows::{
    self,
    Win32::{Foundation::GetLastError, Storage::FileSystem},
};
use winreg::{enums::*, RegKey, RegValue};

use crate::collector;
use crate::component::{ComponentTrait, Kind};
use crate::Error;

const HKLM: RegKey = RegKey::predef(HKEY_LOCAL_MACHINE);
const UNINSTALL_LOCATIONS: &'static [&'static str] = &[
    "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
    "SOFTWARE\\Wow6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
];

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
struct Application {
    pub key: String,
    pub modified: NaiveDateTime,
    pub properties: HashMap<String, String>,

    name: String,
    version: String,
    path: String,
    publishers: Vec<String>,
}

impl Application {
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

    pub fn new(
        key: String,
        modified: NaiveDateTime,
        properties: HashMap<String, RegValue>,
    ) -> Self {
        let mut zelf = Self {
            key,
            modified,
            properties: HashMap::new(),
            name: "".to_owned(),
            version: "".to_owned(),
            path: "".to_owned(),
            publishers: vec![],
        };

        for (name, prop) in properties {
            zelf.properties
                .insert(name, Self::regvalue_to_string(&prop));
        }

        if let Some(prop) = zelf.properties.get("DisplayName") {
            zelf.name = prop.to_string();
        }

        if let Some(prop) = zelf.properties.get("DisplayVersion") {
            zelf.version = prop.to_string();
        } else if let Some(prop) = zelf.properties.get("Version") {
            zelf.version = prop.to_string();
        }

        for path in ["InstallLocation", "InstallSource", "BundleCachePath"] {
            let location = zelf.properties.get(path);
            if location.is_some() && !location.unwrap().is_empty() {
                zelf.path = location.unwrap().to_string();
                break;
            }
        }

        if let Some(prop) = zelf.properties.get("Publisher") {
            zelf.publishers.push(prop.to_string());
        }

        zelf
    }
}

impl ComponentTrait for Application {
    fn kind(&self) -> Kind {
        Kind::Application
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn id(&self) -> &str {
        &self.key
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn path(&self) -> &str {
        &self.path
    }

    fn modified(&self) -> DateTime<Utc> {
        TimeZone::from_utc_datetime(&Utc, &self.modified)
    }

    fn publishers(&self) -> &Vec<String> {
        &self.publishers
    }
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
struct Driver {
    #[serde(rename = "Module Name")]
    pub module_name: String,
    #[serde(rename = "Display Name")]
    pub display_name: String,
    #[serde(rename = "Description")]
    pub description: String,
    #[serde(rename = "Driver Type")]
    pub driver_type: String,
    #[serde(rename = "Start Mode")]
    pub start_mode: String,
    #[serde(rename = "State")]
    pub state: String,
    #[serde(rename = "Status")]
    pub status: String,
    #[serde(rename = "Accept Stop")]
    pub accept_stop: String,
    #[serde(rename = "Accept Pause")]
    pub accept_pause: String,
    #[serde(rename = "Paged Pool(bytes)")]
    pub paged_pool_size: String,
    #[serde(rename = "Code(bytes)")]
    pub code_size: String,
    #[serde(rename = "BSS(bytes)")]
    pub bss_size: String,
    #[serde(rename = "Link Date")]
    pub link_date_string: String,
    #[serde(rename = "Path")]
    pub path: String,
    #[serde(rename = "Init(bytes)")]
    pub init_size: String,
    #[serde(skip_deserializing)]
    pub link_date: DateTime<Utc>,
    #[serde(skip_deserializing)]
    pub publishers: Vec<String>,
    #[serde(skip_deserializing)]
    pub version: String,
}

impl Driver {
    pub fn parse(&mut self) -> Result<(), Error> {
        if !self.link_date_string.is_empty() {
            self.link_date = Utc
                .datetime_from_str(&self.link_date_string, "%m/%e/%Y %l:%M:%S %p")
                .map_err(|e| {
                    format!(
                        "could not parse driver datetime '{}': {:?}",
                        &self.link_date_string, e
                    )
                })?;
        }
        Ok(())
    }
}

impl ComponentTrait for Driver {
    fn kind(&self) -> Kind {
        Kind::Driver
    }

    fn name(&self) -> &str {
        &self.display_name
    }

    fn id(&self) -> &str {
        &self.module_name
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn path(&self) -> &str {
        &self.path
    }

    fn modified(&self) -> DateTime<Utc> {
        self.link_date
    }

    fn publishers(&self) -> &Vec<String> {
        &self.publishers
    }
}

#[derive(Default)]
pub(crate) struct Collector {}

impl Collector {
    // modified from windows::core::w! to process non literals
    fn string_to_u16(s: &str) -> Vec<u16> {
        let input: &[u8] = s.as_bytes();
        let output_len: usize = ::windows::core::utf16_len(input) + 1;
        let result: Vec<u16> = {
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
        result
    }
    fn get_file_version(path: &str) -> Result<String, Error> {
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
        let filename = windows::core::PCWSTR::from_raw(Self::string_to_u16(path).as_ptr());
        let mut handle: u32 = 0;
        let size = unsafe { FileSystem::GetFileVersionInfoSizeW(filename, &mut handle) };
        if size == 0 {
            return Err(format!(
                "GetFileVersionInfoSizeW failed for {} with {}",
                path,
                unsafe { GetLastError() }.0
            ));
        }

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

        Ok(format!(
            "{}.{}.{}.{}",
            pinfo.dwProductVersionMS >> 16,
            pinfo.dwProductVersionMS & 0xFFFF,
            pinfo.dwProductVersionLS >> 16,
            pinfo.dwProductVersionLS & 0xFFFF,
        ))
    }

    fn collect_drivers(&self) -> Result<Vec<Box<dyn ComponentTrait>>, Error> {
        let mut comps: Vec<Box<dyn ComponentTrait>> = vec![];

        let driverquery = Command::new("driverquery.exe")
            .arg("/v")
            .args(&["/FO", "CSV"])
            .output()
            .map_err(|e| format!("could not execute driverquery.exe: {:?}", e))?;

        if !driverquery.status.success() {
            return Err(format!(
                "driverquery exit status {:?}: {:?}",
                driverquery.status,
                String::from_utf8_lossy(&driverquery.stderr)
            ));
        }

        let raw_csv = String::from_utf8_lossy(&driverquery.stdout);
        let mut rdr = csv::Reader::from_reader(raw_csv.as_bytes());
        for result in rdr.deserialize() {
            let mut driver: Driver =
                result.map_err(|e| format!("could not deserialize driver record: {:?}", e))?;

            driver.parse()?;

            let version = Self::get_file_version(driver.path());
            if let Ok(v) = version {
                driver.version = v;
            } else {
                log::warn!("{:?}", version.err().unwrap());
            }

            comps.push(Box::new(driver));
        }

        Ok(comps)
    }

    fn collect_apps(&self) -> Result<Vec<Box<dyn ComponentTrait>>, Error> {
        let mut comps: Vec<Box<dyn ComponentTrait>> = vec![];

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

                let mut props = HashMap::new();

                for (name, value) in sub_key.enum_values().map(|x| x.unwrap()) {
                    props.insert(name, value);
                }

                if props.contains_key("DisplayName") {
                    comps.push(Box::new(Application::new(
                        sub_key_name,
                        sub_key_info.get_last_write_time_chrono(),
                        props,
                    )));
                }
            }
        }

        Ok(comps)
    }
}

impl collector::Collector for Collector {
    fn setup(&mut self) -> Result<(), Error> {
        Ok(())
    }

    fn collect_from_json(&self, _: &str) -> Result<Vec<Box<dyn ComponentTrait>>, Error> {
        Err("not implemented".to_owned())
    }

    fn collect(&self) -> Result<Vec<Box<dyn ComponentTrait>>, Error> {
        log::info!("collecting applications and drivers, please wait ...");

        let mut drivers = self.collect_drivers()?;
        let mut apps = self.collect_apps()?;

        drivers.append(&mut apps);

        Ok(drivers)
    }
}
