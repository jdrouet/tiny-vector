use sqlx::types::Json;
use sqlx::SqliteConnection;
use tracing::Instrument;

use crate::components::name::ComponentName;
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
        Ok(Sink { options })
    }
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

pub struct Sink {
    options: sqlx::sqlite::SqliteConnectOptions,
}

impl Sink {
    async fn execute(self, mut receiver: Receiver) {
        use sqlx::ConnectOptions;

        let Ok(mut conn) = self.options.connect().await else {
            tracing::error!("unable to connect to the database");
            return;
        };
        if let Err(err) = migrate(&mut conn).await {
            tracing::error!("unable to execute migration: {err:?}");
            return;
        }
        tracing::info!("starting");
        while let Some(input) = receiver.recv().await {
            if let Err(err) = persist_event(&mut conn, input).await {
                tracing::error!("unable to persist received event: {err:?}");
            }
        }
        tracing::info!("stopping");
    }

    pub async fn run(
        self,
        name: &ComponentName,
        receiver: Receiver,
    ) -> tokio::task::JoinHandle<()> {
        let span = tracing::info_span!(
            "component",
            name = name.as_ref(),
            kind = "sink",
            flavor = "console"
        );
        tokio::spawn(async move { self.execute(receiver).instrument(span).await })
    }
}
