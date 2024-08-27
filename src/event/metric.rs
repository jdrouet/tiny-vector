use indexmap::IndexMap;

use super::CowStr;

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct EventMetricName {
    pub namespace: CowStr,
    pub name: CowStr,
}

impl EventMetricName {
    pub fn new<N: Into<CowStr>, M: Into<CowStr>>(namespace: N, name: M) -> Self {
        Self {
            namespace: namespace.into(),
            name: name.into(),
        }
    }
}

pub type EventMetricTags = IndexMap<CowStr, CowStr>;

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct EventMetricHeader {
    #[serde(flatten)]
    pub name: EventMetricName,
    pub tags: EventMetricTags,
}

impl EventMetricHeader {
    pub fn add_tag<N: Into<CowStr>, V: Into<CowStr>>(&mut self, name: N, value: V) {
        self.tags.insert(name.into(), value.into());
    }

    pub fn with_tag<N: Into<CowStr>, V: Into<CowStr>>(mut self, name: N, value: V) -> Self {
        self.tags.insert(name.into(), value.into());
        self
    }
}

impl EventMetricHeader {
    pub fn new<N: Into<CowStr>, M: Into<CowStr>>(namespace: N, name: M) -> Self {
        Self {
            name: EventMetricName::new(namespace, name),
            tags: EventMetricTags::default(),
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub enum EventMetricValue {
    Counter(usize),
    Gauge(f64),
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EventMetric {
    #[serde(flatten)]
    pub header: EventMetricHeader,
    pub value: f64,
}

impl EventMetric {
    pub fn new<N: Into<CowStr>, M: Into<CowStr>>(namespace: N, name: M, value: f64) -> Self {
        Self {
            header: EventMetricHeader::new(namespace, name),
            value,
        }
    }

    pub fn with_tag<N: Into<CowStr>, V: Into<CowStr>>(mut self, name: N, value: V) -> Self {
        self.header.add_tag(name, value);
        self
    }

    pub fn add_tag<N: Into<CowStr>, V: Into<CowStr>>(&mut self, name: N, value: V) {
        self.header.add_tag(name, value);
    }
}
