use std::collections::HashMap;

use tokio::sync::mpsc::error::SendError;

use super::output::NamedOutput;
use crate::event::{CowStr, Event};
use crate::prelude::Sender;

#[derive(Clone, Debug, Default)]
pub struct Collector {
    default: Option<Sender>,
    others: HashMap<CowStr, Sender>,
}

impl Collector {
    pub fn add_output(&mut self, named: NamedOutput, sender: Sender) {
        match named {
            NamedOutput::Default => {
                self.default = Some(sender);
            }
            NamedOutput::Named(inner) => {
                self.others.insert(inner, sender);
            }
        }
    }

    pub fn senders(&self) -> impl Iterator<Item = &Sender> {
        self.default.iter().chain(self.others.values())
    }

    #[cfg(test)]
    pub(crate) fn with_output(mut self, named: NamedOutput, sender: Sender) -> Self {
        self.add_output(named, sender);
        self
    }

    pub async fn send_default(&self, event: Event) -> Result<(), SendError<Event>> {
        match self.default {
            Some(ref inner) => inner.send(event).await,
            None => {
                tracing::trace!("no default output, discarding event");
                Ok(())
            }
        }
    }

    pub async fn send_named(
        &self,
        output: &NamedOutput,
        event: Event,
    ) -> Result<(), SendError<Event>> {
        match output {
            NamedOutput::Default => self.send_default(event).await?,
            NamedOutput::Named(inner) => match self.others.get(inner.as_ref()) {
                Some(inner) => inner.send(event).await?,
                None => tracing::trace!("no {inner:?} output, discarding event"),
            },
        };
        Ok(())
    }

    pub async fn send_all(&self, event: Event) -> Result<(), SendError<Event>> {
        for sender in self.senders() {
            sender.send(event.clone()).await?;
        }
        Ok(())
    }
}
