use std::io::{Error, ErrorKind, Result as IOResult};
use std::{collections::HashMap, path::Path};

#[derive(Debug)]
pub enum BuildError {
    Source(crate::sources::BuildError),
    Sink(crate::sinks::BuildError),
    TargetNotFound(String),
}

impl From<crate::sources::BuildError> for BuildError {
    fn from(value: crate::sources::BuildError) -> Self {
        Self::Source(value)
    }
}

impl From<crate::sinks::BuildError> for BuildError {
    fn from(value: crate::sinks::BuildError) -> Self {
        Self::Sink(value)
    }
}

#[derive(Debug, serde::Deserialize)]
struct ConfigWithInputs {
    #[serde(flatten)]
    inner: crate::sinks::Config,
    inputs: Vec<String>,
}

#[derive(Debug, Default, serde::Deserialize)]
pub struct Config {
    sources: HashMap<String, crate::sources::Config>,
    sinks: HashMap<String, ConfigWithInputs>,
}

impl Config {
    pub fn from_path<P: AsRef<Path>>(path: P) -> IOResult<Self> {
        let file = std::fs::read_to_string(path)?;
        toml::de::from_str(&file).map_err(|error| Error::new(ErrorKind::InvalidData, error))
    }

    pub fn build(self) -> Result<Topology, BuildError> {
        let mut sources = HashMap::with_capacity(self.sources.len());
        let mut targets = HashMap::with_capacity(self.sources.len());
        let mut sinks = HashMap::with_capacity(self.sinks.len());

        for (name, ConfigWithInputs { inner, inputs }) in self.sinks.into_iter() {
            let (sink, sender) = inner.build()?;
            for input in inputs {
                targets.insert(input, sender.clone());
            }
            sinks.insert(name, sink);
        }

        for (name, inner) in self.sources.into_iter() {
            if let Some(target) = targets.remove(&name) {
                let source = inner.build(target)?;
                sources.insert(name, source);
            } else {
                return Err(BuildError::TargetNotFound(name));
            }
        }

        Ok(Topology { sources, sinks })
    }
}

pub struct Topology {
    sources: HashMap<String, crate::sources::Source>,
    sinks: HashMap<String, crate::sinks::Sink>,
}

impl Topology {
    pub fn run(self) -> Instance {
        let mut sources = HashMap::with_capacity(self.sources.len());
        let mut sinks = HashMap::with_capacity(self.sinks.len());

        for (name, sink) in self.sinks.into_iter() {
            let handler = sink.run(name.as_str());
            sinks.insert(name, handler);
        }
        for (name, source) in self.sources.into_iter() {
            let handler = source.run(name.as_str());
            sources.insert(name, handler);
        }

        Instance { sources, sinks }
    }
}

pub struct Instance {
    sources: HashMap<String, tokio::task::JoinHandle<()>>,
    sinks: HashMap<String, tokio::task::JoinHandle<()>>,
}

impl Instance {
    pub async fn wait(self) {
        for (name, handler) in self.sources.into_iter().chain(self.sinks.into_iter()) {
            if let Err(err) = handler.await {
                eprintln!("something went wront while waiting for {name:?}: {err:?}");
            }
        }
    }
}
