pub mod random_logs;
pub mod tcp_server;

#[derive(Debug)]
pub enum BuildError {
    RandomLogs(random_logs::BuildError),
    TcpServer(tcp_server::BuildError),
}

impl From<random_logs::BuildError> for BuildError {
    fn from(value: random_logs::BuildError) -> Self {
        Self::RandomLogs(value)
    }
}

impl From<tcp_server::BuildError> for BuildError {
    fn from(value: tcp_server::BuildError) -> Self {
        Self::TcpServer(value)
    }
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Config {
    RandomLogs(self::random_logs::Config),
    TcpServer(self::tcp_server::Config),
}

impl Config {
    pub fn build(self, sender: crate::prelude::Sender) -> Result<Source, BuildError> {
        Ok(match self {
            Self::RandomLogs(inner) => Source::RandomLogs(inner.build(sender)?),
            Self::TcpServer(inner) => Source::TcpServer(inner.build(sender)?),
        })
    }
}

pub enum Source {
    RandomLogs(self::random_logs::Source),
    TcpServer(self::tcp_server::Source),
}

impl Source {
    pub async fn run(self, name: &str) -> tokio::task::JoinHandle<()> {
        match self {
            Self::RandomLogs(inner) => inner.run(name).await,
            Self::TcpServer(inner) => inner.run(name).await,
        }
    }
}
