use indexmap::IndexSet;
use tracing::Instrument;

use crate::components::name::ComponentName;
use crate::event::Event;

#[derive(Debug, thiserror::Error)]
pub enum BuildError {}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct Config {
    fields: IndexSet<String>,
}

impl Config {
    pub fn build(
        self,
        incoming: crate::prelude::Sender,
    ) -> Result<(Transform, crate::prelude::Sender), BuildError> {
        let (sender, receiver) = crate::prelude::create_channel(1000);
        Ok((
            Transform {
                fields: self.fields,
                receiver,
                sender: incoming,
            },
            sender,
        ))
    }
}

pub struct Transform {
    fields: IndexSet<String>,
    receiver: crate::prelude::Receiver,
    sender: crate::prelude::Sender,
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
    pub async fn run(self, name: &ComponentName) -> tokio::task::JoinHandle<()> {
        let span = tracing::info_span!(
            "component",
            name = name.as_ref(),
            kind = "transform",
            flavor = "remove_fields"
        );
        tokio::spawn(async move { self.execute().instrument(span).await })
    }
}
