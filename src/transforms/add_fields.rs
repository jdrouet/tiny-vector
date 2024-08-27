use indexmap::IndexMap;
use tracing::Instrument;

use crate::event::Event;
use crate::prelude::StringOrEnv;

#[derive(Debug, thiserror::Error)]
pub enum BuildError {}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct Config {
    fields: IndexMap<String, StringOrEnv>,
}

impl Config {
    pub fn build(
        self,
        incoming: crate::prelude::Sender,
    ) -> Result<(Transform, crate::prelude::Sender), BuildError> {
        let (sender, receiver) = crate::prelude::create_channel(1000);
        Ok((
            Transform {
                fields: self
                    .fields
                    .into_iter()
                    .filter_map(|(name, value)| value.into_string().map(|v| (name, v)))
                    .collect(),
                receiver,
                sender: incoming,
            },
            sender,
        ))
    }
}

pub struct Transform {
    fields: IndexMap<String, String>,
    receiver: crate::prelude::Receiver,
    sender: crate::prelude::Sender,
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

    async fn execute(mut self) {
        tracing::info!("starting");
        while let Some(event) = self.receiver.recv().await {
            if let Err(err) = self.sender.try_send(self.handle(event)) {
                tracing::error!("unable to send generated log: {err:?}");
                break;
            }
        }
        tracing::info!("stopping");
    }
    pub async fn run(self, name: &str) -> tokio::task::JoinHandle<()> {
        let span =
            tracing::info_span!("component", name, kind = "transform", flavor = "add_fields");
        tokio::spawn(async move { self.execute().instrument(span).await })
    }
}
