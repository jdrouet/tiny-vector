use std::borrow::Cow;

use indexmap::IndexMap;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "content")]
pub enum Event {
    Log(EventLog),
    Metric(EventMetric),
}

impl From<EventLog> for Event {
    fn from(value: EventLog) -> Self {
        Self::Log(value)
    }
}

impl From<EventMetric> for Event {
    fn from(value: EventMetric) -> Self {
        Self::Metric(value)
    }
}

impl Event {
    pub fn into_event_log(self) -> Option<EventLog> {
        match self {
            Self::Log(inner) => Some(inner),
            _ => None,
        }
    }

    pub fn into_event_metric(self) -> Option<EventMetric> {
        match self {
            Self::Metric(inner) => Some(inner),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EventLog {
    #[serde(flatten)]
    pub attributes: IndexMap<Cow<'static, str>, Cow<'static, str>>,
    pub message: String,
}

impl EventLog {
    pub fn new<M: Into<String>>(message: M) -> Self {
        Self {
            attributes: IndexMap::new(),
            message: message.into(),
        }
    }

    pub fn with_attribute<K: Into<Cow<'static, str>>, V: Into<Cow<'static, str>>>(
        mut self,
        name: K,
        value: V,
    ) -> Self {
        self.attributes.insert(name.into(), value.into());
        self
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EventMetric {
    pub namespace: Cow<'static, str>,
    pub name: Cow<'static, str>,
    pub tags: IndexMap<Cow<'static, str>, Cow<'static, str>>,
    pub value: f64,
}

impl EventMetric {
    pub fn new<N: Into<Cow<'static, str>>, M: Into<Cow<'static, str>>>(
        namespace: N,
        name: M,
        value: f64,
    ) -> Self {
        Self {
            namespace: namespace.into(),
            name: name.into(),
            tags: IndexMap::new(),
            value,
        }
    }

    pub fn with_tag<K: Into<Cow<'static, str>>, V: Into<Cow<'static, str>>>(
        mut self,
        name: K,
        value: V,
    ) -> Self {
        self.tags.insert(name.into(), value.into());
        self
    }
}
