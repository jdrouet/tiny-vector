pub mod random_logs;

#[derive(Debug)]
pub enum BuildError {
    RandomLogs(random_logs::BuildError),
}

impl From<random_logs::BuildError> for BuildError {
    fn from(value: random_logs::BuildError) -> Self {
        Self::RandomLogs(value)
    }
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Config {
    RandomLogs(self::random_logs::Config),
}

impl Config {
    pub fn build(self, sender: crate::prelude::Sender) -> Result<Source, BuildError> {
        Ok(match self {
            Self::RandomLogs(inner) => Source::RandomLogs(inner.build(sender)?),
        })
    }
}

pub enum Source {
    RandomLogs(self::random_logs::Source),
}

impl Source {
    pub fn run(self, name: &str) -> tokio::task::JoinHandle<()> {
        match self {
            Self::RandomLogs(inner) => inner.run(name),
        }
    }
}
