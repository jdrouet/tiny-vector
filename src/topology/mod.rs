use std::collections::HashMap;
use std::io::{Error, ErrorKind, Result as IOResult};
use std::path::Path;

use crate::components::name::ComponentName;

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error(transparent)]
    Source(#[from] crate::sources::BuildError),
    #[error(transparent)]
    Transform(#[from] crate::transforms::BuildError),
    #[error(transparent)]
    Sink(#[from] crate::sinks::BuildError),
    #[error("unable to find target {0}")]
    TargetNotFound(ComponentName),
}

#[derive(Debug, serde::Deserialize)]
struct ConfigWithInputs<Inner> {
    #[serde(flatten)]
    inner: Inner,
    inputs: Vec<ComponentName>,
}

#[derive(Debug, Default, serde::Deserialize)]
pub struct Config {
    sources: HashMap<ComponentName, crate::sources::Config>,
    transforms: HashMap<ComponentName, ConfigWithInputs<crate::transforms::Config>>,
    sinks: HashMap<ComponentName, ConfigWithInputs<crate::sinks::Config>>,
}

impl Config {
    pub fn from_path<P: AsRef<Path>>(path: P) -> IOResult<Self> {
        let file = std::fs::read_to_string(path)?;
        toml::de::from_str(&file).map_err(|error| Error::new(ErrorKind::InvalidData, error))
    }

    pub fn build(self) -> Result<Topology, BuildError> {
        let mut sources = HashMap::with_capacity(self.sources.len());
        let mut transforms = HashMap::with_capacity(self.transforms.len());
        let mut sinks = HashMap::with_capacity(self.sinks.len());

        for (name, ConfigWithInputs { inner, inputs }) in self.sinks.into_iter() {
            sinks.insert(name, inner.build()?);
        }

        for (name, ConfigWithInputs { inner, inputs }) in self.transforms.into_iter() {
            transforms.insert(name, inner.build()?);
        }

        for (name, inner) in self.sources.into_iter() {
            sources.insert(name, inner.build()?);
        }

        Ok(Topology {
            sources,
            transforms,
            sinks,
        })
    }
}

pub struct Topology {
    sources: HashMap<ComponentName, crate::sources::Source>,
    transforms: HashMap<ComponentName, crate::transforms::Transform>,
    sinks: HashMap<ComponentName, crate::sinks::Sink>,
}

impl Topology {
    pub(crate) async fn run(self) -> Instance {
        let mut sources = HashMap::with_capacity(self.sources.len());
        let mut transforms = HashMap::with_capacity(self.transforms.len());
        let mut sinks = HashMap::with_capacity(self.sinks.len());

        for (name, sink) in self.sinks.into_iter() {
            let handler = sink.run(&name).await;
            sinks.insert(name, handler);
        }
        for (name, transform) in self.transforms.into_iter() {
            let handler = transform.run(&name).await;
            transforms.insert(name, handler);
        }
        for (name, source) in self.sources.into_iter() {
            let handler = source.run(&name).await;
            sources.insert(name, handler);
        }

        Instance {
            sources,
            transforms,
            sinks,
        }
    }
}

pub(crate) struct Instance {
    sources: HashMap<ComponentName, tokio::task::JoinHandle<()>>,
    transforms: HashMap<ComponentName, tokio::task::JoinHandle<()>>,
    sinks: HashMap<ComponentName, tokio::task::JoinHandle<()>>,
}

impl Instance {
    pub async fn wait(self) {
        for (name, handler) in self
            .sources
            .into_iter()
            .chain(self.transforms.into_iter())
            .chain(self.sinks.into_iter())
        {
            if let Err(err) = handler.await {
                eprintln!("something went wront while waiting for {name:?}: {err:?}");
            }
        }
    }
}
