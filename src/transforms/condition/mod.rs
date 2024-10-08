use self::prelude::{Builder, Evaluate};

pub mod prelude;

mod and;
mod has_attribute;
mod has_tag;
mod is_log;
mod is_metric;
mod not;
mod or;

#[derive(Debug, thiserror::Error)]
pub enum BuildError {}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Config {
    And(self::and::Config),
    HasAttribute(self::has_attribute::Config),
    HasTag(self::has_tag::Config),
    IsLog(self::is_log::Config),
    IsMetric(self::is_metric::Config),
    Not(self::not::Config),
    Or(self::or::Config),
}

#[cfg(test)]
impl Config {
    pub fn is_log() -> Self {
        Self::IsLog(self::is_log::Config)
    }

    pub fn is_metric() -> Self {
        Self::IsMetric(self::is_metric::Config)
    }
}

impl Config {
    pub fn build(self) -> Result<Condition, BuildError> {
        Ok(match self {
            Self::And(inner) => Condition::And(inner.build()?),
            Self::HasAttribute(inner) => Condition::HasAttribute(inner.build()?),
            Self::HasTag(inner) => Condition::HasTag(inner.build()?),
            Self::IsLog(inner) => Condition::IsLog(inner.build()?),
            Self::IsMetric(inner) => Condition::IsMetric(inner.build()?),
            Self::Not(inner) => Condition::Not(inner.build()?),
            Self::Or(inner) => Condition::Or(inner.build()?),
        })
    }
}

#[derive(Clone, Debug)]
#[enum_dispatch::enum_dispatch(Evaluate)]
pub enum Condition {
    And(self::and::Condition),
    Or(self::or::Condition),
    Not(self::not::Condition),
    // Metrics related
    HasAttribute(has_attribute::Condition),
    HasTag(has_tag::Condition),
    IsMetric(is_metric::Condition),
    // Logs related
    IsLog(is_log::Condition),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::log::EventLog;
    use crate::event::metric::{EventMetric, EventMetricValue};
    use crate::event::Event;

