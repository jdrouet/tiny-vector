use indexmap::IndexMap;
use tracing::Instrument;

use crate::components::collector::Collector;
use crate::components::name::ComponentName;
use crate::components::output::ComponentWithOutputs;
use crate::event::Event;
use crate::prelude::{Receiver, StringOrEnv};

#[derive(Debug, thiserror::Error)]
pub enum BuildError {}

#[derive(Clone, Debug, serde::Deserialize)]
#[cfg_attr(test, derive(Default))]
pub struct Config {
    fields: IndexMap<String, StringOrEnv>,
}

impl ComponentWithOutputs for Config {}

impl Config {
    pub fn build(self) -> Result<Transform, BuildError> {
        Ok(Transform {
            fields: self
                .fields
                .into_iter()
                .filter_map(|(name, value)| value.into_string().map(|v| (name, v)))
                .collect(),
        })
    }
}

pub struct Transform {
    fields: IndexMap<String, String>,
}

impl Transform {
    fn handle(&self, event: Event) -> Event {
        match event {
            Event::Log(mut inner) => {
                for (name, value) in self.fields.iter() {
                    inner.add_attribute(name.clone(), value.clone());
                }
                Event::Log(inner)
            }
            Event::Metric(mut inner) => {
                for (name, value) in self.fields.iter() {
                    inner.add_tag(name.clone(), value.clone());
                }
                Event::Metric(inner)
            }
        }
    }

    async fn execute(self, mut receiver: Receiver, collector: Collector) {
        tracing::info!("starting");
        while let Some(event) = receiver.recv().await {
            if let Err(err) = collector.send_default(self.handle(event)).await {
                tracing::error!("unable to send generated log: {err:?}");
                break;
            }
        }
        tracing::info!("stopping");
    }
    pub async fn run(
        self,
        name: &ComponentName,
        receiver: Receiver,
        collector: Collector,
    ) -> tokio::task::JoinHandle<()> {
        let span = tracing::info_span!(
            "component",
            name = name.as_ref(),
            kind = "transform",
            flavor = "add_fields"
        );
        tokio::spawn(async move { self.execute(receiver, collector).instrument(span).await })
    }
}
