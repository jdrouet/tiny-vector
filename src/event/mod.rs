use indexmap::IndexMap;

#[derive(Debug, Clone)]
pub enum Event {
    Log(EventLog),
}

impl From<EventLog> for Event {
    fn from(value: EventLog) -> Self {
        Self::Log(value)
    }
}

impl Event {
    pub fn into_event_log(self) -> Option<EventLog> {
        match self {
            Self::Log(inner) => Some(inner),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EventLog {
    #[serde(flatten)]
    attributes: IndexMap<String, String>,
    message: String,
}

impl EventLog {
    pub fn new<M: Into<String>>(message: M) -> Self {
        Self {
            attributes: IndexMap::new(),
            message: message.into(),
        }
    }

    pub fn with_attribute<K: Into<String>, V: Into<String>>(mut self, name: K, value: V) -> Self {
        self.attributes.insert(name.into(), value.into());
        self
    }
}
