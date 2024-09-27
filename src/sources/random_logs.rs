use crate::components::collector::Collector;
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
            state: Stale {
                duration: tokio::time::Duration::from_millis(self.interval.unwrap_or(1000)),
            },
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum StartingError {}

pub(crate) struct Stale {
    duration: tokio::time::Duration,
}

pub(crate) struct Running {
    timer: tokio::time::Interval,
}

pub struct Source<S = Stale> {
    state: S,
}

impl<S> Source<S> {
    pub const fn flavor(&self) -> &'static str {
        "random_logs"
    }
}

impl super::Preparable for Source<Stale> {
    type Output = Source<Running>;
    type Error = StartingError;

    async fn prepare(self) -> Result<Self::Output, Self::Error> {
        Ok(Source {
            state: Running {
                timer: tokio::time::interval(self.state.duration),
            },
        })
    }
}

impl super::Executable for Source<Running> {
    async fn execute(mut self, collector: Collector) {
        tracing::info!("starting");
        loop {
            let _ = self.state.timer.tick().await;
            tracing::debug!("generating new random log");
            if let Err(err) = collector.send_default(generate()).await {
                tracing::error!("unable to send generated log: {err:?}");
                break;
            }
        }
        tracing::info!("stopping");
    }
}
