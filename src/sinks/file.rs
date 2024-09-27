use std::path::PathBuf;

use tokio::io::AsyncWriteExt;

use crate::event::Event;
use crate::prelude::Receiver;

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Config {
    path: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum BuildError {}

impl Config {
    pub async fn build(self) -> Result<Sink, BuildError> {
        Ok(Sink {
            state: Stale { path: self.path },
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StartingError {
    #[error("unable to open file")]
    UnableToOpenFile(
        #[from]
        #[source]
        std::io::Error,
    ),
}

pub(crate) struct Stale {
    path: PathBuf,
}

pub(crate) struct Running {
    output: tokio::fs::File,
}

pub struct Sink<S = Stale> {
    state: S,
}

impl<S> Sink<S> {
    pub(crate) fn flavor(&self) -> &'static str {
        "file"
    }
}

impl super::Preparable for Sink<Stale> {
    type Output = Sink<Running>;
    type Error = StartingError;

    async fn prepare(self) -> Result<Self::Output, Self::Error> {
        let output = tokio::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .write(true)
            .open(&self.state.path)
            .await?;
        Ok(Sink {
            state: Running { output },
        })
    }
}

impl super::Executable for Sink<Running> {
    async fn execute(mut self, mut receiver: Receiver) {
        tracing::info!("starting");
        while let Some(input) = receiver.recv().await {
            if let Err(err) = self.handle(input).await {
                tracing::error!("unable to persist received event: {err:?}");
            }
        }
        tracing::info!("stopping");
    }
}

impl Sink<Running> {
    async fn handle(&mut self, event: Event) -> std::io::Result<()> {
        let mut encoded = serde_json::to_vec(&event)?;
        encoded.push(b'\n');
        self.state.output.write_all(&encoded).await?;
        Ok(())
    }
}
