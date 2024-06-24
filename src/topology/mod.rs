use std::collections::HashMap;

#[derive(Debug)]
struct ConfigWithInputs {
    inner: crate::sinks::Config,
    inputs: Vec<String>,
}

#[derive(Debug, Default)]
pub struct Config {
    sources: HashMap<String, crate::sources::Config>,
    sinks: HashMap<String, ConfigWithInputs>,
}

impl Config {
    pub fn with_source(mut self, name: impl Into<String>, source: crate::sources::Config) -> Self {
        self.sources.insert(name.into(), source);
        self
    }

    pub fn with_sink<I: Into<String>>(
        mut self,
        name: impl Into<String>,
        inputs: impl IntoIterator<Item = I>,
        sink: crate::sinks::Config,
    ) -> Self {
        self.sinks.insert(
            name.into(),
            ConfigWithInputs {
                inner: sink,
                inputs: inputs.into_iter().map(Into::into).collect(),
            },
        );
        self
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
