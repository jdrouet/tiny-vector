use crate::components::name::ComponentName;
use crate::prelude::Receiver;

pub mod black_hole;
pub mod console;
#[cfg(feature = "sink-datadog-logs")]
pub mod datadog_logs;
#[cfg(feature = "sink-file")]
pub mod file;
#[cfg(feature = "sink-sqlite")]
pub mod sqlite;

const COMPONENT_KIND: &str = "sink";

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error(transparent)]
    BlackHole(#[from] black_hole::BuildError),
    #[error(transparent)]
    Console(#[from] console::BuildError),
    #[cfg(feature = "sink-datadog-logs")]
    #[error(transparent)]
    DatadogLogs(#[from] datadog_logs::BuildError),
    #[cfg(feature = "sink-file")]
    #[error(transparent)]
    File(#[from] file::BuildError),
    #[cfg(feature = "sink-sqlite")]
    #[error(transparent)]
    Sqlite(#[from] sqlite::BuildError),
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
#[cfg_attr(test, derive(derive_more::From))]
pub enum Config {
    BlackHole(self::black_hole::Config),
    Console(self::console::Config),
    #[cfg(feature = "sink-datadog-logs")]
    DatadogLogs(self::datadog_logs::Config),
    #[cfg(feature = "sink-file")]
    File(self::file::Config),
    #[cfg(feature = "sink-sqlite")]
    Sqlite(self::sqlite::Config),
}

impl Config {
    pub async fn build(self) -> Result<Sink, BuildError> {
        Ok(match self {
            Self::BlackHole(inner) => Sink::BlackHole(inner.build()?),
            Self::Console(inner) => Sink::Console(inner.build()?),
            #[cfg(feature = "sink-datadog-logs")]
            Self::DatadogLogs(inner) => Sink::DatadogLogs(inner.build()?),
            #[cfg(feature = "sink-file")]
            Self::File(inner) => Sink::File(inner.build().await?),
            #[cfg(feature = "sink-sqlite")]
            Self::Sqlite(inner) => Sink::Sqlite(inner.build()?),
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StartingError {
    #[error(transparent)]
    BlackHole(#[from] self::black_hole::StartingError),
    #[error(transparent)]
    Console(#[from] self::console::StartingError),
    #[cfg(feature = "sink-datadog-logs")]
    #[error(transparent)]
    DatadogLogs(#[from] self::datadog_logs::StartingError),
    #[cfg(feature = "sink-file")]
    #[error(transparent)]
    File(#[from] self::file::StartingError),
    #[cfg(feature = "sink-sqlite")]
    #[error(transparent)]
    Sqlite(#[from] self::sqlite::StartingError),
}

pub enum Sink {
    BlackHole(self::black_hole::Sink),
    Console(self::console::Sink),
    #[cfg(feature = "sink-datadog-logs")]
    DatadogLogs(self::datadog_logs::Sink),
    #[cfg(feature = "sink-file")]
    File(self::file::Sink),
    #[cfg(feature = "sink-sqlite")]
    Sqlite(self::sqlite::Sink),
}

impl Sink {
    fn flavor(&self) -> &'static str {
        match self {
            Self::BlackHole(inner) => inner.flavor(),
            Self::Console(inner) => inner.flavor(),
            #[cfg(feature = "sink-datadog-logs")]
            Self::DatadogLogs(inner) => inner.flavor(),
            #[cfg(feature = "sink-file")]
            Self::File(inner) => inner.flavor(),
            #[cfg(feature = "sink-sqlite")]
            Self::Sqlite(inner) => inner.flavor(),
        }
    }

    pub async fn start(
        self,
        name: &ComponentName,
        receiver: Receiver,
    ) -> Result<tokio::task::JoinHandle<()>, StartingError> {
        let span = tracing::info_span!(
            "component",
            name = name.as_ref(),
            kind = COMPONENT_KIND,
            flavor = self.flavor(),
        );
        Ok(match self {
            Self::BlackHole(inner) => run(inner, span, receiver).await?,
            Self::Console(inner) => run(inner, span, receiver).await?,
            #[cfg(feature = "sink-datadog-logs")]
            Self::DatadogLogs(inner) => run(inner, span, receiver).await?,
            #[cfg(feature = "sink-file")]
            Self::File(inner) => run(inner, span, receiver).await?,
            #[cfg(feature = "sink-sqlite")]
            Self::Sqlite(inner) => run(inner, span, receiver).await?,
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
    fn execute(self, receiver: Receiver) -> impl std::future::Future<Output = ()> + Send;
}

async fn run<
    O: Executable + Send + 'static,
    E: Into<StartingError>,
    P: Preparable<Output = O, Error = E>,
>(
    element: P,
    span: tracing::Span,
    receiver: Receiver,
) -> Result<tokio::task::JoinHandle<()>, StartingError> {
    use tracing::Instrument;

    let prepared = element.prepare().await.map_err(|err| err.into())?;
    Ok(tokio::spawn(async move {
        prepared.execute(receiver).instrument(span).await
    }))
}
