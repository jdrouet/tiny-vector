use std::path::PathBuf;

use tokio::io::AsyncWriteExt;
use tracing::Instrument;

use crate::event::Event;
use crate::prelude::Receiver;

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Config {
    path: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("unable to open file")]
    UnableToOpenFile(
        #[from]
        #[source]
        std::io::Error,
    ),
}

impl Config {
    pub async fn build(self) -> Result<Sink, BuildError> {
        let output = tokio::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .write(true)
            .open(&self.path)
            .await?;
        Ok(Sink { output })
    }
}

pub struct Sink {
    output: tokio::fs::File,
}

impl Sink {
    pub(crate) fn flavor(&self) -> &'static str {
        "file"
    }
}

impl Sink {
    async fn handle(&mut self, event: Event) -> std::io::Result<()> {
        let mut encoded = serde_json::to_vec(&event)?;
        encoded.push(b'\n');
        self.output.write_all(&encoded).await?;
        Ok(())
    }

    async fn execute(mut self, mut receiver: Receiver) {
        tracing::info!("starting");
        while let Some(input) = receiver.recv().await {
            if let Err(err) = self.handle(input).await {
                tracing::error!("unable to persist received event: {err:?}");
            }
        }
        tracing::info!("stopping");
    }

    pub async fn run(self, span: tracing::Span, receiver: Receiver) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move { self.execute(receiver).instrument(span).await })
    }
}
