use tracing::Instrument;

use crate::components::name::ComponentName;

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
    pub fn build(self) -> Result<(Sink, crate::prelude::Sender), BuildError> {
        let (sender, receiver) = crate::prelude::create_channel(1000);
        Ok((Sink { receiver }, sender))
    }
}

pub struct Sink {
    receiver: crate::prelude::Receiver,
}

impl Sink {
    async fn execute(mut self) {
        tracing::info!("starting");
        while let Some(input) = self.receiver.recv().await {
            drop(input);
        }
        tracing::info!("stopping");
    }

    pub async fn run(self, name: &ComponentName) -> tokio::task::JoinHandle<()> {
        let span = tracing::info_span!(
            "component",
            name = name.as_ref(),
            kind = "sink",
            flavor = "black_hole"
        );
        tokio::spawn(async move { self.execute().instrument(span).await })
    }
}
