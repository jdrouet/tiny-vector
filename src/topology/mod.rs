use std::collections::HashMap;
use std::io::{Error, ErrorKind, Result as IOResult};
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error(transparent)]
    Source(#[from] crate::sources::BuildError),
    #[error(transparent)]
    Transform(#[from] crate::transforms::BuildError),
    #[error(transparent)]
    Sink(#[from] crate::sinks::BuildError),
    #[error("unable to find target {0}")]
    TargetNotFound(String),
}

#[derive(Debug, serde::Deserialize)]
struct ConfigWithInputs<Inner> {
    #[serde(flatten)]
    inner: Inner,
    inputs: Vec<String>,
}

#[derive(Debug, Default, serde::Deserialize)]
pub struct Config {
    sources: HashMap<String, crate::sources::Config>,
    transforms: HashMap<String, ConfigWithInputs<crate::transforms::Config>>,
    sinks: HashMap<String, ConfigWithInputs<crate::sinks::Config>>,
}

impl Config {
    pub fn from_path<P: AsRef<Path>>(path: P) -> IOResult<Self> {
        let file = std::fs::read_to_string(path)?;
        toml::de::from_str(&file).map_err(|error| Error::new(ErrorKind::InvalidData, error))
    }

    pub fn build(self) -> Result<Topology, BuildError> {
        let mut sources = HashMap::with_capacity(self.sources.len());
        let mut targets = HashMap::with_capacity(self.sources.len());
        let mut transforms = HashMap::with_capacity(self.transforms.len());
        let mut sinks = HashMap::with_capacity(self.sinks.len());

        for (name, ConfigWithInputs { inner, inputs }) in self.sinks.into_iter() {
            let (sink, sender) = inner.build()?;
            for input in inputs {
                targets.insert(input, sender.clone());
            }
            sinks.insert(name, sink);
        }

        for (name, ConfigWithInputs { inner, inputs }) in self.transforms.into_iter() {
            if let Some(target) = targets.remove(&name) {
                let (transform, sender) = inner.build(target)?;
                for input in inputs {
                    targets.insert(input, sender.clone());
                }
                transforms.insert(name, transform);
            } else {
                return Err(BuildError::TargetNotFound(name));
            }
        }

        for (name, inner) in self.sources.into_iter() {
            if let Some(target) = targets.remove(&name) {
                let source = inner.build(target)?;
                sources.insert(name, source);
            } else {
                return Err(BuildError::TargetNotFound(name));
            }
        }

        Ok(Topology {
            sources,
            transforms,
            sinks,
        })
    }
}

pub struct Topology {
    sources: HashMap<String, crate::sources::Source>,
    transforms: HashMap<String, crate::transforms::Transform>,
    sinks: HashMap<String, crate::sinks::Sink>,
}

impl Topology {
    pub async fn run(self) -> Instance {
        let mut sources = HashMap::with_capacity(self.sources.len());
        let mut transforms = HashMap::with_capacity(self.transforms.len());
        let mut sinks = HashMap::with_capacity(self.sinks.len());

        for (name, sink) in self.sinks.into_iter() {
            let handler = sink.run(name.as_str()).await;
            sinks.insert(name, handler);
        }
        for (name, transform) in self.transforms.into_iter() {
            let handler = transform.run(name.as_str()).await;
            transforms.insert(name, handler);
        }
        for (name, source) in self.sources.into_iter() {
            let handler = source.run(name.as_str()).await;
            sources.insert(name, handler);
        }

        Instance {
            sources,
            transforms,
            sinks,
        }
    }
}

pub struct Instance {
    sources: HashMap<String, tokio::task::JoinHandle<()>>,
    transforms: HashMap<String, tokio::task::JoinHandle<()>>,
    sinks: HashMap<String, tokio::task::JoinHandle<()>>,
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