    #[test_case::test_case(
        r#"{"type": "and", "value": [{ "type": "is_log" }, { "type": "is_metric" }]}"#,
        &[],
        &[Event::Log(EventLog::new("hello world")), Event::Metric(EventMetric::new(0, "foo", "bar", EventMetricValue::Gauge(12.34)))];
        "and condition"
    )]
    #[test_case::test_case(
        r#"{"type": "or", "value": [{ "type": "is_log" }, { "type": "is_metric" }]}"#,
        &[Event::Log(EventLog::new("hello world")), Event::Metric(EventMetric::new(0, "foo", "bar", EventMetricValue::Gauge(12.34)))],
        &[];
        "or condition"
    )]
    #[test_case::test_case(
        r#"{"type":"is_log"}"#,
        &[Event::Log(EventLog::new("hello world"))],
        &[Event::Metric(EventMetric::new(0, "foo", "bar", EventMetricValue::Gauge(12.34)))];
        "is_log condition"
    )]
    #[test_case::test_case(
        r#"{"type":"has_attribute", "name": "foo"}"#,
        &[Event::Log(EventLog::new("hello world").with_attribute("foo", "bar"))],
        &[Event::Log(EventLog::new("hello world")), Event::Metric(EventMetric::new(0, "foo", "bar", EventMetricValue::Gauge(12.34)))];
        "has_attribute condition"
    )]
    #[test_case::test_case(
        r#"{"type":"is_metric"}"#,
        &[Event::Metric(EventMetric::new(0, "foo", "bar", EventMetricValue::Gauge(12.34)))],
        &[Event::Log(EventLog::new("hello world"))];
        "is_metric condition"
    )]
    #[test_case::test_case(
        r#"{"type":"has_tag", "name": "foo"}"#,
        &[Event::Metric(EventMetric::new(0, "foo", "bar", EventMetricValue::Gauge(12.34)).with_tag("foo", "bar"))],
        &[Event::Log(EventLog::new("hello world")), Event::Metric(EventMetric::new(0, "foo", "bar", EventMetricValue::Gauge(12.34)))];
        "has_tag condition"
    )]
    #[test_case::test_case(
        r#"{"type":"has_tag", "name": "foo", "check": { "type": "exists" } }"#,
        &[Event::Metric(EventMetric::new(0, "foo", "bar", EventMetricValue::Gauge(12.34)).with_tag("foo", "bar"))],
        &[Event::Log(EventLog::new("hello world")), Event::Metric(EventMetric::new(0, "foo", "bar", EventMetricValue::Gauge(12.34)))];
        "has_tag condition with check exists"
    )]
    #[test_case::test_case(
        r#"{"type":"has_tag", "name": "foo", "check": { "type": "equals", "value": "bar" }}"#,
        &[Event::Metric(EventMetric::new(0, "foo", "bar", EventMetricValue::Gauge(12.34)).with_tag("foo", "bar"))],
        &[Event::Log(EventLog::new("hello world")), Event::Metric(EventMetric::new(0, "foo", "bar", EventMetricValue::Gauge(12.34)))];
        "has_tag condition with check equals"
    )]
    #[test_case::test_case(
        r#"{"type":"has_tag", "name": "foo", "check": { "type": "starts_with", "value": "bar" }}"#,
        &[Event::Metric(EventMetric::new(0, "foo", "bar", EventMetricValue::Gauge(12.34)).with_tag("foo", "barzoo"))],
        &[
            Event::Log(EventLog::new("hello world")),
            Event::Metric(EventMetric::new(0, "foo", "bar", EventMetricValue::Gauge(12.34))),
            Event::Metric(EventMetric::new(0, "foo", "bar", EventMetricValue::Gauge(12.34)).with_tag("foo", "baz")),
        ];
        "has_tag condition with check starts_with"
    )]
    #[test_case::test_case(
        r#"{"type":"has_tag", "name": "foo", "check": { "type": "ends_with", "value": "bar" }}"#,
        &[Event::Metric(EventMetric::new(0, "foo", "bar", EventMetricValue::Gauge(12.34)).with_tag("foo", "zoobar"))],
        &[
            Event::Log(EventLog::new("hello world")),
            Event::Metric(EventMetric::new(0, "foo", "bar", EventMetricValue::Gauge(12.34))),
            Event::Metric(EventMetric::new(0, "foo", "bar", EventMetricValue::Gauge(12.34)).with_tag("foo", "zoobaz")),
        ];
        "has_tag condition with check ends_with"
    )]
    #[test_case::test_case(
        r#"{"type":"has_tag", "name": "foo", "check": { "type": "matches", "regex": "^H[3e]ll[o0]$" }}"#,
        &[
            Event::Metric(EventMetric::new(0, "foo", "bar", EventMetricValue::Gauge(12.34)).with_tag("foo", "Hello")),
            Event::Metric(EventMetric::new(0, "foo", "bar", EventMetricValue::Gauge(12.34)).with_tag("foo", "H3ll0")),
        ],
        &[
            Event::Log(EventLog::new("hello world")),
            Event::Metric(EventMetric::new(0, "foo", "bar", EventMetricValue::Gauge(12.34))),
            Event::Metric(EventMetric::new(0, "foo", "bar", EventMetricValue::Gauge(12.34)).with_tag("foo", "hell")),
        ];
        "has_tag condition with check matches"
    )]
    fn should_check_condition(condition: &str, valid: &[Event], invalid: &[Event]) {
        let cond: Config = serde_json::from_str(condition).unwrap();
        let cond = cond.build().unwrap();
        for item in valid {
            assert!(cond.evaluate(item), "should validate {item:?}");
        }
        for item in invalid {
            assert!(!cond.evaluate(item), "should not validate {item:?}");
        }
    }
}
