use std::collections::HashMap;
use std::process::Command;

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use serde::Deserialize;
use winreg::{enums::*, RegKey, RegValue};

use crate::collector;
use crate::component::*;
use crate::Error;

const HKLM: RegKey = RegKey::predef(HKEY_LOCAL_MACHINE);
const UNINSTALL_LOCATIONS: &'static [&'static str] = &[
    "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
    "SOFTWARE\\Wow6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
];

#[allow(dead_code)]
struct Application {
    pub key: String,
    pub modified: NaiveDateTime,
    pub properties: HashMap<String, RegValue>,

    name: String,
    version: String,
    path: String,
    publishers: Vec<String>,
}

impl Application {
    pub fn new(
        key: String,
        modified: NaiveDateTime,
        properties: HashMap<String, RegValue>,
    ) -> Self {
        let mut zelf = Self {
            key,
            modified,
            properties,
            name: "".to_owned(),
            version: "".to_owned(),
            path: "".to_owned(),
            publishers: vec![],
        };

        if let Some(prop) = zelf.properties.get("DisplayName") {
            zelf.name = format!("{}", prop);
        }

        if let Some(prop) = zelf.properties.get("DisplayVersion") {
            zelf.version = format!("{}", prop);
        } else if let Some(prop) = zelf.properties.get("Version") {
            zelf.version = format!("{}", prop);
        }

        for path in ["InstallLocation", "InstallSource", "BundleCachePath"] {
            let location = zelf.properties.get(path);
            if location.is_some() && location.unwrap().bytes.len() > 2 {
                zelf.path = format!("{}", location.unwrap());
                break;
            }
        }

        if let Some(prop) = zelf.properties.get("Publisher") {
            zelf.publishers.push(format!("{}", prop));
        }

        zelf
    }
}

impl Component for Application {
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
#[derive(Debug, Deserialize)]
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

impl Component for Driver {
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
        ""
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
    fn collect_drivers(&self) -> Result<Vec<Box<dyn Component>>, Error> {
        let mut comps: Vec<Box<dyn Component>> = vec![];

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

            comps.push(Box::new(driver));
        }

        Ok(comps)
    }

    fn collect_apps(&self) -> Result<Vec<Box<dyn Component>>, Error> {
        let mut comps: Vec<Box<dyn Component>> = vec![];

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

    fn collect_from_json(&self, _: &str) -> Result<Vec<Box<dyn Component>>, Error> {
        Err("not implemented".to_owned())
    }

    fn collect(&self) -> Result<Vec<Box<dyn Component>>, Error> {
        log::info!("collecting Windows applications and drivers, please wait ...");

        let mut drivers = self.collect_drivers()?;
        let mut apps = self.collect_apps()?;

        drivers.append(&mut apps);

        Ok(drivers)
    }
}
