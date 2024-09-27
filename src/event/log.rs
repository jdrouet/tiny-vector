use indexmap::IndexMap;

use super::CowStr;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, derive_more::From)]
#[serde(untagged)]
pub enum EventLogAttribute {
    Text(CowStr),
    UInteger(u64),
    Integer(i64),
    Float(f64),
    Boolean(bool),
}

#[cfg(test)]
impl EventLogAttribute {
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(inner) => Some(inner.as_ref()),
            _ => None,
        }
    }

    pub fn as_uint(&self) -> Option<u64> {
        match self {
            Self::UInteger(inner) => Some(*inner),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            Self::Integer(inner) => Some(*inner),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            Self::Float(inner) => Some(*inner),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Boolean(inner) => Some(*inner),
            _ => None,
        }
    }
}

impl From<&'static str> for EventLogAttribute {
    fn from(value: &'static str) -> Self {
        Self::Text(CowStr::Borrowed(value))
    }
}

impl From<String> for EventLogAttribute {
    fn from(value: String) -> Self {
        Self::Text(CowStr::Owned(value))
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EventLog {
    #[serde(flatten)]
    pub attributes: IndexMap<CowStr, EventLogAttribute>,
    pub message: String,
}

impl EventLog {
    pub fn new<M: Into<String>>(message: M) -> Self {
        Self {
            attributes: IndexMap::new(),
            message: message.into(),
        }
    }

    pub fn with_attribute<K: Into<CowStr>, V: Into<EventLogAttribute>>(
        mut self,
        name: K,
        value: V,
    ) -> Self {
        self.attributes.insert(name.into(), value.into());
        self
    }

    pub fn add_attribute<K: Into<CowStr>, V: Into<EventLogAttribute>>(
        &mut self,
        name: K,
        value: V,
    ) {
        self.attributes.insert(name.into(), value.into());
    }
}
