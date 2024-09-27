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

#[derive(Debug, thiserror::Error)]
pub enum StartingError {}

pub struct Sink;

impl Sink {
    pub(crate) fn flavor(&self) -> &'static str {
        "console"
    }
}

impl super::Preparable for Sink {
    type Output = Sink;
    type Error = StartingError;

    async fn prepare(self) -> Result<Self::Output, Self::Error> {
        Ok(self)
    }
}

impl super::Executable for Sink {
    async fn execute(self, mut receiver: Receiver) {
        tracing::info!("starting");
        while let Some(input) = receiver.recv().await {
            println!("{input:?}");
        }
        tracing::info!("stopping");
    }
}
