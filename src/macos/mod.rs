use std::process::Command;

use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use crate::collector;
use crate::component::{ComponentTrait, Kind};
use crate::utils::serde::string_as_string_vector;
use crate::Error;

lazy_static! {
    static ref APPLE_DEFAULT_PUBLISHERS: Vec<String> = vec![
        "Apple Code Signing Certification Authority".to_string(),
        "Apple Root CA".to_string(),
    ];
}

#[derive(Serialize, Deserialize)]
struct OS {
    pub os_version: String,
}

impl ComponentTrait for OS {
    fn kind(&self) -> Kind {
        Kind::OS
    }

    fn name(&self) -> &str {
        "macOS"
    }

    fn id(&self) -> &str {
        self.name()
    }

    fn version(&self) -> &str {
        &self.os_version
    }

    fn path(&self) -> &str {
        "/"
    }

    fn modified(&self) -> DateTime<Utc> {
        DateTime::default()
    }

    fn publishers(&self) -> &Vec<String> {
        &APPLE_DEFAULT_PUBLISHERS
    }
}

#[derive(Serialize, Deserialize)]
struct Application {
    #[serde(rename = "_name")]
    pub name: String,
    pub arch_kind: String,
    #[serde(rename = "lastModified")]
    pub modified: DateTime<Utc>,
    pub obtained_from: String,
    pub path: String,
    #[serde(default)]
    pub signed_by: Vec<String>,
    #[serde(default)]
    pub version: String,
}

impl ComponentTrait for Application {
    fn kind(&self) -> Kind {
        Kind::Application
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn id(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn path(&self) -> &str {
        &self.path
    }

    fn modified(&self) -> DateTime<Utc> {
        self.modified
    }

    fn publishers(&self) -> &Vec<String> {
        &self.signed_by
    }
}

#[derive(Serialize, Deserialize)]
struct Extension {
    #[serde(rename = "_name")]
    pub name: String,
    #[serde(rename = "spext_architectures", default)]
    pub architectures: Vec<String>,
    #[serde(rename = "spext_bundleid")]
    pub bundleid: String,
    #[serde(rename = "spext_has64BitIntelCode", default)]
    pub has_64bit_intel_code: String,
    #[serde(rename = "spext_hasAllDependencies")]
    pub has_all_dependencies: String,
    #[serde(rename = "spext_lastModified")]
    pub last_modified: DateTime<Utc>,
    #[serde(rename = "spext_loadable")]
    pub loadable: String,
    #[serde(rename = "spext_loaded")]
    pub loaded: String,
    #[serde(rename = "spext_notarized")]
    pub notarized: String,
    #[serde(rename = "spext_obtained_from")]
    pub obtained_from: String,
    #[serde(rename = "spext_path")]
    pub path: String,
    #[serde(rename = "spext_runtime_environment", default)]
    pub runtime_environment: String,
    #[serde(
        rename = "spext_signed_by",
        deserialize_with = "string_as_string_vector"
    )]
    pub signed_by: Vec<String>,
    pub spext_version: String,
    pub version: String,
}

impl ComponentTrait for Extension {
    fn kind(&self) -> Kind {
        Kind::Driver
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn id(&self) -> &str {
        &self.bundleid
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn path(&self) -> &str {
        &self.path
    }

    fn modified(&self) -> DateTime<Utc> {
        self.last_modified
    }

    fn publishers(&self) -> &Vec<String> {
        &self.signed_by
    }
}

#[derive(Deserialize)]
struct Profile {
    #[serde(rename = "SPSoftwareDataType")]
    pub os: Vec<OS>,
    #[serde(rename = "SPApplicationsDataType")]
    pub apps: Vec<Application>,
    #[serde(rename = "SPExtensionsDataType")]
    pub drivers: Vec<Extension>,
}

#[derive(Default)]
pub(crate) struct Collector {}

impl collector::Collector for Collector {
    fn setup(&mut self) -> Result<(), Error> {
        Ok(())
    }

    fn collect_from_json(&self, json: &str) -> Result<Vec<Box<dyn ComponentTrait>>, Error> {
        let mut comps: Vec<Box<dyn ComponentTrait>> = vec![];

        let profile: Profile = serde_json::from_str(json)
            .map_err(|e| format!("could not parse system_profiler output: {:?}", e))?;

        for mut os in profile.os {
            os.os_version = os.os_version.replace("macOS ", "");
            comps.push(Box::new(os));
        }

        for ext in profile.drivers {
            comps.push(Box::new(ext));
        }

        for app in profile.apps {
            comps.push(Box::new(app));
        }

        Ok(comps)
    }

    fn collect(&self) -> Result<Vec<Box<dyn ComponentTrait>>, Error> {
        log::info!("collecting applications and drivers, please wait ...");

        let profiler = Command::new("system_profiler")
            .arg("SPSoftwareDataType")
            .arg("SPExtensionsDataType")
            .arg("SPApplicationsDataType")
            .args(&["-detailLevel", "full"])
            .arg("-json")
            .output()
            .map_err(|e| format!("could not execute system_profiler: {:?}", e))?;

        if !profiler.status.success() {
            return Err(format!(
                "system_profiler exit status {:?}: {:?}",
                profiler.status,
                String::from_utf8_lossy(&profiler.stderr)
            ));
        }

        let raw_profile = String::from_utf8_lossy(&profiler.stdout);

        self.collect_from_json(&raw_profile)
    }
}
