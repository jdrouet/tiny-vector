use std::collections::HashSet;

use indexmap::IndexMap;
use tokio::sync::mpsc::error::SendError;
use tracing::Instrument;

use super::condition::Condition;
use crate::components::collector::Collector;
use crate::components::name::ComponentName;
use crate::components::output::NamedOutput;
use crate::event::Event;
use crate::prelude::Receiver;

#[derive(Debug, thiserror::Error)]
pub enum BuildError {}

#[derive(Clone, Debug, serde::Deserialize)]
#[cfg_attr(test, derive(Default))]
pub struct Config {
    routes: IndexMap<NamedOutput, Condition>,
}

impl Config {
    pub fn outputs(&self) -> HashSet<NamedOutput> {
        HashSet::from_iter([NamedOutput::Default])
    }

    pub fn build(self) -> Result<Transform, BuildError> {
        Ok(Transform {
            routes: self
                .routes
                .into_iter()
                .map(|(name, condition)| (condition, name))
                .collect(),
        })
    }
}

pub struct Transform {
    routes: Vec<(Condition, NamedOutput)>,
}

impl Transform {
    async fn handle(&self, collector: &Collector, event: Event) -> Result<(), SendError<Event>> {
        for (condition, output) in self.routes.iter() {
            if condition.evaluate(&event) {
                return collector.send_named(output, event).await;
            }
        }
        collector.send_default(event).await
    }

    async fn execute(self, mut receiver: Receiver, collector: Collector) {
        tracing::info!("starting");
        while let Some(event) = receiver.recv().await {
            if let Err(err) = self.handle(&collector, event).await {
                tracing::error!("unable to route event: {err:?}");
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
            flavor = "route"
        );
        tokio::spawn(async move { self.execute(receiver, collector).instrument(span).await })
    }
}
