use std::collections::HashMap;

#[derive(Debug)]
struct ConfigWithTarget {
    inner: crate::sources::Config,
    target: String,
}

#[derive(Debug, Default)]
pub struct Config {
    sources: HashMap<String, ConfigWithTarget>,
    sinks: HashMap<String, crate::sinks::Config>,
}

impl Config {
    pub fn with_source(
        mut self,
        name: impl Into<String>,
        target: impl Into<String>,
        source: crate::sources::Config,
    ) -> Self {
        self.sources.insert(
            name.into(),
            ConfigWithTarget {
                target: target.into(),
                inner: source,
            },
        );
        self
    }

    pub fn with_sink(mut self, name: impl Into<String>, sink: crate::sinks::Config) -> Self {
        self.sinks.insert(name.into(), sink);
        self
    }

    pub fn build(self) -> Topology {
        let mut sources = HashMap::with_capacity(self.sources.len());
        let mut targets = HashMap::with_capacity(self.sources.len());

        for (name, ConfigWithTarget { inner, target }) in self.sources.into_iter() {
            let (source, rx) = inner.build();
            sources.insert(name.clone(), source);
            targets.insert(target, rx);
        }

        let mut sinks = HashMap::with_capacity(self.sinks.len());

        for (name, inner) in self.sinks.into_iter() {
            if let Some(rx) = targets.remove(&name) {
                let sink = inner.build(rx);
                sinks.insert(name, sink);
            } else {
                println!("no reader found for sink {name:?}, ignoring...");
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
