use indexmap::IndexSet;
use tracing::Instrument;

use crate::components::collector::Collector;
use crate::components::name::ComponentName;
use crate::event::Event;
use crate::prelude::Receiver;

#[derive(Debug, thiserror::Error)]
pub enum BuildError {}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct Config {
    fields: IndexSet<String>,
}

impl Config {
    pub fn build(self) -> Result<Transform, BuildError> {
        Ok(Transform {
            fields: self.fields,
        })
    }
}

pub struct Transform {
    fields: IndexSet<String>,
}

impl Transform {
    fn handle(&self, event: Event) -> Event {
        match event {
            Event::Log(mut inner) => {
                inner
                    .attributes
                    .retain(|key, _| !self.fields.contains(key.as_ref()));
                Event::Log(inner)
            }
            Event::Metric(mut inner) => {
                inner
                    .header
                    .tags
                    .retain(|key, _| !self.fields.contains(key.as_ref()));
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
            flavor = "remove_fields"
        );
        tokio::spawn(async move { self.execute(receiver, collector).instrument(span).await })
    }
}
