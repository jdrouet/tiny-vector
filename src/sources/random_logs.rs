use tracing::Instrument;

use crate::components::collector::Collector;
use crate::components::name::ComponentName;
use crate::components::output::ComponentWithOutputs;
use crate::event::log::EventLogAttribute;

#[derive(Debug, thiserror::Error)]
pub struct BuildError;

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unable to build component")
    }
}

#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct Config {
    /// Interval between emitting events, in ms
    pub interval: Option<u64>,
}

fn generate() -> crate::event::Event {
    crate::event::log::EventLog::new("Hello World!")
        .with_attribute("hostname", "fake-server")
        .with_attribute("ddsource", "tiny-vector")
        .with_attribute(
            "timestamp",
            EventLogAttribute::UInteger(crate::helper::now()),
        )
        .into()
}

impl ComponentWithOutputs for Config {}

impl Config {
    pub fn build(self) -> Result<Source, BuildError> {
        Ok(Source {
            duration: tokio::time::Duration::from_millis(self.interval.unwrap_or(1000)),
        })
    }
}

pub struct Source {
    duration: tokio::time::Duration,
}

impl Source {
    pub async fn execute(self, collector: Collector) {
        tracing::info!("starting");
        let mut timer = tokio::time::interval(self.duration);
        loop {
            let _ = timer.tick().await;
            tracing::debug!("generating new random log");
            if let Err(err) = collector.send_default(generate()).await {
                tracing::error!("unable to send generated log: {err:?}");
                break;
            }
        }
        tracing::info!("stopping");
    }

    pub async fn run(
        self,
        name: &ComponentName,
        collector: Collector,
    ) -> tokio::task::JoinHandle<()> {
        let span = tracing::info_span!(
            "component",
            name = name.as_ref(),
            kind = "source",
            flavor = "random_logs"
        );
        tokio::spawn(async move { self.execute(collector).instrument(span).await })
    }
}
