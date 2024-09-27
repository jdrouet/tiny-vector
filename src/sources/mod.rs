use crate::components::collector::Collector;
use crate::components::name::ComponentName;
use crate::components::output::{ComponentWithOutputs, NamedOutput};

pub mod random_logs;
#[cfg(feature = "source-sysinfo")]
pub mod sysinfo;
#[cfg(feature = "source-tcp-server")]
pub mod tcp_server;

const COMPONENT_KIND: &str = "source";

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
#[enum_dispatch::enum_dispatch(ComponentWithOutputs)]
pub enum Config {
    RandomLogs(self::random_logs::Config),
    #[cfg(feature = "source-sysinfo")]
    Sysinfo(self::sysinfo::Config),
    #[cfg(feature = "source-tcp-server")]
    TcpServer(self::tcp_server::Config),
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

#[derive(Debug, thiserror::Error)]
pub enum StartingError {
    #[error(transparent)]
    RandomLogs(#[from] self::random_logs::StartingError),
    #[cfg(feature = "source-sysinfo")]
    #[error(transparent)]
    Sysinfo(#[from] self::sysinfo::StartingError),
    #[cfg(feature = "source-tcp-server")]
    #[error(transparent)]
    TcpServer(#[from] self::tcp_server::StartingError),
}

pub enum Source {
    RandomLogs(self::random_logs::Source),
    #[cfg(feature = "source-sysinfo")]
    Sysinfo(self::sysinfo::Source),
    #[cfg(feature = "source-tcp-server")]
    TcpServer(self::tcp_server::Source),
}

impl Source {
    fn flavor(&self) -> &'static str {
        match self {
            Self::RandomLogs(inner) => inner.flavor(),
            #[cfg(feature = "source-sysinfo")]
            Self::Sysinfo(inner) => inner.flavor(),
            #[cfg(feature = "source-tcp-server")]
            Self::TcpServer(inner) => inner.flavor(),
        }
    }

    pub async fn start(
        self,
        name: &ComponentName,
        collector: Collector,
    ) -> Result<tokio::task::JoinHandle<()>, StartingError> {
        let span = tracing::info_span!(
            "component",
            name = name.as_ref(),
            kind = COMPONENT_KIND,
            flavor = self.flavor(),
        );
        Ok(match self {
            Self::RandomLogs(inner) => run(inner, span, collector).await?,
            #[cfg(feature = "source-sysinfo")]
            Self::Sysinfo(inner) => run(inner, span, collector).await?,
            #[cfg(feature = "source-tcp-server")]
            Self::TcpServer(inner) => run(inner, span, collector).await?,
        })
    }
}

trait Preparable {
    type Output: Executable;
    type Error: Into<StartingError>;

    fn prepare(self)
        -> impl std::future::Future<Output = Result<Self::Output, Self::Error>> + Send;
}

trait Executable {
    fn execute(self, collector: Collector) -> impl std::future::Future<Output = ()> + Send;
}

async fn run<
    O: Executable + Send + 'static,
    E: Into<StartingError>,
    P: Preparable<Output = O, Error = E>,
>(
    element: P,
    span: tracing::Span,
    collector: Collector,
) -> Result<tokio::task::JoinHandle<()>, StartingError> {
    use tracing::Instrument;

    let prepared = element.prepare().await.map_err(|err| err.into())?;
    Ok(tokio::spawn(async move {
        prepared.execute(collector).instrument(span).await
    }))
}
