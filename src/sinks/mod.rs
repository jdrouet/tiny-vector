pub mod console;
pub mod datadog_logs;

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error(transparent)]
    Console(#[from] console::BuildError),
    #[error(transparent)]
    DatadogLogs(#[from] datadog_logs::BuildError),
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Config {
    Console(self::console::Config),
    DatadogLogs(self::datadog_logs::Config),
}

impl Config {
    pub fn build(self) -> Result<(Sink, crate::prelude::Sender), BuildError> {
        match self {
            Self::Console(inner) => {
                let (inner, tx) = inner.build()?;
                Ok((Sink::Console(inner), tx))
            }
            Self::DatadogLogs(inner) => {
                let (inner, tx) = inner.build()?;
                Ok((Sink::DatadogLogs(inner), tx))
            }
        }
    }
}

pub enum Sink {
    Console(self::console::Sink),
    DatadogLogs(self::datadog_logs::Sink),
}

impl Sink {
    pub async fn run(self, name: &str) -> tokio::task::JoinHandle<()> {
        match self {
            Self::Console(inner) => inner.run(name).await,
            Self::DatadogLogs(inner) => inner.run(name).await,
        }
    }
}
