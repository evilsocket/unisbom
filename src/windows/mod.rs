use std::collections::HashMap;
use std::process::Command;

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use crate::collector;
use crate::component::{ComponentTrait, Kind};
use crate::Error;

mod api;

lazy_static! {
    static ref MICROSOFT_DEFAULT_PUBLISHERS: Vec<String> = vec!["Microsoft".to_string(),];
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
struct OS {
    pub name: String,
    pub version: String,
}

impl ComponentTrait for OS {
    fn kind(&self) -> Kind {
        Kind::OS
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn id(&self) -> &str {
        self.name()
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn path(&self) -> &str {
        "/"
    }

    fn modified(&self) -> DateTime<Utc> {
        DateTime::default()
    }

    fn publishers(&self) -> &Vec<String> {
        &MICROSOFT_DEFAULT_PUBLISHERS
    }
}

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
    pub fn new(key: String, modified: NaiveDateTime, properties: HashMap<String, String>) -> Self {
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
    fn collect_os(&self) -> Result<Box<dyn ComponentTrait>, Error> {
        let ver = Command::new("cmd.exe")
            .args(&["/c", "ver"])
            .output()
            .map_err(|e| format!("could not execute ver: {:?}", e))?;

        if !ver.status.success() {
            return Err(format!(
                "ver exit status {:?}: {:?}",
                ver.status,
                String::from_utf8_lossy(&ver.stderr)
            ));
        }

        let raw = String::from_utf8_lossy(&ver.stdout).into_owned();

        Ok(Box::new(OS {
            name: "Microsoft Windows".to_owned(),
            version: raw
                .trim()
                .to_string()
                .split("[Version ")
                .collect::<Vec<&str>>()[1]
                .replace("]", ""),
        }))
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

            let version = api::parse_file_version(driver.path());
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

        for entry in api::enum_registry_uninstall_locations()? {
            if entry.properties.contains_key("DisplayName") {
                comps.push(Box::new(Application::new(
                    entry.key_name,
                    entry.modified,
                    entry.properties,
                )));
            } else {
                log::debug!("skipping uninstall entry: {:?}", &entry);
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

        let os = self.collect_os()?;
        let mut drivers = self.collect_drivers()?;
        let mut apps = self.collect_apps()?;

        drivers.push(os);
        drivers.append(&mut apps);

        Ok(drivers)
    }
}
