use tokio::sync::mpsc::error::SendError;

use crate::components::collector::Collector;
use crate::components::output::{ComponentWithOutputs, NamedOutput};
use crate::event::Event;

#[derive(Debug, thiserror::Error)]
pub enum BuildError {}

#[derive(Clone, Debug, serde::Deserialize)]
#[cfg_attr(test, derive(Default))]
pub struct Config {}

impl ComponentWithOutputs for Config {
    fn has_output(&self, _: &NamedOutput) -> bool {
        // the broadcast transform accepts all the possible outputs
        true
    }
}

impl Config {
    pub fn build(self) -> Result<Transform, BuildError> {
        Ok(Transform {})
    }
}

#[derive(Debug)]
pub struct Transform {}

impl Transform {
    pub(crate) fn flavor(&self) -> &'static str {
        "broadcast"
    }
}

impl super::Executable for Transform {
    async fn handle(
        &self,
        collector: &Collector,
        event: Event,
    ) -> Result<(), SendError<Event>>
    where
        Self: Sync,
    { collector.send_all(event).await }
}

#[cfg(test)]
mod tests {
    use crate::components::collector::Collector;
    use crate::components::output::NamedOutput;
    use crate::event::metric::EventMetricValue;
    use crate::prelude::create_channel;

    #[tokio::test]
    async fn should_broadcast_events_properly() {
        use crate::transforms::Executable;

        let metrics_output = NamedOutput::named("metrics");
        let logs_output = NamedOutput::named("logs");
        let config = super::Config::default();
        let transform = config.build().unwrap();
        let (metric_tx, metric_rx) = create_channel(10);
        let (log_tx, log_rx) = create_channel(10);
        let collector = Collector::default()
            .with_output(metrics_output, metric_tx)
            .with_output(logs_output, log_tx);
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
        assert_eq!(2, metric_rx.len());
        assert_eq!(2, log_rx.len());
    }
}
