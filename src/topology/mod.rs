use std::{collections::HashMap, path::Path};

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
    pub fn from_path<P: AsRef<Path>>(path: P) -> Self {
        let file = std::fs::read_to_string(path).expect("unable to open configuration file");
        toml::de::from_str(&file).expect("unable to parse configuration file")
    }

    pub fn build(self) -> Topology {
        let mut sources = HashMap::with_capacity(self.sources.len());
        let mut targets = HashMap::with_capacity(self.sources.len());
        let mut sinks = HashMap::with_capacity(self.sinks.len());

        for (name, ConfigWithInputs { inner, inputs }) in self.sinks.into_iter() {
            let (sink, sender) = inner.build();
            for input in inputs {
                targets.insert(input, sender.clone());
            }
            sinks.insert(name, sink);
        }

        for (name, inner) in self.sources.into_iter() {
            if let Some(target) = targets.remove(&name) {
                let source = inner.build(target);
                sources.insert(name, source);
            } else {
                eprintln!("no target found for source {name:?}, ignoring");
            }
        }

        Topology { sources, sinks }
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
            sinks.insert(name, sink.run());
        }
        for (name, source) in self.sources.into_iter() {
            sources.insert(name, source.run());
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
