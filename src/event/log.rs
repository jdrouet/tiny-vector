use indexmap::IndexMap;

use super::CowStr;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EventLog {
    #[serde(flatten)]
    pub attributes: IndexMap<CowStr, CowStr>,
    pub message: String,
}

impl EventLog {
    pub fn new<M: Into<String>>(message: M) -> Self {
        Self {
            attributes: IndexMap::new(),
            message: message.into(),
        }
    }

    pub fn with_attribute<K: Into<CowStr>, V: Into<CowStr>>(mut self, name: K, value: V) -> Self {
        self.attributes.insert(name.into(), value.into());
        self
    }

    pub fn add_attribute<K: Into<CowStr>, V: Into<CowStr>>(&mut self, name: K, value: V) {
        self.attributes.insert(name.into(), value.into());
    }
}
