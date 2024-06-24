pub mod console;
pub mod datadog_logs;

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Config {
    Console(self::console::Config),
    DatadogLogs(self::datadog_logs::Config),
}

impl Config {
    pub fn build(self) -> (Sink, crate::prelude::Sender) {
        match self {
            Self::Console(inner) => {
                let (inner, tx) = inner.build();
                (Sink::Console(inner), tx)
            }
            Self::DatadogLogs(inner) => {
                let (inner, tx) = inner.build();
                (Sink::DatadogLogs(inner), tx)
            }
        }
    }
}

pub enum Sink {
    Console(self::console::Sink),
    DatadogLogs(self::datadog_logs::Sink),
}

impl Sink {
    pub fn run(self) -> tokio::task::JoinHandle<()> {
        match self {
            Self::Console(inner) => inner.run(),
            Self::DatadogLogs(inner) => inner.run(),
        }
    }
}
