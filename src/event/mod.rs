use std::borrow::Cow;

pub mod log;
pub mod metric;

pub type CowStr = Cow<'static, str>;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "content")]
pub enum Event {
    Log(log::EventLog),
    Metric(metric::EventMetric),
}

impl From<log::EventLog> for Event {
    fn from(value: log::EventLog) -> Self {
        Self::Log(value)
    }
}

impl From<metric::EventMetric> for Event {
    fn from(value: metric::EventMetric) -> Self {
        Self::Metric(value)
    }
}

impl Event {
    pub fn as_event_log(&self) -> Option<&log::EventLog> {
        match self {
            Self::Log(ref inner) => Some(inner),
            _ => None,
        }
    }

    pub fn as_event_metric(&self) -> Option<&metric::EventMetric> {
        match self {
            Self::Metric(ref inner) => Some(inner),
            _ => None,
        }
    }

    pub fn into_event_log(self) -> Option<log::EventLog> {
        match self {
            Self::Log(inner) => Some(inner),
            _ => None,
        }
    }

    pub fn into_event_metric(self) -> Option<metric::EventMetric> {
        match self {
            Self::Metric(inner) => Some(inner),
            _ => None,
        }
    }
}
