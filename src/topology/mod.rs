use std::collections::{HashMap, HashSet};
use std::io::{Error, ErrorKind, Result as IOResult};
use std::path::Path;

use crate::components::collector::Collector;
use crate::components::name::ComponentName;
use crate::components::output::ComponentOutput;
use crate::prelude::{create_channel, Receiver};
use crate::sinks::Sink;
use crate::sources::Source;
use crate::transforms::Transform;

pub mod validation;

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error(transparent)]
    Source(#[from] crate::sources::BuildError),
    #[error(transparent)]
    Transform(#[from] crate::transforms::BuildError),
    #[error(transparent)]
    Sink(#[from] crate::sinks::BuildError),
    #[error("the configuration is invalid")]
    Validation(Vec<self::validation::ValidationError>),
}

#[derive(Debug, serde::Deserialize)]
struct WithInputs<Inner> {
    #[serde(flatten)]
    inner: Inner,
    inputs: HashSet<ComponentOutput<'static>>,
}

#[derive(Debug, Default, serde::Deserialize)]
pub struct Config {
    sources: HashMap<ComponentName, crate::sources::Config>,
    transforms: HashMap<ComponentName, WithInputs<crate::transforms::Config>>,
    sinks: HashMap<ComponentName, WithInputs<crate::sinks::Config>>,
}

impl Config {
    pub fn from_path<P: AsRef<Path>>(path: P) -> IOResult<Self> {
        let file = std::fs::read_to_string(path)?;
        toml::de::from_str(&file).map_err(|error| Error::new(ErrorKind::InvalidData, error))
    }

    fn compile(self) -> Result<Topology, BuildError> {
        let mut sources = HashMap::with_capacity(self.sources.len());
        let mut transforms = HashMap::with_capacity(self.transforms.len());
        let mut sinks = HashMap::with_capacity(self.sinks.len());

        for (name, WithInputs { inner, inputs }) in self.sinks.into_iter() {
            sinks.insert(
                name,
                WithInputs {
                    inner: inner.build()?,
                    inputs,
                },
            );
        }

        for (name, WithInputs { inner, inputs }) in self.transforms.into_iter() {
            transforms.insert(
                name,
                WithInputs {
                    inner: inner.build()?,
                    inputs,
                },
            );
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

    pub fn build(self) -> Result<Topology, BuildError> {
        self.validate()
            .map_err(BuildError::Validation)
            .and_then(|c| c.compile())
    }
}

pub struct Topology {
    sources: HashMap<ComponentName, Source>,
    transforms: HashMap<ComponentName, WithInputs<Transform>>,
    sinks: HashMap<ComponentName, WithInputs<Sink>>,
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
                let collector = collectors.entry(input.to_owned_name()).or_default();
                collector.add_output(input.to_owned_output(), sender.clone());
            }
        }
        for (name, transform) in self.transforms.iter() {
            let (sender, receiver) = create_channel(1000);
            receivers.insert(name.clone(), receiver);
            for input in transform.inputs.iter() {
                let collector = collectors.entry(input.to_owned_name()).or_default();
                collector.add_output(input.to_owned_output(), sender.clone());
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
            let receiver = receivers.remove(&name).expect("receiver for transform");
            let collector = collectors.remove(&name).unwrap_or_default();
            let handler = transform.inner.run(&name, receiver, collector).await;
            transforms.insert(name, handler);
        }
        for (name, source) in self.sources.into_iter() {
            let collector = collectors.remove(&name).unwrap_or_default();
            let handler = source.run(&name, collector).await;
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

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::collections::HashSet;

    use super::{Config, WithInputs};
    use crate::components::name::ComponentName;
    use crate::components::output::{ComponentOutput, NamedOutput};

    #[test]
    fn component_output_shouldnt_be_used_more_than_once() {
        let mut config = Config::default();
        config.sources.insert(
            ComponentName::from("foo"),
            crate::sources::Config::RandomLogs(crate::sources::random_logs::Config::default()),
        );
        config.sinks.insert(
            ComponentName::from("bar"),
            WithInputs {
                inner: crate::sinks::Config::BlackHole(crate::sinks::black_hole::Config::default()),
                inputs: HashSet::from_iter([ComponentOutput {
                    name: Cow::Owned(ComponentName::from("foo")),
                    output: Cow::Owned(NamedOutput::Default),
                }]),
            },
        );
        config.sinks.insert(
            ComponentName::from("baz"),
            WithInputs {
                inner: crate::sinks::Config::BlackHole(crate::sinks::black_hole::Config::default()),
                inputs: HashSet::from_iter([ComponentOutput {
                    name: Cow::Owned(ComponentName::from("foo")),
                    output: Cow::Owned(NamedOutput::Default),
                }]),
            },
        );
        let errors = config.validate().unwrap_err();
        assert!(!errors.is_empty());
    }
}
