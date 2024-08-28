use tracing::Instrument;

use crate::components::name::ComponentName;
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
    async fn execute(self, mut receiver: Receiver) {
        tracing::info!("starting");
        while let Some(input) = receiver.recv().await {
            drop(input);
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
            flavor = "black_hole"
        );
        tokio::spawn(async move { self.execute(receiver).instrument(span).await })
    }
}
