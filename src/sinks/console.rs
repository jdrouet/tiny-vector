use tracing::Instrument;

use crate::prelude::Receiver;

#[derive(Clone, Debug, Default, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Config {}

#[derive(Debug, thiserror::Error)]
pub struct BuildError;

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unable to build component")
    }
}

impl Config {
    pub fn build(self) -> Result<Sink, BuildError> {
        Ok(Sink)
    }
}

pub struct Sink;

impl Sink {
    pub(crate) fn flavor(&self) -> &'static str {
        "console"
    }
}

impl Sink {
    async fn execute(self, mut receiver: Receiver) {
        tracing::info!("starting");
        while let Some(input) = receiver.recv().await {
            println!("{input:?}");
        }
        tracing::info!("stopping");
    }

    pub async fn run(self, span: tracing::Span, receiver: Receiver) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move { self.execute(receiver).instrument(span).await })
    }
}
