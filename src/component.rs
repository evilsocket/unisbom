use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
pub(crate) enum Kind {
    #[default]
    Application,
    Driver,
    Other,
}

pub(crate) trait Component {
    fn kind(&self) -> Kind;
    fn name(&self) -> &str;
    fn id(&self) -> &str;
    fn version(&self) -> &str;
    fn path(&self) -> &str;
    fn modified(&self) -> &DateTime<Utc>;
    fn signed_by(&self) -> &Vec<String>;
}
