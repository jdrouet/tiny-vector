use crate::components::name::ComponentName;
use crate::prelude::Receiver;

pub mod black_hole;
pub mod console;
#[cfg(feature = "sink-datadog-logs")]
pub mod datadog_logs;

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error(transparent)]
    BlackHole(#[from] black_hole::BuildError),
    #[error(transparent)]
    Console(#[from] console::BuildError),
    #[cfg(feature = "sink-datadog-logs")]
    #[error(transparent)]
    DatadogLogs(#[from] datadog_logs::BuildError),
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Config {
    BlackHole(self::black_hole::Config),
    Console(self::console::Config),
    #[cfg(feature = "sink-datadog-logs")]
    DatadogLogs(self::datadog_logs::Config),
}

impl Config {
    pub fn build(self) -> Result<Sink, BuildError> {
        Ok(match self {
            Self::BlackHole(inner) => Sink::BlackHole(inner.build()?),
            Self::Console(inner) => Sink::Console(inner.build()?),
            #[cfg(feature = "sink-datadog-logs")]
            Self::DatadogLogs(inner) => Sink::DatadogLogs(inner.build()?),
        })
    }
}

pub enum Sink {
    BlackHole(self::black_hole::Sink),
    Console(self::console::Sink),
    #[cfg(feature = "sink-datadog-logs")]
    DatadogLogs(self::datadog_logs::Sink),
}

impl Sink {
    pub async fn run(
        self,
        name: &ComponentName,
        receiver: Receiver,
    ) -> tokio::task::JoinHandle<()> {
        match self {
            Self::BlackHole(inner) => inner.run(name, receiver).await,
            Self::Console(inner) => inner.run(name, receiver).await,
            #[cfg(feature = "sink-datadog-logs")]
            Self::DatadogLogs(inner) => inner.run(name, receiver).await,
        }
    }
}
