use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize, Copy, Clone)]
pub(crate) enum Kind {
    #[default]
    Application,
    Driver,
    Other,
}

pub(crate) trait ComponentTrait {
    fn kind(&self) -> Kind;
    fn name(&self) -> &str;
    fn id(&self) -> &str;
    fn version(&self) -> &str;
    fn path(&self) -> &str;
    fn modified(&self) -> DateTime<Utc>;
    fn publishers(&self) -> &Vec<String>;
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Component {
    pub kind: Kind,
    pub name: String,
    pub id: String,
    pub version: String,
    pub path: String,
    pub modified: DateTime<Utc>,
    pub publishers: Vec<String>,
}

impl Component {
    pub fn from_trait(comp: &dyn ComponentTrait) -> Self {
        Self {
            kind: comp.kind(),
            name: comp.name().to_owned(),
            id: comp.id().to_owned(),
            version: comp.version().to_owned(),
            path: comp.path().to_owned(),
            modified: comp.modified(),
            publishers: comp.publishers().to_owned(),
        }
    }
}
