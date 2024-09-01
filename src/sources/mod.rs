use crate::components::collector::Collector;
use crate::components::name::ComponentName;
use crate::components::output::{ComponentWithOutputs, NamedOutput};

pub mod random_logs;
#[cfg(feature = "source-sysinfo")]
pub mod sysinfo;
#[cfg(feature = "source-tcp-server")]
pub mod tcp_server;

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error(transparent)]
    RandomLogs(#[from] self::random_logs::BuildError),
    #[cfg(feature = "source-sysinfo")]
    #[error(transparent)]
    Sysinfo(#[from] self::sysinfo::BuildError),
    #[cfg(feature = "source-tcp-server")]
    #[error(transparent)]
    TcpServer(#[from] self::tcp_server::BuildError),
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
#[cfg_attr(test, derive(derive_more::From))]
pub enum Config {
    RandomLogs(self::random_logs::Config),
    #[cfg(feature = "source-sysinfo")]
    Sysinfo(self::sysinfo::Config),
    #[cfg(feature = "source-tcp-server")]
    TcpServer(self::tcp_server::Config),
}

impl ComponentWithOutputs for Config {
    fn has_output(&self, output: &NamedOutput) -> bool {
        match self {
            Self::RandomLogs(inner) => inner.has_output(output),
            #[cfg(feature = "source-sysinfo")]
            Self::Sysinfo(inner) => inner.has_output(output),
            #[cfg(feature = "source-tcp-server")]
            Self::TcpServer(inner) => inner.has_output(output),
        }
    }
}

impl Config {
    pub fn build(self) -> Result<Source, BuildError> {
        Ok(match self {
            Self::RandomLogs(inner) => Source::RandomLogs(inner.build()?),
            #[cfg(feature = "source-sysinfo")]
            Self::Sysinfo(inner) => Source::Sysinfo(inner.build()?),
            #[cfg(feature = "source-tcp-server")]
            Self::TcpServer(inner) => Source::TcpServer(inner.build()?),
        })
    }
}

pub enum Source {
    RandomLogs(self::random_logs::Source),
    #[cfg(feature = "source-sysinfo")]
    Sysinfo(self::sysinfo::Source),
    #[cfg(feature = "source-tcp-server")]
    TcpServer(self::tcp_server::Source),
}

impl Source {
    pub async fn run(
        self,
        name: &ComponentName,
        collector: Collector,
    ) -> tokio::task::JoinHandle<()> {
        match self {
            Self::RandomLogs(inner) => inner.run(name, collector).await,
            #[cfg(feature = "source-sysinfo")]
            Self::Sysinfo(inner) => inner.run(name, collector).await,
            #[cfg(feature = "source-tcp-server")]
            Self::TcpServer(inner) => inner.run(name, collector).await,
        }
    }
}
