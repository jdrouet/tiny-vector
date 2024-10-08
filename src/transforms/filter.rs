use tokio::sync::mpsc::error::SendError;

use super::condition::prelude::Evaluate;
use super::condition::Condition;
use crate::components::collector::Collector;
use crate::components::output::{ComponentWithOutputs, NamedOutput};
use crate::event::Event;

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error(transparent)]
    ConditionFailed(super::condition::BuildError),
}

fn default_fallback() -> NamedOutput {
    NamedOutput::Named("dropped".into())
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct Config {
    condition: crate::transforms::condition::Config,
    /// Route being used when condition is not matching.
    fallback: Option<NamedOutput>,
}

impl Config {
    fn is_fallback(&self, output: &NamedOutput) -> bool {
        if let Some(ref named) = self.fallback {
            named.eq(output)
        } else {
            default_fallback().eq(output)
        }
    }
}

impl ComponentWithOutputs for Config {
    fn has_output(&self, output: &NamedOutput) -> bool {
        output.is_default() || self.is_fallback(output)
    }
}

impl Config {
    pub fn build(self) -> Result<Transform, BuildError> {
        let fallback = self.fallback.unwrap_or_else(default_fallback);
        Ok(Transform {
            condition: self
                .condition
                .build()
                .map_err(BuildError::ConditionFailed)?,
            fallback,
        })
    }
}

#[derive(Debug)]
pub struct Transform {
    condition: Condition,
    fallback: NamedOutput,
}

impl Transform {
    pub(crate) fn flavor(&self) -> &'static str {
        "filter"
    }
}

impl super::Executable for Transform {
    async fn handle(&self, collector: &Collector, event: Event) -> Result<(), SendError<Event>>
    where
        Self: Sync,
    {
        if self.condition.evaluate(&event) {
            collector.send_default(event).await
        } else {
            collector.send_named(&self.fallback, event).await
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::components::collector::Collector;
    use crate::components::output::NamedOutput;
    use crate::event::metric::EventMetricValue;
    use crate::prelude::create_channel;
    use crate::transforms::condition;

    #[tokio::test]
    async fn should_route_events_properly() {
        use crate::transforms::Executable;

        let default_output = NamedOutput::default();
        let dropped_output = NamedOutput::named("dropped");
        let config = super::Config {
            condition: condition::Config::is_metric(),
            fallback: None,
        };
        let transform = config.build().unwrap();
        let (default_tx, default_rx) = create_channel(10);
        let (dropped_tx, dropped_rx) = create_channel(10);
        let collector = Collector::default()
            .with_output(dropped_output, dropped_tx)
            .with_output(default_output, default_tx);
        transform
            .handle(
                &collector,
                crate::event::log::EventLog::new("Hello World!")
                    .with_attribute("hostname", "fake-server")
                    .with_attribute("ddsource", "tiny-vector")
                    .into(),
            )
            .await
            .unwrap();
        transform
            .handle(
                &collector,
                crate::event::metric::EventMetric::new(
                    crate::helper::now(),
                    "foo",
                    "bar",
                    EventMetricValue::Gauge(42.0),
                )
                .with_tag("hostname", "fake-server")
                .into(),
            )
            .await
            .unwrap();
        assert_eq!(1, default_rx.len());
        assert_eq!(1, dropped_rx.len());
    }
}
