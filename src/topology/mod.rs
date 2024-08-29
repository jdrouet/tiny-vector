use std::collections::{HashMap, HashSet};
use std::io::{Error, ErrorKind, Result as IOResult};
use std::path::Path;

use crate::components::collector::Collector;
use crate::components::name::ComponentName;
use crate::components::output::{ComponentOutput, NamedOutput};
use crate::prelude::{create_channel, Receiver};
use crate::sinks::Sink;
use crate::sources::Source;
use crate::transforms::Transform;

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
    inputs: HashSet<ComponentOutput>,
}

struct OuterSink {
    inner: Sink,
    inputs: HashSet<ComponentOutput>,
}

struct OuterTransform {
    inner: Transform,
    outputs: HashSet<NamedOutput>,
    inputs: HashSet<ComponentOutput>,
}

struct OuterSource {
    inner: Source,
    outputs: HashSet<NamedOutput>,
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
            sinks.insert(
                name,
                OuterSink {
                    inner: inner.build()?,
                    inputs,
                },
            );
        }

        for (name, ConfigWithInputs { inner, inputs }) in self.transforms.into_iter() {
            transforms.insert(
                name,
                OuterTransform {
                    outputs: inner.outputs(),
                    inner: inner.build()?,
                    inputs,
                },
            );
        }

        for (name, inner) in self.sources.into_iter() {
            sources.insert(
                name,
                OuterSource {
                    outputs: inner.outputs(),
                    inner: inner.build()?,
                },
            );
        }

        Ok(Topology {
            sources,
            transforms,
            sinks,
        })
    }
}

pub struct Topology {
    sources: HashMap<ComponentName, OuterSource>,
    transforms: HashMap<ComponentName, OuterTransform>,
    sinks: HashMap<ComponentName, OuterSink>,
}

impl Topology {
    fn prepare_wiring(
        &self,
    ) -> (
        HashMap<ComponentName, Collector>,
        HashMap<ComponentName, Receiver>,
    ) {
        let mut receivers = HashMap::new();
        let mut collectors = HashMap::<ComponentName, Collector>::new();
        for (name, sink) in self.sinks.iter() {
            let (sender, receiver) = create_channel(1000);
            receivers.insert(name.clone(), receiver);
            for input in sink.inputs.iter() {
                let collector = collectors.entry(input.name.clone()).or_default();
                collector.add_output(input.output.clone(), sender.clone());
            }
        }
        for (name, transform) in self.transforms.iter() {
            let (sender, receiver) = create_channel(1000);
            receivers.insert(name.clone(), receiver);
            for input in transform.inputs.iter() {
                let collector = collectors.entry(input.name.clone()).or_default();
                collector.add_output(input.output.clone(), sender.clone());
            }
        }
        (collectors, receivers)
    }

    pub(crate) async fn run(self) -> Instance {
        let (mut collectors, mut receivers) = self.prepare_wiring();

        let mut sources = HashMap::with_capacity(self.sources.len());
        let mut transforms = HashMap::with_capacity(self.transforms.len());
        let mut sinks = HashMap::with_capacity(self.sinks.len());

        for (name, sink) in self.sinks.into_iter() {
            let receiver = receivers.remove(&name).expect("receiver for sink");
            let handler = sink.inner.run(&name, receiver).await;
            sinks.insert(name, handler);
        }
        for (name, transform) in self.transforms.into_iter() {
            let receiver = receivers.remove(&name).expect("receiver for sink");
            let collector = collectors.remove(&name).unwrap_or_default();
            let handler = transform.inner.run(&name, receiver, collector).await;
            transforms.insert(name, handler);
        }
        for (name, source) in self.sources.into_iter() {
            let collector = collectors.remove(&name).unwrap_or_default();
            let handler = source.inner.run(&name, collector).await;
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
