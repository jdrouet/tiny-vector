use crate::event::Event;

mod is_log;
mod is_metric;

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Config {
    IsLog(self::is_log::Config),
    IsMetric(self::is_metric::Config),
}

impl Config {
    pub fn build(self) -> Condition {
        match self {
            Self::IsLog(inner) => Condition::IsLog(inner.build()),
            Self::IsMetric(inner) => Condition::IsMetric(inner.build()),
        }
    }
}

#[enum_dispatch::enum_dispatch]
pub trait Evaluate {
    fn evaluate(&self, event: &Event) -> bool;
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct Not(Box<Condition>);

impl Evaluate for Not {
    fn evaluate(&self, event: &Event) -> bool {
        !self.0.evaluate(event)
    }
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct And {
    value: Vec<Condition>,
}

impl Evaluate for And {
    fn evaluate(&self, event: &Event) -> bool {
        self.value.iter().all(|c| c.evaluate(event))
    }
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct Or {
    value: Vec<Condition>,
}

impl Evaluate for Or {
    fn evaluate(&self, event: &Event) -> bool {
        self.value.iter().any(|c| c.evaluate(event))
    }
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
#[enum_dispatch::enum_dispatch(Evaluate)]
pub enum Condition {
    And(And),
    Or(Or),
    Not(Not),
    // Metrics related
    IsMetric(is_metric::Condition),
    // Logs related
    IsLog(is_log::Condition),
}

#[cfg(test)]
impl Condition {
    pub fn is_metric() -> Self {
        Self::IsMetric(is_metric::Condition)
    }

    pub fn is_log() -> Self {
        Self::IsLog(is_log::Condition)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_evaluate_and() {
        let cond: Condition = serde_json::from_value(serde_json::json!({
            "type": "and",
            "value": [
                { "type": "is_log" },
                { "type": "is_metric" },
            ],
        }))
        .unwrap();
        assert!(matches!(cond, Condition::And(_)));
        assert!(
            !cond.evaluate(&crate::event::Event::Log(crate::event::log::EventLog::new(
                "hello world"
            )))
        );
        assert!(!cond.evaluate(&crate::event::Event::Metric(
            crate::event::metric::EventMetric::new(crate::helper::now(), "foo", "bar", 42.0)
        )));
    }

    #[test]
    fn should_evaluate_or() {
        let cond: Condition = serde_json::from_value(serde_json::json!({
            "type": "or",
            "value": [
                { "type": "is_log" },
                { "type": "is_metric" },
            ],
        }))
        .unwrap();
        assert!(matches!(cond, Condition::Or(_)));
        assert!(
            cond.evaluate(&crate::event::Event::Log(crate::event::log::EventLog::new(
                "hello world"
            )))
        );
        assert!(cond.evaluate(&crate::event::Event::Metric(
            crate::event::metric::EventMetric::new(crate::helper::now(), "foo", "bar", 42.0)
        )));
    }

    #[test]
    fn should_evaluate_is_log() {
        let cond: Condition = serde_json::from_value(serde_json::json!({
            "type": "is_log"
        }))
        .unwrap();
        assert!(matches!(cond, Condition::IsLog(_)));
        assert!(
            cond.evaluate(&crate::event::Event::Log(crate::event::log::EventLog::new(
                "hello world"
            )))
        );
        assert!(!cond.evaluate(&crate::event::Event::Metric(
            crate::event::metric::EventMetric::new(crate::helper::now(), "foo", "bar", 42.0)
        )));
    }

    #[test]
    fn should_evaluate_is_metric() {
        let cond: Condition = serde_json::from_value(serde_json::json!({
            "type": "is_metric"
        }))
        .unwrap();
        assert!(matches!(cond, Condition::IsMetric(_)));
        assert!(
            !cond.evaluate(&crate::event::Event::Log(crate::event::log::EventLog::new(
                "hello world"
            )))
        );
        assert!(cond.evaluate(&crate::event::Event::Metric(
            crate::event::metric::EventMetric::new(crate::helper::now(), "foo", "bar", 42.0)
        )));
    }
}
