use tokio::sync::mpsc::error::SendError;

use crate::event::Event;
use crate::prelude::Sender;

#[derive(Clone, Debug)]
pub struct Collector {
    default: Sender,
}

impl Collector {
    pub async fn send_default(&self, event: Event) -> Result<(), SendError<Event>> {
        self.default.send(event).await
    }
}
