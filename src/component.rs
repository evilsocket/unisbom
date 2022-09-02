use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
pub(crate) enum Kind {
    #[default]
    Application,
    Driver,
    Other,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub(crate) struct Component {
    pub kind: Kind,
    pub name: String,
    pub id: String,
    pub version: String,
    pub path: String,
    pub modified: DateTime<Utc>,
    pub signed_by: Vec<String>,
    // as returned by os specific code
    pub raw_info: String,
}
