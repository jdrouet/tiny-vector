use crate::event::Event;

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Condition {
    IsMetric,
    IsLog,
}

impl Condition {
    pub fn evaluate(&self, event: &Event) -> bool {
        match self {
            Self::IsLog => matches!(event, Event::Log(_)),
            Self::IsMetric => matches!(event, Event::Metric(_)),
        }
    }
}
