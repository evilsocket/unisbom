use std::process::Command;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::collector;
use crate::component::*;
use crate::Error;

#[derive(Serialize, Deserialize)]
struct Application {
    #[serde(rename = "_name")]
    pub name: String,
    pub arch_kind: String,
    #[serde(rename = "lastModified")]
    pub modified: DateTime<Utc>,
    pub obtained_from: String,
    pub path: String,
    pub signed_by: Option<Vec<String>>,
    pub version: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct Extension {
    #[serde(rename = "_name")]
    pub name: String,
    #[serde(rename = "spext_architectures")]
    pub architectures: Option<Vec<String>>,
    #[serde(rename = "spext_bundleid")]
    pub bundleid: String,
    #[serde(rename = "spext_has64BitIntelCode")]
    pub has_64bit_intel_code: Option<String>,
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
    #[serde(rename = "spext_runtime_environment")]
    pub runtime_environment: Option<String>,
    #[serde(rename = "spext_signed_by")]
    pub signed_by: String,
    pub spext_version: String,
    pub version: String,
}

#[derive(Deserialize)]
struct Profile {
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

    fn collect(&self) -> Result<Vec<Component>, Error> {
        log::info!("collecting macOS applications and drivers, please wait ...");

        let mut comps = vec![];
        let profiler = Command::new("system_profiler")
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
        let profile: Profile = serde_json::from_str(&raw_profile)
            .map_err(|e| format!("could not parse system_profiler output: {:?}", e))?;

        for ext in profile.drivers {
            comps.push(Component {
                kind: Kind::Driver,
                name: ext.name.to_owned(),
                id: ext.bundleid.to_owned(),
                version: ext.version.to_owned(),
                path: ext.path.to_owned(),
                modified: ext.last_modified.to_owned(),
                signed_by: vec![ext.signed_by.to_owned()],
                raw_info: serde_json::to_string(&ext)
                    .map_err(|e| format!("could not serialize kext raw info: {:?}", e))?,
            });
        }

        for app in profile.apps {
            comps.push(Component {
                kind: Kind::Application,
                name: app.name.to_owned(),
                id: app.name.to_owned(),
                version: app.version.to_owned().unwrap_or_default(),
                path: app.path.to_owned(),
                modified: app.modified.to_owned(),
                signed_by: app.signed_by.to_owned().unwrap_or_default(),
                raw_info: serde_json::to_string(&app)
                    .map_err(|e| format!("could not serialize app raw info: {:?}", e))?,
            });
        }

        Ok(comps)
    }
}
