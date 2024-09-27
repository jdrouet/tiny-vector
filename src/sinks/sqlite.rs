use sqlx::types::Json;
use sqlx::SqliteConnection;

use crate::event::log::EventLog;
use crate::event::metric::EventMetric;
use crate::event::Event;
use crate::helper::now;
use crate::prelude::Receiver;

#[derive(Clone, Debug, Default, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Config {
    url: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("invalid database connection url")]
    InvalidUrl(
        #[from]
        #[source]
        sqlx::Error,
    ),
}

impl Config {
    pub fn build(self) -> Result<Sink, BuildError> {
        use std::str::FromStr;

        let options = if let Some(ref url) = self.url {
            sqlx::sqlite::SqliteConnectOptions::from_str(url)?.create_if_missing(true)
        } else {
            sqlx::sqlite::SqliteConnectOptions::from_str("sqlite::memory:")?
        };
        Ok(Sink {
            state: Stale { options },
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StartingError {
    #[error("unable to connect")]
    UnableToConnect(#[source] sqlx::Error),
    #[error("unable to execute migrations")]
    UnableToMigrate(#[source] sqlx::Error),
}

async fn migrate(connection: &mut SqliteConnection) -> Result<(), sqlx::Error> {
    sqlx::query("create table if not exists event_logs (timestamp integer not null, attributes json not null default '{}', message text not null);").execute(&mut *connection).await?;
    sqlx::query("create table if not exists event_metrics (timestamp integer not null, namespace text not null, name text not null, tags json not null default '{}', value json not null);").execute(&mut *connection).await?;
    Ok(())
}

async fn persist_event_log(
    connection: &mut SqliteConnection,
    event: EventLog,
) -> Result<(), sqlx::Error> {
    sqlx::query("insert into event_logs (timestamp, attributes, message) values (?,?,?)")
        .bind(now() as i64)
        .bind(Json(event.attributes))
        .bind(&event.message)
        .execute(&mut *connection)
        .await?;
    Ok(())
}

async fn persist_event_metric(
    connection: &mut SqliteConnection,
    event: EventMetric,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "insert into event_metrics (timestamp, namespace, name, tags, value) values (?,?,?,?,?)",
    )
    .bind(now() as i64)
    .bind(event.header.name.namespace)
    .bind(event.header.name.name)
    .bind(Json(event.header.tags))
    .bind(Json(event.value))
    .execute(&mut *connection)
    .await?;
    Ok(())
}

async fn persist_event(connection: &mut SqliteConnection, event: Event) -> Result<(), sqlx::Error> {
    match event {
        Event::Log(event_log) => persist_event_log(connection, event_log).await,
        Event::Metric(event_metric) => persist_event_metric(connection, event_metric).await,
    }
}

pub(crate) struct Stale {
    options: sqlx::sqlite::SqliteConnectOptions,
}

pub(crate) struct Running {
    connection: sqlx::sqlite::SqliteConnection,
}

pub struct Sink<S = Stale> {
    state: S,
}

impl<S> Sink<S> {
    pub(crate) fn flavor(&self) -> &'static str {
        "sqlite"
    }
}

impl super::Preparable for Sink<Stale> {
    type Output = Sink<Running>;
    type Error = StartingError;

    async fn prepare(self) -> Result<Self::Output, Self::Error> {
        use sqlx::ConnectOptions;

        let mut conn = self
            .state
            .options
            .connect()
            .await
            .map_err(StartingError::UnableToConnect)?;
        migrate(&mut conn)
            .await
            .map_err(StartingError::UnableToMigrate)?;

        Ok(Sink {
            state: Running { connection: conn },
        })
    }
}

impl super::Executable for Sink<Running> {
    async fn execute(mut self, mut receiver: Receiver) {
        tracing::info!("starting");
        while let Some(input) = receiver.recv().await {
            if let Err(err) = persist_event(&mut self.state.connection, input).await {
                tracing::error!("unable to persist received event: {err:?}");
            }
        }
        tracing::info!("stopping");
    }
}
