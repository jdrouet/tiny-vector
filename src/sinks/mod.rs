pub mod console;
pub mod datadog_log;

#[derive(Clone, Debug)]
pub enum Config {
    Console(self::console::Config),
    DatadogLog(self::datadog_log::Config),
}

impl Config {
    pub fn build(self) -> (Sink, crate::prelude::Sender) {
        match self {
            Self::Console(inner) => {
                let (inner, tx) = inner.build();
                (Sink::Console(inner), tx)
            }
            Self::DatadogLog(inner) => {
                let (inner, tx) = inner.build();
                (Sink::DatadogLog(inner), tx)
            }
        }
    }
}

pub enum Sink {
    Console(self::console::Sink),
    DatadogLog(self::datadog_log::Sink),
}

impl Sink {
    pub fn run(self) -> tokio::task::JoinHandle<()> {
        match self {
            Self::Console(inner) => inner.run(),
            Self::DatadogLog(inner) => inner.run(),
        }
    }
}
