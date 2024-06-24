pub mod random_logs;

#[derive(Clone, Debug)]
pub enum Config {
    RandomLogs(self::random_logs::Config),
}

impl Config {
    pub fn build(self, sender: crate::prelude::Sender) -> Source {
        match self {
            Self::RandomLogs(inner) => Source::RandomLogs(inner.build(sender)),
        }
    }
}

pub enum Source {
    RandomLogs(self::random_logs::Source),
}

impl Source {
    pub fn run(self) -> tokio::task::JoinHandle<()> {
        match self {
            Self::RandomLogs(inner) => inner.run(),
        }
    }
}
