use tokio::sync::mpsc::error::SendError;
use tracing::Instrument;

use crate::components::collector::Collector;
use crate::components::name::ComponentName;
use crate::components::output::{ComponentWithOutputs, NamedOutput};
use crate::event::Event;
use crate::prelude::Receiver;

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
    #[inline]
    async fn handle(&self, collector: &Collector, event: Event) -> Result<(), SendError<Event>> {
        collector.send_all(event).await
    }

    async fn execute(self, mut receiver: Receiver, collector: Collector) {
        tracing::info!("starting");
        while let Some(event) = receiver.recv().await {
            if let Err(err) = self.handle(&collector, event).await {
                tracing::error!("unable to route event: {err:?}");
                break;
            }
        }
        tracing::info!("stopping");
    }

    pub async fn run(
        self,
        name: &ComponentName,
        receiver: Receiver,
        collector: Collector,
    ) -> tokio::task::JoinHandle<()> {
        let span = tracing::info_span!(
            "component",
            name = name.as_ref(),
            kind = "transform",
            flavor = "broadcast"
        );
        tokio::spawn(async move { self.execute(receiver, collector).instrument(span).await })
    }
}

#[cfg(test)]
mod tests {
    use crate::components::collector::Collector;
    use crate::components::output::NamedOutput;
    use crate::prelude::create_channel;

    #[tokio::test]
    async fn should_broadcast_events_properly() {
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
                crate::event::metric::EventMetric::new(crate::helper::now(), "foo", "bar", 42.0)
                    .with_tag("hostname", "fake-server")
                    .into(),
            )
            .await
            .unwrap();
        assert_eq!(2, metric_rx.len());
        assert_eq!(2, log_rx.len());
    }
}
